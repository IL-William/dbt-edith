// A model's tests as slips under it, and the box they hang from (0160): what a
// row says in two runs, the bar in its three shapes inside 3 by 10, the room a
// row and a box have for text, the +N pill, and the glow's ring. The browser
// half has no DOM (0013): a few fake elements stand in for the SVG ones.
// Run from the repository root: jsc web/tests/slips.js
var lin = read('web/lineage.js');
var css = read('web/app.css');
var NS = 'http://www.w3.org/2000/svg';
function Fake(name) { this.name = name; this.attrs = {}; this.kids = []; this._text = ''; }
Fake.prototype.setAttribute = function (k, v) { this.attrs[k] = String(v); };
Fake.prototype.getAttribute = function (k) { return k in this.attrs ? this.attrs[k] : null; };
Fake.prototype.appendChild = function (c) { this.kids.push(c); return c; };
Object.defineProperty(Fake.prototype, 'textContent', {
  get: function () { return this._text + this.kids.map(function (k) { return k.textContent; }).join(''); },
  set: function (v) { this._text = v; this.kids = []; },
});
var document = { createElementNS: function (ns, name) { return new Fake(name); } };
var MODEL_W = +/const MODEL_W = (\d+)/.exec(lin)[1];
// One eval, whose consts come back by name: a const does not leave an eval.
var C = eval(lin.slice(lin.indexOf('const el = (name'), lin.indexOf('  /* Second line of a node box.'))
  + lin.slice(lin.indexOf('  function rowLabel('), lin.indexOf('  // What a fold chip says'))
  + lin.slice(lin.indexOf('  const SLIP_BAR'), lin.indexOf('  // The room a warn mark'))
  + lin.slice(lin.indexOf('  function pillSize'), lin.indexOf('  // One glow, on the box clicked'))
  + '; ({ SLIP_BAR: SLIP_BAR, NAME_PX: NAME_PX, SUB_PX: SUB_PX, PAD_L: PAD_L, PAD_R: PAD_R })');
var SLIP_BAR = C.SLIP_BAR, NAME_PX = C.NAME_PX, SUB_PX = C.SUB_PX, PAD_L = C.PAD_L, PAD_R = C.PAD_R;

function check(label, got, want) {
  var g = JSON.stringify(got), w = JSON.stringify(want);
  print((g === w ? 'PASS  ' : 'FAIL  ') + label + (g === w ? '' : '\n        expected ' + w + '\n        got      ' + g));
}
function ok(label, cond) { print((cond ? 'PASS  ' : 'FAIL  ') + label); }

print('--- a row, the generic and its column in two runs ---');
check('a column test: the name, then its column', rowParts('not_null · customer_id', true), { name: 'not_null', col: 'customer_id' });
check('a column cut by rowClip keeps the runs apart', rowParts('accepted_v… · pay…', true), { name: 'accepted_v…', col: 'pay…' });
check('even when only the dot is left', rowParts('accepted_values ·…', true), { name: 'accepted_values', col: '…' });
check('a singular test is its name alone, dots or not', rowParts('assert_a · b', false), { name: 'assert_a · b', col: '' });
check('a row with no dot is one run', rowParts('unique', true), { name: 'unique', col: '' });
function row(text, col) {
  var t = new Fake('text');
  writeRow(t, text, col, function (n) { return new Fake(n); });
  return t;
}
var two = row('not_null · customer_id', true);
check('written as two tspans: the generic, then the column at a gap, with no dot between them',
  [two.kids.map(function (k) { return [k.name, k.attrs.class, k.textContent]; }), two.kids[1].attrs.dx, two.textContent],
  [[['tspan', 'rn', 'not_null'], ['tspan', 'rc', 'customer_id']], '6', 'not_nullcustomer_id']);
check('a singular test is plain text, no tspan', [row('assert_a', false).kids.length, row('assert_a', false).textContent], [0, 'assert_a']);
check('a lone column test says its column to rowLabel', [
  rowLabel({ type: 'row', level: 'column', v: 0 }, { nodes: [{ test_name: 'not_null', column: 'id', name: 'x' }] }, 'orders'),
  rowLabel({ type: 'row', level: 'singular', v: 0 }, { nodes: [{ name: 'assert_x' }] }, 'orders'),
  rowLabel({ type: 'row', level: 'model', v: 0 }, { nodes: [{ test_name: 'unique_combination_of_columns', name: 'y' }] }, 'orders'),
], [{ text: 'not_null · id', keep: 5, col: true }, { text: 'assert_x', keep: 0 }, { text: 'unique_combination_of_columns', keep: 0 }]);

