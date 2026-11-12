// Search palette: nodes, macros and plain files in one list, without duplicates.
// Run from the repository root: jsc web/tests/palette.js
var S = { site: false };
var src = read('web/app.js');
// A single eval: a `const` declaration does not leak from one eval to the next,
// so `base` has to live in the same scope as `mergeHits`.
eval(src.slice(src.indexOf('const base = (path)'), src.indexOf('const dirOf'))
   + src.slice(src.indexOf('function splitMatch'), src.indexOf('/* What the status line says'))
   + src.slice(src.indexOf('function mergeHits'), src.indexOf('/* One box for everything')));

function check(label, ok) { print((ok ? 'PASS  ' : 'FAIL  ') + label); }

var nodes = [
  { id: 'model.p.stg_x', name: 'stg_x', file: 'models/stg/stg_x.sql' },
  { id: 'source.p.raw.y', name: 'raw.y', file: '' },
];
var paths = [
  'models/stg/stg_x.sql',                                  // already represented by a node
  'tests/generic/assert_positive_value.sql',     // generic test: not a node
  '.env',                                                  // not a node at all
  'macros/grants/grant_select.sql',
];
var out = mergeHits(nodes, [], paths);

var tr = splitMatch('İSTANBUL_orders', 'ord');
check('a match after a letter that lowercases to two is cut where it is', tr[0] === 'İSTANBUL_' && tr[1] === 'ord' && tr[2] === 'ers');
check('a query with regex characters is taken literally', splitMatch('a.b(c)', '.b(').join('|') === 'a|.b(|c)');
check('a query in another case keeps the text\'s own', splitMatch('Stg_ORDERS', 'orders').join('|') === 'Stg_|ORDERS|');

check('nodes come first', out[0].kind === 'node' && out[1].kind === 'node');
check('a model .sql is not listed twice', out.filter(function (h) {
  return h.sub === 'models/stg/stg_x.sql'; }).length === 1);
check('a generic test can be found', out.some(function (h) {
  return h.kind === 'file' && h.label === 'assert_positive_value.sql'; }));
check('.env can be found', out.some(function (h) { return h.label === '.env'; }));
check('expected total', out.length === 5);
check('a file shows its name, not its path', out[2].label.indexOf('/') === -1);
check('a node without a file hides nothing', mergeHits([{ name: 'n', file: '' }], [], ['a.sql']).length === 2);
check('no node: files only', mergeHits([], [], paths).length === 4);
check('no file: nodes only', mergeHits(nodes, [], []).length === 2);

var macros = [
  { call: 'cents_to_dollars', name: 'cents_to_dollars', file: 'macros/cents_to_dollars.sql' }, // named after its file
  { call: 'cents_rounded', name: 'cents_rounded', file: 'macros/money.sql' },         // one of several in it
  { call: 'automate_dv.hub', name: 'hub', file: 'dbt_packages/automate_dv/macros/tables/hub.sql' },
  { call: 'test_is_positive', name: 'test_is_positive', file: 'tests/generic/is_positive.sql' },
];
var withMacros = mergeHits(nodes, macros, paths.concat(['macros/cents_to_dollars.sql', 'macros/money.sql',
  'dbt_packages/automate_dv/macros/tables/hub.sql',
  'tests/generic/is_positive.sql']));
check('macros come after the nodes, before the files', withMacros.map(function (h) { return h.kind; }).join() ===
  'node,node,macro,macro,macro,macro,file,file,file,file');
check('a macro reads as the project calls it', withMacros[4].label === 'automate_dv.hub');
check('a file named after its macro is not listed twice', !withMacros.some(function (h) {
  return h.kind === 'file' && (h.path === 'macros/cents_to_dollars.sql' || h.path.indexOf('/hub.sql') > 0); }));
check('nor one named after its generic test', !withMacros.some(function (h) {
  return h.kind === 'file' && h.path === 'tests/generic/is_positive.sql'; }));
check('a file holding more than the macro found stays', withMacros.some(function (h) {
  return h.kind === 'file' && h.path === 'macros/money.sql'; }));

function same(label, got, want) { check(label, JSON.stringify(got) === JSON.stringify(want)); }
// The groups: five rows then a row for the rest, a title that says what the node group holds.
same('one kind: its plural', nodeGroupTitle(['source']), 'Sources');
same('two kinds: "A and B"', nodeGroupTitle(['seed', 'model']), 'Models and seeds');
same('three kinds: "A, B and C", in the canvas order', nodeGroupTitle(['seed', 'source', 'model']), 'Models, sources and seeds');
same('a kind the box does not ask for is no title', nodeGroupTitle([]), 'Nodes');

