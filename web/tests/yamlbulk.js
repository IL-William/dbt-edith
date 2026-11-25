// Updating the YAML of a folder from Reports (0188): the tree and its counts, which models a folder's
// button takes, what is left out and why, the files the patches fall in, and what the card says.
// Fixtures are invented.
// Run from the repository root: jsc web/tests/yamlbulk.js
var src = read('web/app.js');
eval(src.slice(src.indexOf('function yamlIndent'), src.indexOf('/* ATX headings')));
eval(src.slice(src.indexOf('function yamlValueText'), src.indexOf('/* The project\'s `name:`')));

// The plan to its end, which the page runs between frames instead (yamlBulkDrive).
function yamlBulkPlan(syncs, reads, dirty, withBehind, cache) {
  var run = yamlBulkSteps(syncs, reads, dirty, withBehind, cache || new Map());
  var step = run.next();
  while (!step.done) step = run.next();
  return step.value;
}

function check(label, got, want) {
  var g = JSON.stringify(got), w = JSON.stringify(want);
  print((g === w ? 'PASS  ' : 'FAIL  ') + label + (g === w ? '' : '\n        expected ' + w + '\n        got      ' + g));
}

function model(name, folder, extra) {
  return Object.assign({ id: 'model.shop.' + name, name: name, file: folder + '/' + name + '.sql', yml: folder + '/schema.yml', status: 'disagrees',
    missing: [{ name: 'note', data_type: 'varchar', after: 'id' }] }, extra || {});
}
var models = [
  model('orders', 'models/marts/sales'),
  model('refunds', 'models/marts/sales'),
  model('customers', 'models/marts', { missing: undefined, extra: [{ name: 'old' }], status: 'disagrees' }),
  model('stg_orders', 'models/staging/shop', { status: 'untyped', missing: undefined, untyped: [{ name: 'id', to: 'number' }] }),
  model('stg_users', 'models/staging/shop', { status: 'agrees', missing: undefined }),
  model('marts_old', 'models/marts_old'),
  { id: 'model.shop.snap', name: 'snap', file: 'models/snap.sql', yml: '', status: 'not_checked', reason: 'it is not in catalog.json' },
  { id: 'model.shop.eph', name: 'eph', file: 'models/eph.sql', yml: '', status: 'not_checked', reason: 'it is ephemeral: it has no table' },
  { id: 'model.shop.snap2', name: 'snap2', file: 'models/snap2.sql', yml: '', status: 'not_checked', reason: 'it is not in catalog.json' },
];

// The tree and its counts.
check('the counts by what was found, types only left out of what would be updated',
  yamlBulkCounts(models, false), { disagrees: 4, untyped: 1, agrees: 1, not_checked: 3, update: 4 });
check('and in with the switch', yamlBulkCounts(models, true).update, 5);
check('a model written and waiting is not counted', yamlBulkCounts(models, false, new Set(['model.shop.orders'])).update, 3);
var tree = yamlBulkTree(models, false);
check('the tree holds the models that differ, any depth, and counts what would be updated', [tree.update, tree.shown, tree.folders.map((f) => [f.name, f.update, f.shown])],
  [4, 5, [['models', 4, 5]]]);
var under = tree.folders[0];
check('folders and models are by name', [under.folders.map((f) => f.path), under.models.map((m) => m.name)], [['models/marts', 'models/marts_old', 'models/staging'], []]);
var marts = under.folders[0];
check('a folder says how many it would update and how many it lists', [marts.update, marts.shown, marts.folders.map((f) => [f.path, f.update]), marts.models.map((m) => m.name)],
  [3, 3, [['models/marts/sales', 2]], ['customers']]);
var staging = under.folders[2];
check('a model with only types is listed and counted only when included', [staging.update, staging.shown, yamlBulkTree(models, true).update, yamlBulkTree(models, true).folders[0].folders[2].update], [0, 1, 5, 1]);
check('a written model is listed and not counted', yamlBulkTree(models, false, new Set(['model.shop.orders'])).folders[0].folders[0].folders[0].update, 1);
check('agrees and not checked are not in the tree', JSON.stringify(tree).includes('stg_users') || JSON.stringify(tree).includes('snap'), false);
check('a tree of nothing differing is empty', yamlBulkTree([models[4], models[6]], false).shown, 0);

// A folder takes the models under it and no folder that begins with its name.
check('a folder\'s button takes what is under it', yamlBulkScope(models, 'models/marts', false).map((m) => m.name), ['orders', 'refunds', 'customers']);
check('not a folder that merely begins with its name', yamlBulkScope(models, 'models/marts/sales', false).map((m) => m.name), ['orders', 'refunds']);
check('the whole project is the empty folder', yamlBulkScope(models, '', false).length, 4);
check('with types only when included', yamlBulkScope(models, 'models/staging', true).map((m) => m.name), ['stg_orders']);
check('and none when not', yamlBulkScope(models, 'models/staging', false), []);

