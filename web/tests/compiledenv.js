// The Compiled and Run tabs for another environment: each relation the SQL
// holds swapped for the one the Location table resolves for the chosen .env
// file, kept and marked when it cannot be, and the words under the bar.
// Fixtures mirror the /api/compiled shape with invented names.
// Run from the repository root: jsc web/tests/compiledenv.js
var src = read('web/app.js');
eval(src.slice(src.indexOf('function locationRows'), src.indexOf('function buildsIds')));

function check(label, got, want) {
  var g = JSON.stringify(got), w = JSON.stringify(want);
  print((g === w ? 'PASS  ' : 'FAIL  ') + label + (g === w ? '' : '\n        expected ' + w + '\n        got      ' + g));
}

var QA = '.env.qa', UAT = '.env.uat';
function cell(kind, vars) { return { kind: kind, vars: vars || [], branch: false }; }
function place(database, schema, alias) { return { database: database, schema: schema, alias: alias }; }
// A node written as env_var('DBT_DB') for its database, built where the
// manifest says; `envs` maps a file to the database it resolves to.
function rel(name, relation, built, envs, over) {
  var o = over || {};
  var e = {};
  Object.keys(envs).forEach(function (file) {
    e[file] = {
      place: place(envs[file], built.schema, built.alias),
      status: { database: cell('env', ['DBT_DB']), schema: cell('literal'), alias: cell('literal') },
    };
  });
  return {
    id: 'model.shop.' + name, name: name, relation: relation, materialized: o.materialized || 'table', own: !!o.own,
    location: {
      written: place("{{ env_var('DBT_DB') }}", built.schema, built.alias),
      parsed: place(o.parsed || built.database, built.schema, built.alias),
      built: built,
      envs: e,
    },
  };
}
var orders = rel('orders', 'raw.crm.orders', place('raw', 'crm', 'orders'), { '.env.qa': 'RAW_QA', '.env.uat': 'RAW_UAT' });
var items = rel('orders_items', 'raw.crm.orders_items', place('raw', 'crm', 'orders_items'), { '.env.qa': 'RAW_QA', '.env.uat': 'RAW_UAT' });

print('--- a relation is swapped on identifier boundaries ---');
var sql = 'select * from raw.crm.orders o join raw.crm.orders_items i on 1=1 where x = \'raw.crm.orders\'';
var r = envSql(sql, [orders, items], QA);
check('ORDERS and ORDERS_ITEMS are told apart', r.text,
  'select * from RAW_QA.crm.orders o join RAW_QA.crm.orders_items i on 1=1 where x = \'RAW_QA.crm.orders\'');
check('both relations counted once each', r.counts, { swapped: 2, kept: 0, same: 0 });
check('every swap points at the new text in the new string', r.swaps.map(function (w) { return r.text.slice(w.start, w.end); }),
  ['RAW_QA.crm.orders', 'RAW_QA.crm.orders_items', 'RAW_QA.crm.orders']);
check('and remembers what was there', r.swaps.map(function (w) { return w.was; }),
  ['raw.crm.orders', 'raw.crm.orders_items', 'raw.crm.orders']);
check('a name inside a longer path is not a relation', envSql('from other.raw.crm.orders, raw.crm.orders_old', [orders], QA).text,
  'from other.raw.crm.orders, raw.crm.orders_old');
check('a column of the relation is still reached', envSql('select raw.crm.orders.id from raw.crm.orders', [orders], QA).text,
  'select RAW_QA.crm.orders.id from RAW_QA.crm.orders');

print('\n--- quoted relations stay quoted ---');
var q = rel('payments', '"RAW"."CRM"."PAYMENTS"', place('RAW', 'CRM', 'PAYMENTS'), { '.env.qa': 'RAW_QA' });
check('each part keeps its quotes', envSql('from "RAW"."CRM"."PAYMENTS" p', [q], QA).text, 'from "RAW_QA"."CRM"."PAYMENTS" p');

print('\n--- the environments never chain ---');
var a = rel('a', 'db_a.s.t', place('db_a', 's', 't'), { '.env.qa': 'db_b' });
var b = rel('b', 'db_b.s.t', place('db_b', 's', 't'), { '.env.qa': 'db_a' });
check('two relations that trade places trade once', envSql('from db_a.s.t join db_b.s.t', [a, b], QA).text,
  'from db_b.s.t join db_a.s.t');

print('\n--- the node\'s own relation, as a run names it ---');
var self = rel('orders', 'analytics.marts.orders', place('analytics', 'marts', 'orders'), { '.env.qa': 'ANALYTICS_QA' }, { own: true });
var run = 'create or replace transient table analytics.marts.orders__dbt_tmp as (select 1);\n'
  + 'alter table analytics.marts.orders rename to analytics.marts.orders__dbt_backup;';
