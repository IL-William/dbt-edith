// The canvas drawn by folder (0029): which folder each node is filed under,
// the order the folders go in, a run of columns for each, and what the line
// under the canvas says about them.
// Run from the repository root: jsc web/tests/folders.js
var lin = read('web/lineage.js');
eval(lin.slice(lin.indexOf('const MAT = {'), lin.indexOf('let svg, root')));
var app = read('web/app.js');
eval(app.slice(app.indexOf('function selectKindCounts'), app.indexOf('async function loadSidecar')));

function check(label, got, want) {
  var g = JSON.stringify(got), w = JSON.stringify(want);
  print((g === w ? 'PASS  ' : 'FAIL  ') + label + (g === w ? '' : '\n        expected ' + w + '\n        got      ' + g));
}
function ok(label, cond) { print((cond ? 'PASS  ' : 'FAIL  ') + label); }
// A throw would stop the harness where it stands: this names the group and
// lets the others run.
function group(name, fn) {
  print('\n--- ' + name + ' ---');
  try { fn(); } catch (e) { print('FAIL  ' + name + ' threw ' + e); }
}

var MODEL = { w: 200, h: 48, hgap: 80, vgap: 14, lane: 10, badge: 0, bundle: 4 };
var COLUMN = { w: 180, h: 40, hgap: 80, vgap: 14, lane: 10, badge: 6, bundle: 4 };

// [name, file, kind, package]: kind defaults to model, package to shop.
function graph(specs, links) {
  var at = {};
  var nodes = specs.map(function (s, i) {
    at[s[0]] = i;
    var kind = s[2] || 'model';
    return { id: kind + '.' + (s[3] || 'shop') + '.' + s[0], name: s[0], kind: kind, file: s[1], depth: 0 };
  });
  return { mode: 'model', nodes: nodes, edges: links.map(function (l) { return [at[l[0]], at[l[1]]]; }), at: at };
}
function folders(d, project) {
  var n = d.nodes.length, R = dagRank(d.nodes);
  return dagFolders(d.nodes, d.edges, project === undefined ? 'shop' : project, dagLayers(n, d.edges, R).layer);
}
function names(F) { return F.bands.map(function (b) { return b.name; }); }
// Each node's folder, by name.
function filed(d, F) {
  var out = {};
  d.nodes.forEach(function (x, i) { out[x.name] = F.bands[F.band[i]].name; });
  return out;
}
function columnsOf(d, L) {
  var out = {};
  d.nodes.forEach(function (x, i) { out[x.name] = L.boxes[i].x / (MODEL.w + MODEL.hgap); });
  return out;
}

// A small shop in numbered layers: raw sources, cleaned models, a vault, marts.
var layered = graph([
  ['orders', 'models/shop/10_raw/raw.yml', 'source'],
  ['payments', 'models/shop/10_raw/raw.yml', 'source'],
  ['stg_orders', 'models/shop/20_clean/orders/stg_orders.sql'],
  ['stg_payments', 'models/shop/20_clean/payments/stg_payments.sql'],
  ['hub_order', 'models/shop/30_vault/hub_order.sql'],
  ['sat_order', 'models/shop/30_vault/sat_order.sql'],
  ['fct_orders', 'models/shop/40_marts/fct_orders.sql'],
], [['orders', 'stg_orders'], ['payments', 'stg_payments'], ['stg_orders', 'hub_order'], ['stg_orders', 'sat_order'],
  ['hub_order', 'fct_orders'], ['sat_order', 'fct_orders'], ['stg_payments', 'fct_orders']]);