// The models not checked, grouped by why.
check('the models not checked by reason, the most first', yamlBulkNotChecked(models).map((g) => [g.reason, g.models.map((m) => m.name)]),
  [['it is not in catalog.json', ['snap', 'snap2']], ['it is ephemeral: it has no table', ['eph']]]);

// What leaves a model out.
check('a run newer than the catalog, a table behind its code and a code newer than the catalog each stand against',
  yamlBulkWhy({ catalog_behind_run: true, table_behind_code: true, code_after_catalog: true }),
  ['its last run is newer than catalog.json', 'its table is older than its code', 'its code is newer than catalog.json']);
check('a deployed catalog alone stands against nothing', yamlBulkWhy({ deployed: true, catalog_env: '.env.prod' }), []);
check('it is said once, with the environment', yamlBulkDeployed('.env.prod'),
  'catalog.json describes the tables of .env.prod, built from code that may be behind this branch: a column to remove may be one this branch added.');
check('the head says what it was read from', [
  yamlBulkAbout({ catalog_at: 5, catalog_env: 'manifest' }, (d) => 'D' + d.getTime() / 1000),
  yamlBulkAbout({ catalog_at: 5, catalog_env: '.env.prod' }, (d) => 'D' + d.getTime() / 1000),
], ["catalog.json, the manifest's target, written D5", 'catalog.json, .env.prod, written D5']);

// The chips.
check('a model\'s chips are +added, -removed and ~typed', yamlBulkChips({ status: 'disagrees', missing: [1, 2, 3], extra: [1], retype: [1], untyped: [1] }),
  [{ text: '+3', kind: 'add' }, { text: '−1', kind: 'del' }, { text: '~2', kind: 'types' }]);
check('only the ones there are', yamlBulkChips({ status: 'disagrees', extra: [1] }), [{ text: '−1', kind: 'del' }]);
check('a model with only types says types', yamlBulkChips({ status: 'untyped', untyped: [1, 2] }), [{ text: 'types', kind: 'types' }]);

// The files the patches fall in.
var yml = 'version: 2\nmodels:\n  - name: orders\n    columns:\n      - name: id\n  - name: refunds\n    columns:\n      - name: id\n';
function sync(name, extra) {
  return Object.assign({ id: 'model.shop.' + name, name: name, file: 'models/' + name + '.sql', yml: 'models/schema.yml', status: 'disagrees',
    missing: [{ name: 'note', data_type: 'varchar', after: 'id' }] }, extra || {});
}
function reads(map) { return new Map(Object.entries(map)); }
var plan = yamlBulkPlan([sync('orders'), sync('refunds')], reads({ 'models/schema.yml': { content: yml, stamp: '10-1' } }), new Set(), false);
check('the models of one file are patched in turn on its text, and it is written once',
  [plan.files.length, plan.files[0].file, plan.files[0].models.map((m) => m.sync.name), plan.files[0].stamp, plan.left],
  [1, 'models/schema.yml', ['orders', 'refunds'], '10-1', []]);
check('both are in the text', plan.files[0].text, 'version: 2\nmodels:\n  - name: orders\n    columns:\n      - name: id\n      - name: note\n        data_type: varchar\n'
  + '  - name: refunds\n    columns:\n      - name: id\n      - name: note\n        data_type: varchar\n');

// Freshness leaves models out unless they are asked for.
var behind = sync('refunds', { catalog_behind_run: true });
var held = yamlBulkPlan([sync('orders'), behind], reads({ 'models/schema.yml': { content: yml, stamp: 's' } }), new Set(), false);
check('a model behind the catalog is left out, with why, and the others written',
  [held.files[0].models.map((m) => m.sync.name), held.left.map((l) => [l.sync.name, l.kind, l.why])], [['orders'], [['refunds', 'behind', 'its last run is newer than catalog.json']]]);
var also = yamlBulkPlan([sync('orders'), behind], reads({ 'models/schema.yml': { content: yml, stamp: 's' } }), new Set(), true);
check('and written when the ones behind are asked for', [also.files[0].models.map((m) => m.sync.name), also.left], [['orders', 'refunds'], []]);
var dep = yamlBulkPlan([sync('orders', { deployed: true, catalog_env: '.env.prod' }), sync('refunds', { deployed: true, catalog_env: '.env.prod' })],
  reads({ 'models/schema.yml': { content: yml, stamp: 's' } }), new Set(), false);
