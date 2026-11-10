// A file's icon: its shape says what the file is, its colour only what the
// manifest builds it as (0151). Each extension, a model by each
// materialization, a seed, a snapshot, the files the design colours and the
// tree's git letters.
// Run from the repository root: jsc web/tests/fileicons.js
var src = read('web/app.js');
var css = read('web/app.css');
var lin = read('web/lineage.js');
eval(lin.slice(lin.indexOf('const MAT = {'), lin.indexOf('let svg, root')));
var Lineage = { nodeColor: nodeColor, matLabel: matLabel };
var S = { fileMarks: new Map() };
var api;
var document = { createElement: function () {
  var cls = [];
  return { innerHTML: '', style: {}, title: '', className: '',
    classList: { add: function (c) { cls.push(c); }, all: cls } };
} };
// `const` in an eval is the eval's own; `var` puts the names where the checks read them.
eval(src.slice(src.indexOf('const FILE_SVG'), src.indexOf('// ------------------------------------------------------------------ tree --')).replace(/^const /gm, 'var '));

function check(label, got, want) {
  var g = JSON.stringify(got), w = JSON.stringify(want);
  print((g === w ? 'PASS  ' : 'FAIL  ') + label + (g === w ? '' : '\n        expected ' + w + '\n        got      ' + g));
}
function ok(label, cond) { print((cond ? 'PASS  ' : 'FAIL  ') + label); }

