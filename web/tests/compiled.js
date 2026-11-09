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
check('leads with the age, as the manifest card says one', fresh.stat, 'Compiled 12 min ago');
ok('and keeps the date for the hover', fresh.date.indexOf('Compiled ') === 0);
has('that date is the file\'s own', fresh.date, String(new Date((now - 720) * 1000).getFullYear()));
check('nothing moved, so nothing follows the age', fresh.changed, '');
check('a moment is said as one', artifactBar(payload({ age_secs: 4 }), 'orders').stat, 'Compiled a moment ago');
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
check('the age still leads the line', stale.stat, 'Compiled 12 min ago');
check('what moved follows it, with what that costs', stale.changed, 'changed since: the model file (may be out of date)');
check('a stale file that names nothing still says what it costs', artifactBar(payload({ stale: true }), 'orders').changed, '(may be out of date)');
check('the button now says redo', stale.button, 'Recompile');

print('--- several inputs moved at once ---');
var both = artifactBar(payload({
  stale: true,
  changed: ['the model file', 'the schema file', 'dbt_project.yml', 'the macro macros/dates/fiscal_year.sql'],
}), 'orders');
has('all of them, introduced once', both.changed,
  'changed since: the model file, the schema file, dbt_project.yml, the macro macros/dates/fiscal_year.sql');
check('and "changed since" is said once, not per entry', both.changed.split('changed since').length, 2);

print('--- the run file is a different question ---');
var run = artifactBar(payload({ kind: 'run', rel: 'target/run/shop/models/marts/orders.sql' }), 'orders');
check('described as a run, not a compile', [run.stat, run.date.indexOf('Last run ') === 0], ['Last run 12 min ago', true]);
check('and redone with dbt run', run.command, 'dbt run --select orders');
check('the button says so too', run.button, 'Run again');
var behind = artifactBar(payload({
  kind: 'run', stale: true, age_secs: 200000, reasons: ['compiled again after this was run'] }), 'orders');
check('a run that is behind is still only ever rerun', behind.button, 'Run again');
check('its age is said the card\'s way', behind.stat, 'Last run 2 d ago');
check('and says what put it behind', behind.changed, 'compiled again after this was run (may be out of date)');

print('--- age never colours anything on its own ---');
// A file nothing has touched since is what dbt would write, at any age.
var ancient = artifactBar(payload({ age_secs: 9 * 86400, stale: false, changed: [], reasons: [] }), 'orders');
check('green', ancient.tone, 'ok');
check('the age is reported and nothing more', [ancient.stat, ancient.changed], ['Compiled 9 d ago', '']);

print('--- the amber bar of a stale file stays on the line (0164) ---');
// paintArtifact needs the editor and a page; what ties the line's class to the
// state is read from the source and the sheet.
var css = read('web/app.css');
var paint = src.slice(src.indexOf('function paintArtifact'), src.indexOf('function artifactEnvPick'));
ok('the line takes "stale" when the file is stale and no class when it is fresh', /line\.className = 'chead' \+ \(info\.stale \? ' stale' : ''\);/.test(paint));
ok('the line and the bar say stale from the same answer', /tone: info\.stale \? 'stale' : 'ok'/.test(src));
ok('a stale line is washed in the warning colour with a 2px rule inside it', /\.chead\.stale \{[^}]*background: var\(--warn-wash\);[^}]*box-shadow: inset 2px 0 0 var\(--warn\);/.test(css));
ok('the button types the command and never presses Enter (0071)', /sendToTerminal\(said\.command, false, null, from\)/.test(paint));

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
has('the inputs are grouped', mixed.changed, 'changed since: the model file, dbt_project.yml');
has('the rest stands on its own', mixed.changed, '\u00b7 compiled again after this was run');
has('and the warning closes the line', mixed.changed, '(may be out of date)');

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

