// Where the lineage canvas puts each box and how an edge gets there: a column
// by the longest path over the edges drawn, never by the server's distance from
// the focus; a lane between the boxes in every column a long edge skips; no two
// boxes on top of each other; and all of it safely when the edges loop, which
// column lineage can.
// Run from the repository root: jsc web/tests/layout.js
var lin = read('web/lineage.js');
eval(lin.slice(lin.indexOf('const MAT = {'), lin.indexOf('let svg, root')));

function check(label, got, want) {
  var g = JSON.stringify(got), w = JSON.stringify(want);
  print((g === w ? 'PASS  ' : 'FAIL  ') + label + (g === w ? '' : '\n        expected ' + w + '\n        got      ' + g));
}
function ok(label, cond) { print((cond ? 'PASS  ' : 'FAIL  ') + label); }
// A throw would stop the harness where it stands, and check.sh would only say
// that it exited badly: this names the group and lets the others run.
function group(name, fn) {
  print('\n--- ' + name + ' ---');
  try { fn(); } catch (e) { print('FAIL  ' + name + ' threw ' + e); }
}

function graph(names, links, extra) {
  var at = {};
  var nodes = names.map(function (s, i) { at[s] = i; return { id: 'model.shop.' + s, name: s, depth: 0, kind: 'model' }; });
  (extra || []).forEach(function (f) { f(nodes, at); });
  return { mode: 'model', nodes: nodes, edges: links.map(function (l) { return [at[l[0]], at[l[1]]]; }), at: at };
}
function layersOf(d) { return dagLayers(d.nodes.length, d.edges, dagRank(d.nodes)); }
function columns(d) {
  var layer = layersOf(d).layer, out = {};
  d.nodes.forEach(function (n, i) { out[n.name] = layer[i]; });
  return out;
}
function allRight(d, layer) {
  return d.edges.every(function (e) { return e[0] === e[1] || layer[e[1]] > layer[e[0]]; });
}
// A deterministic stand-in for the random graphs a project produces: never
// Math.random, so a failure can be run again.
function generated(n, seed, loops) {
  var s = seed >>> 0;
  function rnd() { s = (s * 1664525 + 1013904223) >>> 0; return s / 4294967296; }
  var names = [], links = [];
  for (var i = 0; i < n; i++) names.push('int_' + ('000' + i).slice(-4));
  for (i = 1; i < n; i++) {
    for (var k = 1 + Math.floor(rnd() * 3); k > 0; k--) {
      var p = rnd() < 0.15 ? Math.floor(rnd() * Math.min(i, 8)) : Math.max(0, i - 1 - Math.floor(rnd() * Math.min(i, 12)));
      links.push(loops && rnd() < 0.1 ? [names[i], names[p]] : [names[p], names[i]]);
    }
  }
  return graph(names, links);
}
// The same graph listed in another order, as another reload of the server
// could list it.
function shuffled(d) {
  var n = d.nodes.length, to = d.nodes.map(function (_, i) { return n - 1 - i; });
  var nodes = d.nodes.slice().reverse();
  var edges = d.edges.map(function (e) { return [to[e[0]], to[e[1]]]; });
  edges = edges.slice(3).concat(edges.slice(0, 3));
  return { mode: d.mode, nodes: nodes, edges: edges };
}
// How many boxes land elsewhere than in `want`, by id: a count rather than two
// dumps, which on 400 boxes would bury the one that moved.
function moved(d, got, want, dWant) {
  var at = {}, n = 0;
  dWant.nodes.forEach(function (x, i) { at[x.id] = JSON.stringify(want[i]); });
  d.nodes.forEach(function (x, i) { if (at[x.id] !== JSON.stringify(got[i])) n++; });
  return n;
}