group('which folder a node is filed under', function () {
  var F = folders(layered);
  check('the first level where the drawn models and sources split', names(F), ['10_raw', '20_clean', '30_vault', '40_marts']);
  check('read below what they all share', F.root, 'models/shop');
  check('a source goes under the folder its properties file sits in', filed(layered, F).orders, '10_raw');
  check('a model under the folder at that level, however deep it sits', filed(layered, F).stg_payments, '20_clean');
  check('each folder counts what it holds', F.bands.map(function (b) { return b.size; }), [2, 2, 2, 1]);

  var clean = graph([
    ['stg_orders', 'models/shop/20_clean/21_orders/stg_orders.sql'],
    ['stg_payments', 'models/shop/20_clean/22_payments/stg_payments.sql'],
    ['int_order_payments', 'models/shop/20_clean/23_joined/int_order_payments.sql'],
  ], [['stg_orders', 'int_order_payments'], ['stg_payments', 'int_order_payments']]);
  check('everything drawn in one folder: its sub-folders', names(folders(clean)), ['21_orders', '22_payments', '23_joined']);

  var one = graph([['stg_orders', 'models/shop/20_clean/21_orders/stg_orders.sql']], []);
  check('one model: its own folder', names(folders(one)), ['21_orders']);

  var seeded = graph(layered.nodes.map(function (x) { return [x.name, x.file, x.kind]; })
    .concat([['country_codes', 'seeds/reference/country_codes.csv', 'seed']]),
  [['country_codes', 'hub_order'], ['stg_orders', 'hub_order']]);
  var S = folders(seeded);
  check('a seed goes under the first folder of its own path', filed(seeded, S).country_codes, 'seeds');
  check('and does not lift the level: the models still split where they did', S.root, 'models/shop');

  var snap = graph([['stg_orders', 'models/shop/20_clean/stg_orders.sql'], ['hub_order', 'models/shop/30_vault/hub_order.sql'],
    ['orders_snapshot', 'snapshots/orders_snapshot.sql', 'snapshot']], [['stg_orders', 'orders_snapshot'], ['orders_snapshot', 'hub_order']]);
  check('a snapshot likewise', filed(snap, folders(snap)).orders_snapshot, 'snapshots');

  var stray = graph([
    ['dim_date', 'models/dim_date.sql'],
    ['legacy', 'models/sources.yml', 'source'],
    ['stg_orders', 'models/shop/20_clean/stg_orders.sql'],
    ['hub_order', 'models/shop/30_vault/hub_order.sql'],
  ], [['legacy', 'stg_orders'], ['stg_orders', 'hub_order'], ['dim_date', 'hub_order']]);
  var T = folders(stray);
  check('a model or a source filed higher up does not lift the level for the rest', T.root, 'models/shop');
  check('it keeps its own folder', [filed(stray, T).dim_date, filed(stray, T).legacy, filed(stray, T).hub_order],
    ['models', 'models', '30_vault']);

  var pkg = graph([
    ['stg_orders', 'models/staging/stg_orders.sql'],
    ['fct_orders', 'models/marts/fct_orders.sql'],
    ['stg_runs', 'models/staging/stg_runs.sql', 'model', 'audit_kit'],
  ], [['stg_orders', 'fct_orders'], ['stg_runs', 'fct_orders']]);
  var P = folders(pkg);
  check('a node from another package goes under the package, not the folder of this project with the same name',
    filed(pkg, P).stg_runs, 'audit_kit');
  ok('and is marked as one', P.bands[P.band[pkg.at.stg_runs]].package === true);
  check('when the project is not known, every node is filed by its path',
    filed(pkg, folders(pkg, '')).stg_runs, 'staging');

  var tested = graph([
    ['stg_orders', 'models/shop/20_clean/stg_orders.sql'],
    ['hub_order', 'models/shop/30_vault/hub_order.sql'],
    ['not_null_stg_orders_id', 'models/shop/20_clean/schema.yml', 'test'],
    ['relationships_hub_order_id', 'models/shop/20_clean/schema.yml', 'test'],
    ['assert_orders_balance', 'tests/assert_orders_balance.sql', 'test'],
  ], [['stg_orders', 'hub_order'], ['stg_orders', 'not_null_stg_orders_id'],
    ['stg_orders', 'relationships_hub_order_id'], ['hub_order', 'relationships_hub_order_id']]);
  var TT = folders(tested), tf = filed(tested, TT);
  check('a test goes with the model it tests', tf.not_null_stg_orders_id, '20_clean');
  check('with the later of two, whatever file declares it', tf.relationships_hub_order_id, '30_vault');
  check('a test drawn without what it tests is filed by its own path', tf.assert_orders_balance, 'tests');
  ok('a test makes no folder of its own when it has a model to go with',
    names(TT).indexOf('schema') < 0 && TT.bands.length === 3);

  check('nothing drawn, no folder', dagFolders([], [], 'shop'), null);
  var windows = graph([['stg_orders', 'models/shop/20_clean/stg_orders.sql'], ['hub_order', 'models/shop/30_vault/hub_order.sql']],
    [['stg_orders', 'hub_order']]);
  windows.mode = 'column';
  windows.nodes.forEach(function (x) { x.id += '::order_id'; });
  check('a column is filed under its model\'s folder, its id carrying the package', names(folders(windows)), ['20_clean', '30_vault']);
});

group('the order of the folders', function () {
  var plain = graph([
    ['stg_orders', 'models/staging/stg_orders.sql'],
    ['int_orders', 'models/intermediate/int_orders.sql'],
    ['fct_orders', 'models/marts/fct_orders.sql'],
  ], [['stg_orders', 'int_orders'], ['int_orders', 'fct_orders']]);
  check('by the edges between them, never by how their names sort', names(folders(plain)), ['staging', 'intermediate', 'marts']);

  // The one edge between the two folders this canvas draws runs from 30 to 20.
  var backwards = graph([
    ['country_codes', 'seeds/country_codes.csv', 'seed'],
    ['ref_country', 'models/shop/30_vault/ref_country.sql'],
    ['stg_country', 'models/shop/20_clean/stg_country.sql'],
    ['stg_country_named', 'models/shop/20_clean/stg_country_named.sql'],
  ], [['country_codes', 'ref_country'], ['ref_country', 'stg_country'], ['stg_country', 'stg_country_named']]);
  check('a numbered folder keeps its number\'s place, whichever way the edges drawn run',
    names(folders(backwards)), ['seeds', '20_clean', '30_vault']);
  var nine = graph([['a', 'models/9_clean/a.sql'], ['b', 'models/10_marts/b.sql']], [['b', 'a']]);
  check('numbers compare as numbers', names(folders(nine)), ['9_clean', '10_marts']);

  var tangled = graph([
    ['a1', 'models/core/a1.sql'], ['a2', 'models/core/a2.sql'], ['a3', 'models/core/a3.sql'],
    ['b1', 'models/edge/b1.sql'], ['b2', 'models/edge/b2.sql'],
  ], [['a1', 'b1'], ['a2', 'b1'], ['a3', 'b2'], ['b1', 'a3']]);
  check('two unnumbered folders feeding each other: the one fed least goes first', names(folders(tangled)), ['core', 'edge']);

  var loose = graph([
    ['orders', 'models/shop/10_raw/raw.yml', 'source'],
    ['stg_orders', 'models/shop/20_clean/stg_orders.sql'],
    ['fct_orders', 'models/shop/40_marts/fct_orders.sql'],
    ['country_codes', 'seeds/country_codes.csv', 'seed'],
  ], [['orders', 'stg_orders'], ['stg_orders', 'fct_orders']]);
  check('a seed nothing drawn reads starts the canvas with the inputs, not after every number',
    names(folders(loose)), ['10_raw', 'seeds', '20_clean', '40_marts']);

  var late = graph([
    ['stg_orders', 'models/shop/20_clean/stg_orders.sql'],
    ['hub_order', 'models/shop/30_vault/hub_order.sql'],
    ['fct_orders', 'models/shop/40_marts/fct_orders.sql'],
    ['fx_rates', 'seeds/fx_rates.csv', 'seed'],
  ], [['stg_orders', 'hub_order'], ['hub_order', 'fct_orders'], ['fx_rates', 'fct_orders']]);
  check('a seed only the marts read goes just before them', names(folders(late)), ['20_clean', '30_vault', 'seeds', '40_marts']);

  var shuffledNames = function (d) {
    var n = d.nodes.length, to = d.nodes.map(function (_, i) { return n - 1 - i; });
    return { mode: d.mode, nodes: d.nodes.slice().reverse(), edges: d.edges.map(function (e) { return [to[e[0]], to[e[1]]]; }).reverse() };
  };
  [layered, backwards, tangled, loose].forEach(function (d, k) {
    check('graph ' + (k + 1) + ': the same folders in the same order, whatever order the server lists them in',
      names(folders(shuffledNames(d))), names(folders(d)));
  });
});

