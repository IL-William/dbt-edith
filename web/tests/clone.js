// Clone into my schema (0058): what each node is copied as, from where, to
// where, and the script that comes out. Invented names throughout.
// Run from the repository root: jsc web/tests/clone.js
var src = read('web/app.js');
eval(src.slice(src.indexOf('function locationRows'), src.indexOf('function catalogLocation')));
var block = src.slice(src.indexOf('function cloneShape'), src.indexOf('async function openClonePlan'));
eval(block);

function check(label, got, want) {
  var g = JSON.stringify(got), w = JSON.stringify(want);
  print((g === w ? 'PASS  ' : 'FAIL  ') + label + (g === w ? '' : '\n        expected ' + w + '\n        got      ' + g));
}

/* A node as the clone route sends it: built into a developer sandbox, its
   database read from SHOP_DB, resolved against .env.uat. */
function node(name, materialized, extra) {
  var n = {
    id: 'model.shop.' + name, name: name, kind: 'model', materialized: materialized, why: 'parent',
    relation: materialized === 'ephemeral' ? '' : 'DEV_DB.dbt_someone.' + name,
    location: {
      written: { database: "{{ env_var('SHOP_DB') }}", schema: 'marts', alias: '' },
      parsed: { database: 'ANALYTICS_QA', schema: 'marts', alias: '' },
      built: { database: 'DEV_DB', schema: 'dbt_someone', alias: name },
      envs: {
        '.env.uat': {
          place: { database: 'ANALYTICS_UAT', schema: 'marts', alias: '' },
          status: { database: { kind: 'env', vars: ['SHOP_DB'] }, schema: { kind: 'literal' }, alias: { kind: 'literal' } },
        },
        '.env.qa': {
          place: { database: '', schema: 'marts', alias: '' },
          status: { database: { kind: 'missing', vars: ['SHOP_DB'] }, schema: { kind: 'literal' }, alias: { kind: 'literal' } },
        },
      },
    },
  };
  for (var k in extra || {}) n[k] = extra[k];
  return n;
}
var into = cloneInto('DEV_DB.dbt_someone');

print('--- what each materialization is copied as ---');
check('a table is cloned', cloneShape('table', 'clone'), { action: 'clone', drop: '', reason: '' });
['incremental', 'snapshot', 'seed'].forEach(function (m) {
  check(m + ' is cloned', cloneShape(m, 'clone').action, 'clone');
  check(m + ' is copied in CTAS mode', cloneShape(m, 'ctas').action, 'ctas');
});
check('a view is always copied, its view dropped first', cloneShape('view', 'clone'), { action: 'ctas', drop: 'view', reason: '' });
check('a dynamic table drops its own kind', cloneShape('dynamic_table', 'clone').drop, 'dynamic table');
check('a materialized view drops its own kind', cloneShape('materialized_view', 'ctas').drop, 'materialized view');
check('a semantic view is skipped', cloneShape('semantic_view', 'clone').action, 'skip');
check('an unknown materialization is copied, nothing dropped', cloneShape('custom_mat', 'clone'), { action: 'ctas', drop: '', reason: '' });

print('\n--- the statements, to the character ---');
check('clone', cloneStatement('A.b.c', 'D.e.c', cloneShape('table', 'clone')),
  'create or replace transient table D.e.c\n    clone A.b.c;');
check('create as select', cloneStatement('A.b.c', 'D.e.c', cloneShape('table', 'ctas')),
  'create or replace transient table D.e.c as\nselect * from A.b.c;');
check('a view', cloneStatement('A.b.v', 'D.e.v', cloneShape('view', 'clone')),
  'drop view if exists D.e.v;\ncreate or replace transient table D.e.v as\nselect * from A.b.v;');

print('\n--- where a node is copied from ---');
var orders = node('orders', 'table');
check('built: where the manifest built it', cloneSource(orders, ''), { text: 'DEV_DB.dbt_someone.orders', reason: '' });
check('with a file: the Location table\'s own relation', cloneSource(orders, '.env.uat'),
  resolvedRelation(locationRows(orders.location, '.env.uat'), orders.relation, '.env.uat'));
check('which is the environment\'s', cloneSource(orders, '.env.uat').text, 'ANALYTICS_UAT.marts.orders');
check('a missing variable says so', cloneSource(orders, '.env.qa'), { text: '', reason: 'SHOP_DB is not defined in .env.qa' });
check('a file it was not resolved against: no fallback to the manifest', cloneSource(orders, '.env.stg'),
  { text: '', reason: 'orders was not resolved against .env.stg' });