// The shape the change was made for: stg_payments reaches int_payment_keys in
// one hop and in three, and the server's distance put it at the one.
var reported = graph(
  ['stg_payments', 'int_payments', 'int_payments_net', 'int_payment_keys', 'pit_payments'],
  [['stg_payments', 'int_payments'], ['int_payments', 'int_payments_net'], ['int_payments_net', 'int_payment_keys'],
    ['stg_payments', 'int_payment_keys'], ['int_payment_keys', 'pit_payments']],
  [function (nodes) { [0, 1, 2, 1, 2].forEach(function (d, i) { nodes[i].depth = d; }); }]);

group('columns', function () {
  var c = columns(reported);
  ok('every edge points right, where the distance from the focus sent one left', allRight(reported, layersOf(reported).layer));
  check('the model reached by a short and a long path sits after the long one', c.int_payment_keys, 3);
  check('and just before the model that reads it', c.pit_payments - c.int_payment_keys, 1);

  var scrambled = graph(reported.nodes.map(function (n) { return n.name; }),
    [['stg_payments', 'int_payments'], ['int_payments', 'int_payments_net'], ['int_payments_net', 'int_payment_keys'],
      ['stg_payments', 'int_payment_keys'], ['int_payment_keys', 'pit_payments']],
    [function (nodes) { nodes.forEach(function (n, i) { n.depth = 7 - 3 * i; }); }]);
  check('depth is not read: it stays the distance the export turns into dbt operators', columns(scrambled), c);

  var shapes = graph(['a', 'b', 'c', 'd', 'x', 'y'], [['a', 'b'], ['a', 'c'], ['b', 'd'], ['c', 'd'], ['x', 'y']]);
  check('each piece of a selection starts at the left', columns(shapes), { a: 0, b: 1, c: 1, d: 2, x: 0, y: 1 });

  var late = graph(['stg_orders', 'int_a', 'int_b', 'fct_orders', 'stg_rates'],
    [['stg_orders', 'int_a'], ['int_a', 'int_b'], ['int_b', 'fct_orders'], ['stg_rates', 'fct_orders']]);
  check('an input only a late model reads sits just left of it, not at the far left', columns(late).stg_rates, 2);

  var mid = graph(['stg_a', 'int_a', 'int_b', 'int_c', 'fct_a', 'int_side'],
    [['stg_a', 'int_a'], ['int_a', 'int_b'], ['int_b', 'int_c'], ['int_c', 'fct_a'], ['stg_a', 'int_side'], ['int_side', 'fct_a']]);
  check('a model with as many parents as readers stays beside its parents', columns(mid).int_side, 1);

  var tested = graph(['stg_a', 'int_a', 'fct_a', 'not_null_stg_a_id', 'relationships_fct_a_id'],
    [['stg_a', 'int_a'], ['int_a', 'fct_a'], ['stg_a', 'not_null_stg_a_id'], ['int_a', 'relationships_fct_a_id'], ['fct_a', 'relationships_fct_a_id']],
    [function (nodes) { nodes[3].kind = 'test'; nodes[4].kind = 'test'; }]);
  var t = columns(tested);
  check('a test sits one column right of its model', t.not_null_stg_a_id, t.stg_a + 1);
  check('and one right of the later of two', t.relationships_fct_a_id, t.fct_a + 1);

  var focus = reported.at.int_payments, layer = layersOf(reported).layer;
  ok('around a focus, what it reads is left of it and what reads it is right of it',
    layer[reported.at.stg_payments] < layer[focus]
    && ['int_payments_net', 'int_payment_keys', 'pit_payments'].every(function (s) { return layer[reported.at[s]] > layer[focus]; }));

  [generated(150, 1), generated(400, 7), late, tested].forEach(function (d, k) {
    var L = layersOf(d).layer, used = {}, top = 0;
    for (var i = 0; i < L.length; i++) { used[L[i]] = 1; top = Math.max(top, L[i]); }
    var gap = false;
    for (i = 0; i <= top; i++) if (!used[i]) gap = true;
    ok('graph ' + (k + 1) + ': every edge points right, and no column between is left empty', allRight(d, L) && !gap);
  });
});

