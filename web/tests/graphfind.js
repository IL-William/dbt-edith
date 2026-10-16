// Find on the lineage canvas (0081): which names a box is found by, how well a
// query names it, the order Enter steps in, where a step reads from, the box
// typing lands on, and the row or chip a folded test is found under, opened
// in at most two turns. Then the page's half: which pane Cmd+F and Cmd+G
// reach, the bar's keys, its count, and where the focus goes back.
// Run from the repository root: jsc web/tests/graphfind.js
var lin = read('web/lineage.js');
eval(lin.slice(lin.indexOf('const MAT = {'), lin.indexOf('let svg, root')));
eval(lin.slice(lin.indexOf('function findNames'), lin.indexOf('function pastSlop')));

function check(label, got, want) {
  var g = JSON.stringify(got), w = JSON.stringify(want);
  print((g === w ? 'PASS  ' : 'FAIL  ') + label + (g === w ? '' : '\n        expected ' + w + '\n        got      ' + g));
}
function group(name, fn) {
  print('\n--- ' + name + ' ---');
  try { fn(); } catch (e) { print('FAIL  ' + name + ' threw ' + e); }
}

var MODEL = { w: 200, h: 48, hgap: 80, vgap: 14, lane: 10, badge: 0, bundle: 4 };
var STEP = MODEL.w + MODEL.hgap;

group('the names a box is found by', function () {
  check('a model, by its name', findNames({ name: 'stg_orders', kind: 'model' }, false), ['stg_orders']);
  check('a source, by its table alone too', findNames({ name: 'shop.raw_orders', kind: 'source' }, false), ['shop.raw_orders', 'raw_orders']);
  check('a test, by its generic and its column, which a hashed name may have lost',
    findNames({ name: 'dbt_utils_accepted_range_orde_4a1f0c2e', kind: 'test', test_name: 'accepted_range', column: 'amount' }, false),
    ['dbt_utils_accepted_range_orde_4a1f0c2e', 'accepted_range', 'amount']);
  check('a singular test, by its name alone', findNames({ name: 'assert_orders_positive', kind: 'test' }, false), ['assert_orders_positive']);
  check('a column, by the model its second line names', findNames({ name: 'customer_id', kind: 'model', sub: 'stg_orders  ·  NUMBER' }, true),
    ['customer_id', 'stg_orders']);
  check('  and with no type, the whole line is the model', findNames({ name: 'id', kind: 'model', sub: 'orders' }, true), ['id', 'orders']);
});

group('how well a query names it', function () {
  var names = ['stg_orders'];
  check('the whole name, in any case', [findQuality(names, 'stg_orders'), findQuality(names, 'STG_Orders')], [0, 0]);
  check('its start', findQuality(names, 'stg_'), 1);
  check('a part of it', findQuality(names, 'orders'), 2);
  check('nothing of it', findQuality(names, 'customers'), -1);
  check('nothing typed matches nothing', findQuality(names, ''), -1);
  check('the best of its names counts: a source\'s table named whole',
    findQuality(['shop.raw_orders', 'raw_orders'], 'raw_orders'), 0);
  check('a dot and brackets mean themselves', [findQuality(['shop.raw'], 'p.r'), findQuality(['a[1]'], '[1]')], [2, 2]);
});