function nodeHit(name, kind, file) { return { kind: 'node', node: { name: name, kind: kind || 'model', file: file || 'models/' + name + '.sql' }, label: name, sub: file || 'models/' + name + '.sql' }; }
function macroHit(call) { return { kind: 'macro', macro: { call: call, name: call, file: 'macros/' + call + '.sql' }, label: call, sub: 'macros/' + call + '.sql' }; }
function fileHit(path) { return { kind: 'file', path: path, label: path.split('/').pop(), sub: path }; }
function many(make, n) { var out = []; for (var i = 0; i < n; i++) out.push(make('x' + i)); return out; }
var none = { nodes: false, macros: false, files: false };
var noneOpen = new Set();

var few = paletteGroups([nodeHit('a'), nodeHit('b', 'seed'), macroHit('m'), fileHit('docs/readme.md')], 'a', none, noneOpen);
same('groups in order: nodes, macros, files', few.map(function (g) { return g.key; }).join(), 'nodes,macros,files');
same('the node group is titled by what it holds', few[0].title, 'Models and seeds');
same('a short group has no row for the rest', few[0].rows.length, 2);
same('and says how many it holds', few.map(function (g) { return g.count; }).join(), '2,1,1');

var long = paletteGroups(many(nodeHit, 8), 'x', none, noneOpen)[0];
same('five rows then a row for the rest', long.rows.map(function (r) { return r.kind; }).join(), 'node,node,node,node,node,more');
same('which says how many and of what', long.rows[5].label, '+3 more models');
same('the header counts them all', long.count, '8');
var wide = paletteGroups(many(nodeHit, 3).concat(many(function (n) { return nodeHit(n, 'seed'); }, 4)), 'x', none, noneOpen)[0];
same('the words are the title, lower case', wide.rows[5].label, '+2 more models and seeds');
same('exactly five has no row for the rest', paletteGroups(many(fileHit, 5), 'x', none, noneOpen)[0].rows.length, 5);
same('six has one: +1 more', paletteGroups(many(fileHit, 6), 'x', none, noneOpen)[0].rows[5].label, '+1 more files');
var opened = paletteGroups(many(nodeHit, 8), 'x', none, new Set(['nodes']))[0];
same('a group opened shows every row, and no row for the rest', opened.rows.length === 8 && opened.rows.every(function (r) { return r.kind === 'node'; }), true);
same('another group stays folded', paletteGroups(many(nodeHit, 8).concat(many(fileHit, 8)), 'x', none, new Set(['nodes'])).map(function (g) { return g.rows.length; }).join(), '8,6');

same('the Search tab is offered from three letters', paletteGroups([], 'ord', none, noneOpen).map(function (g) { return g.key; }).join(), 'grep');
same('not from two', paletteGroups([], 'or', none, noneOpen).length, 0);
same('spaces do not count', paletteGroups([], ' or ', none, noneOpen).length, 0);
var grepRow = paletteGroups([nodeHit('orders')], ' ord ', none, noneOpen)[1];
same('it is last, with no count, and carries the query trimmed', [grepRow.title, grepRow.count, grepRow.rows[0].kind, grepRow.rows[0].q], ['Inside the files', '', 'grep', 'ord']);
same('the empty box lists no macros', paletteGroups([macroHit('m')], '', none, noneOpen).length, 0);

var cut = { nodes: true, macros: false, files: true };
var cutNodes = paletteGroups(many(nodeHit, 40), 'x', cut, noneOpen)[0];
same('a group the server cut says 40+', cutNodes.count, '40+');
same('and its row for the rest does not claim the number', cutNodes.rows[5].label, '35+ more models');
var cutOpen = paletteGroups(many(nodeHit, 40), 'x', cut, new Set(['nodes']))[0];
same('opened, it ends on a line that cannot be picked', [cutOpen.rows.length, cutOpen.rows[40].kind, cutOpen.rows[40].label],
  [41, 'note', 'The search stops at 40 a group: type more letters for the rest.']);
same('a group not cut has no such line', paletteGroups(many(fileHit, 40), 'x', { nodes: true, macros: false, files: false }, new Set(['files']))[0].rows.length, 40);
var fewCut = paletteGroups(many(fileHit, 3), 'x', { nodes: false, macros: false, files: true }, noneOpen)[0];
same('a cut group left with five rows or fewer still says so', [fewCut.count, fewCut.rows.length, fewCut.rows[3].kind], ['3+', 4, 'note']);
same('a group not cut has no plus', paletteGroups(many(fileHit, 40), 'x', none, noneOpen)[0].count, '40');