group('edges that loop, and nothing at all', function () {
  var self = layersOf(graph(['int_a'], [['int_a', 'int_a']]));
  check('a self-loop alone', [Array.from(self.layer), self.edgePair], [[0], [-1]]);

  var two = graph(['int_a', 'int_b'], [['int_a', 'int_b'], ['int_b', 'int_a']]), L2 = layersOf(two);
  check('a 2-cycle turns one edge and draws the pair in two columns',
    [Array.from(L2.flip).reduce(function (s, f) { return s + f; }, 0), L2.layer[0] !== L2.layer[1]], [1, true]);

  var three = graph(['int_a', 'int_b', 'int_c'],
    [['int_a', 'int_b'], ['int_b', 'int_c'], ['int_c', 'int_a'], ['int_a', 'int_a'], ['int_a', 'int_b']]), L3 = layersOf(three);
  check('a 3-cycle with a self-loop and a repeated edge: one turned, each edge once',
    [Array.from(L3.flip).reduce(function (s, f) { return s + f; }, 0), L3.pairs.length, L3.edgePair], [1, 3, [0, 1, 2, -1, 0]]);
  ok('turned round, every pair points right', L3.pairs.every(function (p, k) {
    var a = p[L3.flip[k]], b = p[1 - L3.flip[k]];
    return L3.layer[b] > L3.layer[a];
  }));

  var names = ['int_a', 'int_b', 'int_c', 'int_d', 'int_e'], links = [];
  names.forEach(function (a) { names.forEach(function (b) { if (a !== b) links.push([a, b]); }); });
  var K = layersOf(graph(names, links));
  ok('five models all reading each other still get a column each', K.layer.length === 5 && K.pairs.every(function (p, k) {
    return K.layer[p[1 - K.flip[k]]] > K.layer[p[K.flip[k]]];
  }));

  var loops = generated(400, 3, true), GL = layersOf(loops);
  ok('a looping graph of 400 gets a finite column for every model',
    GL.layer.length === 400 && Array.from(GL.layer).every(function (l) { return l >= 0 && l < 400; }));

  check('no node, no column', Array.from(layersOf({ nodes: [], edges: [] }).layer), []);
  check('nodes without edges share the first column', Array.from(layersOf(graph(['stg_a', 'stg_b', 'stg_c'], [])).layer), [0, 0, 0]);
});

group('the same answer every time', function () {
  [reported, generated(150, 1), generated(400, 3, true)].forEach(function (d, k) {
    var before = JSON.stringify({ nodes: d.nodes, edges: d.edges });
    var once = layersOf(d).layer, again = layersOf(d).layer, other = shuffled(d);
    ok('graph ' + (k + 1) + ': twice the same', JSON.stringify(Array.from(once)) === JSON.stringify(Array.from(again)));
    check('graph ' + (k + 1) + ': whatever order the server lists it in, no box moves',
      moved(other, layersOf(other).layer, once, d), 0);
    ok('graph ' + (k + 1) + ': the payload is left as it came', JSON.stringify({ nodes: d.nodes, edges: d.edges }) === before);
  });
});

var MODEL = { w: 200, h: 48, hgap: 80, vgap: 14, lane: 10, badge: 0, bundle: 4 };
var COLUMN = { w: 180, h: 40, hgap: 80, vgap: 14, lane: 10, badge: 6, bundle: 4 };