// Three columns: a source, two models under it, and orders with its tests.
function canvas() {
  var names = ['shop.raw_orders', 'stg_orders', 'stg_payments', 'orders', 'customers'];
  var kinds = ['source', 'model', 'model', 'model', 'model'];
  var nodes = names.map(function (s, i) { return { id: kinds[i] + '.shop.' + s, name: s, depth: 0, kind: kinds[i] }; });
  var at = {};
  nodes.forEach(function (n, i) { at[n.name] = i; });
  var edges = [[0, 1], [0, 2], [1, 3], [2, 3], [1, 4]];
  function test(name, host, extra) {
    var n = { id: 'test.shop.' + name, name: name, depth: 0, kind: 'test', attached: host };
    Object.keys(extra || {}).forEach(function (k) { n[k] = extra[k]; });
    at[name] = nodes.length;
    nodes.push(n);
    edges.push([host, nodes.length - 1]);
  }
  // orders: seven rows, three singular tests, then accepted_values alone, a
  // closed group of four not_null, relationships alone and a group of five
  // unique, the last two folded under the chip.
  ['d', 'b', 'c', 'a'].forEach(function (c) { test('not_null_orders_' + c, at.orders, { test_name: 'not_null', column: c }); });
  ['p', 'q', 'r', 's', 't'].forEach(function (c) { test('unique_orders_' + c, at.orders, { test_name: 'unique', column: c }); });
  test('accepted_values_orders_status', at.orders, { test_name: 'accepted_values', column: 'status' });
  test('relationships_orders_customer_id', at.orders, { test_name: 'relationships', column: 'customer_id' });
  test('assert_orders_positive', at.orders);
  test('assert_orders_recent', at.orders);
  test('assert_orders_signed', at.orders);
  test('unique_customers_id', at.customers, { test_name: 'unique', column: 'id' });
  return { mode: 'model', nodes: nodes, edges: edges, at: at };
}
// Where each node is drawn, or stands while folded, as matches() hands it on.
function anchors(d, L) {
  return d.nodes.map(function (_, v) {
    if (L.boxes[v]) return L.boxes[v];
    var u = L.under && L.under.get(v);
    return u ? { x: u.box.x, y: u.box.y, i: u.i } : null;
  });
}
function named(d, list) { return list.map(function (v) { return d.nodes[v].name; }); }

group('what a folded test is found under', function () {
  var d = canvas(), L = dagLayout(d, MODEL, null, new Set());
  var folded = d.nodes.map(function (_, v) { return v; }).filter(function (v) { return !L.boxes[v]; });
  check('every test not drawn stands somewhere', folded.every(function (v) { return L.under.has(v); }), true);
  check('none that is drawn does', d.nodes.every(function (_, v) { return !L.boxes[v] || !L.under.has(v); }), true);
  var keys = {};
  folded.forEach(function (v) { keys[L.under.get(v).key] = (keys[L.under.get(v).key] || 0) + 1; });
  check('under the chip, by its host\'s id, or under a closed group\'s row, by its key',
    Object.keys(keys).sort().map(function (k) { return k + ' ' + keys[k]; }),
    ['model.shop.orders 6', 'model.shop.orders#column:not_null 4']);
  var stands = (L.chips || []).concat(L.groups || []);
  check('and that row or chip is one the canvas draws',
    folded.every(function (v) { return stands.indexOf(L.under.get(v).box) >= 0; }), true);
  check('a canvas with nothing folded has no map at all', dagLayout(canvas(), MODEL, null, new Set(['model.shop.orders',
    groupKey('model.shop.orders', 'column', '', 'not_null'), groupKey('model.shop.orders', 'column', '', 'unique')])).under, undefined);

  // What a step does through findUnfold, with dagLayout for the redraw.
  var turns = 0, left = [];
  folded.forEach(function (v) {
    var o = new Set(), K = L, k = 0;
    var under = findUnfold(v, o, L.under, function () { k++; K = dagLayout(d, MODEL, null, o); return K.under || new Map(); });
    if (K.boxes[v] && !under.has(v)) turns = Math.max(turns, k);
    else left.push(d.nodes[v].name);
  });
  check('every folded test is drawn after two turns at most, a chip onto a closed group', [turns, left], [2, []]);
  var o = new Set(), calls = 0;
  findUnfold(d.at.orders, o, L.under, function () { calls++; return L.under; });
  check('a box already drawn opens nothing', [calls, o.size], [0, 0]);
  o = new Set();
  findUnfold(d.at.unique_orders_t, o, L.under, function () { return dagLayout(d, MODEL, null, o).under || new Map(); });
  check('the keys turned are the ones two clicks would turn: the chip, then the closed group under it',
    Array.from(o), ['model.shop.orders', groupKey('model.shop.orders', 'column', '', 'unique')]);
  o = new Set();
  findUnfold(d.at.not_null_orders_a, o, L.under, function () { return dagLayout(d, MODEL, null, o).under || new Map(); });
  check('  and one for a closed group outside the chip', Array.from(o), [groupKey('model.shop.orders', 'column', '', 'not_null')]);
});

group('where a step reads from', function () {
  check('nothing current or clicked: from the top forward', findStart(-1, -1, 1, false), 0);
  check('  and from the bottom backward', findStart(-1, -1, -1, false), -1);
  check('past the current match, either way', [findStart(3, 5, 1, false), findStart(3, 5, -1, false)], [4, 2]);
  check('past the box last clicked when none is current', [findStart(-1, 5, 1, false), findStart(-1, 5, -1, false)], [6, 4]);
  check('a current match typing left folded: forward starts on it, to open it', findStart(3, -1, 1, true), 3);
  check('  backward goes past it, as from any other', findStart(3, -1, -1, true), 2);
});