same('the foot counts the hits', paletteCount([nodeHit('a'), fileHit('x.md')], none), '2 results');
same('one result', paletteCount([nodeHit('a')], none), '1 result');
same('a cut makes the count a floor', paletteCount(many(nodeHit, 41), cut), '41+ results');
same('no hits', paletteCount([], none), '0 results');

same('the letters are marked in the name', paletteMarks('stg_orders', 'models/staging/stg_orders.sql', 'ORD'),
  { name: ['stg_', 'ord', 'ers'], path: ['models/staging/stg_orders.sql', '', ''] });
same('and in the path when the name lacks them', paletteMarks('stg_x', 'models/staging/stg_x.sql', 'staging'),
  { name: ['stg_x', '', ''], path: ['models/', 'staging', '/stg_x.sql'] });
same('nowhere when nothing is typed', paletteMarks('stg_x', 'models', ' '),
  { name: ['stg_x', '', ''], path: ['models', '', ''] });
same('a file at the root has no path to mark', paletteMarks('.env', '', 'zzz').path, ['', '', '']);

// Answers in the order asked, whatever order the server sends them in.
eval(src.slice(src.indexOf('function lastAsked'), src.indexOf('/* Nodes first, then macros')));

var pending = {}, shown = [];
function fakeFetch(q) { return new Promise(function (resolve) { pending[q] = resolve; }); }
var answers = lastAsked(fakeFetch, function (hits, q) { shown.push(q + '=' + hits); });
var asyncChecks = 0, asyncWant = 10;
function turns(n) { var p = Promise.resolve(); while (n--) p = p.then(function () {}); return p; }
function later(label, f) { asyncChecks++; check(label, f()); }

// Marks of files in folders nothing listed: each folder asked once, true when a path gained one.
var S = { fileMarks: new Map() }, asked = [];
var api = { get: function (u) {
  asked.push(u);
  return Promise.resolve([{ path: 'models/a.sql', kind: 'model', materialized: 'view' }, { path: 'models/b.sql' }]);
} };
eval(src.slice(src.indexOf('const markOf'), src.indexOf("/* A file's icon.")));
asyncWant += 3;
var marksDone = askMarks(['models/a.sql', 'models/b.sql', 'README.md']).then(function (gained) {
  later('a path that gained a mark says so', function () { return gained === true && S.fileMarks.has('models/a.sql'); });
  later('only the folder of a file that could have one was asked, once', function () {
    return asked.join() === '/api/dir?path=models'; });
  return askMarks(['models/b.sql', 'models/c.sql']);
}).then(function (gained) {
  later('a folder already asked is not asked again, and nothing new is no news', function () {
    return asked.length === 1 && gained === false; });
});


answers.ask('');
answers.ask('d');
answers.ask('do');
pending.do('do-rows');
pending.d('d-rows');      // the shorter word comes back last
pending['']('empty-rows'); // and the empty box's listing after it
var q = Promise.resolve().then(function () { return answers.settled(); }).then(function () {
  later('only the last query asked is shown', function () { return shown.join() === 'do=do-rows'; });
  shown = [];
  // Enter right after a key: settled waits for that key's answer.
  answers.ask('dol');
  var entered = answers.settled().then(function () { return shown.join(); });
  pending.dol('dol-rows');
  return entered;
}).then(function (seen) {
  later('settled waits for the query typed last', function () { return seen === 'dol=dol-rows'; });
  shown = [];
  // A key pressed while waiting: settled follows it to its own answer.
  answers.ask('doll');
  var done = false;
  var entered = answers.settled().then(function () { done = true; return shown.join(); });
  answers.ask('dolla');
  pending.doll('doll-rows');
  return turns(20).then(function () {
    later('the answer to a key pressed before is no reason to stop waiting', function () { return !done; });
    pending.dolla('dolla-rows');
    return entered;
  });
}).then(function (seen) {
  later('a key pressed while waiting is waited for too', function () { return seen === 'dolla=dolla-rows'; });
});

