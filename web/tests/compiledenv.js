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
check('and the node each stands for, for the hover (0164)', r.swaps.map(function (w) { return w.name; }),
  ['orders', 'orders_items', 'orders']);
check('and the call each was written as, for the hover', r.swaps.map(function (w) { return w.ref; }),
  ["ref('orders')", "ref('orders_items')", "ref('orders')"]);
print('\n--- the call a swapped relation stands for ---');
check('a model is a ref', relationRef({ id: 'model.shop.stg_customers', name: 'stg_customers' }), "ref('stg_customers')");
check('a seed and a snapshot are refs too', [relationRef({ id: 'seed.shop.countries', name: 'countries' }), relationRef({ id: 'snapshot.shop.snap_orders', name: 'snap_orders' })],
  ["ref('countries')", "ref('snap_orders')"]);
check('a source is source(name, table)', relationRef({ id: 'source.shop.jaffle_shop.raw_orders', name: 'jaffle_shop.raw_orders' }), "source('jaffle_shop', 'raw_orders')");
check('a versioned model says its version', [relationRef({ id: 'model.shop.orders.v2', name: 'orders.v2' }), relationRef({ id: 'model.shop.orders.v1.5', name: 'orders.v1.5' })],
  ["ref('orders', v=2)", "ref('orders', v=1.5)"]);
check('a version that is a word is quoted', relationRef({ id: 'model.shop.orders.vbeta', name: 'orders.vbeta' }), "ref('orders', v='beta')");
check('a source named with a dot of its own falls back to the node\'s name when the server sent no source name', relationRef({ id: 'source.shop.a.b.c', name: 'a.b.c' }), "source('a.b.c')");
check('the source name the server sends splits a table name that holds a dot', relationRef({ id: 'source.shop.a.b.c', name: 'a.b.c', source_name: 'a' }), "source('a', 'b.c')");
check('and a source name that holds one', relationRef({ id: 'source.shop.a.b.c', name: 'a.b.c', source_name: 'a.b' }), "source('a.b', 'c')");
check('and a plain source is the same either way', relationRef({ id: 'source.shop.jaffle_shop.raw_orders', name: 'jaffle_shop.raw_orders', source_name: 'jaffle_shop' }), "source('jaffle_shop', 'raw_orders')");
check('the hover is the call and the environment, then what was there',
  swapTitle({ ref: "ref('stg_customers')", was: '"jaffle_shop"."main"."stg_customers"' }, 'PROD'),
  'ref(\'stg_customers\'), for PROD\nwas "jaffle_shop"."main"."stg_customers" as compiled');
print('\n--- a relation is swapped on identifier boundaries (continued) ---');
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
check('the hover names {{ this }} for the plain form and says which suffix for the others', rs.swaps.map(function (w) { return w.ref; }),
  ['{{ this }} with the __dbt_tmp suffix', '{{ this }}', '{{ this }} with the __dbt_backup suffix']);
check('and the hover then reads without a stray comma', swapTitle({ ref: rs.swaps[0].ref, was: rs.swaps[0].was }, 'QA').split('\n')[0],
  '{{ this }} with the __dbt_tmp suffix, for QA');
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

print('\n--- a sandbox manifest, where every model sits away from its config ---');
var SB = 'dev_db.dbt_someone';
// Parsed as raw.crm, built into the sandbox, as a developer target does.
var boxed = rel('orders', 'dev_db.dbt_someone.orders', place('dev_db', 'dbt_someone', 'orders'), { '.env.qa': 'RAW_QA' }, { parsed: 'raw' });
boxed.location.parsed = place('raw', 'crm', 'orders');
boxed.location.envs[QA].place = place('RAW_QA', 'crm', 'orders');
var sb = envSql('from dev_db.dbt_someone.orders', [boxed], QA, SB);
check('a parent built into the sandbox is swapped to the file\'s relation', sb.text, 'from RAW_QA.crm.orders');
check('and counted as swapped', sb.counts, { swapped: 1, kept: 0, same: 0 });
var nb = envSql('from dev_db.dbt_someone.orders', [boxed], QA, '');
check('with no sandbox it stays kept, for the same reason', [nb.text, nb.kept[0].reason],
  ['from dev_db.dbt_someone.orders', 'dbt built it elsewhere than configured, through a macro that is not evaluated']);