var rs = envSql(run, [self], QA);
check('the tmp and backup names follow', rs.text,
  'create or replace transient table ANALYTICS_QA.marts.orders__dbt_tmp as (select 1);\n'
  + 'alter table ANALYTICS_QA.marts.orders rename to ANALYTICS_QA.marts.orders__dbt_backup;');
check('and count as one relation', rs.counts, { swapped: 1, kept: 0, same: 0 });
var selfQ = rel('orders', '"A"."M"."ORDERS"', place('A', 'M', 'ORDERS'), { '.env.qa': 'A_QA' }, { own: true });
check('a quoted name takes its suffix inside the quotes', envSql('create table "A"."M"."ORDERS__dbt_tmp"', [selfQ], QA).text,
  'create table "A_QA"."M"."ORDERS__dbt_tmp"');
check('the node itself is never reported unseen', envSql('select 1', [self], QA).unseen, []);

print('\n--- a relation kept as compiled says why ---');
var missing = rel('m1', 'raw.crm.m1', place('raw', 'crm', 'm1'), { '.env.qa': '' });
missing.location.envs[QA].status.database = cell('missing', ['DBT_DB']);
var holder = rel('m2', 'raw.crm.m2', place('raw', 'crm', 'm2'), { '.env.qa': 'x' });
holder.location.envs[QA].status.database = cell('placeholder', ['DBT_DB']);
var profile = rel('m3', 'raw.crm.m3', place('raw', 'crm', 'm3'), { '.env.qa': '' });
profile.location.envs[QA].status.database = cell('parsed');
var moved = rel('m4', 'raw.dbt_x.m4', place('raw', 'dbt_x', 'm4'), { '.env.qa': 'RAW_QA' });
moved.location.parsed = place('raw', 'crm', 'm4');
var k = envSql('a raw.crm.m1 b raw.crm.m2 c raw.crm.m3 d raw.dbt_x.m4', [missing, holder, profile, moved], QA);
check('the text is left as dbt compiled it', k.text, 'a raw.crm.m1 b raw.crm.m2 c raw.crm.m3 d raw.dbt_x.m4');
check('four kept, none swapped', k.counts, { swapped: 0, kept: 4, same: 0 });
check('a missing variable', k.kept[0].reason, 'DBT_DB is not defined in .env.qa');
check('a placeholder', k.kept[1].reason, 'database is a placeholder in .env.qa, not a real name');
check('a database left to the profile', k.kept[2].reason.indexOf('database is not set in the config') === 0, true);
check('a model dbt built elsewhere than configured', k.kept[3].reason,
  'dbt built it elsewhere than configured, through a macro that is not evaluated');
check('the marks cover the relation', k.kept.map(function (x) { return k.text.slice(x.start, x.end); }),
  ['raw.crm.m1', 'raw.crm.m2', 'raw.crm.m3', 'raw.dbt_x.m4']);
check('a file the node was not resolved against keeps it', envSql('from raw.crm.orders', [orders], '.env.stg').kept[0].reason,
  'not resolved against .env.stg');

print('\n--- a relation already where the file puts it ---');
var same = rel('s', 'RAW_QA.crm.s', place('RAW_QA', 'crm', 's'), { '.env.qa': 'RAW_QA' });
var sm = envSql('from RAW_QA.crm.s', [same], QA);
check('is not a swap', sm.counts, { swapped: 0, kept: 0, same: 1 });
check('and leaves no mark', sm.swaps, []);

print('\n--- a parent the SQL never names ---');
check('is reported by name', envSql('select 1', [self, orders], QA).unseen, ['orders']);
check('a relation with none is not looked for', envSql('select 1', [{ name: 'e', relation: '', own: false, location: {} }], QA).unseen, []);

print('\n--- no environment picked ---');
var none = envSql(sql, [orders, items], '');
check('the text is what dbt left', none.text, sql);
check('with nothing marked', [none.swaps, none.kept, none.unseen], [[], [], []]);

print('\n--- the words under the bar ---');
var words = artifactEnvWords(envSql(sql, [orders, items], QA), 'QA', QA, []);
check('what was swapped', words.head, 'Shown for QA (.env.qa): 2 relations swapped');
check('nothing beyond the relations to say', [words.differ, words.findings], ['', []]);
check('one relation', artifactEnvWords(envSql('from raw.crm.orders', [orders], QA), 'QA', QA, []).head,
  'Shown for QA (.env.qa): 1 relation swapped');
