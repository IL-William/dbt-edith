// Node colour: the visual language of the whole tool, so it gets a test.
// Run from the repository root: jsc web/tests/colours.js
var lin = read('web/lineage.js');
eval(lin.slice(lin.indexOf('const MAT = {'), lin.indexOf('let svg, root')));

function check(label, got, want) {
  print((got === want ? 'PASS  ' : 'FAIL  ') + label + (got === want ? '' : '   expected ' + want + ', got ' + got));
}
var distinct = {};
function unique(label, colour) {
  if (distinct[colour] && distinct[colour] !== label) {
    print('FAIL  ' + label + ' shares its colour with ' + distinct[colour]);
  } else { distinct[colour] = label; print('PASS  ' + label + ' has a distinct colour'); }
}

print('--- common materializations ---');
check('view',        matLabel({ kind: 'model', materialized: 'view' }), 'view');
check('table',       matLabel({ kind: 'model', materialized: 'table' }), 'table');
check('incremental', matLabel({ kind: 'model', materialized: 'incremental' }), 'incremental');
check('ephemeral',   matLabel({ kind: 'model', materialized: 'ephemeral' }), 'ephemeral');

['view', 'table', 'incremental', 'ephemeral'].forEach(function (m) {
  unique(m, nodeColor({ kind: 'model', materialized: m }));
});

print('\n--- custom materializations ---');
var custom = nodeColor({ kind: 'model', materialized: 'dynamic_transient' });
check('a custom one does not fall back to neutral grey', custom !== nodeColor({ kind: 'model', materialized: '' }), true);
check('two custom ones share the "custom" colour',
      custom, nodeColor({ kind: 'model', materialized: 'my_own_materialization' }));
check('case is ignored', nodeColor({ kind: 'model', materialized: 'TABLE' }), nodeColor({ kind: 'model', materialized: 'table' }));

print('\n--- other resource types ---');
check('a source ignores its materialization', matLabel({ kind: 'source', materialized: 'source' }), 'source');
unique('source', nodeColor({ kind: 'source' }));
unique('seed', nodeColor({ kind: 'seed' }));
unique('snapshot', nodeColor({ kind: 'snapshot' }));
check('a test on a column', matLabel({ kind: 'test', test_name: 'not_null', column: 'id' }), 'column test');
check('a generic on the whole model', matLabel({ kind: 'test', test_name: 'unique_combination_of_columns' }), 'model test');
check('a test with no generic behind it', matLabel({ kind: 'test' }), 'singular test');
unique('model test', nodeColor({ kind: 'test', test_name: 'unique_combination_of_columns' }));
unique('singular test', nodeColor({ kind: 'test' }));
check('a column test keeps the grey every test had', nodeColor({ kind: 'test', test_name: 'not_null', column: 'id' }), '#7a879a');
check('a model without a materialization does not break', typeof nodeColor({ kind: 'model' }), 'string');
check('an empty node does not break', typeof nodeColor({}), 'string');