group('each folder its own run of columns', function () {
  [layered, generatedLayers(150, 1), generatedLayers(400, 7)].forEach(function (d, k) {
    var R = dagRank(d.nodes), F = folders(d), lay = dagLayers(d.nodes.length, d.edges, R, F.band);
    var apart = true, right = true;
    for (var u = 0; u < d.nodes.length; u++) {
      for (var v = 0; v < d.nodes.length; v++) if (F.band[u] < F.band[v] && !(lay.layer[u] < lay.layer[v])) apart = false;
    }
    lay.pairs.forEach(function (p, i) {
      var a = p[lay.flip[i]], b = p[1 - lay.flip[i]];
      if (!(lay.layer[b] > lay.layer[a])) right = false;
    });
    ok('graph ' + (k + 1) + ': every node of a folder sits left of every node of a later one', apart);
    ok('graph ' + (k + 1) + ': and every edge points right once the ones against the folders are turned', right);
    var joined = lay.span.every(function (s, i) { return !i || s[0] === lay.span[i - 1][1] + 1; }), used = {};
    Array.prototype.forEach.call(lay.layer, function (l) { used[l] = 1; });
    var full = true;
    for (var c = 0; c <= lay.span[lay.span.length - 1][1]; c++) if (!used[c]) full = false;
    ok('graph ' + (k + 1) + ': the runs follow one another, and no column in them is left empty', joined && full);
  });

  var L = dagLayout(layered, MODEL, 'shop'), c = columnsOf(layered, L);
  check('one folder after another', [c.orders, c.stg_orders, c.hub_order, c.fct_orders], [0, 1, 2, 3]);

  // Without folders, the raw source only the marts read sits just before them.
  var early = graph([
    ['orders', 'models/shop/10_raw/raw.yml', 'source'],
    ['rates', 'models/shop/10_raw/raw.yml', 'source'],
    ['stg_orders', 'models/shop/20_clean/stg_orders.sql'],
    ['int_orders', 'models/shop/20_clean/int_orders.sql'],
    ['fct_orders', 'models/shop/40_marts/fct_orders.sql'],
  ], [['orders', 'stg_orders'], ['stg_orders', 'int_orders'], ['int_orders', 'fct_orders'], ['rates', 'fct_orders']]);
  check('without folders, a source only a late model reads sits beside it', columnsOf(early, dagLayout(early, MODEL)).rates, 2);
  check('with them, it is pulled right no further than its own folder', columnsOf(early, dagLayout(early, MODEL, 'shop')).rates, 0);

  var tested = graph([
    ['stg_orders', 'models/shop/20_clean/stg_orders.sql'],
    ['hub_order', 'models/shop/30_vault/hub_order.sql'],
    ['not_null_stg_orders_id', 'models/shop/20_clean/schema.yml', 'test'],
    ['unique_hub_order_id', 'models/shop/30_vault/schema.yml', 'test'],
  ], [['stg_orders', 'hub_order'], ['stg_orders', 'not_null_stg_orders_id'], ['hub_order', 'unique_hub_order_id']]);
  var tl = dagLayout(tested, MODEL, 'shop'), tb = function (s) { return tl.boxes[tested.at[s]]; };
  // A test takes no column of its own: it hangs under its model (0034), so it
  // stays in its model's band whatever folder its YAML sits in.
  check('a test hangs under its model, inside its model\'s band, and takes no column',
    [columnsOf(tested, tl).hub_order, tb('not_null_stg_orders_id').x - tb('stg_orders').x,
      tb('unique_hub_order_id').x - tb('hub_order').x, tb('unique_hub_order_id').y > tb('hub_order').y], [1, 18, 18, true]);

  var off = generatedLayers(150, 3);
  ok('off, the canvas is drawn exactly as it always was',
    JSON.stringify(dagLayout(off, MODEL)) === JSON.stringify(dagLayout(off, MODEL, null))
    && JSON.stringify(dagLayout(off, MODEL)) === JSON.stringify(dagLayout(off, MODEL, undefined)));
});