var other = rel('items', 'dev_db.dbt_other.items', place('dev_db', 'dbt_other', 'items'), { '.env.qa': 'RAW_QA' });
other.location.parsed = place('raw', 'crm', 'items');
check('a parent built in another schema of a sandbox manifest stays kept', envSql('from dev_db.dbt_other.items', [other], QA, SB).counts,
  { swapped: 0, kept: 1, same: 0 });
var ownBoxed = rel('orders', 'dev_db.dbt_someone.orders', place('dev_db', 'dbt_someone', 'orders'), { '.env.qa': 'RAW_QA' }, { own: true });
ownBoxed.location.parsed = place('raw', 'crm', 'orders');
ownBoxed.location.envs[QA].place = place('RAW_QA', 'crm', 'orders');
check('the node\'s own relation built into the sandbox swaps its __dbt_tmp form', envSql('create table dev_db.dbt_someone.orders__dbt_tmp', [ownBoxed], QA, SB).text,
  'create table RAW_QA.crm.orders__dbt_tmp');
var aliased = rel('orders', 'dev_db.dbt_someone.orders_v2', place('dev_db', 'dbt_someone', 'orders_v2'), { '.env.qa': 'RAW_QA' });
aliased.location.parsed = place('raw', 'crm', 'orders');
var al = envSql('from dev_db.dbt_someone.orders_v2', [aliased], QA, SB);
check('a model whose alias moved stays kept in a sandbox manifest, for the same reason', [al.text, al.kept[0].reason],
  ['from dev_db.dbt_someone.orders_v2', 'dbt built it elsewhere than configured, through a macro that is not evaluated']);
check('builtInSandbox compares lowercased', [builtInSandbox({ built: place('DEV_DB', 'DBT_SOMEONE', 'x') }, SB), builtInSandbox({ built: place('raw', 'crm', 'x') }, SB), builtInSandbox(boxed.location, '')],
  [true, false, false]);

print('\n--- a parent the SQL never names ---');
check('is reported by name', envSql('select 1', [self, orders], QA).unseen, ['orders']);
check('a relation with none is not looked for', envSql('select 1', [{ name: 'e', relation: '', own: false, location: {} }], QA).unseen, []);

print('\n--- no environment picked ---');
var none = envSql(sql, [orders, items], '');
check('the text is what dbt left', none.text, sql);
check('with nothing marked', [none.swaps, none.kept, none.unseen], [[], [], []]);

print('\n--- the note under the line ---');
var words = artifactEnvWords(envSql(sql, [orders, items], QA), 'QA', QA, []);
check('led by the environment\'s tag', words.tag, 'QA');
check('what was swapped, in bold', words.head, '2 relations swapped');
check('and where it was read from', words.rest, ', read from .env.qa.');
check('nothing beyond the relations to say', [words.differ, words.findings], ['', []]);
var one = artifactEnvWords(envSql('from raw.crm.orders', [orders], QA), 'QA', QA, []);
check('one relation', [one.head, one.rest], ['1 relation swapped', ', read from .env.qa.']);
var kn = artifactEnvWords(envSql('from raw.crm.m1 join raw.crm.orders', [missing, orders, items], QA), 'QA', QA, []);
check('a swap with one kept as compiled and one not found', [kn.head, kn.rest],
  ['1 relation swapped', ', read from .env.qa; 1 kept as compiled; 1 not found in this SQL.']);