print('--- the shape follows the extension ---');
var shapes = {
  'a.sql': SQL_ICON, 'a.yml': YML_ICON, 'a.yaml': YML_ICON, 'a.md': MD_ICON, 'a.py': PY_ICON, 'a.sh': SH_ICON,
  'a.json': BRACES, 'a.csv': TABLE, 'a.SQL': SQL_ICON, '.gitignore': DOC, 'LICENSE': DOC, 'a.txt': DOC,
};
Object.keys(shapes).forEach(function (name) {
  check(name + ' is drawn as its own glyph', fileIcon(name).innerHTML === shapes[name], true);
});
var all = [SQL_ICON, YML_ICON, MD_ICON, PY_ICON, SH_ICON, BRACES, TABLE, DOC];
check('the eight glyphs are eight different drawings', all.filter(function (g, i) { return all.indexOf(g) === i; }).length, 8);
ok('each is a 16 by 16 stroke drawing with round caps, 14 px, the design\'s 1.4',
  all.every(function (g) { return /viewBox="0 0 16 16" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round"/.test(g); }));
ok('the cylinder is an ellipse and a body; the YAML glyph three dots and three lines',
  /<ellipse/.test(SQL_ICON) && (YML_ICON.match(/<circle/g) || []).length === 3 && /M5\.25 3\.5h8\.5M7\.75 8h6M7\.75 12\.5h6/.test(YML_ICON));
check('its class is the extension\'s kind', ['a.sql', 'a.yml', 'a.yaml', 'a.md', 'a.py', 'a.sh', 'a.json', 'a.csv', 'a.txt'].map(function (n) { return fileIcon(n).className; }),
  ['ficon sql', 'ficon yml', 'ficon yml', 'ficon md', 'ficon py', 'ficon sh', 'ficon json', 'ficon csv', 'ficon other']);

print('\n--- a file the manifest builds takes its box\'s colour ---');
var mats = { view: 'var(--k-view)', table: 'var(--k-table)', incremental: 'var(--k-incr)', ephemeral: 'var(--k-ephemeral)',
  materialized_view: 'var(--k-mview)', dynamic_table: 'var(--k-mview)', 'custom_thing': 'var(--k-custom)', '': 'var(--k-other)' };
Object.keys(mats).forEach(function (m) {
  var icon = fileIcon('stg_orders.sql', { kind: 'model', materialized: m });
  check('a model built as "' + m + '"', [icon.style.color, icon.classList.all.indexOf('node') >= 0], [mats[m], true]);
});
check('a model\'s icon says what it is built as when the pointer rests on it', fileIcon('x.sql', { kind: 'model', materialized: 'view' }).title, 'view');
check('a seed\'s csv is the table glyph in the seed colour', (function () {
  var i = fileIcon('raw_customers.csv', { kind: 'seed', materialized: '' });
  return [i.innerHTML === TABLE, i.style.color, i.title];
}()), [true, 'var(--k-seed)', 'seed']);
check('a snapshot\'s sql is the cylinder in the snapshot colour', (function () {
  var i = fileIcon('orders_snapshot.sql', { kind: 'snapshot', materialized: '' });
  return [i.innerHTML === SQL_ICON, i.style.color];
}()), [true, 'var(--k-snapshot)']);
check('a Python model keeps the Py glyph and takes its box\'s colour, so a table reads as a table', (function () {
  var i = fileIcon('py_model.py', { kind: 'model', materialized: 'table' });
  return [i.innerHTML === PY_ICON, i.style.color];
}()), [true, 'var(--k-table)']);
check('a .py that is no model stays Py\'s yellow, from the stylesheet', fileIcon('export_docs.py').style.color || '', '');
check('no other extension takes a colour from a node', ['a.yml', 'a.md', 'a.sh', 'a.json', 'a.txt'].map(function (n) {
  return fileIcon(n, { kind: 'model', materialized: 'view' }).style.color || '';
}), ['', '', '', '', '']);
check('a .sql the manifest does not define keeps the cylinder and no colour of its own', (function () {
  var i = fileIcon('adhoc.sql');
  return [i.innerHTML === SQL_ICON, i.style.color || '', i.classList.all.length];
}()), [true, '', 0]);
check('and a .csv that is not a seed has none either', fileIcon('data.csv').style.color || '', '');

print('\n--- the page remembers a file\'s mark by its path ---');
S.fileMarks.set('models/stg.sql', { kind: 'model', materialized: 'view' });
check('known', fileMark('models/stg.sql'), { kind: 'model', materialized: 'view' });
check('unknown', fileMark('models/other.sql'), null);

print('\n--- a listing is remembered by path, row or no row ---');
rememberMarks([{ name: 'a.sql', path: 'models/a.sql', dir: false, kind: 'model', materialized: 'view' }, { name: 'models', path: 'models', dir: true },
  { name: 'b.sql', path: 'models/b.sql', dir: false }]);
check('a node\'s file is marked, a folder and a plain file are not', [fileMark('models/a.sql'), fileMark('models'), fileMark('models/b.sql')], [{ kind: 'model', materialized: 'view' }, null, null]);
rememberMarks([{ name: 'a.sql', path: 'models/a.sql', dir: false }]);
check('a file that stops being a node loses its mark', fileMark('models/a.sql'), null);
check('the breadcrumb\'s row and the tree\'s draw the same icon', (function () {
  var entry = { name: 'a.sql', path: 'models/a.sql', dir: false, kind: 'model', materialized: 'incremental' };
  return fileIcon(entry.name, markOf(entry)).style.color;
}()), 'var(--k-incr)');

print('\n--- a new manifest redraws the icons whose mark moved ---');
// The tabs and Open editors read the same marks, so a new manifest draws them again.
var tabsDrawn = 0, barAtDraw = [], listed = [];
// What the real one reads: whether the open tab's file has its mark at the moment of the draw.
function projectPath(key) { return key; }
function renderTabs() { tabsDrawn++; barAtDraw.push(!!fileMark('analyses/x.sql')); }
var refreshSrc = src.slice(src.indexOf('async function refreshFileMarks'), src.indexOf('// ------------------------------------------------------------ git status --'));
function makeRow(name, path) {
  var icon = fileIcon(name, fileMark(path)), row = { el: { isConnected: true, loading: null, querySelector: function () { return icon; } }, dir: false, icon: icon };
  icon.replaceWith = function (n) { row.icon = n; row.redrawn = (row.redrawn || 0) + 1; };
  return row;
}
var listing = { '': [{ name: 'models', path: 'models', dir: true }, { name: 'LICENSE', path: 'LICENSE', dir: false }],
  'models': [{ name: 'a.sql', path: 'models/a.sql', dir: false, kind: 'model', materialized: 'table' },
    { name: 'b.sql', path: 'models/b.sql', dir: false, kind: 'model', materialized: 'view' }, { name: 'c.sql', path: 'models/c.sql', dir: false }] };
listing['analyses'] = [{ name: 'x.sql', path: 'analyses/x.sql', dir: false, kind: 'model', materialized: 'view' }];
var api = { get: function (url) { listed.push(decodeURIComponent(url.split('path=')[1])); return Promise.resolve(listing[decodeURIComponent(url.split('path=')[1])]); } };
eval(refreshSrc.replace(/^async function refreshFileMarks/, 'var refreshFileMarks = async function'));
S.rows = new Map();
S.order = ['analyses/x.sql'];
S.fileMarks = new Map([['analyses/x.sql', { kind: 'model', materialized: 'view' }], ['models/a.sql', { kind: 'model', materialized: 'view' }], ['models/b.sql', { kind: 'model', materialized: 'view' }], ['models/gone.sql', { kind: 'model', materialized: 'view' }]]);
var folder = { el: { isConnected: true, loading: Promise.resolve() }, dir: true };
S.rows.set('models', folder);
S.rows.set('models/a.sql', makeRow('a.sql', 'models/a.sql'));
S.rows.set('models/b.sql', makeRow('b.sql', 'models/b.sql'));
S.rows.set('models/c.sql', makeRow('c.sql', 'models/c.sql'));
var done = false;
refreshFileMarks().then(function () { done = true; });
drainMicrotasks();
ok('the refresh finished', done);
check('the tab bar is drawn again once the marks moved', tabsDrawn, 1);
check('a tab in a folder the tree never opened keeps its bar through the refresh: one draw, with the bar, and its folder listed once',
  [barAtDraw, listed.filter(function (d) { return d === 'analyses'; }).length, marksAsked.has('analyses')], [[true], 1, true]);
check('a model now built as a table is drawn again', S.rows.get('models/a.sql').redrawn, 1);
check('one whose mark did not move is left alone', S.rows.get('models/b.sql').redrawn || 0, 0);
check('marks are what the listings say now: a.sql table, b.sql view, c.sql none, and the file no listing names forgotten',
  [fileMark('models/a.sql'), fileMark('models/b.sql'), fileMark('models/c.sql'), fileMark('models/gone.sql')],
  [{ kind: 'model', materialized: 'table' }, { kind: 'model', materialized: 'view' }, null, null]);
ok('and it is asked for on a new manifest, only once a tree exists', /if \(moved && S\.rows\.size\) refreshFileMarks\(\);/.test(src));

print('\n--- the stylesheet ---');
function rule(sel) {
  var re = new RegExp('^' + sel.replace(/[.*+?^${}()|[\]\\]/g, '\\$&') + ' \\{([^}]*)\\}', 'm'), m = re.exec(css);
  return m ? m[1].replace(/\s+/g, ' ').trim() : null;
}
check('a file is neutral by default', /color: var\(--fg-mute\)/.test(rule('.ficon')), true);
check('YAML takes dbt\'s violet, Py its yellow, Markdown and shell the dimmer text',
  [rule('.ficon.yml'), rule('.ficon.py'), rule('.ficon.md, .ficon.sh')],
  ['color: var(--dbt-yaml);', 'color: var(--py);', 'color: var(--fg-dim);']);
check('no colour is left for sql, json, csv or other: a colour on a file only means its materialization',
  ['.ficon.sql', '.ficon.json', '.ficon.csv', '.ficon.other'].map(rule), [null, null, null, null]);
ok('and the --f- names nothing reads are gone', !/--f-(sql|yml|json|csv|other)\b/.test(css + src));
check('--f-md and --f-macro are still read', [/var\(--f-md\)/.test(css), /var\(--f-macro\)/.test(css)], [true, true]);
ok('git\'s status is a letter at the end of the row: M and the pencil in --git-m, U and A in --git-u',
  /\.gs\.m \{ color: var\(--git-m\); \}/.test(css) && /\.gs\.u, \.gs\.a \{ color: var\(--git-u\); \}/.test(css) && /\.gs\.w \{[^}]*var\(--git-m\)[^}]*var\(--m-edit\)/.test(css));
ok('a folder\'s letter is dimmed by mixing it into the sidebar, not by opacity', /\.row\.folder \.gs\.m \{ color: color-mix\(in srgb, var\(--git-m\) 85%, var\(--bg-2\)\); \}/.test(css) && !/\.gs[^{]*\{[^}]*opacity/.test(css));
ok('and each carries its words as an aria-label', /row\.gs\.setAttribute\('aria-label', mark\[2\]\)/.test(src));
ok('and no colour of it reaches a name', !/\.row\.st-[a-z]+ \.nm/.test(css));
