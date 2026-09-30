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
  var tc = columnsOf(tested, dagLayout(tested, MODEL, 'shop'));
  check('a test sits one column right of its model, inside its model\'s run',
    [tc.stg_orders, tc.not_null_stg_orders_id, tc.hub_order, tc.unique_hub_order_id], [0, 1, 2, 3]);

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
  check('with them, how many', canvasStatus(sub, { folders: 4, against: 0 }), '12 nodes · 15 edges · 4 folders');
  check('and the edges drawn against their order', canvasStatus(sub, { folders: 3, against: 1 }),
    '12 nodes · 15 edges · 3 folders · 1 edge against their order');
  check('in selection mode too', canvasStatus({ mode: 'select', matched: 3, nodes: new Array(3), edges: new Array(2) },
    { folders: 1, against: 2 }), '3 nodes · 2 edges · 1 folder · 2 edges against their order');
  check('nothing matched, nothing said', canvasStatus({ mode: 'select', matched: 0, nodes: [], edges: [] }, null), '');
});

group('the page asks for it', function () {
  var html = read('web/index.html');
  ok('a checkbox beside the tests eye', /id="with-tests"[\s\S]{0,400}id="by-folder" type="checkbox"/.test(html));
  ok('which redraws the canvas like the others', /\[\$\('#up'\), \$\('#down'\), \$\('#by-folder'\)\]/.test(app));
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