var ko = artifactEnvWords(envSql('from raw.crm.m1', [missing, orders], QA), 'QA', QA, []);
check('only kept ones: nothing was swapped, and the rest is as before', [ko.head, ko.rest],
  ['No relation swapped', ', read from .env.qa; 1 kept as compiled; 1 not found in this SQL.']);
var sa = artifactEnvWords(envSql('from RAW_QA.crm.s', [same], QA), 'QA', QA, []);
check('nothing changed: today\'s words take the bold place', [sa.head, sa.rest],
  ['Every relation reads the same as compiled', ' (read from .env.qa).']);
var no = artifactEnvWords(envSql('select 1', [], QA), 'QA', QA, []);
check('nothing to swap', [no.head, no.rest], ['No relation in this SQL to swap', ' (read from .env.qa).']);
var reads = artifactEnvWords(envSql('select 1', [], QA), 'QA', QA, [
  { kind: 'target', detail: 'name', line: 4 },
  { macro_id: 'macro.shop.stamp', macro_name: 'stamp', file: 'macros/stamp.sql', kind: 'target', detail: 'name', line: 19 },
  { macro_id: 'macro.pkg.x', macro_name: 'x', kind: 'env_var', detail: '', line: 2 },
  { macro_id: 'macro.shop.generate_schema_name', macro_name: 'generate_schema_name', file: 'macros/g.sql', kind: 'placing', line: 3 },
  { kind: 'incremental', line: 7 },
  { macro_id: 'macro.shop.q', macro_name: 'q', file: 'macros/q.sql', kind: 'warehouse', detail: 'run_query', line: 5 },
]);
var all = reads.findings;
check('the model\'s own SQL is named', all[0].text, 'this model, line 4, reads target.name');
check('a macro by name and line', all[1].text, 'stamp, line 19, reads target.name');
check('a macro that opens carries its file and line', all[1].macro, { file: 'macros/stamp.sql', name: 'stamp', line: 19 });
check('a finding in a macro carries the macro\'s name apart, the model\'s own none', [all[1].name, all[2].name, all[0].name], ['stamp', 'x', undefined]);
check('a model\'s own line opens nothing', all[0].macro, null);
check('a macro outside the project opens nothing', all[2].macro, null);
check('a withheld variable is not named', all[2].text, 'x, line 2, reads an environment variable');
check('a placing macro', all[3].text,
  'generate_schema_name, line 3, follows the target to decide where a model builds, which is not evaluated');
check('is_incremental', all[4].text,
  'this model, line 7, branches on is_incremental(), which depends on the table existing there');
check('a warehouse call', all[5].text, 'q, line 5, asks the warehouse with run_query()');
check('a placing macro that reads the target through another says which, with no line', artifactEnvWords(
  envSql('select 1', [], QA), 'QA', QA,
  [{ macro_id: 'macro.shop.generate_schema_name', macro_name: 'generate_schema_name', file: 'macros/g.sql', kind: 'placing', through: 'schema_part', line: 0 }]).findings[0],
  { text: 'generate_schema_name, follows the target to decide where a model builds, which is not evaluated, through schema_part',
    name: 'generate_schema_name', through: 'schema_part',
    macro: { file: 'macros/g.sql', name: 'generate_schema_name', line: 1 } });
check('every finding is listed, and the fold says how many', [reads.findings.length, reads.differ], [6, 'May differ beyond its relations (6)']);

print('\n--- no file picked ---');
check('a sandbox manifest says where the SQL was written', artifactOwnWords('dev_db.dbt_someone'),
  { tag: '', head: 'As your own compile wrote it,', rest: ' into dev_db.dbt_someone, nothing swapped.', differ: '', findings: [] });
check('a manifest with no sandbox says whose target it is', artifactOwnWords(''),
  { tag: '', head: 'As dbt compiled it.', rest: ' The relations are the manifest\'s own, nothing swapped.', differ: '', findings: [] });