// A path as its commands, every number absolute: M x,y then C and L.
function parse(s) {
  var t = s.match(/[MCL]|-?[\d.]+(?:e-?\d+)?/g), out = [], i = 0, c = '';
  while (i < t.length) {
    if (/[MCL]/.test(t[i])) { c = t[i++]; continue; }
    var k = c === 'C' ? 6 : 2, p = t.slice(i, i + k).map(Number);
    out.push({ c: c, p: p }); i += k;
  }
  return out;
}
function points(s) {
  var out = [], at = null;
  parse(s).forEach(function (seg) {
    if (seg.c !== 'C') { at = [seg.p[0], seg.p[1]]; out.push(at); return; }
    var p = [at[0], at[1]].concat(seg.p);
    for (var u = 0.25; u <= 1; u += 0.25) {
      var v = 1 - u;
      out.push([v * v * v * p[0] + 3 * v * v * u * p[2] + 3 * v * u * u * p[4] + u * u * u * p[6],
        v * v * v * p[1] + 3 * v * v * u * p[3] + 3 * v * u * u * p[5] + u * u * u * p[7]]);
    }
    at = [seg.p[4], seg.p[5]];
  });
  return out;
}
function finite(L) {
  return L.paths.every(function (s) { return !/NaN|Infinity|undefined/.test(s); })
    && L.boxes.every(function (b) { return isFinite(b.x) && isFinite(b.y); });
}
// The gap between neighbouring boxes of each column, the smallest one found.
function tightest(L) {
  var cols = {}, min = Infinity;
  L.boxes.forEach(function (b) { (cols[b.x] = cols[b.x] || []).push(b); });
  Object.keys(cols).forEach(function (x) {
    var c = cols[x].sort(function (a, b) { return a.y - b.y; });
    for (var i = 1; i < c.length; i++) min = Math.min(min, c[i].y - c[i - 1].y - c[i - 1].h);
  });
  return min;
}
// Every lane, as the column it crosses and its height.
function lanes(L, dim) {
  var out = [];
  L.paths.forEach(function (s, e) {
    parse(s).forEach(function (seg, j, all) {
      if (seg.c === 'L') out.push({ edge: e, x0: all[j - 1].p[all[j - 1].c === 'C' ? 4 : 0], x1: seg.p[0], y: seg.p[1] });
    });
  });
  return out;
}
function laneClearance(L, dim) {
  var min = Infinity;
  lanes(L, dim).forEach(function (l) {
    L.boxes.forEach(function (b) {
      if (b.x !== l.x0) return;
      min = Math.min(min, l.y < b.y ? b.y - l.y : l.y > b.y + b.h ? l.y - b.y - b.h : -1);
    });
  });
  return min;
}
function within(L) {
  var b = L.bbox, fits = function (x, y) { return x >= b.x0 - 0.5 && x <= b.x1 + 0.5 && y >= b.y0 - 0.5 && y <= b.y1 + 0.5; };
  return L.boxes.every(function (r) { return fits(r.x, r.y) && fits(r.x + r.w, r.y + r.h); })
    && L.paths.every(function (s) { return points(s).every(function (p) { return fits(p[0], p[1]); }); });
}
function best(fn) {
  var t = Infinity;
  for (var i = 0; i < 3; i++) { var s = Date.now(); fn(); t = Math.min(t, Date.now() - s); }
  return t;
}
function column(names, links, kinds) {
  var d = graph(names, links);
  d.mode = 'column';
  if (kinds) d.edge_kinds = kinds;
  return d;
}