print('\n--- the bar: 3 by 10, inset 6, a shape for each level ---');
function bars(level) {
  var g = new Fake('g'), x = riderBars(g, level, 22, 'var(--t-column)');
  return { x: x, rects: g.kids.map(function (k) { return [+k.attrs.x, +k.attrs.y, +k.attrs.width, +k.attrs.height, k.attrs.class]; }) };
}
check('a generic on a column is one bar, 3 by 10, centred in the 22px row', bars('column'), { x: 15, rects: [[6, 6, 3, 10, 'kindbar']] });
check('on the whole model, that bar and a thin one beside it', bars('model'), { x: 18, rects: [[6, 6, 3, 10, 'kindbar'], [10, 6, 2, 10, 'kindbar']] });
check('a singular test is three pieces, 3 by 2 and 2 apart, filling the same 3 by 10', bars('singular'),
  { x: 15, rects: [[6, 6, 3, 2, 'kindbar seg'], [6, 10, 3, 2, 'kindbar seg'], [6, 14, 3, 2, 'kindbar seg']] });
ok('the pieces end where the whole bar does', 14 + 2 === 6 + SLIP_BAR.h);
ok('and the three look different from one another', JSON.stringify(bars('column').rects) !== JSON.stringify(bars('model').rects)
  && JSON.stringify(bars('model').rects) !== JSON.stringify(bars('singular').rects));
ok('the bar\'s colour is an inline style, which no rule repaints', /class: 'kindbar', x: SLIP_BAR\.x[^}]*style: `fill:\$\{colour\}`/.test(lin));

print('\n--- the room a row and a box have ---');
var rowW = MODEL_W - 18;
check('a row has room for a name the width allows, after its bar and a margin', riderMax(rowW, 15, 0), Math.floor((rowW - 15 - 8) / 6.5));
ok('which fits the row at 6.5px a character', riderMax(rowW, 15, 0) * 6.5 <= rowW - 15 - 8);
ok('less beside a warn mark', riderMax(rowW, 15, 17) < riderMax(rowW, 15, 0));
var room = boxRoom(false);
ok('the longest name a model box keeps fits it at 13px (7.4px a character)', room.name * NAME_PX <= MODEL_W - PAD_L - PAD_R);
ok('and one character more would not', (room.name + 1) * NAME_PX > MODEL_W - PAD_L - PAD_R);
ok('the second line, 11px mono (6.6px), fits the same', room.sub * SUB_PX <= MODEL_W - PAD_L - PAD_R + 2);
check('a column box keeps its room until its own step', boxRoom(true), { name: 24, sub: 30 });
check('the model box is 220 wide', MODEL_W, 220);

print('\n--- the +N pill ---');
check('24 by 18 for a short number', pillSize('+1'), { w: 24, h: 18 });
check('wider where the number is', [pillSize('+12').w, pillSize('+123').w], [26, 32]);

print('\n--- the glow: a ring just outside the border ---');
var nd = { classList: { contains: function (c) { return c === 'rider'; } }, querySelector: function () { return { getAttribute: function (k) { return k === 'width' ? '202' : '22'; } }; } };
var g = glowOf(nd);
check('a ring 2px out, its radius two more than a slip\'s', [g.attrs.class, +g.attrs.x, +g.attrs.y, +g.attrs.width, +g.attrs.height, +g.attrs.rx], ['glow', -2, -2, 206, 26, 7]);
ok('3px wide at 16% of the accent, in the stylesheet, and no filter or shadow anywhere on the canvas',
  /\.glow \{ fill: none; stroke: rgba\(var\(--accent-rgb\), \.16\); stroke-width: 3; \}/.test(css) && !/filter:\s*drop-shadow/.test(css.slice(css.indexOf('.nd rect.box'), css.indexOf('.bdot {'))));
ok('hidden under Halloween, whose states have colours of their own', /html\[data-theme="halloween"\] \.glow \{ display: none; \}/.test(css));