print('\n--- the picker ---');
check('the arrows step to the next, and Right and Down are the same', [radioStep(0, 4, 'ArrowRight'), radioStep(1, 4, 'ArrowDown')], [1, 2]);
check('and to the previous', [radioStep(2, 4, 'ArrowLeft'), radioStep(3, 4, 'ArrowUp')], [1, 2]);
check('past the last they wrap, as a radio group does', [radioStep(3, 4, 'ArrowRight'), radioStep(0, 4, 'ArrowLeft')], [0, 3]);
check('Home and End go to the ends', [radioStep(2, 4, 'Home'), radioStep(1, 4, 'End')], [0, 3]);
check('any other key is left alone', [radioStep(1, 4, 'a'), radioStep(1, 4, 'Tab'), radioStep(1, 4, 'Enter')], [-1, -1, -1]);

print('\n--- the row of buttons ---');
// S is the page's state; only what the row's helpers read is here.
var S = { envs: null, env: '' };
function envList(files, referenced) {
  return { referenced: referenced === undefined ? [{ var: 'DBT_DB', nodes: 3 }] : referenced,
    files: files.map(function (f) { return { file: f[0], name: f[1], hidden: !!f[2], target_mismatch: false, target: '' }; }) };
}
var listed = [['.env.qa', 'QA'], ['.env.uat', 'UAT']];
S.envs = envList(listed);
check('a file to pick draws the row', envRowShown(), true);
var keyed = envRowKey();
S.envs = envList(listed);
check('the same list again keeps the key', envRowKey(), keyed);
S.envs = envList([['.env.qa', 'Quality'], ['.env.uat', 'UAT']]);
check('a rename changes it', envRowKey() !== keyed, true);
S.envs = envList([['.env.qa', 'QA', true], ['.env.uat', 'UAT']]);
check('so does hiding a file', envRowKey() !== keyed, true);
S.envs = envList([['.env.qa', 'QA', true], ['.env.uat', 'UAT', true]]);
check('every file hidden draws no row', envRowShown(), false);
S.envs = envList([]);
check('and no file at all draws none', envRowShown(), false);
S.envs = envList(listed, []);
check('nor does a project no env var is read by', envRowShown(), false);
S.envs = null;
check('nor a list that failed to load', envRowShown(), false);

print('\n--- a stored choice that the list no longer backs ---');
S.env = '.env.qa';
S.envs = envList(listed);
check('is the choice while environments are in use', activeEnv(), '.env.qa');
S.envs = null;
check('falls back to the manifest when the list failed to load', activeEnv(), '');
S.envs = envList(listed, []);
check('and when no config reads an env var any more', activeEnv(), '');
S.env = '';

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

print('\n--- the picker and the note, drawn (0164) ---');
// The two builders with a page of just enough around them: elements that keep
// what they are given, so the group's roles, its keys and the note's lines can
// be read back.
function El(tag) {
  this.tag = tag; this.kids = []; this.attrs = {}; this.dataset = {}; this.on = {}; this.className = ''; this.textContent = ''; this.title = '';
}
El.prototype.append = function () { for (var i = 0; i < arguments.length; i++) this.kids.push(arguments[i]); };
El.prototype.setAttribute = function (k, v) { this.attrs[k] = v; };
El.prototype.addEventListener = function (k, f) { this.on[k] = f; };
El.prototype.text = function () {
  return this.textContent + this.kids.map(function (k) { return typeof k === 'string' ? k : k.text(); }).join('');
};
El.prototype.find = function (cls) {
  var out = [];
  (function walk(e) { if (typeof e === 'string') return; if ((' ' + e.className + ' ').indexOf(' ' + cls + ' ') >= 0) out.push(e); e.kids.forEach(walk); }(this));
  return out;
};
var document = { createElement: function (t) { return new El(t); } };
var picks = [], macros = [];
function selectEnv(file) { picks.push(file); }
function openMacro(t) { macros.push(t); }
function envDisplayName(file) { return file.replace('.env.', '').toUpperCase(); }
S = { env: '.env.qa', meta: { sandbox: '' } };
var menuOrder = [
  { label: 'DEV', sub: '.env.dev', tone: 'dev', current: false, title: '', file: '.env.dev' },
  { label: 'QA', sub: '.env.qa', tone: 'qa', current: true, title: '', file: '.env.qa' },
  { label: 'PROD ⚠', sub: '.env.prod', tone: 'prod', current: false, title: 'DBT_TARGET in .env.prod is x', file: '.env.prod' },
  { label: 'manifest', sub: 'as dbt parsed it', tone: 'manifest', current: false, title: 'the config', file: '' },
];
function envChoices() { return menuOrder; }
eval(src.slice(src.indexOf('/* An environment by its name, in its colour (envTone)'), src.indexOf('async function loadEnvs')));
eval(src.slice(src.indexOf('function artifactEnvPick'), src.indexOf('/* The file\'s own line in the bar.')));