print('\n--- regression: CSS must not be able to repaint the bar ---');
// A CSS rule always beats an SVG presentation attribute. The colour bar stayed
// invisible for as long as `.nd rect` set a fill.
var css = read('web/app.css');
var offenders = css.split('\n').filter(function (l) {
  return /^\s*\.nd[^{]*rect\s*(\{|,)/.test(l) && /fill\s*:/.test(l) && !/rect\.box/.test(l);
});
check('no unqualified .nd rect rule sets a fill', offenders.length, 0);
if (offenders.length) offenders.forEach(function (o) { print('        ' + o.trim()); });
check('the colour is set as an inline style', /kindbar[^)]*style:\s*`fill:/.test(lin), true);
check('so is every bar of a hanging test, the shape of its kind included',
      /const bar = \(attrs\) => g\.appendChild\(el\('rect', Object\.assign\(\{ class: 'kindbar'[^}]*style: `fill:/.test(lin), true);
check('a test on the whole model has a second, thin bar', /bar\(\{ x: 8, width: 2 \}\)/.test(lin), true);
check('and a singular test a bar in three pieces', /\[0, 1, 2\]\.forEach\(\(i\) => bar\(\{ class: 'kindbar seg'/.test(lin), true);
check('and a singular test is dashed', /^\.nd\.rider\.singular rect\.box \{[^}]*stroke-dasharray/m.test(css), true);

print('\n--- edge roles (column mode) ---');
// Roles colour the edges while materializations still colour the boxes, so both
// channels are on screen together in column mode. They must not share a colour,
// or the eye reads a relationship that is not there.
var ROLES = ['passthrough', 'rename', 'cast', 'aggregate', 'window', 'transform', 'inferred'];
var roleSeen = {};
ROLES.forEach(function (r) {
  var c = roleColor(r);
  if (roleSeen[c]) print('FAIL  role ' + r + ' shares its colour with role ' + roleSeen[c]);
  else { roleSeen[c] = r; print('PASS  role ' + r + ' has a distinct colour'); }
});

// One test of each kind (0040).
var TESTS = [{ kind: 'test', test_name: 'not_null', column: 'id' }, { kind: 'test', test_name: 'unique_combination_of_columns' }, { kind: 'test' }];
var boxColours = ['view', 'table', 'incremental', 'ephemeral', 'materialized_view']
  .map(function (m) { return nodeColor({ kind: 'model', materialized: m }); })
  .concat(['source', 'seed', 'snapshot'].map(function (k) { return nodeColor({ kind: k }); }))
  .concat(TESTS.map(function (t) { return nodeColor(t); }))
  .concat([nodeColor({ kind: 'model', materialized: 'something_custom' })]);
var clash = ROLES.filter(function (r) { return boxColours.indexOf(roleColor(r)) >= 0; });
check('no role reuses a box colour', clash.join(',') || 'none', 'none');

check('case is ignored', roleColor('Passthrough'), roleColor('passthrough'));
check('an unknown role falls back to the plain edge colour',
      roleColor('teleported'), roleColor(''));
check('a missing role does not break', typeof roleColor(undefined), 'string');
check('inferred is not dressed up as a parsed role',
      roleColor('inferred') !== roleColor('passthrough'), true);

print('\n--- regression: a selected edge must still highlight ---');
// Setting `stroke` inline would beat `.edge.hi` and leave a selected edge in its
// role colour. The role goes into a custom property for that reason.
check('the role is set as a custom property, not as stroke',
      /setProperty\('--edge-col'/.test(lin), true);
var edgeRule = css.split('\n').filter(function (l) { return /^\s*\.edge\s*\{/.test(l); })[0] || '';
check('.edge reads the custom property', /var\(--edge-col/.test(edgeRule), true);
check('.edge.hi still sets stroke outright',
      /\.edge\.hi\s*\{[^}]*stroke:\s*var\(--accent\)/.test(css), true);

print('\n--- the role badge on a column box ---');
// The badge says what produced the column it sits on, so it is read off the
// incoming edges rather than taken from the payload.
var chain = {
  nodes: [{ name: 'a', kind: 'source' }, { name: 'b' }, { name: 'c' }],
  edges: [[0, 1], [1, 2]],
  edge_kinds: ['passthrough', 'aggregate'],
};
var r = nodeRoles(chain);
check('a source\'s column has nothing feeding it, so it is raw', r[0], 'raw');
check('a column takes the role of its incoming edge', r[1], 'passthrough');
check('and so does the next one', r[2], 'aggregate');

var merge = {
  nodes: [{ name: 'a' }, { name: 'b' }, { name: 'c' }],
  edges: [[0, 2], [1, 2]],
  edge_kinds: ['passthrough', 'transform'],
};
check('two different roles feeding one column is mixed', nodeRoles(merge)[2], 'mixed');
check('mixed claims no colour of its own', roleColor('mixed'), roleColor(''));

var agree = {
  nodes: [{ name: 'a' }, { name: 'b' }, { name: 'c' }],
  edges: [[0, 2], [1, 2]],
  edge_kinds: ['passthrough', 'passthrough'],
};
check('two edges that agree are not mixed', nodeRoles(agree)[2], 'passthrough');

// A node with upstream out of view is not the start of anything, so it gets no
// badge rather than a wrong "raw".
var cut = { nodes: [{ name: 'a', kind: 'source', hidden_up: 3 }], edges: [], edge_kinds: [] };
check('a truncated upstream is not called raw', nodeRoles(cut)[0], '');

var model = { nodes: [{ name: 'a', kind: 'seed' }, { name: 'b', kind: 'source' }], edges: [[0, 1]] };
check('no edge kinds at all leaves the roles empty except the starts',
      nodeRoles(model).join(','), 'raw,raw');

// With roots in the cache, where data enters the project is raw, and a
// model's column with nothing feeding it is a literal collin names, or a
// lineage nobody knows (0096).
var starts = { roots_known: true, nodes: [
  { name: 'id', kind: 'source' }, { name: 'id', kind: 'seed' }, { name: 'code', kind: 'model', outside: true },
  { name: 'lost', kind: 'model' }, { name: 'kept', kind: 'snapshot' },
  { name: 'loaded_at', kind: 'model', generated: ['current_timestamp()'] }, { name: 'day', kind: 'model', generated: [] },
  { name: 'channel', kind: 'model', generated: ["'web'"] }, { name: 'gone', kind: 'model', generated: ['NULL'], hidden_up: 4 },
], edges: [[0, 7]], edge_kinds: ['filter'] };
check('a source\'s and a seed\'s column are raw, and so is one read from a table no node owns',
      nodeRoles(starts).slice(0, 3).join(','), 'raw,raw,raw');
check('a model\'s column with nothing feeding it that is no root has no badge, nor a snapshot\'s',
      nodeRoles(starts).slice(3, 5).join(','), ',');
check('a root is generated, with its value or without one', nodeRoles(starts).slice(5, 7).join(','), 'generated,generated');
check('a filter edge into a root never takes its badge away', nodeRoles(starts)[7], 'generated');
check('nor does an upstream out of view', nodeRoles(starts)[8], 'generated');
// A cache without roots, Fusion's, Snowflake's or an older collin's, cannot
// tell a literal from a source: its starts keep raw, as before roots.
var noRoots = { nodes: [{ name: 'loaded_at', kind: 'model' }, { name: 'code', kind: 'model', outside: true },
  { name: 'kept', kind: 'snapshot' }, { name: 'b', kind: 'model' }], edges: [[0, 3]], edge_kinds: ['rename'] };
check('without roots in the cache, a model\'s or a snapshot\'s column with nothing feeding it stays raw',
      nodeRoles(noRoots).join(','), 'raw,raw,raw,rename');

print('\n--- indirect edges: dashed, a hue each, and read by no badge (0097, 0118) ---');
// collin's filter, join key and dedup key edges say which rows a model keeps,
// not where a value comes from: one dash and no badge from them, and a hue
// each, since the page names them by their SQL clause.
var ROWS = ['filter', 'join_key', 'dedup_key'];
check('each indirect role has a hue of its own',
      ROWS.map(roleColor).filter(function (c, i, all) { return all.indexOf(c) === i; }).length, 3);
check('in any case', [roleColor('FILTER'), roleColor('Join_Key'), roleColor('DEDUP_KEY')].join(), ROWS.map(roleColor).join());
check('which no value role and no badge uses',
      ROLES.concat(['raw', 'generated', '']).filter(function (r) { return ROWS.indexOf(r) < 0 && ROWS.map(roleColor).indexOf(roleColor(r)) >= 0; }).join(), '');
check('and the legend\'s indirect entry, which lights all three, is none of them',
      ROWS.filter(function (r) { return roleColor(r) === roleColor('indirect'); }).join(), '');
var rowsIn = { roots_known: true, nodes: [
  { name: 'status', kind: 'seed' }, { name: 'loaded_at', kind: 'model' }, { name: 'id', kind: 'model' },
  { name: 'amount', kind: 'model' }, { name: 'at', kind: 'model', generated: ['now()'] },
], edges: [[0, 1], [0, 2], [3, 2], [0, 4]], edge_kinds: ['filter', 'join_key', 'rename', 'filter'] };
check('a column only an indirect edge reaches reads as one nothing feeds', nodeRoles(rowsIn)[1], '');
check('one a value edge reaches as well takes that edge\'s role, not mixed', nodeRoles(rowsIn)[2], 'rename');
check('a root reached by a filter stays generated', nodeRoles(rowsIn)[4], 'generated');
var oldRows = { nodes: rowsIn.nodes.slice(0, 2), edges: [[0, 1]], edge_kinds: ['filter'] };
check('without roots in the cache, such a column is raw, as one nothing feeds', nodeRoles(oldRows)[1], 'raw');
check('a +N of indirect edges only is no feeding either',
      nodeRoles({ roots_known: true, nodes: [{ name: 'x', kind: 'model', hidden_up: 3, hidden_up_rows: 3 }], edges: [] })[0], '');
check('a +N holding a value edge still is',
      nodeRoles({ roots_known: true, nodes: [{ name: 'x', kind: 'model', hidden_up: 3, hidden_up_rows: 2 }], edges: [] })[0], '');
check('and without roots it reads as before, fed', nodeRoles({ nodes: [{ name: 'x', kind: 'model', hidden_up: 1 }], edges: [] })[0], '');
function dash(rule) {
  var line = css.split('\n').filter(function (l) { return l.indexOf(rule + ' {') === 0; })[0] || '';
  return (line.match(/stroke-dasharray:\s*([^;]+);/) || [])[1] || '';
}
var dashes = ['.edge.test', '.edge.back', '.edge.test.back', '.edge.indirect', '.edge.indirect.back'].map(dash);
check('every dash is its own: a test, a back edge, an indirect edge and both', dashes.filter(function (v, i) {
  return v && dashes.indexOf(v) === i; }).length, 5);
check('an indirect edge is drawn before .edge.hi, which keeps its dash and takes the stroke',
      css.indexOf('.edge.indirect {') < css.indexOf('.edge.hi {') && !/\.edge\.hi \{[^}]*dasharray/.test(css), true);
check('a lit indirect edge keeps its role\'s hue, after .edge.hi, and still its dash',
      css.indexOf('.edge.hi {') < css.indexOf('.edge.indirect.hi {') && /\.edge\.indirect\.hi \{[^}]*stroke:\s*var\(--edge-col\)/.test(css)
      && !/\.edge\.indirect\.hi \{[^}]*dasharray/.test(css), true);
check('the canvas gives an indirect edge its class', /isRowKind\(d\.edge_kinds && d\.edge_kinds\[i\]\) \? ' indirect' : ''/.test(lin), true);

print('\n--- a seed does not pass for a table ---');
// Two colours that merely differ can still read as one: a seed's green sat 4
// from a table's on the scale below, and every seed was drawn as a table. The
// scale is OKLab distance x100, where 15 is the floor for two marks to read
// apart at a glance, and 8 the floor under protanopia and deuteranopia,
// simulated with the Machado, Oliveira and Fernandes (2009) matrices that
// floor was set against. Any box can sit beside a seed, and in column mode any
// role colour too, so the seed is held apart from every one of them.
var CVD = {
  protan: [[0.152286, 1.052583, -0.204868], [0.114503, 0.786281, 0.099216], [-0.003882, -0.048116, 1.051998]],
  deutan: [[0.367322, 0.860646, -0.227968], [0.280085, 0.672501, 0.047413], [-0.011820, 0.042940, 0.968881]],
};
function linearRgb(hex) {
  return [1, 3, 5].map(function (i) {
    var c = parseInt(hex.slice(i, i + 2), 16) / 255;
    return c <= 0.04045 ? c / 12.92 : Math.pow((c + 0.055) / 1.055, 2.4);
  });
}
function oklab(rgb, sim) {
  if (sim) {
    rgb = sim.map(function (row) {
      return Math.min(1, Math.max(0, row[0] * rgb[0] + row[1] * rgb[1] + row[2] * rgb[2]));
    });
  }
  var l = Math.cbrt(0.4122214708 * rgb[0] + 0.5363325363 * rgb[1] + 0.0514459929 * rgb[2]);
  var m = Math.cbrt(0.2119034982 * rgb[0] + 0.6806995451 * rgb[1] + 0.1073969566 * rgb[2]);
  var s = Math.cbrt(0.0883024619 * rgb[0] + 0.2817188376 * rgb[1] + 0.6299787005 * rgb[2]);
  return [0.2104542553 * l + 0.7936177850 * m - 0.0040720468 * s,
          1.9779984951 * l - 2.4285922050 * m + 0.4505937099 * s,
          0.0259040371 * l + 0.7827717662 * m - 0.8086757660 * s];
}
function apart(a, b, sim) {
  var p = oklab(linearRgb(a), sim), q = oklab(linearRgb(b), sim);
  return 100 * Math.sqrt(Math.pow(p[0] - q[0], 2) + Math.pow(p[1] - q[1], 2) + Math.pow(p[2] - q[2], 2));
}
check('the scale reads a colour as itself', apart('#4ec78d', '#4ec78d'), 0);
check('and black as a whole lightness from white', Math.round(apart('#000000', '#ffffff')), 100);

var seed = nodeColor({ kind: 'seed' });
var beside = {};
['view', 'table', 'incremental', 'ephemeral', 'materialized_view'].forEach(function (m) {
  beside[m] = nodeColor({ kind: 'model', materialized: m });
});
['source', 'snapshot', 'exposure'].forEach(function (k) { beside[k] = nodeColor({ kind: k }); });
TESTS.forEach(function (t) { beside['a ' + matLabel(t)] = nodeColor(t); });
beside['a custom materialization'] = nodeColor({ kind: 'model', materialized: 'something_custom' });
ROLES.forEach(function (r) { beside['the ' + r + ' role'] = roleColor(r); });
beside['the raw badge'] = roleColor('raw');
beside['the generated badge'] = roleColor('generated');
beside['the indirect edges'] = roleColor('indirect');
ROWS.forEach(function (r) { beside['the ' + r + ' edges'] = roleColor(r); });
Object.keys(beside).forEach(function (k) {
  var seen = apart(seed, beside[k]);
  var redGreen = Math.min(apart(seed, beside[k], CVD.protan), apart(seed, beside[k], CVD.deutan));
  var ok = seen >= 15 && redGreen >= 8;
  print((ok ? 'PASS  ' : 'FAIL  ') + 'a seed stands apart from ' + k + ': ' + seen.toFixed(1)
        + ', and ' + redGreen.toFixed(1) + ' to a red-green colour-blind eye' + (ok ? '' : '   expected 15 and 8'));
});

// The three indirect hues sit side by side wherever a model is filtered,
// joined and deduplicated, so they are held apart from each other, to a
// red-green colour-blind eye too, and from every colour a box or an edge of
// another role may have. The palette is crowded: ten is as far as three more
// hues go from each other, six and a half from the rest (0118).
var rowHues = ROWS.map(roleColor);
var rowPairs = [[0, 1], [0, 2], [1, 2]].map(function (p) {
  return Math.min(apart(rowHues[p[0]], rowHues[p[1]]), apart(rowHues[p[0]], rowHues[p[1]], CVD.protan), apart(rowHues[p[0]], rowHues[p[1]], CVD.deutan));
});
check('the three indirect hues are ' + Math.min.apply(null, rowPairs).toFixed(1) + ' from each other at least, ten at that',
      Math.min.apply(null, rowPairs) >= 10, true);
var elsewhere = boxColours.concat(ROLES.map(roleColor), [roleColor('raw'), roleColor('generated'), roleColor('')]);
var rowNearest = Math.min.apply(null, rowHues.map(function (c) {
  return Math.min.apply(null, elsewhere.map(function (o) { return apart(c, o); }));
}));
check('and ' + rowNearest.toFixed(1) + ' from every other colour, six and a half at least', rowNearest >= 6.5, true);

// The two badges of a start sit side by side wherever a model reads a source
// beside a literal, so they are held to a seed's floors against each other.
var gen = roleColor('generated'), raw = roleColor('raw');
var genRedGreen = Math.min(apart(gen, raw, CVD.protan), apart(gen, raw, CVD.deutan));
check('generated stands apart from raw, to a red-green colour-blind eye too', apart(gen, raw) >= 15 && genRedGreen >= 8, true);
check('generated reuses no box colour and no role colour', boxColours.concat(ROLES.map(roleColor), [raw]).indexOf(gen), -1);
// A seed's floor of 15 against every colour of the canvas is out of reach:
// the one hue left that far from all of them is a neon mint at the edge of
// sRGB, 15.3 from a materialized view and 15.8 from a table, the boxes the
// badge sits on most. The pink is held to what the roles already hold:
// farther from its nearest colour than any role or raw but window is from
// theirs, and never under 12 (0096).
function nearest(c, among) {
  return Math.min.apply(null, among.filter(function (o) { return o !== c; }).map(function (o) { return apart(c, o); }));
}
var canvas = boxColours.concat(ROLES.map(roleColor), [raw, gen, roleColor('')]);
var genNearest = nearest(gen, canvas);
check('generated is ' + genNearest.toFixed(1) + ' from its nearest colour on the canvas, 12 at least', genNearest >= 12, true);
check('which no role and not raw keeps from theirs, window apart',
      ROLES.concat(['raw']).filter(function (r) { return r !== 'window' && nearest(roleColor(r), canvas) >= genNearest; }).join(','), '');

print('\n--- the kinds of test stand apart ---');
// The three kinds of test are greys, slate at most like the rest of the
// palette, so a row reads as a test beside the coloured boxes, and they are
// held to the seed's floor against each other (0040). Not against the boxes: no light grey clears a materialized view's
// cyan to a red-green colour-blind eye, and the column test's grey sat 3.4
// from an ephemeral model before there were kinds. A one-line row on a stem
// is what tells a test from a model, and every kind has a shape of its own.
TESTS.forEach(function (t) {
  var lab = oklab(linearRgb(nodeColor(t))), chroma = Math.sqrt(lab[1] * lab[1] + lab[2] * lab[2]);
  print((chroma < 0.04 ? 'PASS  ' : 'FAIL  ') + 'a ' + matLabel(t) + ' is a grey: chroma ' + chroma.toFixed(3) + (chroma < 0.04 ? '' : '   expected under 0.04'));
});
TESTS.forEach(function (t, i) {
  TESTS.slice(i + 1).forEach(function (u) {
    var a = nodeColor(t), b = nodeColor(u), seen = apart(a, b);
    var redGreen = Math.min(apart(a, b, CVD.protan), apart(a, b, CVD.deutan));
    var ok = seen >= 15 && redGreen >= 8;
    print((ok ? 'PASS  ' : 'FAIL  ') + 'a ' + matLabel(t) + ' stands apart from a ' + matLabel(u) + ': ' + seen.toFixed(1)
          + ', and ' + redGreen.toFixed(1) + ' to a red-green colour-blind eye' + (ok ? '' : '   expected 15 and 8'));
  });
});
var light = TESTS.map(function (t) { return oklab(linearRgb(nodeColor(t)))[0]; });
check('lighter in the order the rows go: the model\'s, the singular, the column\'s', light[1] > light[2] && light[2] > light[0], true);