group('the order Enter steps in', function () {
  var d = canvas(), L = dagLayout(d, MODEL, null, new Set());
  var order = findOrder(anchors(d, L), STEP);
  check('column by column from the left, and down each, a model\'s tests after it',
    named(d, order).slice(0, 3), ['shop.raw_orders', 'stg_orders', 'stg_payments']);
  var o = order.indexOf(d.at.orders), u = order.indexOf(d.at.unique_customers_id), c = order.indexOf(d.at.customers);
  check('a test comes after its model and before the next box down its column',
    L.boxes[d.at.orders].y < L.boxes[d.at.customers].y ? [o < c, c < u] : [c < u, u < o], [true, true]);
  check('every node is in it once', order.slice().sort(function (a, b) { return a - b; }), d.nodes.map(function (_, v) { return v; }));

  // Opening what a test is folded under must not reorder the tests: a step
  // that lands on one opens it, and the next step goes on from there.
  var tests = function (ord) { return named(d, ord).filter(function (s) { return /^(not_null|unique_orders|assert|accepted|relationships)/.test(s); }); };
  var before = tests(order);
  var nn = groupKey('model.shop.orders', 'column', '', 'not_null');
  var un = groupKey('model.shop.orders', 'column', '', 'unique');
  var opened = [new Set(['model.shop.orders']), new Set(['model.shop.orders', nn, un])].map(function (open) {
    var K = dagLayout(d, MODEL, null, open);
    return tests(findOrder(anchors(d, K), STEP));
  });
  check('the tests of orders keep their order once the chip is opened', opened[0], before);
  check('  and once the groups under it are opened too', opened[1], before);
  check('  which is the order they are drawn in, a group\'s by column', before.slice(before.indexOf('not_null_orders_a'), before.indexOf('not_null_orders_a') + 4),
    ['not_null_orders_a', 'not_null_orders_b', 'not_null_orders_c', 'not_null_orders_d']);
});

group('the box typing lands on, and each step', function () {
  var d = canvas(), L = dagLayout(d, MODEL, null, new Set());
  var order = findOrder(anchors(d, L), STEP);
  var q = function (s) { return d.nodes.map(function (n) { return findQuality(findNames(n, false), s); }); };
  var name = function (v) { return v < 0 ? null : d.nodes[v].name; };
  check('a name typed whole finds that box before every one holding it', name(findPick(order, q('orders'), -1)), 'orders');
  check('  a source\'s table typed whole too', name(findPick(order, q('raw_orders'), -1)), 'shop.raw_orders');
  check('the start of names finds the first of them on the canvas', name(findPick(order, q('stg_'), -1)), 'stg_orders');
  check('typing on after a step stays on the box it landed on, while it matches as well',
    name(findPick(order, q('stg_'), order.indexOf(d.at.stg_payments))), 'stg_payments');
  check('  and reads on from it, round the canvas, when it no longer does',
    name(findPick(order, q('stg_o'), order.indexOf(d.at.stg_payments))), 'stg_orders');
  check('nothing matches', findPick(order, q('nowhere'), -1), -1);

  var hits = q('stg_');
  var a = findNext(order, hits, 0, 1);
  check('the first match from the top', name(a), 'stg_orders');
  var b = findNext(order, hits, order.indexOf(a) + 1, 1);
  check('the next one', name(b), 'stg_payments');
  check('round the canvas after the last', name(findNext(order, hits, order.indexOf(b) + 1, 1)), 'stg_orders');
  check('and backwards from the first to the last', name(findNext(order, hits, order.indexOf(a) - 1, -1)), 'stg_payments');
  check('backwards with none current, from the bottom', name(findNext(order, hits, -1, -1)), 'stg_payments');
  check('one match, every step is it', name(findNext(order, q('payments'), order.indexOf(d.at.stg_payments) + 1, 1)), 'stg_payments');
  check('no match, no step', findNext(order, q('nowhere'), 0, 1), -1);
  check('an empty canvas', [findNext([], [], 0, 1), findPick([], [], -1)], [-1, -1]);
});