var pickBox = artifactEnvPick('compiled');
var grp = pickBox.find('envseg')[0];
var pickLabel = pickBox.kids[0];
check('the visible label names a radio group, one id per tab', [pickLabel.textContent, grp.attrs.role, grp.attrs['aria-labelledby'], pickLabel.id, artifactEnvPick('run').kids[0].id],
  ['Relations for', 'radiogroup', 'compiled-env-label', 'compiled-env-label', 'run-env-label']);
check('and no aria-label beside it', 'aria-label' in grp.attrs, false);
var btns = grp.kids;
check('the files in the menu\'s order, the manifest\'s own last (0147)', btns.map(function (b) { return b.textContent; }), ['DEV', 'QA', 'PROD ⚠', 'manifest']);
check('each is a radio carrying its tone, and the picked one is checked', btns.map(function (b) { return [b.attrs.role, b.dataset.tone, b.attrs['aria-checked']].join(); }),
  ['radio,dev,false', 'radio,qa,true', 'radio,prod,false', 'radio,manifest,false']);
check('only the picked one is a tab stop', btns.map(function (b) { return b.tabIndex; }), [-1, 0, -1, -1]);
check('a file that does not match its name keeps its title', btns[2].title, 'DBT_TARGET in .env.prod is x');
btns[1].on.click();
check('a click on the picked one does nothing', picks, []);
btns[2].on.click();
check('a click on another picks its file', picks, ['.env.prod']);
picks = [];
function key(k) { var stopped = false; grp.on.keydown({ key: k, preventDefault: function () { stopped = true; } }); return stopped; }
check('Right picks the next in the order drawn, and the key is taken', [key('ArrowRight'), picks], [true, ['.env.prod']]);
picks = [];
check('Left picks the one before', [key('ArrowLeft'), picks], [true, ['.env.dev']]);
picks = [];
check('Home picks the first and End the manifest, which has no file', [key('Home'), key('End'), picks], [true, true, ['.env.dev', '']]);
picks = [];
check('a key it does not know is left to the page', [key('a'), picks], [false, []]);
menuOrder[3].current = true; menuOrder[1].current = false;
picks = [];
// A rebuilt group with the last one picked: Right wraps to the first.
pickBox = artifactEnvPick('run'); grp = pickBox.find('envseg')[0];
check('Right from the last wraps to the first', [key('ArrowRight'), picks], [true, ['.env.dev']]);

menuOrder.forEach(function (c) { c.current = false; });
check('with none current the first radio is the one tab stop, as in the settings', artifactEnvPick('compiled').find('envseg')[0].kids.map(function (b) { return b.tabIndex; }), [0, -1, -1, -1]);
menuOrder[1].current = true; menuOrder[3].current = false;