check('a deployed catalog leaves no model out, and names its environment once', [dep.files[0].models.length, dep.left, dep.env], [2, [], '.env.prod']);

// A file open with unsaved edits, a file not read, a refused model.
var unsaved = yamlBulkPlan([sync('orders')], reads({ 'models/schema.yml': { content: yml, stamp: 's' } }), new Set(['models/schema.yml']), true);
check('a file open with unsaved edits is left out, even when the ones behind are asked for', [unsaved.files.length, unsaved.left.map((l) => l.kind)], [0, ['unsaved']]);
var unread = yamlBulkPlan([sync('orders')], reads({ 'models/schema.yml': { error: 'no such file' } }), new Set(), false);
check('a file that cannot be read leaves its models out', [unread.files.length, unread.left[0].kind, unread.left[0].why], [0, 'unread', 'its file could not be read: no such file']);
var big = yamlBulkPlan([sync('orders')], reads({ 'models/schema.yml': { content: 'x', stamp: 's', truncated: true } }), new Set(), false);
check('and one too large to be read whole', big.left[0].why, 'its file is too large to be read whole');
var flowing = 'models:\n  - name: orders\n    columns: [id]\n  - name: refunds\n    columns:\n      - name: id\n';
var refused = yamlBulkPlan([sync('orders'), sync('refunds')], reads({ 'models/schema.yml': { content: flowing, stamp: 's' } }), new Set(), false);
check('a refused model is left out with its reason and the others of the file are written', [refused.files[0].models.map((m) => m.sync.name), refused.left.map((l) => [l.sync.name, l.kind, l.why])],
  [['refunds'], [['orders', 'refused', 'its columns: is written in flow style']]]);
var idle = yamlBulkPlan([sync('orders', { missing: [] })], reads({ 'models/schema.yml': { content: yml, stamp: 's' } }), new Set(), false);
check('a model the file already says right is idle and the file is not written', [idle.files.length, idle.idle.length], [0, 1]);

// A model whose patch changes nothing is idle only when it skips nothing.
var skipping = yamlBulkPlan([sync('orders', { missing: [], retype: [{ name: 'id', from: 'varchar', to: 'number' }] })],
  reads({ 'models/schema.yml': { content: 'models:\n  - name: orders\n    columns:\n      - name: id\n        data_type: date\n', stamp: 's' } }), new Set(), false);
check('a retype the file no longer matches is left out with its reason, not idle', [skipping.idle.length, skipping.files.length, skipping.left.map((l) => [l.kind, l.why])],
  [0, 0, [['skipped', 'id (retype): its type is no longer varchar']]]);

// A model with no YAML gets a new file with its content, or the file that has appeared by then.
var bare = { id: 'model.shop.bare', name: 'bare', file: 'models/marts/bare.sql', yml: '', status: 'disagrees', missing: [{ name: 'id', data_type: 'number' }] };
var made = yamlBulkPlan([bare], reads({ 'models/marts/bare.yml': { missing: true } }), new Set(), false);
check('a model with no YAML gets a new file beside its SQL, with the full content', [made.files[0].file, made.files[0].isNew, made.files[0].stamp, made.files[0].text],
  ['models/marts/bare.yml', true, '', 'version: 2\n\nmodels:\n  - name: bare\n    columns:\n      - name: id\n        data_type: number\n']);
var there = yamlBulkPlan([bare], reads({ 'models/marts/bare.yml': { content: 'version: 2\n', stamp: '4-5' } }), new Set(), false);
check('a file that exists by then is patched, never made again', [there.files[0].isNew, there.files[0].stamp, there.files[0].text.startsWith('version: 2\n')], [false, '4-5', true]);

// What a patch says in the preview.
var one = plan.files[0].models[0];
check('a model\'s lines say what it adds', yamlBulkLines(one.sync, one.patch), ['Adds 1: note']);
var cut = yamlSyncPatch('models:\n  - name: orders\n    columns:\n      - name: id\n      - name: old\n        description: x\n', 'orders', { extra: [{ name: 'old', described: true, tests: 2 }] });
check('what a removal loses is said', yamlBulkLines({ extra: [{ name: 'old', described: true, tests: 2 }], held: [{ name: 'p', reason: 'no precision' }] }, cut),
  ['Removes 1: old', 'old loses a description and 2 tests', 'Does not type p: no precision']);

// The card.
check('the card says what was written, what was skipped and why, what was left out, and that a parse is awaited',
  yamlBulkResult({ written: [{ file: 'a.yml', models: 2 }, { file: 'b.yml', models: 1 }], skipped: [{ file: 'c.yml', why: 'it changed on disk since it was read' }, { file: 'd.yml', why: 'it changed on disk since it was read' }] },
    [{ kind: 'behind' }, { kind: 'behind' }, { kind: 'refused' }]), [
    'Wrote 2 files (3 models): a.yml, b.yml',
    'Skipped c.yml, d.yml: it changed on disk since it was read',
    'Left out, behind the catalog or the code: 2 models',
    'Left out, refused: 1 model',
    'The section shows those models as waiting for a parse until the manifest reloads.',
  ]);