group('a keystroke on a crowded canvas', function () {
  // 150 models in a chain, 20 tests under each: five rows and a chip apiece,
  // 2250 tests folded. What a keystroke costs before any redraw.
  var nodes = [], edges = [];
  for (var m = 0; m < 150; m++) {
    nodes.push({ id: 'model.shop.m' + m, name: 'model_' + m, depth: 0, kind: 'model' });
    if (m) edges.push([m - 1, m]);
  }
  for (m = 0; m < 150; m++) {
    for (var k = 0; k < 20; k++) {
      nodes.push({ id: 'test.shop.t' + m + '_' + k, name: 'not_null_model_' + m + '_c' + k, depth: 0, kind: 'test',
        attached: m, test_name: 'not_null', column: 'c' + k });
      edges.push([m, nodes.length - 1]);
    }
  }
  var d = { mode: 'model', nodes: nodes, edges: edges }, L = dagLayout(d, MODEL, null, new Set());
  var t0 = Date.now(), runs = 20, order, hits;
  for (var r = 0; r < runs; r++) {
    order = findOrder(anchors(d, L), STEP);
    var quality = d.nodes.map(function (n) { return findQuality(findNames(n, false), 'model_14'); });
    hits = order.filter(function (v) { return quality[v] >= 0; });
    findPick(order, quality, -1);
  }
  var ms = (Date.now() - t0) / runs;
  check('every folded test can be reached', d.nodes.every(function (_, v) { return L.boxes[v] || L.under.has(v); }), true);
  check('model_14 and its tests, model_140 to 149 and theirs', hits.length, 11 * 21);
  check('3150 nodes ordered and matched in under 50 ms a keystroke (' + ms.toFixed(1) + ' ms here)', ms < 50, true);
});

// The page's half (web/app.js): which pane a key reaches, the bar's own keys,
// what it says, and where the focus goes back. A few stand-in elements do for
// the DOM, and a recorder for Lineage.
var app = read('web/app.js');
eval(app.slice(app.indexOf('function findLabel'), app.indexOf('function findOverlay')));
eval(app.slice(app.indexOf('function findButton'), app.indexOf('/* One editor\'s bar')));
eval(app.slice(app.indexOf('function findInPane'), app.indexOf('const base = (path)')));
eval(app.slice(app.indexOf('function panelOpen'), app.indexOf('/* Rolls a panel down')));

function fakeEl(tag) {
  var e = { tagName: String(tag).toUpperCase(), className: '', value: '', title: '', textContent: '', placeholder: '',
    parent: null, kids: [], on: {}, attrs: {}, rects: [{}] };
  e.classList = {
    has: function (c) { return (' ' + e.className + ' ').indexOf(' ' + c + ' ') >= 0; },
    contains: function (c) { return this.has(c); },
    add: function (c) { if (!this.has(c)) e.className = (e.className + ' ' + c).trim(); },
    remove: function (c) { e.className = e.className.split(' ').filter(function (x) { return x && x !== c; }).join(' '); },
    toggle: function (c, on) { if (on === undefined ? !this.has(c) : on) this.add(c); else this.remove(c); },
  };
  e.setAttribute = function (k, v) { e.attrs[k] = String(v); };
  e.append = function () { for (var i = 0; i < arguments.length; i++) { arguments[i].parent = e; e.kids.push(arguments[i]); } };
  e.addEventListener = function (t, fn) { e.on[t] = fn; };
  e.contains = function (x) { for (; x; x = x.parent) if (x === e) return true; return false; };
  e.focus = function () { document.activeElement = e; };
  e.blur = function () { if (document.activeElement === e) document.activeElement = document.body; };
  e.select = function () {};
  e.getClientRects = function () { return e.rects; };
  Object.defineProperty(e, 'isConnected', { get: function () { return document.body.contains(e); } });
  return e;
}
var document = { createElement: fakeEl };
document.body = fakeEl('body');
document.activeElement = document.body;
var top = fakeEl('div'), selectInput = fakeEl('input'), modalBox = fakeEl('div');
modalBox.className = 'hidden';
document.body.append(top, selectInput, modalBox);
function $(sel) { return { '#lineage-top': top, '#modal': modalBox }[sel] || null; }
// A chip on the canvas: SVG, so no offsetParent, ever.
var chip = fakeEl('g');
document.body.append(chip);
function isTyping(el) { return !!el && el.tagName === 'INPUT'; }
var paneOfTarget = {};
function paneOf(el) { return el === document.body ? '' : paneOfTarget[el.tagName] || 'other'; }
var IS_MAC = true;
function keyLabel(k) { return k; }
function keyCombo(e) { return e.combo; }
var called = [];
var Lineage = {
  find: function (q, land) { called.push('find ' + JSON.stringify(q) + ' ' + land); },
  findStep: function (dir) { called.push('step ' + dir); },
  findEnd: function () { called.push('end'); },
  placeHeads: function () { called.push('heads'); },
};
var S = { dock: 'lineage', panel: '', graphMode: 'model', graphFind: null };
function focusColumnFilter() { called.push('column filter'); return true; }
function activeEditor() { return null; }
function openFind() { called.push('editor find'); }
function key(combo, target) {
  var e = { combo: combo, target: target || S.graphFind.input, prevented: false, preventDefault: function () { this.prevented = true; } };
  S.graphFind.bar.on.keydown(e);
  return e.prevented;
}
function take() { var c = called; called = []; return c; }