group('edges run left to right, through lanes', function () {
  [reported, generated(150, 1), generated(400, 7)].forEach(function (d, k) {
    var L = dagLayout(d, MODEL), step = MODEL.w + MODEL.hgap, good = true, shape = true;
    d.edges.forEach(function (e, i) {
      var a = L.boxes[e[0]], b = L.boxes[e[1]], P = parse(L.paths[i]), first = P[0].p, end = P[P.length - 1].p;
      if (L.back[i] || !(b.x > a.x) || first[0] !== a.x + a.w || Math.abs(first[1] - (a.y + a.h / 2)) > 0.1
        || end[end.length - 2] !== b.x || Math.abs(end[end.length - 1] - (b.y + b.h / 2)) > 0.1) good = false;
      var ls = P.filter(function (s) { return s.c === 'L'; });
      if (ls.length !== (b.x - a.x) / step - 1) shape = false;
      var x = first[0];
      P.slice(1).forEach(function (s) {
        var to = s.c === 'C' ? s.p[4] : s.p[0];
        if (s.c === 'C' && (x % step !== MODEL.w || to !== x + MODEL.hgap)) shape = false;
        if (s.c === 'L' && (x % step !== 0 || to !== x + MODEL.w)) shape = false;
        x = to;
      });
    });
    ok('graph ' + (k + 1) + ': every edge leaves its model on the right and reaches its reader on the left', good);
    ok('graph ' + (k + 1) + ': one lane across each column skipped, a curve across each gap', shape);
    check('graph ' + (k + 1) + ': a lane keeps at least VGAP from every box in its column',
      laneClearance(L, MODEL) >= MODEL.vgap, true);
  });

  var col = dagLayout(column(['amount', 'amount_net', 'amount_x', 'amount_y', 'total'],
    [['amount', 'amount_net'], ['amount_net', 'amount_x'], ['amount_x', 'amount_y'], ['amount_y', 'total'], ['amount', 'total']]), COLUMN);
  check('in column mode a lane keeps clear of the role tag drawn above a box too',
    laneClearance(col, COLUMN) >= COLUMN.vgap + COLUMN.badge, true);

  var straight = graph(['stg_a', 'int_a', 'int_b', 'int_c', 'fct_a'],
    [['stg_a', 'int_a'], ['int_a', 'int_b'], ['int_b', 'int_c'], ['int_c', 'fct_a'], ['stg_a', 'fct_a']]);
  var S = dagLayout(straight, MODEL), ys = lanes(S, MODEL).map(function (l) { return l.y; });
  check('a long edge crosses every column on one level', [ys.length, ys.every(function (y) { return y === ys[0]; })], [3, true]);
  var chain = dagLayout(graph(['stg_a', 'int_a', 'fct_a'], [['stg_a', 'int_a'], ['int_a', 'fct_a']]), MODEL);
  check('a chain comes out on one line', [chain.boxes[1].y - chain.boxes[0].y, chain.boxes[2].y - chain.boxes[0].y], [0, 0]);

  var R = dagLayout(reported, MODEL), at = reported.at;
  ok('around a focus, what it reads is drawn left of it and what reads it right of it',
    R.boxes[at.stg_payments].x < R.boxes[at.int_payments].x
    && ['int_payments_net', 'int_payment_keys', 'pit_payments'].every(function (s) { return R.boxes[at[s]].x > R.boxes[at.int_payments].x; }));
});

var loops = {
  'a self-loop alone': column(['amount'], [['amount', 'amount']]),
  'a 2-cycle': column(['amount', 'total'], [['amount', 'total'], ['total', 'amount']]),
  'a 3-cycle with a self-loop and a repeated edge': column(['amount', 'net', 'total'],
    [['amount', 'net'], ['net', 'total'], ['total', 'amount'], ['amount', 'amount'], ['amount', 'net']]),
  'five columns all reading each other': (function () {
    var names = ['a', 'b', 'c', 'd', 'e'], links = [];
    names.forEach(function (x) { names.forEach(function (y) { if (x !== y) links.push([x, y]); }); });
    return column(names, links);
  })(),
  'two pieces': column(['a', 'b', 'x', 'y'], [['a', 'b'], ['x', 'y']]),
  'no node': { mode: 'column', nodes: [], edges: [] },
  'one node': column(['amount'], []),
  'nodes without edges': column(['a', 'b', 'c'], []),
  'a looping graph of 400': (function () { var d = generated(400, 3, true); d.mode = 'column'; return d; })(),
};

