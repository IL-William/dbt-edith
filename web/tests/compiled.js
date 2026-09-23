// The Compiled and Run tabs: which words each payload puts in the bar, and
// which of the two files it is describing. The bar carries the words, so most
// of these assert on sentences rather than on numbers. Fixtures mirror the
// /api/compiled shape with invented paths.
// Run from the repository root: jsc web/tests/compiled.js
var src = read('web/app.js');
eval(src.slice(src.indexOf('function humanAge'), src.indexOf('function freshnessBadge')));

function check(label, got, want) {
  var g = JSON.stringify(got), w = JSON.stringify(want);
  print((g === w ? 'PASS  ' : 'FAIL  ') + label + (g === w ? '' : '\n        expected ' + w + '\n        got      ' + g));
}
function ok(label, cond) { check(label, !!cond, true); }
function has(label, text, want) { ok(label, text.indexOf(want) >= 0); }

var now = Math.floor(Date.now() / 1000);
function payload(over) {
  var base = {
    kind: 'compiled', found: true,
    path: '/Users/dev/shop/target/compiled/shop/models/marts/orders.sql',
    rel: 'target/compiled/shop/models/marts/orders.sql',
    candidates: [], compiled_at: now - 720, age_secs: 720, source_at: now - 900,
    stale: false, changed: [], reasons: [], content: 'select 1', truncated: false, bytes: 8,
  };
  for (var k in over) base[k] = over[k];
  return base;
}

print('--- a compiled file dbt wrote a moment ago ---');
var fresh = artifactBar(payload({}), 'orders');
check('green', fresh.tone, 'ok');
check('names the file it is describing', fresh.kind, 'compiled');
ok('leads with the date, not the age', fresh.when.indexOf('compiled ') === 0);
has('and that date is the file\'s own', fresh.when, String(new Date((now - 720) * 1000).getFullYear()));
check('the age rides beside it', fresh.why, '12min ago');
check('nothing to redo, so the button just offers', fresh.button, 'Compile again');
check('the command matches the tab', fresh.command, 'dbt compile --select orders');

print('--- the path is a link into the file tree ---');
ok('linked', fresh.link);
check('shown project-relative, which is what the tree is keyed by', fresh.path,
  'target/compiled/shop/models/marts/orders.sql');
has('the tooltip keeps the absolute path', fresh.pathTitle, '/Users/dev/shop/target/compiled');
has('and says what clicking does', fresh.pathTitle, 'click to show it in the file tree');

print('--- a target outside the project has no tree row ---');
// What --manifest /elsewhere/manifest.json produces: still read, still shown,
// but there is nothing in the tree to walk down to.
var away = artifactBar(payload({ rel: '' }), 'orders');
ok('not linked', !away.link);
check('so the absolute path is what is shown', away.path,
  '/Users/dev/shop/target/compiled/shop/models/marts/orders.sql');
has('and the tooltip says why', away.pathTitle, 'outside the project');

print('--- the source moved under it ---');
var stale = artifactBar(payload({ stale: true, changed: ['the model file'] }), 'orders');
check('amber', stale.tone, 'stale');
has('the age still leads the line', stale.why, '12min ago');
has('what moved follows it', stale.why, 'changed since: the model file');
has('and what that costs', stale.why, '(may be out of date)');
check('the button now says redo', stale.button, 'Recompile');

print('--- several inputs moved at once ---');
var both = artifactBar(payload({
  stale: true,
  changed: ['the model file', 'the schema file', 'dbt_project.yml', 'the macro fiscal_year.sql'],
}), 'orders');
has('all of them, introduced once', both.why,
  'changed since: the model file, the schema file, dbt_project.yml, the macro fiscal_year.sql');
check('and "changed since" is said once, not per entry', both.why.split('changed since').length, 2);

print('--- the run file is a different question ---');
var run = artifactBar(payload({ kind: 'run', rel: 'target/run/shop/models/marts/orders.sql' }), 'orders');
check('described as a run, not a compile', run.when.indexOf('last run ') === 0, true);
check('and redone with dbt run', run.command, 'dbt run --select orders');
check('the button says so too', run.button, 'Run again');
var behind = artifactBar(payload({
  kind: 'run', stale: true, age_secs: 200000, reasons: ['compiled again after this was run'] }), 'orders');
check('a run that is behind is still only ever rerun', behind.button, 'Run again');
has('and says what put it behind', behind.why, '2d ago \u00b7 compiled again after this was run');

print('--- age never colours anything on its own ---');
// A file nothing has touched since is what dbt would write, at any age.
var ancient = artifactBar(payload({ age_secs: 9 * 86400, stale: false, changed: [], reasons: [] }), 'orders');
check('green', ancient.tone, 'ok');
check('the age is reported and nothing more', ancient.why, '9d ago');

print('--- dbt has not written it yet ---');
var none = artifactBar({ kind: 'compiled', found: false, candidates: ['/a/target/compiled/shop/x.sql', '/a/target/compiled/x.sql'] }, 'orders');
ok('no bar to colour', !none.found);
has('names the model', none.head, 'orders');
has('and the directory it looked in', none.head, 'target/compiled/');
check('offers the command that would produce it', none.command, 'dbt compile --select orders');
check('every path probed is handed on', none.candidates.length, 2);

var noneRun = artifactBar({ kind: 'run', found: false, candidates: [] }, 'orders');
has('the run tab names its own directory', noneRun.head, 'target/run/');
has('and its own command', noneRun.command, 'dbt run --select orders');

print('--- nothing came back at all ---');
var empty = artifactBar(undefined, 'orders');
ok('treated as not found rather than thrown', !empty.found);
check('and still says what to type', empty.command, 'dbt compile --select orders');

print('--- what moved and what is wrong are different lists ---');
var mixed = artifactBar(payload({
  kind: 'run', stale: true, changed: ['the model file', 'dbt_project.yml'],
  reasons: ['compiled again after this was run'] }), 'orders');
has('the inputs are grouped', mixed.why, 'changed since: the model file, dbt_project.yml');
has('the rest stands on its own', mixed.why, '· compiled again after this was run');
has('and the warning closes the line', mixed.why, '(may be out of date)');

print('\n--- a test is not run by dbt run ---');
// Its run file is written by `dbt test`; `dbt run --select <a test>` selects
// nothing and would leave the tab saying exactly what it said before.
var testRun = artifactBar(payload({ kind: 'run', rel: 'target/run/shop/x.sql' }), 'not_null_orders_id', true);
check('the run button offers dbt test', testRun.command, 'dbt test --select not_null_orders_id');
check('and says so on the button', testRun.button, 'Test again');
var testCompiled = artifactBar(payload({}), 'not_null_orders_id', true);
check('compiling is the same command for both', testCompiled.command, 'dbt compile --select not_null_orders_id');
var missing = artifactBar({ kind: 'run', found: false, candidates: [] }, 'not_null_orders_id', true);
check('a missing test run still offers dbt test', missing.command, 'dbt test --select not_null_orders_id');
var modelRun = artifactBar(payload({ kind: 'run', rel: 'target/run/shop/x.sql' }), 'orders', false);
check('a model is still run by dbt run', modelRun.command, 'dbt run --select orders');
