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
  nodes: [{ name: 'a' }, { name: 'b' }, { name: 'c' }],
  edges: [[0, 1], [1, 2]],
  edge_kinds: ['passthrough', 'aggregate'],
};
var r = nodeRoles(chain);
check('the first column has nothing feeding it, so it is raw', r[0], 'raw');
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
var cut = { nodes: [{ name: 'a', hidden_up: 3 }], edges: [], edge_kinds: [] };
check('a truncated upstream is not called raw', nodeRoles(cut)[0], '');

var model = { nodes: [{ name: 'a' }, { name: 'b' }], edges: [[0, 1]] };
check('no edge kinds at all leaves the roles empty except the start',
      nodeRoles(model).join(','), 'raw,raw');

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
Object.keys(beside).forEach(function (k) {
  var seen = apart(seed, beside[k]);
  var redGreen = Math.min(apart(seed, beside[k], CVD.protan), apart(seed, beside[k], CVD.deutan));
  var ok = seen >= 15 && redGreen >= 8;
  print((ok ? 'PASS  ' : 'FAIL  ') + 'a seed stands apart from ' + k + ': ' + seen.toFixed(1)
        + ', and ' + redGreen.toFixed(1) + ' to a red-green colour-blind eye' + (ok ? '' : '   expected 15 and 8'));
});

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