group('an edge against the folders', function () {
  var backwards = graph([
    ['country_codes', 'seeds/country_codes.csv', 'seed'],
    ['ref_country', 'models/shop/30_vault/ref_country.sql'],
    ['stg_country', 'models/shop/20_clean/stg_country.sql'],
    ['stg_country_named', 'models/shop/20_clean/stg_country_named.sql'],
  ], [['country_codes', 'ref_country'], ['ref_country', 'stg_country'], ['stg_country', 'stg_country_named']]);
  var L = dagLayout(backwards, MODEL, 'shop'), c = columnsOf(backwards, L);
  check('is counted', L.against, 1);
  check('and drawn against its direction, dashed like a loop in column mode', L.back, [0, 1, 0]);
  ok('the model it feeds stays in its own folder, left of the one reading it', c.stg_country < c.ref_country);
  check('without folders nothing is against anything', dagLayout(backwards, MODEL).against, undefined);

  // Loops inside one folder, which column lineage can hold, are still found.
  var loops = graph([
    ['amount', 'models/shop/20_clean/stg_payments.sql'], ['net', 'models/shop/20_clean/stg_payments.sql'],
    ['total', 'models/shop/30_vault/sat_payment.sql'],
  ], [['amount', 'net'], ['net', 'amount'], ['net', 'total'], ['total', 'amount'], ['total', 'total']]);
  loops.mode = 'column';
  var C = dagLayout(loops, COLUMN, 'shop');
  ok('in column mode, a loop inside a folder is turned as before and every number stays finite',
    C.paths.every(function (s) { return !/NaN|Infinity|undefined/.test(s); }) && C.back.length === 5);
  check('the edge from the later folder is the one against them', C.against, 1);
});

group('where the bands are drawn', function () {
  var L = dagLayout(layered, MODEL, 'shop');
  check('one per folder, in order', L.bands.map(function (b) { return b.name; }), ['10_raw', '20_clean', '30_vault', '40_marts']);
  ok('each covers its boxes, with room for a +N badge on either side', layered.nodes.every(function (x, i) {
    var b = L.bands[folders(layered).band[i]], box = L.boxes[i];
    return b.x0 <= box.x - 27 && b.x1 >= box.x + box.w + 27;
  }));
  ok('with a gutter between two', L.bands.every(function (b, i) { return !i || b.x0 > L.bands[i - 1].x1; }));
  ok('the frame holds them all', L.bbox.x0 <= L.bands[0].x0 && L.bbox.x1 >= L.bands[L.bands.length - 1].x1);
  var P = dagLayout(layered, MODEL);
  ok('and leaves room above the boxes for their names in an exported file', L.bbox.y0 < P.bbox.y0);
  check('with the folder each is, for its tooltip', L.bands[1].key, 'models/shop/20_clean');
});

group('the same picture every time', function () {
  [layered, generatedLayers(150, 1)].forEach(function (d, k) {
    var before = JSON.stringify(d), L = dagLayout(d, MODEL, 'shop');
    ok('graph ' + (k + 1) + ': twice the same', JSON.stringify(L) === JSON.stringify(dagLayout(d, MODEL, 'shop')));
    ok('graph ' + (k + 1) + ': the payload is left as it came', JSON.stringify(d) === before);
  });
});

group('fast enough to redraw on every click', function () {
  var big = generatedLayers(400, 11), s = 99;
  for (var i = 0; i < 200; i++) {
    s = (s * 1664525 + 1013904223) >>> 0;
    big.nodes.push({ id: 'test.shop.t' + i, name: 'not_null_' + i, kind: 'test', depth: 0, file: 'models/shop/schema.yml' });
    big.edges.push([s % 400, 400 + i]);
  }
  var t = Infinity;
  for (var r = 0; r < 3; r++) { var at = Date.now(); dagLayout(big, MODEL, 'shop'); t = Math.min(t, Date.now() - at); }
  ok('400 models in five folders and 200 tests, well under 100 ms (' + t + ' ms here)', t < 100);
});

group('the line under the canvas', function () {
  var sub = { mode: 'model', nodes: new Array(12), edges: new Array(15), truncated: false };
  check('without folders, as before', canvasStatus(sub), '12 nodes · 15 edges');
  // collin's indirect edges left out of a column graph are said (0097).
  check('a column graph says its indirect edges are hidden',
    canvasStatus({ mode: 'column', focus_column: 'loaded_at', nodes: new Array(2), edges: new Array(1), indirect_hidden: true }),
    'loaded_at · 2 columns · 1 edges · indirect edges hidden');
  check('with them, how many', canvasStatus(sub, { folders: 4, against: 0 }), '12 nodes · 15 edges · 4 folders');
  check('and the edges drawn against their order', canvasStatus(sub, { folders: 3, against: 1 }),
    '12 nodes · 15 edges · 3 folders · 1 edge against their order');
  check('in selection mode too', canvasStatus({ mode: 'select', matched: 3, nodes: new Array(3), edges: new Array(2) },
    { folders: 1, against: 2 }), '3 nodes · 2 edges · 1 folder · 2 edges against their order');
  check('nothing matched, nothing said', canvasStatus({ mode: 'select', matched: 0, nodes: [], edges: [] }, null), '');
  check('tests folded under their models are counted, folders or not', canvasStatus(sub, { folders: 0, against: 0, folded: 7 }),
    '12 nodes · 15 edges · 7 tests folded');
});