var unset = node('pkg_model', 'table');
unset.location.envs['.env.uat'].status.schema = { kind: 'parsed' };
unset.location.envs['.env.uat'].place.schema = 'dbt_qa';
check('a schema the config leaves to the target is not guessed', cloneSource(unset, '.env.uat').text, '');
unset.location.envs['.env.qa'].status.schema = { kind: 'parsed' };
check('a missing variable is said before that', cloneSource(unset, '.env.qa').reason, 'SHOP_DB is not defined in .env.qa');
var quoted = node('Orders', 'table', { relation: 'DEV_DB.dbt_someone."Orders"' });
quoted.location.built.alias = 'Orders';
check('a quoted alias stays quoted', cloneSource(quoted, '.env.uat').text, 'ANALYTICS_UAT.marts."Orders"');
check('an ephemeral has nothing to copy from', cloneSource(node('int_x', 'ephemeral'), ''), { text: '', reason: 'int_x has no relation in the warehouse' });

print('\n--- the destination ---');
check('database.schema', into, { text: 'DEV_DB.dbt_someone', reason: '' });
check('a quoted schema with a dot is two names', cloneInto(' DEV_DB."dbt.someone" ').text, 'DEV_DB."dbt.someone"');
check('three names is not a destination', cloneInto('DEV_DB.dbt_someone.orders').text, '');
check('one name is not either', cloneInto('DEV_DB').reason, 'the destination is a database and a schema: database.schema');
check('a control character is refused', cloneInto('DEV_DB.dbt\u0007').text, '');
check('empty asks for one', cloneInto('  ').reason, 'type the database.schema to clone into');
var profile = { profile: 'shop', target: 'dev', target_from: 'profile', database: 'DEV_DB', schema: 'dbt_someone' };
check('what was typed last wins', cloneIntoDefault({ remembered: 'X.y', profile: profile }), { text: 'X.y', from: 'what you typed last time' });
check('then the profile', cloneIntoDefault({ profile: profile }), { text: 'DEV_DB.dbt_someone', from: 'profile shop, target dev' });
check('naming what picked the target', cloneIntoDefault({ profile: { profile: 'shop', target: 'ci', target_from: 'DBT_TARGET', database: 'CI', schema: 's' } }).from,
  'profile shop, target ci, picked by DBT_TARGET');
check('then the sandbox, to confirm', cloneIntoDefault({ profile: { why: 'target dev sets no schema' }, sandbox: 'dev_db.dbt_someone' }),
  { text: 'dev_db.dbt_someone', from: 'where this manifest built nearly every model, since target dev sets no schema: confirm it' });
check('else why not', cloneIntoDefault({ profile: { why: 'profiles.yml has no profile named shop' } }), { text: '', from: 'profiles.yml has no profile named shop' });

print('\n--- the rows ---');
var plan = {
  name: 'fct_orders', ephemeral: ['int_orders'], other: [], sources: [],
  items: [
    node('stg_orders', 'view', { why: 'via', via: 'int_orders' }),
    node('Payments', 'table', { relation: 'DEV_DB.dbt_someone."Payments"' }),
    node('metrics', 'semantic_view'),
    node('old', 'table', { disabled: true }),
    node('fct_orders', 'incremental', { why: 'model' }),
  ],
};
plan.items[1].location.built.alias = 'Payments';
var rows = cloneRows(plan, '.env.uat', into, 'clone');
check('one row per node, in the plan\'s order', rows.map(function (r) { return r.name; }), ['stg_orders', 'Payments', 'metrics', 'old', 'fct_orders']);
check('the copy keeps the source\'s name, quotes and all', rows[1].target, 'DEV_DB.dbt_someone."Payments"');
check('actions', rows.map(function (r) { return r.action; }), ['ctas', 'clone', 'skip', 'refused', 'clone']);
check('a disabled node says why', rows[3].reason, 'old is disabled, so it is not built');
var built = cloneRows(plan, '', into, 'clone');
check('built into the destination itself: refused, not copied onto itself', built[1].reason, 'it already is where the copy would go');
check('case alone does not make another object', cloneSameObject('dev_db.DBT_SOMEONE.x', 'DEV_DB.dbt_someone.X'), true);
check('a quoted name keeps its case', cloneSameObject('DEV_DB.s."x"', 'DEV_DB.s.X'), false);
var none = cloneRows(plan, '.env.uat', cloneInto(''), 'clone');
check('no destination, no statement, the source still shown', [none[0].source, none[0].sql], ['ANALYTICS_UAT.marts.stg_orders', '']);

