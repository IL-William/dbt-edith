// A legend entry clicked keeps what it names lit and fades the rest: which
// boxes and edges stay, in both modes, and that the legend never lists an
// entry that would light nothing.
// Run from the repository root: jsc web/tests/legendpick.js
var lin = read('web/lineage.js');
eval(lin.slice(lin.indexOf('const MAT = {'), lin.indexOf('let svg, root')));
var Lineage = { nodeRoles: nodeRoles, roleColor: roleColor, matLabel: matLabel, nodeColor: nodeColor };

var src = read('web/app.js');
eval(src.slice(src.indexOf('function legendEntries'), src.indexOf('function paintLegend')));
var css = read('web/app.css');

function check(label, got, want) {
  var g = JSON.stringify(got), w = JSON.stringify(want);
  print((g === w ? 'PASS  ' : 'FAIL  ') + label + (g === w ? '' : '\n        expected ' + w + '\n        got      ' + g));
}
function ok(label, cond) { print((cond ? 'PASS  ' : 'FAIL  ') + label); }
function sorted(set) { return Array.from(set).sort(function (a, b) { return a - b; }); }
function keeps(d, label) {
  var k = legendKeeps(d, label);
  return { nodes: sorted(k.nodes), edges: sorted(k.edges) };
}

print('--- model mode: a materialization ---');
var modelSub = { mode: 'model', nodes: [
  { name: 'stg_rides', kind: 'model', materialized: 'view' },
  { name: 'int_rides', kind: 'model', materialized: 'incremental' },
  { name: 'fct_rides', kind: 'model', materialized: 'incremental' },
  { name: 'dim_stations', kind: 'model', materialized: 'table' },
  { name: 'raw_rides', kind: 'source' },
], edges: [[4, 0], [0, 1], [1, 2], [2, 3]] };
check('incremental keeps both incremental boxes, and only the edge between them',
  keeps(modelSub, 'incremental'), { nodes: [1, 2], edges: [2] });
check('a box alone of its kind keeps no edge', keeps(modelSub, 'view'), { nodes: [0], edges: [] });
check('a source is picked like any kind', keeps(modelSub, 'source'), { nodes: [4], edges: [] });
check('a label nothing on the canvas carries keeps nothing', keeps(modelSub, 'snapshot'), { nodes: [], edges: [] });

var testSub = { mode: 'model', nodes: [
  { name: 'rides', kind: 'model', materialized: 'table' },
  { name: 'not_null_rides_id', kind: 'test', test_name: 'not_null', column: 'id' },
  { name: 'combo', kind: 'test', test_name: 'unique_combination_of_columns' },
  { name: 'assert_positive_fare', kind: 'test' },
], edges: [[0, 1], [0, 2], [0, 3]] };
check('each kind of test is picked apart from the others', [
  keeps(testSub, 'column test').nodes, keeps(testSub, 'model test').nodes, keeps(testSub, 'singular test').nodes,
], [[1], [2], [3]]);
check('a test keeps no stem to its faded model', keeps(testSub, 'column test').edges, []);

print('\n--- column mode: a role ---');
// Two raw columns; amount is cast, total transformed from amount and from
// qty, and share renamed from one and aggregated from the other: mixed.
var colSub = { mode: 'column', nodes: [
  { name: 'amount' }, { name: 'qty' }, { name: 'amount' }, { name: 'total' }, { name: 'share' },
], edges: [[0, 2], [2, 3], [1, 3], [2, 4], [1, 4]],
  edge_kinds: ['cast', 'transform', 'transform', 'rename', 'aggregate'] };
check('the roles read as the badges say', nodeRoles(colSub), ['raw', 'raw', 'cast', 'transform', 'mixed']);
check('transform keeps its edges and both columns each one joins',
  keeps(colSub, 'transform'), { nodes: [1, 2, 3], edges: [1, 2] });
check('cast keeps its one edge and its two ends', keeps(colSub, 'cast'), { nodes: [0, 2], edges: [0] });
check('raw, a badge only, keeps its columns and no edge', keeps(colSub, 'raw'), { nodes: [0, 1], edges: [] });
check('mixed, a badge only, keeps its column and no edge', keeps(colSub, 'mixed'), { nodes: [4], edges: [] });
check('a column whose parents are not drawn is no raw one',
  keeps({ mode: 'column', nodes: [{ name: 'a', hidden_up: 2 }, { name: 'b' }], edges: [[0, 1]], edge_kinds: ['cast'] }, 'raw'),
  { nodes: [], edges: [] });

print('\n--- column mode without roles ---');
// The legend falls back to the materializations then, so the pick does too.
var bare = { mode: 'column', nodes: [
  { name: 'id', kind: 'model', materialized: 'view' }, { name: 'id', kind: 'model', materialized: 'view' },
], edges: [[0, 1]] };
check('the legend lists the materialization', labels(legendEntries(bare)), ['view']);
check('and picking it keeps the columns of those models', keeps(bare, 'view'), { nodes: [0, 1], edges: [0] });

print('\n--- the legend never lists an entry that lights nothing ---');
function labels(entries) { return entries.map(function (e) { return e[0]; }); }
[['model', modelSub], ['tests', testSub], ['column', colSub], ['bare column', bare]].forEach(function (f) {
  var dark = labels(legendEntries(f[1])).filter(function (l) { return legendKeeps(f[1], l).nodes.size === 0; });
  check(f[0] + ': every entry keeps a box', dark, []);
});

print('\n--- wiring ---');
ok('a redraw paints the pick again, after the selection',
  /select\(selected\);\s*paintPick\(\);\s*return drawn;/.test(lin));
ok('an exported picture drops the pick', /g\.classList\.remove\('picking'\)/.test(lin)
  && /querySelectorAll\('\.kept'\)\.forEach\(\(x\) => x\.classList\.remove\('kept'\)\)/.test(lin));
ok('a pick the new legend no longer lists is dropped',
  /function paintLegend\(sub\) \{[\s\S]{0,400}Lineage\.pick\(null\)/.test(src));
// The export harness copies every rule starting with these into the file,
// which has no fade: a pick rule written that way would have to follow it.
var pickRules = css.split('\n').filter(function (l) { return /picking/.test(l) && /\{/.test(l); });
ok('the fade has rules', pickRules.length > 0);
check('none of them is one the exported file must copy',
  pickRules.filter(function (l) { return /^(#graph text|\.edge|\.nd|\.role|\.band)/.test(l); }), []);
ok('the strip around the entries still lets a pan through',
  /\.legend \{[^}]*pointer-events: none/.test(css) && /\.legend span \{[^}]*pointer-events: auto/.test(css));