group('sub-folders, a level or more below the bands (0080)', function () {
  var nested = graph([
    ['orders', 'models/shop/10_raw/raw.yml', 'source'],
    ['stg_orders', 'models/shop/20_clean/21_orders/stg_orders.sql'],
    ['stg_payments', 'models/shop/20_clean/22_payments/stg_payments.sql'],
    ['int_order_payments', 'models/shop/20_clean/int_order_payments.sql'],
    ['customers', 'models/shop/30_marts/core/customers.sql'],
    ['revenue', 'models/shop/30_marts/finance/revenue.sql'],
    ['fct_orders', 'models/shop/40_serve/fct_orders.sql'],
  ], [['orders', 'stg_orders'], ['stg_orders', 'int_order_payments'], ['stg_payments', 'int_order_payments'],
    ['int_order_payments', 'customers'], ['customers', 'revenue'], ['revenue', 'fct_orders']]);
  var at = function (d, levels) {
    var n = d.nodes.length, R = dagRank(d.nodes);
    return dagFolders(d.nodes, d.edges, 'shop', dagLayers(n, d.edges, R).layer, levels);
  };
  var levelsOf = function (F) { return F.folders.map(function (f) { return f.level + ':' + f.name; }); };

  check('one level is the bands as they were', JSON.stringify(at(nested, 1).bands), JSON.stringify(folders(nested).bands));
  check('and so is a level the box cannot hold', JSON.stringify(at(nested, 0).band), JSON.stringify(folders(nested).band));
  // Written down from main before 0080, so that one level is held to what
  // main drew, not only to this code's own answer with no level given.
  var M = dagLayout(nested, MODEL, 'shop', undefined, 1);
  check('one level draws what main drew: the folders, each box where it stood, the bands\' edges', {
    band: Array.from(at(nested, 1).band), boxes: M.boxes.map(function (b) { return [b.x, b.y]; }),
    drawn: M.bands.map(function (b) { return [b.name, b.x0, b.x1]; }),
  }, {
    band: [0, 1, 1, 1, 2, 2, 3], boxes: [[0, -24], [280, -24], [280, 38], [560, 7], [840, 7], [1120, 7], [1400, 7]],
    drawn: [['10_raw', -30, 230], ['20_clean', 250, 790], ['30_marts', 810, 1350], ['40_serve', 1370, 1630]],
  });
  var fnv = function (str) {
    var h = 2166136261;
    for (var i = 0; i < str.length; i++) { h ^= str.charCodeAt(i); h = Math.imul(h, 16777619) >>> 0; }
    return h;
  };
  var G1 = dagLayout(generatedLayers(150, 1), MODEL, 'shop', undefined, 1);
  check('and on 150 generated models in five folders, the picture main drew, by its digest',
    fnv(G1.boxes.map(function (b) { return b.x + ':' + b.y; }).join(' ') + '|'
      + G1.bands.map(function (b) { return b.name + ':' + b.x0 + ':' + b.x1; }).join(' ')), 1222070158);

  var F = at(nested, 2);
  check('two levels: a run of columns for each sub-folder, inside its folder',
    names(F), ['10_raw', '21_orders', '22_payments', '20_clean', 'core', 'finance', '40_serve']);
  check('every folder drawn, the bands first, then their sub-folders',
    levelsOf(F), ['0:10_raw', '0:20_clean', '0:30_marts', '0:40_serve', '1:21_orders', '1:22_payments', '1:core', '1:finance']);
  check('a band covers the runs of all it holds', [F.folders[1].from, F.folders[1].to, F.folders[1].size], [1, 3, 3]);
  check('a model filed beside the sub-folders is a run of its own, with no sub-folder drawn round it',
    filed(nested, F).int_order_payments, '20_clean');
  check('placed by its edges, the folders it reads coming first, not by its folder\'s number', names(F).indexOf('20_clean'), 3);
  check('a folder with no sub-folder stays one run', filed(nested, F).fct_orders, '40_serve');
  check('three levels where the paths stop at two: the same', names(at(nested, 3)), names(F));

  var numbered = graph([
    ['stg_a', 'models/shop/20_clean/22_late/stg_a.sql'],
    ['stg_b', 'models/shop/20_clean/21_early/stg_b.sql'],
    ['orders', 'models/shop/30_marts/orders.sql'],
  ], [['stg_a', 'stg_b'], ['stg_b', 'orders']]);
  var N = at(numbered, 2), NL = dagLayout(numbered, MODEL, 'shop', undefined, 2);
  check('numbered sub-folders keep their numbers\' order inside their folder', names(N), ['21_early', '22_late', '30_marts']);
  check('and an edge against it is counted and dashed like one between bands', [NL.against, NL.back], [1, [1, 0]]);

  var pkg = graph([
    ['stg_orders', 'models/staging/orders/stg_orders.sql'],
    ['fct_orders', 'models/marts/fct_orders.sql'],
    ['stg_runs', 'models/staging/runs/stg_runs.sql', 'model', 'audit_kit'],
  ], [['stg_orders', 'fct_orders'], ['stg_runs', 'fct_orders']]);
  var P = at(pkg, 3);
  check('another package stays one band, its paths being its own', filed(pkg, P).stg_runs, 'audit_kit');
  ok('with no sub-folder', P.folders.filter(function (f) { return f.package && f.level; }).length === 0);
  check('while this project\'s folders are cut', filed(pkg, P).stg_orders, 'orders');

  var tested = graph([
    ['stg_orders', 'models/shop/20_clean/orders/stg_orders.sql'],
    ['customers', 'models/shop/30_marts/core/customers.sql'],
    ['relationships_customers_id', 'models/shop/20_clean/schema.yml', 'test'],
  ], [['stg_orders', 'customers'], ['stg_orders', 'relationships_customers_id'], ['customers', 'relationships_customers_id']]);
  check('a test still goes with the later of the models it tests, in its sub-folder', filed(tested, at(tested, 2)).relationships_customers_id, 'core');

  var shuffled = { mode: nested.mode, nodes: nested.nodes.slice().reverse(),
    edges: nested.edges.map(function (e) { return [6 - e[0], 6 - e[1]]; }).reverse() };
  check('the same sub-folders in the same order, whatever order the server lists them in', names(at(shuffled, 2)), names(F));

  [generatedLayers(150, 1), nested, numbered].forEach(function (d, k) {
    var R = dagRank(d.nodes), G = at(d, 3), lay = dagLayers(d.nodes.length, d.edges, R, G.band), apart = true;
    for (var u = 0; u < d.nodes.length; u++) {
      for (var v = 0; v < d.nodes.length; v++) if (G.band[u] < G.band[v] && !(lay.layer[u] < lay.layer[v])) apart = false;
    }
    ok('graph ' + (k + 1) + ': every node of a run sits left of every node of a later one', apart);
  });

  var L = dagLayout(nested, MODEL, 'shop', undefined, 2), plainL = dagLayout(nested, MODEL, 'shop');
  var top = L.bands.filter(function (b) { return !b.level; }), sub = L.bands.filter(function (b) { return b.level; });
  check('drawn: the bands, then the sub-folders', L.bands.map(function (b) { return b.name; }),
    ['10_raw', '20_clean', '30_marts', '40_serve', '21_orders', '22_payments', 'core', 'finance']);
  ok('each sub-folder inside its band, with the band showing round it', sub.every(function (b) {
    var p = top.filter(function (t) { return t.from <= b.from && b.to <= t.to; })[0];
    return p && p.x0 < b.x0 && b.x1 < p.x1;
  }));
  ok('and between two of them', sub.every(function (b, i) { return !i || b.x0 > sub[i - 1].x1; }));

  var heads = dagHeads(L.bands), by = {};
  L.bands.forEach(function (b, i) { by[b.level + ':' + b.name] = { b: b, h: heads[i] }; });
  check('each name on the row of its level', heads.map(function (h) { return h.row; }), L.bands.map(function (b) { return b.level; }));
  ok('a folder\'s name may run on over the gutter, up to the next folder\'s', by['0:20_clean'].h.x1 === by['0:30_marts'].b.x0);
  ok('a sub-folder\'s up to a sub-folder right beside it in the same folder', by['1:21_orders'].h.x1 === by['1:22_payments'].b.x0);
  ok('but never over the models filed beside the sub-folders', by['1:22_payments'].h.x1 === by['1:22_payments'].b.x1);
  ok('nor past the last of them', by['1:finance'].h.x1 === by['1:finance'].b.x1);
  var apart = graph([
    ['stg_orders', 'models/shop/20_clean/21_orders/stg_orders.sql'],
    ['customers', 'models/shop/30_marts/core/customers.sql'],
    ['revenue', 'models/shop/30_marts/finance/revenue.sql'],
  ], [['stg_orders', 'customers'], ['customers', 'revenue']]);
  var AL = dagLayout(apart, MODEL, 'shop', undefined, 2), AH = dagHeads(AL.bands);
  var ix = AL.bands.map(function (b) { return b.level + ':' + b.name; });
  var i21 = ix.indexOf('1:21_orders'), icore = ix.indexOf('1:core');
  ok('nor into the next folder\'s first sub-folder, even with no column between them',
    AL.bands[i21].to + 1 === AL.bands[icore].from && AH[i21].x1 === AL.bands[i21].x1 && AH[icore].x1 === AL.bands[icore + 1].x0);
  var lift = function (Lx) { return Math.min.apply(null, Lx.boxes.map(function (b) { return b.y; })) - Lx.bbox.y0; };
  check('a row of names kept above the graph for each level drawn', [lift(L) - lift(plainL), lift(dagLayout(nested, MODEL, 'shop', undefined, 3)) - lift(L)], [20, 0]);
  ok('each covers its boxes', nested.nodes.every(function (x, i) {
    var lf = F.band[i], box = L.boxes[i];
    return L.bands.every(function (b) { return b.from > lf || b.to < lf || (b.x0 < box.x && b.x1 > box.x + box.w); });
  }));
  var minY = Math.min.apply(null, L.boxes.map(function (b) { return b.y; }));
  ok('and the frame keeps a row above the boxes for each level of names', L.bbox.y0 <= minY - 40);
  check('a band keeps its fields without sub-folders', Object.keys(plainL.bands[0]).sort(),
    ['from', 'key', 'level', 'name', 'package', 'size', 'to', 'x0', 'x1']);
  ok('the same picture twice', JSON.stringify(L) === JSON.stringify(dagLayout(nested, MODEL, 'shop', undefined, 2)));

  var big = generatedLayers(400, 11);
  big.nodes.forEach(function (x, i) { x.file = x.file.replace(/\/m_/, '/' + (i % 3 ? 'a' : 'b') + '/' + (i % 2) + '/m_'); });
  var t = Infinity;
  for (var r = 0; r < 3; r++) { var s0 = Date.now(); dagLayout(big, MODEL, 'shop', undefined, 3); t = Math.min(t, Date.now() - s0); }
  ok('400 models in five folders cut twice, well under 100 ms (' + t + ' ms here)', t < 100);

  var line = { mode: 'model', nodes: new Array(12), edges: new Array(15), truncated: false };
  check('the line under the canvas counts the sub-folders', canvasStatus(line, { folders: 4, sub: 4, against: 1 }),
    '12 nodes · 15 edges · 4 folders · 4 sub-folders · 1 edge against their order');
  check('and says nothing of them on one level', canvasStatus(line, { folders: 4, sub: 0, against: 0 }), '12 nodes · 15 edges · 4 folders');

  var box = { value: '' }, $ = function () { return box; };
  eval(app.slice(app.indexOf('function folderLevels'), app.indexOf('function paintTestsToggle')));
  check('the box is read as a whole number from 1 to 4', ['2', '9', '0', '', 'x', '2.6'].map(function (v) { box.value = v; return folderLevels(); }),
    [2, 4, 1, 1, 1, 3]);
});