var noteShown = envSql('from raw.crm.orders', [orders], QA);
var noteReads = [];
for (var n = 1; n <= 5; n++) noteReads.push({ macro_id: 'macro.shop.m' + n, macro_name: 'm' + n, file: 'macros/m' + n + '.sql', kind: 'target', detail: 'name', line: n });
S.env = '.env.qa';
var note = artifactEnvNote(noteShown, noteReads, '.env.qa', 'qa');
check('the note takes the environment\'s tone', note.dataset.tone, 'qa');
check('the tag is the environment tag, in the note\'s tone', [note.find('envtag')[0].dataset.tone], ['qa']);
check('led by the tag, then the bold count, then the rest', [note.find('envtag')[0].textContent, note.kids[0].kids.filter(function (k) { return k.tag === 'b'; })[0].textContent, note.kids[0].kids[3]],
  ['QA', '1 relation swapped', ', read from .env.qa.']);
var fold = note.find('cdiffer')[0];
check('the findings sit in a fold under the sentence, with their number', [note.kids[1] === fold, fold.kids[0].tag, fold.kids[0].textContent],
  [true, 'summary', 'May differ beyond its relations (5)']);
check('the fold is open when drawn, on every paint', [fold.open, artifactEnvNote(noteShown, noteReads, '.env.qa', 'qa').find('cdiffer')[0].open], [true, true]);
var items = fold.kids[1].kids;
check('one line per finding, none left out', items.map(function (l) { return l.tag; }), ['li', 'li', 'li', 'li', 'li']);
check('each says the macro and its line', items.map(function (l) { return l.text(); }),
  ['m1, line 1, reads target.name', 'm2, line 2, reads target.name', 'm3, line 3, reads target.name', 'm4, line 4, reads target.name', 'm5, line 5, reads target.name']);
items[0].find('filelink')[0].on.click({ preventDefault: function () {} });
check('a finding in a macro opens the macro at its line', macros, [{ file: 'macros/m1.sql', name: 'm1', line: 1 }]);
var modelOnly = artifactEnvNote(noteShown, [{ kind: 'target', detail: 'name', line: 4 }], '.env.qa', 'qa').find('cdiffer')[0];
check('a finding in the model\'s own SQL is plain text', [modelOnly.kids[1].kids[0].find('filelink').length, modelOnly.kids[1].kids[0].textContent], [0, 'this model, line 4, reads target.name']);
var macroItem = items[0].kids;
check('a macro\'s name is drawn in mono, a link, and the rest of the line follows as text', [macroItem[0].tag, macroItem[0].className, macroItem[0].textContent, macroItem[1]],
  ['a', 'filelink mono', 'm1', ', line 1, reads target.name']);
var mixed = artifactEnvNote(noteShown, [
  { macro_id: 'macro.pkg.x', macro_name: 'x', kind: 'env_var', detail: '', line: 2 },
  { macro_id: 'macro.shop.generate_schema_name', macro_name: 'generate_schema_name', file: 'macros/g.sql', kind: 'placing', through: 'schema_part', line: 0 },
], '.env.qa', 'qa').find('cdiffer')[0].kids[1].kids;
check('a macro that cannot be opened is mono text and no link', [mixed[0].kids[0].tag, mixed[0].kids[0].className, mixed[0].find('filelink').length, mixed[0].text()],
  ['span', 'mono', 0, 'x, line 2, reads an environment variable']);
check('the macro it goes through is mono too, apart from the link', [mixed[1].kids.map(function (k) { return typeof k === 'string' ? k : k.className + ':' + k.textContent; })],
  [['filelink mono:generate_schema_name', ', follows the target to decide where a model builds, which is not evaluated, through ', 'mono:schema_part']]);
var plain = artifactEnvNote(noteShown, [], '', 'manifest');
check('with none picked there is no tag and no finding', [plain.find('envtag').length, plain.find('cdiffer').length, plain.dataset.tone], [0, 0, 'manifest']);
check('and the note says the relations are the manifest\'s own', plain.text(), 'As dbt compiled it. The relations are the manifest\'s own, nothing swapped.');
S.meta.sandbox = 'dev_db.dbt_someone';
check('on a sandbox manifest it says into where',
  artifactEnvNote(noteShown, [], '', 'manifest').text(), 'As your own compile wrote it, into dev_db.dbt_someone, nothing swapped.');