group('loops, and nothing at all', function () {
  Object.keys(loops).forEach(function (name) {
    var d = loops[name], L = dagLayout(d, COLUMN);
    ok(name + ': one path per edge, every number finite, no box on another',
      L.paths.length === d.edges.length && finite(L) && !(tightest(L) < COLUMN.vgap));
  });
  var two = dagLayout(loops['a 2-cycle'], COLUMN);
  check('in a 2-cycle one edge is drawn against its direction', two.back, [0, 1]);
  ok('and the two halves are drawn apart, not on one line', two.paths[0] !== two.paths[1]);
  var turned = parse(two.paths[1]), from = two.boxes[1], to = two.boxes[0];
  ok('the turned edge runs from its target\'s right side to its source\'s left side, the way the reader reads',
    turned[0].p[0] === to.x + to.w && turned[turned.length - 1].p[4] === from.x);
  check('a 3-cycle turns one edge, and the self-loop is marked too; the repeat is drawn like its twin',
    dagLayout(loops['a 3-cycle with a self-loop and a repeated edge'], COLUMN).back, [0, 0, 1, 1, 0]);
  var self = dagLayout(loops['a self-loop alone'], COLUMN), box = self.boxes[0];
  ok('a self-loop is an ear on the right of its box, clear of the role tag and the +N badge',
    points(self.paths[0]).every(function (p) { return p[0] >= box.x + box.w && p[1] <= box.y + 8; }));
  check('no node, no frame', dagLayout(loops['no node'], COLUMN).bbox, null);
  var one = dagLayout(loops['one node'], COLUMN);
  check('one node, its own box as the frame', one.bbox, { x0: one.boxes[0].x, y0: one.boxes[0].y, x1: one.boxes[0].x + 180, y1: one.boxes[0].y + 40 });
});

group('the frame holds everything drawn', function () {
  [reported, generated(150, 1), loops['a self-loop alone'], loops['a 3-cycle with a self-loop and a repeated edge'],
    loops['a looping graph of 400']].forEach(function (d, k) {
    ok('graph ' + (k + 1) + ': every box and every point of every edge, lanes and loops included',
      within(dagLayout(d, d.mode === 'column' ? COLUMN : MODEL)));
  });
});

group('no box on another', function () {
  [reported, generated(150, 1), generated(400, 7), generated(400, 3, true)].forEach(function (d, k) {
    check('graph ' + (k + 1) + ', model boxes: VGAP at least between neighbours', tightest(dagLayout(d, MODEL)) >= MODEL.vgap, true);
    check('graph ' + (k + 1) + ', column boxes: VGAP at least between neighbours', tightest(dagLayout(d, COLUMN)) >= COLUMN.vgap, true);
  });
});

group('a hub shares its lanes', function () {
  function hub(readers, kinds) {
    var names = ['dim_date', 'int_a', 'int_b', 'int_c', 'int_d'], links = [['dim_date', 'int_a'], ['int_a', 'int_b'], ['int_b', 'int_c'], ['int_c', 'int_d']];
    for (var i = 1; i <= readers; i++) { names.push('fct_' + i); links.push(['int_d', 'fct_' + i], ['dim_date', 'fct_' + i]); }
    var d = graph(names, links);
    if (kinds) d.edge_kinds = links.map(function (l, i) { return l[0] === 'dim_date' && i > 3 ? kinds[(i - 5) / 2] : 'passthrough'; });
    return d;
  }
  // The lanes the hub's long edges cross column 1 on, told apart by height.
  function levels(d) {
    var L = dagLayout(d, MODEL), seen = {};
    lanes(L, MODEL).forEach(function (l) { if (d.nodes[d.edges[l.edge][0]].name === 'dim_date' && l.x0 === 280) seen[l.y] = 1; });
    return Object.keys(seen).length;
  }
  check('four long edges out of one model share one lane', levels(hub(4)), 1);
  check('three keep one each', levels(hub(3)), 3);
  check('a lane has one colour, so two kinds share two lanes', levels(hub(4, ['rename', 'rename', 'cast', 'cast'])), 2);
});