group('the levels box opens on the pointer and closes a moment after (0080)', function () {
  // Just enough of a page: elements that keep listeners and classes, a focus,
  // and timers that run when the test says how much time has passed.
  var pressed = false, now = 0, timers = [], seq = 0;
  function node() {
    var e = { on: {}, cls: {}, visible: false };
    e.addEventListener = function (t, f) { (e.on[t] = e.on[t] || []).push(f); };
    e.fire = function (t) { (e.on[t] || []).forEach(function (f) { f({}); }); };
    e.matches = function (sel) { return sel === ':focus-visible' && e.visible && document.activeElement === e; };
    e.classList = {
      toggle: function (c, on) { e.cls[c] = on === undefined ? !e.cls[c] : !!on; },
      remove: function (c) { e.cls[c] = false; },
    };
    return e;
  }
  var page = { '#folder-tool': node(), '#by-folder': node(), '#folder-levels': node(), '#folder-levels-box': node() };
  var $ = function (q) { return page[q]; }, document = { activeElement: null };
  var foldersOn = function () { return pressed; };
  var setTimeout = function (f, ms) { timers.push({ f: f, at: now + ms, id: ++seq }); return seq; };
  var clearTimeout = function (id) { timers = timers.filter(function (t) { return t.id !== id; }); };
  function wait(ms) {
    now += ms;
    timers.filter(function (t) { return t.at <= now; }).forEach(function (t) { clearTimeout(t.id); t.f(); });
  }
  eval(app.slice(app.indexOf('function folderLevels'), app.indexOf('function paintTestsToggle')));
  var tool = page['#folder-tool'], toggle = page['#by-folder'], input = page['#folder-levels'], box = page['#folder-levels-box'];
  var open = function () { return !!box.cls.open; };
  function focus(e, keyboard) {
    var from = document.activeElement;
    if (from) from.fire('focusout');
    if (from && from !== tool) tool.fire('focusout');
    if (e) e.visible = !!keyboard;
    document.activeElement = e;
    if (e) tool.fire('focusin');
  }
  wireFolderLevels(2500);

  tool.fire('mouseenter');
  check('with the toggle off, the pointer opens nothing: there are no bands to cut', open(), false);
  tool.fire('mouseleave');

  pressed = true;
  tool.fire('mouseenter');
  check('with it on, the pointer on the toggle opens it', open(), true);
  wait(10000);
  check('and it stays while the pointer does', open(), true);
  tool.fire('mouseleave'); wait(2400);
  check('left, it stays a moment, time to reach it', open(), true);
  wait(100);
  check('then closes', open(), false);

  tool.fire('mouseenter'); tool.fire('mouseleave'); wait(2000); tool.fire('mouseenter'); wait(1000);
  check('back on it before then, it stays', open(), true);
  tool.fire('mouseleave'); wait(2500);
  check('and closes a moment after the second leave, not the first', open(), false);

  tool.fire('mouseenter'); focus(input); tool.fire('mouseleave'); wait(60000);
  check('clicked into, it stays open with the pointer gone', open(), true);
  focus(null); wait(2400);
  check('out of the box, it stays a moment', open(), true);
  wait(100);
  check('then closes', open(), false);

  tool.fire('mouseenter'); focus(toggle, false); tool.fire('mouseleave'); wait(2500);
  check('the focus a click leaves on the toggle keeps nothing open', open(), false);

  focus(null); focus(toggle, true);
  check('the keyboard on the toggle opens it', open(), true);
  wait(60000);
  check('and keeps it open', open(), true);
  focus(input, true); wait(60000);
  check('on into the box too', open(), true);
  focus(null); wait(2500);
  check('and it closes a moment after the keyboard leaves', open(), false);

  tool.fire('mouseenter');
  pressed = false; toggle.fire('click');
  check('turning the bands off closes it at once', open(), false);
  pressed = true; toggle.fire('click');
  check('turning them on under the pointer opens it', open(), true);
  check('held: on, and the pointer or the keyboard in it', [levelsHeld(true, true, false), levelsHeld(true, false, true),
    levelsHeld(true, false, false), levelsHeld(false, true, true)], [true, true, false, false]);
  input.value = '9'; input.fire('change');
  check('a 9 typed reads 4 once changed, the most it draws', input.value, '4');
  input.value = ''; input.fire('change');
  check('and an emptied box 1', input.value, '1');
});