print('\n--- the script ---');
var head = cloneHead(plan, '.env.uat', into);
check('the head says what and where', head[0], '-- fct_orders and its direct parents, from where .env.uat puts them, into DEV_DB.dbt_someone.');
check('or where they were built', cloneHead(plan, '', cloneInto(''))[0],
  '-- fct_orders and its direct parents, from where this manifest built them, into a destination still to type.');
check('the head says who runs it', head[1], '-- Written by dbt-edith, which runs none of it: read it, then run it yourself.');
var all = new Set(rows.filter(function (r) { return r.sql; }).map(function (r) { return r.id; }));
check('parents first, the model last, every gap said', cloneScript(rows, all, head, ''), head.join('\n') + '\n\n'
  + 'drop view if exists DEV_DB.dbt_someone.stg_orders;\ncreate or replace transient table DEV_DB.dbt_someone.stg_orders as\nselect * from ANALYTICS_UAT.marts.stg_orders;\n\n'
  + 'create or replace transient table DEV_DB.dbt_someone."Payments"\n    clone ANALYTICS_UAT.marts."Payments";\n\n'
  + '-- skipped metrics: a semantic view holds no rows to copy\n\n'
  + '-- skipped old: old is disabled, so it is not built\n\n'
  + 'create or replace transient table DEV_DB.dbt_someone.fct_orders\n    clone ANALYTICS_UAT.marts.fct_orders;\n');
var some = new Set(['model.shop.fct_orders']);
check('an unticked node is left out', cloneScript(rows, some, [], '').indexOf('stg_orders'), -1);
check('nothing ticked says so', cloneScript(rows, new Set(), [], '').trim().split('\n').pop(), '-- nothing to run');
check('no destination, no statements', cloneScript(rows, all, ['-- h'], 'type the database.schema to clone into'),
  '-- h\n\n-- type the database.schema to clone into\n');

print('\n--- the notes ---');
var notes = cloneNotes(plan, rows, { file: '.env.uat', stale: '', sandbox: '', dotenvTarget: 'uat', profileTarget: 'dev' });
var has = function (start) { return notes.some(function (n) { return n.indexOf(start) === 0; }); };
check('an ephemeral walked through', has('int_orders is ephemeral, with no object of its own'), true);
check('a view becomes a table', has('A view is copied as a table'), true);
check('an incremental keeps its rows', has('An incremental model is copied'), true);
check('no snapshot, no snapshot note', has('A snapshot'), false);
check('the macros are not run', has('Locations are the config resolved with .env.uat, before the generate_*_name macros'), true);
check('a .env target dbt does not read', has('Your .env sets DBT_TARGET=uat'), true);
check('the same target, no hint', cloneNotes(plan, rows, { file: '', dotenvTarget: 'dev', profileTarget: 'dev' }).some(function (n) { return n.indexOf('Your .env') === 0; }), false);
var changed = cloneRows(plan, '.env.uat', into, 'clone');
changed[4].changed = true;
check('a changed file asks for a parse, over the badge', cloneNotes(plan, changed, { stale: 'Your saved edits are not in it yet' })[0],
  'fct_orders changed after this manifest was written: run dbt parse, or the parents listed may not be the ones it reads now.');
check('else the badge speaks', cloneNotes(plan, rows, { stale: 'Your saved edits are not in it yet' })[0],
  'Your saved edits are not in it yet: the parents listed are the ones this manifest knew.');
var eph = { name: 'int_orders', ephemeral: ['int_orders'], other: [], sources: [], items: [node('stg_orders', 'view')] };
check('an ephemeral focus copies only what it reads', cloneNotes(eph, cloneRows(eph, '', into, 'clone'), {})[0],
  'int_orders is ephemeral, so nothing of it is copied: only what it reads.');
check('a sandbox manifest built into the destination', cloneNotes(plan, built, { file: '', sandbox: 'dev_db.dbt_someone' })
  .indexOf('This manifest was built into the destination: pick an environment to copy from.') >= 0, true);

print('\n--- house style ---');
check('no em or en dash in anything written for the user', /[\u2013\u2014]/.test(block), false);
