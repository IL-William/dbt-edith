// Search palette: nodes, macros and plain files in one list, without duplicates.
// Run from the repository root: jsc web/tests/palette.js
var src = read('web/app.js');
// A single eval: a `const` declaration does not leak from one eval to the next,
// so `base` has to live in the same scope as `mergeHits`.
eval(src.slice(src.indexOf('const base = (path)'), src.indexOf('const dirOf'))
   + src.slice(src.indexOf('function mergeHits'), src.indexOf('/* One box for everything')));

function check(label, ok) { print((ok ? 'PASS  ' : 'FAIL  ') + label); }

var nodes = [
  { id: 'model.p.stg_x', name: 'stg_x', file: 'models/stg/stg_x.sql' },
  { id: 'source.p.raw.y', name: 'raw.y', file: '' },
];
var paths = [
  'models/stg/stg_x.sql',                                  // already represented by a node
  'tests/generic/data_quality/assert_ids_ordered.sql',     // generic test: not a node
  '.env',                                                  // not a node at all
  'macros/project_ops/audit.sql',
];
var out = mergeHits(nodes, [], paths);

check('nodes come first', out[0].kind === 'node' && out[1].kind === 'node');
check('a model .sql is not listed twice', out.filter(function (h) {
  return h.sub === 'models/stg/stg_x.sql'; }).length === 1);
check('a generic test can be found', out.some(function (h) {
  return h.kind === 'file' && h.label === 'assert_ids_ordered.sql'; }));
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

// Answers in the order asked, whatever order the server sends them in.
eval(src.slice(src.indexOf('function lastAsked'), src.indexOf('/* Nodes first, then macros')));

var pending = {}, shown = [];
function fakeFetch(q) { return new Promise(function (resolve) { pending[q] = resolve; }); }
var answers = lastAsked(fakeFetch, function (hits, q) { shown.push(q + '=' + hits); });
var asyncChecks = 0, asyncWant = 7;
function turns(n) { var p = Promise.resolve(); while (n--) p = p.then(function () {}); return p; }
function later(label, f) { asyncChecks++; check(label, f()); }

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
});

// Enter: in the box it was pressed in, on the row the arrows picked.
eval(src.slice(src.indexOf('async function enterPalette'), src.indexOf('function showPalette')));
eval(src.slice(src.indexOf('function movePalette'), src.indexOf('function choosePalette')));
var palOpened = 0, palMoved = false, picked = [];
function row() { return { classList: { add: function () {}, remove: function () {} }, scrollIntoView: function () {} }; }
var rows = [row(), row(), row()];
function $$() { return rows; }
var palAnswers = lastAsked(fakeFetch, function () { palIndex = 0; palMoved = false; });
asyncWant += 4;
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