group('the same picture every time', function () {
  var twins = column(['amount', 'amount', 'total', 'net'], [['net', 'amount'], ['amount', 'total'], ['total', 'net']], ['rename', 'cast', 'transform']);
  twins.nodes[0].id = 'model.shop.fct_a::amount'; twins.nodes[1].id = 'model.shop.fct_b::amount';
  twins.edges.push([1, 2]); twins.edge_kinds.push('rename');
  [reported, generated(150, 1), generated(400, 3, true), twins].forEach(function (d, k) {
    var dim = d.mode === 'column' ? COLUMN : MODEL, before = JSON.stringify(d);
    var L = dagLayout(d, dim);
    ok('graph ' + (k + 1) + ': twice the same', JSON.stringify(L) === JSON.stringify(dagLayout(d, dim)));
    var other = shuffled(d);
    if (d.edge_kinds) other.edge_kinds = d.edge_kinds.slice(3).concat(d.edge_kinds.slice(0, 3));
    var M = dagLayout(other, dim), paths = {};
    d.edges.forEach(function (e, i) { paths[d.nodes[e[0]].id + '>' + d.nodes[e[1]].id] = L.paths[i]; });
    check('graph ' + (k + 1) + ': whatever order the server lists it in, no box moves', moved(other, M.boxes, L.boxes, d), 0);
    ok('graph ' + (k + 1) + ': and no edge is drawn differently', other.edges.every(function (e, i) {
      return paths[other.nodes[e[0]].id + '>' + other.nodes[e[1]].id] === M.paths[i];
    }));
    ok('graph ' + (k + 1) + ': the payload is left as it came', JSON.stringify(d) === before);
  });
});

group('fast enough to redraw on every click', function () {
  var big = generated(400, 11), s = 99;
  for (var i = 0; i < 200; i++) {
    s = (s * 1664525 + 1013904223) >>> 0;
    big.nodes.push({ id: 'test.shop.t' + i, name: 'not_null_' + i, kind: 'test', depth: 0 });
    big.edges.push([s % 400, 400 + i]);
  }
  var t = best(function () { dagLayout(big, MODEL); });
  ok('400 models and 200 tests, ' + big.edges.length + ' edges, well under 100 ms (' + t + ' ms here)', t < 100);

  var names = [], links = [];
  for (i = 0; i < 400; i++) names.push('int_' + i);
  for (i = 1; i < 400; i++) links.push([names[i - 1], names[i]]);
  for (i = 0; i < 400; i++) {
    s = (s * 1664525 + 1013904223) >>> 0;
    var a = s % 400;
    s = (s * 1664525 + 1013904223) >>> 0;
    var b = s % 400;
    if (a !== b) links.push([names[Math.min(a, b)], names[Math.max(a, b)]]);
  }
  var chain = graph(names, links);
  t = best(function () { dagLayout(chain, MODEL); });
  ok('a chain of 400 with 400 edges skipping across it, the worst shape found, under 250 ms (' + t + ' ms here)', t < 250);

  var cyclic = generated(400, 5, true);
  cyclic.mode = 'column';
  for (i = 0; i < 20; i++) cyclic.edges.push([i * 7, i * 7]);
  t = best(function () { dagLayout(cyclic, COLUMN); });
  ok('a looping column graph of 400, self-loops included, under 250 ms (' + t + ' ms here)', t < 250);
});