group('which pane Cmd+F and Cmd+G reach', function () {
  check('Cmd+F over the lineage opens its bar, paints the last query and moves nothing',
    [findInPane('lineage'), S.graphFind.open, S.graphFind.bar.classList.has('hidden'), take()],
    [true, true, false, ['heads', 'find "" false']]);
  check('  the box has the focus, in the bar on the canvas', [document.activeElement === S.graphFind.input, top.contains(S.graphFind.bar)], [true, true]);
  check('again with the bar open: the box again, nothing painted twice', [findInPane('lineage'), take()], [true, []]);
  closeGraphFind(); take();
  check('the Catalog keeps its column filter', [findInPane('catalog'), take()], [true, ['column filter']]);
  check('the terminal keeps the browser\'s find', [findInPane('terminal'), take()], [false, []]);
  S.dock = 'catalog';
  check('the lineage remembered but not on screen: the browser\'s', [findInPane('lineage'), S.graphFind.open], [false, false]);
  S.dock = 'lineage'; S.panel = 'reports';
  check('a panel over everything: the browser\'s', [findInPane('lineage'), S.graphFind.open], [false, false]);
  // The selection's panel holds the canvas, whatever tab the dock had (0092).
  S.dock = 'terminal'; S.panel = 'select';
  check('the canvas up in the selection\'s panel keeps its own', [findInPane('lineage'), S.graphFind.open, take()], [true, true, ['heads', 'find "" false']]);
  closeGraphFind(); take();
  S.dock = 'lineage'; S.panel = '';

  S.graphFind.input.value = '';
  check('Cmd+G from the canvas with no query yet: the browser\'s', [stepGraphFind('lineage', 1), take()], [false, []]);
  S.graphFind.input.value = 'stg_';
  check('with one: the bar opens on it and steps', [stepGraphFind('lineage', 1), S.graphFind.open, take()], [true, true, ['heads', 'find "stg_" false', 'step 1']]);
  check('  Shift+Cmd+G back, the bar already open', [stepGraphFind('lineage', -1), take()], [true, ['step -1']]);
  check('not from another pane', [stepGraphFind('editor', 1), stepGraphFind('catalog', 1), take()], [false, false, []]);
  S.dock = 'terminal';
  check('nor with the lineage off screen', stepGraphFind('lineage', 1), false);
  S.dock = 'lineage';
});

group('the bar\'s own keys', function () {
  take();
  check('Enter and Shift+Enter in the box step', [key('Enter'), key('Shift+Enter'), take()], [true, true, ['step 1', 'step -1']]);
  check('Cmd+G, F3 and their Shift forms too', [key('Mod+G'), key('F3'), key('Mod+Shift+G'), key('Shift+F3'), take()],
    [true, true, true, true, ['step 1', 'step 1', 'step -1', 'step -1']]);
  var close = S.graphFind.bar.kids[S.graphFind.bar.kids.length - 1];
  check('Enter on a button is the button\'s', [key('Enter', close), take()], [false, []]);
  check('a letter is the box\'s', [key('A'), take()], [false, []]);
  S.graphFind.input.value = 'orders';
  S.graphFind.input.on.input();
  check('typing lands, the query as typed', take(), ['find "orders" true']);
  check('Escape closes, the paint and the folder names with it', [key('Escape'), S.graphFind.open, take()], [true, false, ['end', 'heads']]);
  check('  and closing twice does nothing', [closeGraphFind(), take()], [undefined, []]);
});