check('kept and not found', artifactEnvWords(envSql('from raw.crm.m1', [missing, orders], QA), 'QA', QA, []).head,
  'Shown for QA (.env.qa): 1 kept as compiled, 1 not found in this SQL');
check('nothing changed', artifactEnvWords(envSql('from RAW_QA.crm.s', [same], QA), 'QA', QA, []).head,
  'Shown for QA (.env.qa): every relation reads the same as compiled');
check('nothing to swap', artifactEnvWords(envSql('select 1', [], QA), 'QA', QA, []).head,
  'Shown for QA (.env.qa): no relation in this SQL to swap');
var reads = artifactEnvWords(envSql('select 1', [], QA), 'QA', QA, [
  { kind: 'target', detail: 'name', line: 4 },
  { macro_id: 'macro.shop.stamp', macro_name: 'stamp', file: 'macros/stamp.sql', kind: 'target', detail: 'name', line: 19 },
  { macro_id: 'macro.pkg.x', macro_name: 'x', kind: 'env_var', detail: '', line: 2 },
  { macro_id: 'macro.shop.generate_schema_name', macro_name: 'generate_schema_name', file: 'macros/g.sql', kind: 'placing', line: 3 },
  { kind: 'incremental', line: 7 },
  { macro_id: 'macro.shop.q', macro_name: 'q', file: 'macros/q.sql', kind: 'warehouse', detail: 'run_query', line: 5 },
]);
check('the model\'s own SQL is named', reads.findings[0].text, 'this model, line 4, reads target.name');
check('a macro by name and line', reads.findings[1].text, 'stamp, line 19, reads target.name');
check('a macro that opens carries its file and line', reads.findings[1].macro, { file: 'macros/stamp.sql', name: 'stamp', line: 19 });
check('a model\'s own line opens nothing', reads.findings[0].macro, null);
check('a macro outside the project opens nothing', reads.findings[2].macro, null);
check('a withheld variable is not named', reads.findings[2].text, 'x, line 2, reads an environment variable');
check('a placing macro', reads.findings[3].text,
  'generate_schema_name, line 3, follows the target to decide where a model builds, which is not evaluated');
check('is_incremental', reads.findings[4].text,
  'this model, line 7, branches on is_incremental(), which depends on the table existing there');
check('a warehouse call', reads.findings[5].text, 'q, line 5, asks the warehouse with run_query()');
check('a placing macro that reads the target through another says which, with no line', artifactEnvWords(
  envSql('select 1', [], QA), 'QA', QA,
  [{ macro_id: 'macro.shop.generate_schema_name', macro_name: 'generate_schema_name', file: 'macros/g.sql', kind: 'placing', through: 'schema_part', line: 0 }]).findings[0],
  { text: 'generate_schema_name, follows the target to decide where a model builds, which is not evaluated, through schema_part',
    macro: { file: 'macros/g.sql', name: 'generate_schema_name', line: 1 } });
check('the list is introduced once', reads.differ, 'May differ beyond its relations (6)');

print('\n--- a file dbt wrote on Windows ---');
var crlf = artifactContent({ content: 'select 1\r\nfrom x\r\n\r\nfrom raw.crm.orders o\r\n', truncated: false });
check('breaks are one character each', crlf, 'select 1\nfrom x\n\nfrom raw.crm.orders o\n');
var cr = envSql(crlf, [orders], QA);
// CodeMirror's posFromIndex: the line is the number of breaks before the index.
function posOf(text, i) { var before = text.slice(0, i).split('\n'); return { line: before.length - 1, ch: before[before.length - 1].length }; }
check('a relation on line 4 is marked on exactly its characters', [posOf(cr.text, cr.swaps[0].start), posOf(cr.text, cr.swaps[0].end)],
  [{ line: 3, ch: 5 }, { line: 3, ch: 22 }]);
check('and its text is the swapped relation', cr.text.slice(cr.swaps[0].start, cr.swaps[0].end), 'RAW_QA.crm.orders');
check('a truncated file keeps its note on lines of its own', artifactContent({ content: 'a\r\nb', truncated: true }),
  'a\nb\n\n-- truncated by dbt-edith\n');

print('\n--- two entries on one table ---');
var twin = rel('orders_model', 'raw.crm.orders', place('raw', 'crm', 'orders'), { '.env.qa': 'RAW_QA' });
var tw = envSql('from raw.crm.orders', [orders, twin], QA);
check('the table is swapped once', tw.text, 'from RAW_QA.crm.orders');
check('and neither entry is reported unseen', tw.unseen, []);