group('level lanes, unless they cost the height', function () {
  // Models with their tests ticked: a stack of sinks beside every model, the
  // shape that sent Brandes and Kopf into a staircase twice the needed height.
  function tested(models, each) {
    var d = generated(models, 2);
    for (var m = 0; m < models; m++) {
      for (var t = 0; t < each; t++) {
        d.nodes.push({ id: 'test.shop.t' + m + '_' + t, name: 'not_null_' + m + '_' + t, depth: 0, kind: 'test' });
        d.edges.push([m, d.nodes.length - 1]);
      }
    }
    return d;
  }
  function staged(d, dim) {
    var n = d.nodes.length, R = dagRank(d.nodes), lay = dagLayers(n, d.edges, R);
    var g = dagProper(n, lay, R.rank, d.edge_kinds || [], dim.bundle);
    dagOrder(g, R);
    return g;
  }
  var d = tested(60, 5), g = staged(d, MODEL), tallest = dagTallest(g, MODEL);
  ok('on its own, Brandes and Kopf would spend more than half again the tallest column here',
    dagSpan(g, bkCoords(g, MODEL), MODEL) > 1.5 * tallest);
  var L = dagLayout(d, MODEL);
  ok('so the compact heights are taken, within half again the tallest column', L.bbox.y1 - L.bbox.y0 <= 1.5 * tallest);
  check('still no box on another', tightest(L) >= MODEL.vgap, true);
  check('and every lane clear of the boxes', laneClearance(L, MODEL) >= MODEL.vgap, true);
  var C = dagLayout((function () { var c = tested(60, 5); c.mode = 'column'; return c; })(), COLUMN);
  check('in column mode too, clear of the role tags', [tightest(C) >= COLUMN.vgap, laneClearance(C, COLUMN) >= COLUMN.vgap + COLUMN.badge], [true, true]);
  ok('whole pixels, so no gap is lost to rounding', L.boxes.every(function (b) { return b.y === Math.round(b.y); }));

  var cheap = staged(reported, MODEL);
  ok('where level lanes cost little, Brandes and Kopf keeps them',
    dagSpan(cheap, bkCoords(cheap, MODEL), MODEL) <= 1.5 * dagTallest(cheap, MODEL));

  // Sources, staging, one intermediate and marts, where Brandes and Kopf spends
  // too much and the isotonic step, found by a review, comes out taller still.
  var odd = graph(['src_0', 'stg_0', 'src_1', 'stg_1', 'src_2', 'stg_2', 'int_0', 'mart_0', 'mart_1', 'mart_2', 'mart_3', 'mart_4', 'mart_5'],
    [['src_0', 'stg_0'], ['src_1', 'stg_1'], ['src_2', 'stg_2'], ['stg_0', 'int_0'], ['stg_0', 'mart_0'], ['mart_0', 'mart_1'],
      ['mart_1', 'mart_2'], ['mart_0', 'mart_2'], ['stg_0', 'mart_2'], ['mart_0', 'mart_3'], ['mart_0', 'mart_4'], ['stg_2', 'mart_5'], ['stg_1', 'mart_5']]);
  var go = staged(odd, MODEL), bkSpan = dagSpan(go, bkCoords(go, MODEL), MODEL);
  ok('here Brandes and Kopf spends more than half again, and the isotonic heights are taller still',
    bkSpan > 1.5 * dagTallest(go, MODEL) && dagSpan(go, isoCoords(go, MODEL), MODEL) > bkSpan);
  var O = dagLayout(odd, MODEL);
  check('so Brandes and Kopf stays, rather than a switch worse on both counts', O.bbox.y1 - O.bbox.y0, bkSpan);
});

group('the canvas draws what the layout marks', function () {
  var css = read('web/app.css');
  ok('render() gives a turned edge its own class', /class:\s*L\.back\[i\]\s*\?\s*'edge back'\s*:\s*'edge'/.test(lin));
  ok('which is dashed', /\.edge\.back\s*\{[^}]*stroke-dasharray/.test(css));
  ok('without touching its colour, so a selected loop still turns accent', !/\.edge\.back\s*\{[^}]*stroke:/.test(css));
  // Edges sharing a lane lie on one line, so the one selected has to be the
  // last drawn, on the canvas and in an exported file alike.
  var app = read('web/app.js');
  ok('a selected edge is raised above the ones sharing its lane',
    /function select\(id\)[\s\S]{0,700}if \(hit\) p\.parentNode\.appendChild\(p\)/.test(lin));
  ok('in an exported file too', /const pick = \(id\) =>[\s\S]{0,400}if \(hit\) p\.parentNode\.appendChild\(p\)/.test(app));
});
