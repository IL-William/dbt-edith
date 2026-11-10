// The editor's tabs and the Open editors list say what the manifest makes of a
// file (0148, 0151): a tab draws its kind's bar before the name, a row draws the
// file's icon between the close slot and the name, and a mark that arrives late
// draws the tabs once more.
// Run from the repository root: jsc web/tests/tabkinds.js
var src = read('web/app.js');
var css = read('web/app.css');
var lin = read('web/lineage.js');
eval(lin.slice(lin.indexOf('const MAT = {'), lin.indexOf('let svg, root')));
var Lineage = { nodeColor: nodeColor, matLabel: matLabel };

function slice(from, to) { return src.slice(src.indexOf(from), src.indexOf(to)).replace(/^const /gm, 'var '); }
// `const` in an eval is the eval's own; `var` puts the names where the checks read them.
eval(slice('const base = (path)', 'async function openFile(path'));
eval(slice('function collinTab', 'function activate(path'));
eval(slice('function projectPath(key)', 'function dropTab'));
eval(slice('const dot = (n)', '/* Whether CodeMirror'));
eval(slice('const FILE_SVG', '// ------------------------------------------------------------------ tree --'));
eval(slice('function renderTabs', '/* The commands a tab\'s own menu offers'));

function el() {
  var e = { children: [], dataset: {}, style: {}, className: '', title: '', innerHTML: '', isConnected: true,
    classList: { add: function (c) { e.className += ' ' + c; }, toggle: function () {} },
    append: function () { for (var i = 0; i < arguments.length; i++) e.children.push(arguments[i]); },
    appendChild: function (c) { e.children.push(c); return c; },
    addEventListener: function () {}, scrollIntoView: function () {}, replaceWith: function () {} };
  var text = '';
  Object.defineProperty(e, 'textContent', { get: function () { return text; }, set: function (v) { text = v; e.children = []; } });
  return e;
}
var document = { createElement: el };
var parts = { '#tabbar': el(), '#oe-list': el(), '#oe-count': el(), '#save-all-btn': el(), '#close-all-btn': el(), '#reveal-btn': el() };
function $(sel) { return parts[sel]; }

var S = { open: new Map(), order: [], active: null, preview: null, fileMarks: new Map(), tabsAsk: 0 };
var drawn = 0, listed = [];
function paintTree() { drawn++; }
function closeFile() {}
function activate() {}
function toast() {}
var listing = { models: [
  { name: 'late.sql', path: 'models/late.sql', dir: false, kind: 'model', materialized: 'view' },
  { name: 'free.sql', path: 'models/free.sql', dir: false }] };
var api = { get: function (url) {
  var dir = decodeURIComponent(url.split('path=')[1]);
  listed.push(dir);
  return Promise.resolve(listing[dir] || []);
} };

function check(label, got, want) {
  var g = JSON.stringify(got), w = JSON.stringify(want);
  print((g === w ? 'PASS  ' : 'FAIL  ') + label + (g === w ? '' : '\n        expected ' + w + '\n        got      ' + g));
}
function ok(label, cond) { print((cond ? 'PASS  ' : 'FAIL  ') + label); }

function open(key, f) { S.open.set(key, Object.assign({ dirty: false, path: key }, f || {})); S.order.push(key); }
function tabOf(key) { return $('#tabbar').children.filter(function (t) { return t.dataset.key === key; })[0]; }
function parts_(t) { return t.children.map(function (c) { return c.className; }); }

S.fileMarks.set('models/a.sql', { kind: 'model', materialized: 'table' });
open('models/a.sql');
open('dbt_project.yml');
open('macros/m.sql');
open('diff:models/a.sql', { kind: 'diff', path: 'models/a.sql' });
open('profile:p', { kind: 'profile', path: '/home/me/.dbt/profiles.yml' });
open('collin:compare', { kind: 'compare', path: 'collin 0.11.0 against 0.12.0' });
renderTabs();

print('--- the tab ---');
var model = tabOf('models/a.sql');
check('a model\'s tab is its bar, its name, then the close slot', parts_(model), ['dot', 'nm', 'x']);
check('the bar is the kind\'s colour and says what it is built as', [model.children[0].style.background, model.children[0].title], ['var(--k-table)', 'table']);
check('a YAML file, and a .sql no node stands for, have no bar and no room for one',
  [parts_(tabOf('dbt_project.yml')), parts_(tabOf('macros/m.sql'))], [['nm', 'x'], ['nm', 'x']]);
check('a model\'s diff tab has its bar too', parts_(tabOf('diff:models/a.sql')), ['dot', 'nm', 'x']);
check('the profile and a collin comparison have none', [parts_(tabOf('profile:p')), parts_(tabOf('collin:compare'))], [['nm', 'x'], ['nm', 'x']]);

print('\n--- Open editors ---');
var rows = $('#oe-list').children;
var row = rows.filter(function (r) { return r.dataset.key === 'models/a.sql'; })[0];
check('a row is the close slot, the file\'s icon, the name, the folder and Save', row.children.map(function (c) { return c.className.split(' ')[0]; }), ['state', 'ficon', 'nm', 'dir', 'icon']);
check('a model\'s icon takes its box\'s colour', row.children[1].style.color, 'var(--k-table)');
var diffRow = rows.filter(function (r) { return r.dataset.key === 'diff:models/a.sql'; })[0];
check('a diff row draws the icon of the file it compares', diffRow.children[1].style.color, 'var(--k-table)');
check('every row has an icon, the comparison\'s and the profile\'s too', rows.map(function (r) { return /^ficon/.test(r.children[1].className); }), rows.map(function () { return true; }));

print('\n--- a mark that arrives late ---');
S.open.clear(); S.order = []; S.fileMarks.clear();
open('models/late.sql');
open('models/free.sql');
drawn = 0; listed = [];
renderTabs();
check('the tab is first drawn with no bar', parts_(tabOf('models/late.sql')), ['nm', 'x']);
drainMicrotasks();
check('the folder was asked once, and the tabs are drawn once more', [listed, drawn], [['models'], 2]);
check('now the model has its bar and the other file still none', [parts_(tabOf('models/late.sql')), parts_(tabOf('models/free.sql'))], [['dot', 'nm', 'x'], ['nm', 'x']]);
renderTabs();
drainMicrotasks();
check('and a file with no mark does not ask again, so no loop', [listed, drawn], [['models'], 3]);

print('\n--- the stylesheet ---');
ok('no rule of the tab still reaches its name as the first span', !/\.ftab[^{,]*span:first-child/.test(css));
ok('the italic, the changed colour and the strike are on the name', /\.ftab\.preview \.nm \{ font-style: italic; \}/.test(css)
  && /\.ftab\.disk-changed \.nm \{ color: var\(--warn\); \}/.test(css) && /\.ftab\.disk-gone \.nm \{ text-decoration: line-through; \}/.test(css));