// What a pick does with each kind of row.
eval(src.slice(src.indexOf('function choosePalette'), src.indexOf('// ------------------------------------------------------------------ tabs --')));
var did = [];
function closePalette() { did.push('close'); }
function closePanel() { did.push('panel'); }
function pushJump() { did.push('jump'); }
function openMacro(m) { did.push('macro:' + m.file + ':' + m.name); }
function openFile(p) { did.push('open:' + p); }
function revealInTree() {}
function focusNode(id) { did.push('focus:' + id); }
function showSide(w) { did.push('side:' + w); }
function runGrep() { did.push('grep:' + grepBox.value); }
var grepBox = { value: '' }, inputBox = { focus: function () { did.push('focus'); }, setAttribute: function () {}, removeAttribute: function () {} };
function $(sel) { return sel === '#grep-input' ? grepBox : inputBox; }
function $$() { return []; }
var palGroupsOpen = new Set();
var palIndex = 0;
var palHits = mergeHits([], [{ call: 'automate_dv.hub', name: 'hub', file: 'dbt_packages/automate_dv/macros/tables/hub.sql' }],
  ['macros/money.sql']);
q = q.then(function () {
  choosePalette(0);
  later('a macro opens through openMacro, which records the jump itself', function () {
    return did.join() === 'close,panel,macro:dbt_packages/automate_dv/macros/tables/hub.sql:hub'; });
  did = [];
  choosePalette(1);
  later('a file records the jump here', function () { return did.join() === 'close,panel,jump,open:macros/money.sql'; });
  did = [];
  choosePalette(5);
  later('no row there, nothing done', function () { return did.length === 0; });
  // The row for the rest shows the group where it is: nothing closes, the focus stays.
  // Assigned, not declared: the real one is evaluated further down.
  drawPalette = function () { did.push('draw'); };
  palHits = [{ kind: 'node', node: { id: 'n' } }, { kind: 'more', group: 'nodes', label: '+3 more models' }];
  palIndex = 1;
  choosePalette();
  later('the row for the rest opens its group and keeps the box', function () {
    return did.join() === 'draw,focus' && palGroupsOpen.has('nodes') && palIndex === 1; });
  palIndex = 0;
  choosePalette(1);
  later('a click on it selects it before the group is drawn', function () { return palIndex === 1; });
  did = [];
  // The grep row hands the letters to the Search tab.
  palHits = [{ kind: 'grep', q: 'cents' }];
  choosePalette(0);
  later('the grep row closes the box and opens the Search tab with the query', function () {
    return did.join() === 'close,panel,side:search,grep:cents' && grepBox.value === 'cents'; });
});

// Enter: in the box it was pressed in, on the row the arrows picked.
eval(src.slice(src.indexOf('async function enterPalette'), src.indexOf('function showPalette')));
eval(src.slice(src.indexOf('function movePalette'), src.indexOf('function choosePalette')));
var palOpened = 0, palMoved = false, picked = [];
function row() { return { id: 'r', classList: { add: function () {}, remove: function () {} }, setAttribute: function () {}, scrollIntoView: function () {} }; }
var rows = [row(), row(), row()];
$$ = function () { return rows; };
var palAnswers = lastAsked(fakeFetch, function () { palIndex = 0; palMoved = false; });
asyncWant += 5;
q = q.then(function () {
  // Assigned here, not declared: a declaration would be hoisted and replaced
  // by the real one evaluated above, and the checks of the real one run first.
  choosePalette = function () { picked.push(palIndex); };
  // Typed, Enter while it loads, Esc, opened again: the answer opens nothing.
  palOpened++; palAnswers.ask('foo');
  var entered = enterPalette();
  palOpened++;                      // Esc
  palOpened++; palAnswers.ask('');  // Cmd+K again
  pending.foo('foo-rows');
  pending['']('empty-rows');
  return entered;
}).then(function () {
  later('an Enter left waiting does nothing in a box opened since', function () { return picked.length === 0; });
  // Rows of fewer letters shown, the next key's still coming, two arrows down.
  palMoved = false; palIndex = 0;
  selectPalette(1, false);   // what the pointer does
  later('a pointer selection leaves the row unpicked, so Enter still waits for the answer', function () {
    return palMoved === false && palIndex === 1; });
  palIndex = 0;
  palAnswers.ask('stg_ord');
  movePalette(1); movePalette(1);
  var entered = enterPalette();
  later('a row the arrows moved to is opened at once', function () { return picked.join() === '2'; });
  pending.stg_ord('stg_ord-rows');
  return entered;
}).then(function () {
  later('and once only', function () { return picked.join() === '2'; });
  picked = [];
  // Typed, no arrow: Enter waits for that answer and takes its first row.
  palIndex = 2;
  palAnswers.ask('stg_orders');
  var entered = enterPalette();
  pending.stg_orders('stg_orders-rows');
  return entered;
}).then(function () {
  later('Enter after a key takes the first row of what it lists', function () { return picked.join() === '0'; });
}).then(function () {
  check('every asynchronous check ran', asyncChecks === asyncWant);
}, function (e) { print('FAIL  ' + e); });