print('\n--- the description in the dock\'s row (0164) ---');
var html = read('web/index.html');
var row = html.match(/<div id="artifact-tools"[^>]*>[\s\S]*?<\/div>/);
check('the row has a place for it, hidden to start with, beside the Jobs and terminal tools', [!!row, row ? /class="tools hidden"/.test(row[0]) : false, row ? /id="artifact-about"/.test(row[0]) : false,
  html.indexOf('id="jobs-tools"') < html.indexOf('id="artifact-tools"'), html.indexOf('id="artifact-tools"') < html.indexOf('id="term-tools"')], [true, true, true, true, true]);
var showDock = src.slice(src.indexOf('function showDock'), src.indexOf('function foldDockTabs'));
check('showDock shows it for Compiled and Run only', /\$\('#artifact-tools'\)\.classList\.toggle\('hidden', which !== 'compiled' && which !== 'run'\)/.test(showDock), true);
check('and the fold measures with it hidden, as with the others', /const others = \[[^\]]*\$\('#artifact-tools'\)[^\]]*\];/.test(src), true);
// The words come from the tab button's own title, read once by wireTabs into data-about.
(function () {
  var about = { textContent: '', title: '' };
  var tabs = { compiled: { dataset: { about: 'The model\'s SQL with the Jinja gone, from target/compiled/' } }, run: { dataset: { about: 'The statement dbt executed to materialise it, from target/run/' } } };
  function $(sel) { return sel === '#artifact-about' ? about : tabs[/data-dock="(\w+)"/.exec(sel)[1]]; }
  eval(src.slice(src.indexOf('function paintArtifactTools'), src.indexOf('/* `look` is off for a tab')));
  paintArtifactTools('run');
  check('the Run tab shows its own description, and the hover holds it whole', [about.textContent, about.title],
    ['The statement dbt executed to materialise it, from target/run/', 'The statement dbt executed to materialise it, from target/run/']);
  paintArtifactTools('compiled');
  check('the Compiled tab its own', about.textContent, 'The model\'s SQL with the Jinja gone, from target/compiled/');
}());
check('the tab buttons keep those titles for the wire that reads them', [/data-dock="compiled" title="The model's SQL with the Jinja gone, from target\/compiled\/"/.test(html),
  /data-dock="run" title="The statement dbt executed to materialise it, from target\/run\/"/.test(html)], [true, true]);
// selectEnv with a page of fakes: the pick is made from a radio in the Compiled pane's group, the repaint
// checks another, and that one must end up holding the focus (0147, 0164).
(function () {
  var focused = null, posts = [];
  function radio(name) { return { name: name, focus: function () { focused = this; } }; }
  var checked = radio('PROD');
  var pane = { id: 'compiled-head' };
  var group = { closest: function (sel) { return sel === '#compiled-head, #run-head' ? pane : null; } };
  var document = { activeElement: { closest: function (sel) { return sel === '.envseg' ? group : null; } } };
  var S = { env: '', node: null };
  var asked = [];
  function $(sel) { asked.push(sel); return sel === '#compiled-head .envseg [aria-checked=true]' ? checked : null; }
  function paintEnvSelect() {}
  function renderCatalog() {}
  function paintLineageBuilds() {}
  function repaintArtifacts() { asked.push('repaint'); }
  var api = { send: function (url, method, body) { posts.push(body.file); return Promise.resolve(); } };
  eval(src.slice(src.indexOf('function selectEnv'), src.indexOf('/* The environment menu, shared by')));
  selectEnv('.env.prod');
  check('a pick from the group gives the focus to the radio now checked, after the repaint',
    [focused && focused.name, asked.indexOf('repaint') < asked.indexOf('#compiled-head .envseg [aria-checked=true]'), S.env, posts], ['PROD', true, '.env.prod', ['.env.prod']]);
  focused = null;
  document.activeElement = { closest: function () { return null; } };
  selectEnv('');
  check('a pick from elsewhere (the status bar, the settings) moves no focus', focused, null);
}());
check('the old chips are gone from the page', /envchip|fresh-bar|artifactEnvBar/.test(src), false);