group('the page asks for it', function () {
  var html = read('web/index.html');
  ok('a toggle beside the tests eye, off at first', /id="with-tests"[\s\S]{0,500}id="by-folder"[^>]*type="button" aria-pressed="false"/.test(html));
  ok('which turns over and redraws the canvas on a click, like the eye',
    /\$\('#by-folder'\)\.addEventListener\('click', \(\) => \{ \$\('#by-folder'\)\.setAttribute\('aria-pressed', String\(!foldersOn\(\)\)\); rerender\(\); \}\)/.test(app));
  ok('and is read from its pressed state', /function foldersOn\(\) \{\s*return \$\('#by-folder'\)\.getAttribute\('aria-pressed'\) === 'true';/.test(app));
  ok('which is what the canvas is handed', /folders: foldersOn\(\) \?/.test(app));
  ok('a levels box right after it, from 1 to 4, at 1 first',
    /<span>by folder<\/span><\/button>\n\s*<!--[^\n]*-->\n\s*<label id="folder-levels-box"[^>]*>levels <input id="folder-levels" type="number" min="1" max="4" value="1"><\/label>/.test(html));
  var css = read('web/app.css');
  ok('in a pop-over under the toggle, shut until opened', /#folder-levels-box \{\s*display: none; position: absolute; top: calc\(100% \+ 3px\)/.test(css)
    && /#folder-levels-box\.open \{ display: flex; \}/.test(css) && /<span id="folder-tool" class="foldertool">\s*<button id="by-folder"/.test(html));
  ok('wired once the toggle has its own listener', /aria-pressed', String\(!foldersOn\(\)\)\); rerender\(\); \}\);\s*wireFolderLevels\(\);/.test(app));
  ok('an export measures a name in the class it writes it in, sized alike on the page',
    /const cls = b\.level \? `band-name l\$\{b\.level\}` : 'band-name';\s*const t = el\('text', \{ class: cls,[\s\S]{0,200}measure\(cls, b\.name\)/.test(lin)
    && /^\.band-name\.l1, \.band-name\.l2, \.band-name\.l3 \{ font-size: 12px; font-weight: 500; \}$/m.test(css));
  ok('which redraws on a change, like the depth boxes', /for \(const el of \[\$\('#up'\), \$\('#down'\), \$\('#folder-levels'\)\]\) \{\s*el\.addEventListener\('change', rerender\);/.test(app));
  ok('and is handed to the canvas', /levels: folderLevels\(\),/.test(app) && /layout\(d, opts\.folders, opts\.levels\)/.test(lin));
  ok('each level of sub-folder has its hue, on the canvas and in an exported file',
    [1, 2, 3].every(function (l) { return css.indexOf('.band.l' + l + ' {') >= 0 && app.indexOf("'.band.l" + l + ' {') >= 0; }));
  ok('and so does the eye, on a click', /\$\('#with-tests'\)\.addEventListener\('click', \(\) => \{ paintTestsToggle\(!testsOn\(\)\); rerender\(\); \}\)/.test(app));
  ok('every mode hands the canvas the same options', (app.match(/Lineage\.render\(sub, canvasOptions\(\)\)/g) || []).length === 3);
  ok('an exported file names the folders in its header', /canvasStatus\(sub, ctx\.folders\)/.test(app) && /folders: shot\.folders/.test(app));
  ok('and writes each band\'s name, which the canvas pins to the window instead',
    /function snapshot\([\s\S]*class: 'band-name'[\s\S]*function clear|const clear/.test(lin));
});

// Layered graphs in the shape of a project: each model reads a few earlier
// ones, mostly from its own folder or the one before. Deterministic.
function generatedLayers(n, seed) {
  var s = seed >>> 0;
  function rnd() { s = (s * 1664525 + 1013904223) >>> 0; return s / 4294967296; }
  var layers = ['10_raw', '20_clean', '30_vault', '40_marts', '50_serve'], specs = [], links = [];
  for (var i = 0; i < n; i++) {
    var f = layers[Math.min(layers.length - 1, Math.floor(i * layers.length / n))];
    specs.push(['m_' + ('000' + i).slice(-4), 'models/shop/' + f + '/m_' + i + '.sql', i < n / layers.length ? 'source' : 'model']);
  }
  for (i = Math.ceil(n / layers.length); i < n; i++) {
    for (var k = 1 + Math.floor(rnd() * 3); k > 0; k--) {
      var p = Math.max(0, i - 1 - Math.floor(rnd() * Math.min(i, 40)));
      links.push([specs[p][0], specs[i][0]]);
    }
  }
  return graph(specs, links);
}