check('a long list of files is cut', yamlBulkNames(['1', '2', '3', '4', '5', '6', '7', '8'], 3), '1, 2, 3 and 5 more');
check('nothing written says so', yamlBulkResult({ written: [], skipped: [] }, []), ['Nothing was written.']);

// A file's result is kept: asking for the models behind patches only the files they are in.
var count = 0;
var patchOnce = yamlSyncPatch;
yamlSyncPatch = function () { count++; return patchOnce.apply(this, arguments); };
function two(name, file, extra) {
  return Object.assign({ id: 'model.shop.' + name, name: name, file: 'models/' + name + '.sql', yml: file, status: 'disagrees',
    missing: [{ name: 'note', data_type: 'varchar', after: 'id' }] }, extra || {});
}
var yml1 = 'models:\n  - name: a\n    columns:\n      - name: id\n  - name: b\n    columns:\n      - name: id\n';
var yml2 = 'models:\n  - name: c\n    columns:\n      - name: id\n';
var set = [two('a', 'models/one.yml'), two('b', 'models/one.yml', { catalog_behind_run: true }), two('c', 'models/two.yml')];
var files = reads({ 'models/one.yml': { content: yml1, stamp: '1' }, 'models/two.yml': { content: yml2, stamp: '2' } });
var cache = new Map();
count = 0;
var first = yamlBulkPlan(set, files, new Set(), false, cache);
check('the first plan patches each model it writes once', [count, first.files.map((f) => f.file)], [2, ['models/one.yml', 'models/two.yml']]);
count = 0;
var again = yamlBulkPlan(set, files, new Set(), false, cache);
check('asked again, nothing is patched again', [count, JSON.stringify(again.files) === JSON.stringify(first.files)], [0, true]);
count = 0;
var withBehind = yamlBulkPlan(set, files, new Set(), true, cache);
check('with the models behind, only the file that gains one is patched again', [count, withBehind.files.map((f) => f.models.map((m) => m.sync.name))], [2, [['a', 'b'], ['c']]]);
count = 0;
yamlBulkPlan(set, files, new Set(['models/two.yml']), true, cache);
check('a file that turns unsaved is made again, left out', count, 0);
var steps = yamlBulkSteps(set, files, new Set(), true, new Map());
var yields = 0;
for (var st = steps.next(); !st.done; st = steps.next()) yields++;
check('a page may stop between two patches, and the plan is the same at the end', [yields, st.value.files.length], [3, 2]);
yamlSyncPatch = patchOnce;

// A live read is said as one.
check('the head of a live read says Snowflake, its environment and when it was read', [
  yamlBulkAbout({ source: 'snowflake', catalog_env: 'manifest', read_at: 7 }, (d) => 'D' + d.getTime() / 1000),
  yamlBulkAbout({ source: 'snowflake', catalog_env: '.env.prod', read_at: 7 }, (d) => 'D' + d.getTime() / 1000),
], ["Snowflake, the manifest's target, read D7", 'Snowflake, .env.prod, read D7']);
check('the deployed environment is said once, by Snowflake', yamlBulkDeployed('.env.prod', 'snowflake'),
  'Snowflake has the tables of .env.prod, built from code that may be behind this branch: a column to remove may be one this branch added.');
check('with a live read no catalog flag stands against a model, only its table behind its code', yamlBulkWhy({ table_behind_code: true }), ['its table is older than its code']);

// A read that found nothing is not a file that says it right.
var unknown = [sync('orders', { status: 'not_checked', reason: 'not found in .env.prod, or not visible to this role', missing: undefined }),
  sync('refunds', { status: 'not_checked', reason: 'its location is not resolved under .env.prod', missing: undefined })];
var nothing = yamlBulkPlan(unknown, reads({ 'models/schema.yml': { content: yml, stamp: 's' } }), new Set(), true);
check('every model not compared is left out with its reason, and none is idle or written',
  [nothing.idle.length, nothing.files.length, nothing.left.map((l) => [l.sync.name, l.kind, l.why])],
  [0, 0, [['orders', 'unchecked', 'not found in .env.prod, or not visible to this role'], ['refunds', 'unchecked', 'its location is not resolved under .env.prod']]]);
check('and says so in the card', yamlBulkResult({ written: [], skipped: [] }, nothing.left), ['Left out, not compared: 2 models', 'Nothing was written.']);