print('\n--- the tab paints the picker, the note and the marks as one (0164) ---');
// paintArtifact itself, with the page stubbed: the picker and the note are drawn
// only when envRowShown() says so, and every mark is made after the editor
// carries the environment's tone, which the marks read --env from.
S.artifactInfo = {};
S.artifactCm = {};
var heads = {}, timers = [], typed = [];
var $ = function (sel) { var k = sel.slice(1); heads[k] = heads[k] || new El('div'); return heads[k]; };
// The shim's textContent does not empty its children, so each paint starts from fresh elements.
function paint(keep) { heads = {}; paintArtifact('compiled', keep); return $('#compiled-head'); }
var sendToTerminal = function () { typed.push(Array.prototype.slice.call(arguments)); };
var setTimeout = function (f) { timers.push(f); };
El.prototype.appendChild = El.prototype.append;
var marks = [];
var CodeMirror = function (host) {
  var wrapper = new El('div');
  return {
    getWrapperElement: function () { return wrapper; },
    setValue: function () {}, getScrollInfo: function () { return { left: 0, top: 0 }; }, scrollTo: function () {}, refresh: function () {},
    getDoc: function () {
      return { posFromIndex: function (i) { return i; },
        markText: function (a, b, o) { marks.push([o.className, wrapper.dataset.tone]); } };
    },
  };
};
eval(src.slice(src.indexOf('function envTone('), src.indexOf('\n}\n', src.indexOf('function envTone(')) + 2));
eval(src.slice(src.indexOf('function paintArtifact('), src.indexOf('/* "Relations for"')));
eval(src.slice(src.indexOf('function artifactPath('), src.indexOf('/* What the Git panel')));
function held(over) {
  var info = { found: true, kind: 'compiled', rel: 'target/compiled/shop/models/x.sql', path: '/p/target/compiled/shop/models/x.sql',
    content: 'from raw.crm.orders join raw.crm.m1', relations: [orders, missing], env_reads: [], age_secs: 100, compiled_at: 1, stale: false };
  S.artifactInfo.compiled = { info: Object.assign(info, over || {}), name: 'x', isTest: false };
}
held();
S.env = '.env.qa';
S.envs = { referenced: ['DBT_DB'], files: [{ file: '.env.qa', hidden: false }] };
marks = [];
var drawn = paint(false);
check('with a file to pick the line has the picker and the note follows it', [drawn.find('cenv').length, drawn.find('cnote').length], [1, 1]);
check('a swap and a kept relation are both marked, each after the tone was set', marks, [['cm-env-swap', 'qa'], ['cm-env-kept', 'qa']]);
S.envs = { referenced: ['DBT_DB'], files: [{ file: '.env.qa', hidden: true }] };
marks = [];
drawn = paint(false);
check('with every file hidden neither is drawn', [drawn.find('cenv').length, drawn.find('cnote').length], [0, 0]);
S.envs = { referenced: [], files: [] };
drawn = paint(false);
check('and with no environment in use, none either', [drawn.find('cenv').length, drawn.find('cnote').length], [0, 0]);
S.envs = { referenced: ['DBT_DB'], files: [{ file: '.env.qa', hidden: false }] };
marks = [];
// The wrapper still says qa from the paint above: emptied, the check holds only if the repaint sets it again first.
S.artifactCm.compiled.getWrapperElement().dataset.tone = '';
paint(true);
check('a repaint of the same editor still sets the tone before it marks', marks, [['cm-env-swap', 'qa'], ['cm-env-kept', 'qa']]);

var css = read('web/app.css');
check('the editor reads --env from the tone it was given', css.indexOf('.compiled-body .CodeMirror { height: 100%; --env: var(--tone); }') >= 0, true);
