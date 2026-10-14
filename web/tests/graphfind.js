// Find on the lineage canvas (0081): which names a box is found by, how well a
// query names it, the order Enter steps in, the box typing lands on, and the
// row or chip a folded test is found under, opened in at most two turns.
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

  // What go() does: turn the key a test stands under, lay out again, twice at most.
  var open = new Set(), turns = 0;
  folded.forEach(function (v) {
    var o = new Set(open), K = dagLayout(d, MODEL, null, o), k = 0;
    for (; k < 2 && !K.boxes[v] && K.under && K.under.has(v); k++) {
      var key = K.under.get(v).key;
      if (o.has(key)) o.delete(key); else o.add(key);
      K = dagLayout(d, MODEL, null, o);
    }
    if (K.boxes[v]) turns = Math.max(turns, k);
    else turns = Infinity;
  });
  check('every folded test is drawn after two turns at most, a chip onto a closed group', turns, 2);
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