group('what the bar says', function () {
  openGraphFind(); take();
  S.graphFind.input.value = 'stg_';
  paintGraphFind({ hits: ['a', 'b', 'c'], at: 1 });
  check('where the current match is in reading order', [S.graphFind.count.textContent, S.graphFind.count.classList.has('none')], ['2 of 3', false]);
  paintGraphFind({ hits: ['a', 'b'], at: -1 });
  check('the matches alone before any lands', S.graphFind.count.textContent, '2 results');
  S.graphFind.input.value = 'nowhere';
  paintGraphFind({ hits: [], at: -1 });
  check('none, marked so it reads in red', [S.graphFind.count.textContent, S.graphFind.count.classList.has('none')], ['No results', true]);
  S.graphFind.input.value = '';
  paintGraphFind({ hits: [], at: -1 });
  check('nothing typed says nothing', [S.graphFind.count.textContent, S.graphFind.count.classList.has('none')], ['', false]);
  check('a node in model mode', S.graphFind.input.placeholder, 'Find a node');
  S.graphMode = 'column';
  paintGraphFind({ hits: [], at: -1 });
  check('a column in column mode', [S.graphFind.input.placeholder, S.graphFind.input.attrs['aria-label']], ['Find a column', 'Find a column on the canvas']);
  S.graphMode = 'model';
  closeGraphFind();
  S.graphFind.open = false;
  paintGraphFind({ hits: ['a'], at: 0 });
  check('closed, it says nothing new', S.graphFind.count.textContent, '');
});

group('where the focus goes back', function () {
  selectInput.focus();
  openGraphFind();
  check('opened from the selector\'s box, the bar takes the focus', document.activeElement === S.graphFind.input, true);
  key('Escape');
  check('  and Escape gives it back', document.activeElement === selectInput, true);
  document.activeElement = document.body;
  openGraphFind();
  key('Escape');
  check('opened from the canvas, nothing has it after', document.activeElement === document.body, true);
  selectInput.focus();
  openGraphFind();
  selectInput.rects = [];
  key('Escape');
  check('what had it hidden since: nothing has it', document.activeElement === document.body, true);
  selectInput.rects = [{}];
  chip.focus();
  openGraphFind();
  key('Escape');
  check('a chip on the canvas, which is SVG, gets it back too', document.activeElement === chip, true);
  chip.focus();
  openGraphFind();
  chip.parent.kids.splice(chip.parent.kids.indexOf(chip), 1);
  chip.parent = null;
  key('Escape');
  check('  unless a redraw took the chip away: nothing has it', document.activeElement === document.body, true);
  selectInput.focus();
  openGraphFind();
  document.activeElement = document.body;
  closeGraphFind();
  check('closed with the focus out of the bar, it stays where it is', document.activeElement === document.body, true);
  selectInput.focus();
  openGraphFind();
  S.graphFind.input.focus();
  openGraphFind();
  key('Escape');
  check('Cmd+F again from inside the bar keeps the first place to go back to', document.activeElement === selectInput, true);
});

group('which Escape from outside the bar closes it', function () {
  paneOfTarget = { SVG: 'lineage', DIV: 'catalog', INPUT: 'lineage' };
  S.lastPane = 'lineage';
  var esc = function (target, prevented) { return { target: target, defaultPrevented: !!prevented }; };
  var canvas = fakeEl('svg');
  openGraphFind();
  check('from the canvas after a click on a box', escapesGraphFind(esc(canvas)), true);
  check('from the page itself, the lineage the pane last used', escapesGraphFind(esc(document.body)), true);
  check('not once something has taken it', escapesGraphFind(esc(canvas, true)), false);
  check('not from a text box, the selector\'s say', escapesGraphFind(esc(selectInput)), false);
  check('not from another pane', escapesGraphFind(esc(fakeEl('div'))), false);
  S.lastPane = 'catalog';
  check('not from the page when another pane was the last used', escapesGraphFind(esc(document.body)), false);
  S.lastPane = 'lineage';
  modalBox.className = '';
  check('not with a dialog open over the page', escapesGraphFind(esc(document.body)), false);
  modalBox.className = 'hidden';
  S.panel = 'reports';
  check('not under a panel', escapesGraphFind(esc(canvas)), false);
  S.panel = '';
  S.dock = 'terminal';
  check('not with the lineage off screen', escapesGraphFind(esc(canvas)), false);
  S.dock = 'lineage';
  closeGraphFind();
  check('not with the bar closed', escapesGraphFind(esc(canvas)), false);
});
