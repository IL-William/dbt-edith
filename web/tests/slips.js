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
var MODEL_W = +/const MODEL_W = (\d+)/.exec(lin)[1], COLUMN_W = +/COLUMN_W = (\d+)/.exec(lin)[1];
var W = COLUMN_W;
// One eval, whose consts come back by name: a const does not leave an eval.
var C = eval(lin.slice(lin.indexOf('const el = (name'), lin.indexOf('  /* Second line of a node box.'))
  + lin.slice(lin.indexOf('  function rowLabel('), lin.indexOf('  // What a fold chip says'))
  + lin.slice(lin.indexOf('  const SLIP_BAR'), lin.indexOf('  // The room a warn mark'))
  + lin.slice(lin.indexOf('  function pillSize'), lin.indexOf('  // One glow, on the box clicked'))
  + lin.slice(lin.indexOf('const MAT = {'), lin.indexOf('let svg, root'))
  + lin.slice(lin.indexOf('  function writeSub('), lin.indexOf('  /* What a box\'s strategy says to a screen reader'))
  + lin.slice(lin.indexOf('  function subLine('), lin.indexOf('  /* What a generated column is set to'))
  + lin.slice(lin.indexOf('  function subJoin('), lin.indexOf('  /* The second line in three parts'))
  + lin.slice(lin.indexOf('  const ROLE_TEXT'), lin.indexOf('  /* The count of a column\'s transformations'))
  + '; ({ SLIP_BAR: SLIP_BAR, NAME_PX: NAME_PX, SUB_PX: SUB_PX, PAD_L: PAD_L, PAD_R: PAD_R, COLUMN_NAME_PX: COLUMN_NAME_PX, TAG: TAG, ROLE_TEXT: ROLE_TEXT })');
var SLIP_BAR = C.SLIP_BAR, NAME_PX = C.NAME_PX, SUB_PX = C.SUB_PX, PAD_L = C.PAD_L, PAD_R = C.PAD_R;
var COLUMN_NAME_PX = C.COLUMN_NAME_PX, TAG = C.TAG, ROLE_TEXT = C.ROLE_TEXT;

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
check('a member told by its column is that column\'s run alone, no generic and no gap',
  [rowParts('customer_id', 'alone'), (function () { var t = row('customer_id', 'alone'); return [t.kids.map(function (k) { return [k.attrs.class, k.textContent, k.attrs.dx]; }), t.textContent]; })()],
  [{ name: '', col: 'customer_id' }, [[['rc', 'customer_id', null]], 'customer_id']]);
check('a member told by its name stays one run',
  rowLabel({ type: 'member', byName: true, v: 0 }, { nodes: [{ name: 'orders_min_1', test_name: 'row_count', column: 'id' }] }, 'orders'),
  { text: 'min_1', keep: 0 });
check('a member told by its column says it alone to rowLabel',
  rowLabel({ type: 'member', byName: false, v: 0 }, { nodes: [{ name: 'x', test_name: 'not_null', column: 'id' }] }, 'orders'),
  { text: 'id', keep: 0, col: 'alone' });
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
check('the model box is 220 wide', MODEL_W, 220);

print('\n--- a column box: 250 wide, the role tag at the right of its second line (0162) ---');
check('the column box is 250 wide', COLUMN_W, 250);
var croom = boxRoom(true);
ok('the longest name a column box keeps fits it at 12.5px mono (7.5px a character)', croom.name * COLUMN_NAME_PX <= COLUMN_W - PAD_L - PAD_R);
ok('and one character more would not', (croom.name + 1) * COLUMN_NAME_PX > COLUMN_W - PAD_L - PAD_R);
ok('the second line with no tag has the box to the right, less the margin',
  croom.sub * SUB_PX <= COLUMN_W - PAD_R + 2 - PAD_L && (croom.sub + 1) * SUB_PX > COLUMN_W - PAD_R + 2 - PAD_L);
check('a tag is its word at 6.1px a character and 12px of padding', [tagWidth('raw'), tagWidth('aggregate'), tagWidth('passthrough')], [30, 67, 79]);
var ROLES = Array.from(ROLE_TEXT).concat(['mixed']);
check('the second line stops 8px short of the tag, for each role the box can wear', ROLES.filter(function (r) {
  var room = boxRoom(true, r), tagLeft = COLUMN_W - TAG.edge - tagWidth(r);
  return !(PAD_L + room.sub * SUB_PX <= tagLeft - TAG.gap && PAD_L + (room.sub + 1) * SUB_PX > tagLeft - TAG.gap);
}), []);
// What is drawn is cut to the room, name and all: a bare name, which has no spaces to fold, fills it, so it must not pass the tag.
function drawnRight(sub, role) {
  var t = new Fake('text'), room = boxRoom(true, role).sub;
  writeSub(t, { lead: sub, struck: '', rest: '' }, room, function (n) { return new Fake(n); });
  return PAD_L + t.textContent.length * SUB_PX;
}
var LONGNAME = 'stg_jaffle_shop__payments', TYPED = LONGNAME + '  \u00b7  DOUBLE';
check('a 25 character model name, with a type or none, stops 8px short of each tag at the most', ROLES.filter(function (r) {
  var tagLeft = COLUMN_W - TAG.edge - tagWidth(r);
  return drawnRight(LONGNAME, r) > tagLeft - TAG.gap || drawnRight(TYPED, r) > tagLeft - TAG.gap;
}), []);
check('and is cut with an ellipsis where it does not fit, the AGGREGATE tag included', [boxRoom(true, 'aggregate').sub < LONGNAME.length, /\u2026$/.test((function () {
  var t = new Fake('text'); writeSub(t, { lead: LONGNAME, struck: '', rest: '' }, boxRoom(true, 'aggregate').sub, function (n) { return new Fake(n); }); return t.textContent; })())], [true, true]);
// The line as subtitleParts hands it, one space to a run as SVG draws it: a type that fits beside the tag is drawn whole.
check('stg_payments \u00b7 DOUBLE fits whole beside TRANSFORM', (function () {
  var t = new Fake('text'); writeSub(t, { lead: 'stg_payments \u00b7 DOUBLE', struck: '', rest: '' }, boxRoom(true, 'transform').sub, function (n) { return new Fake(n); }); return t.textContent; })(),
  'stg_payments \u00b7 DOUBLE');
ok('a longer word leaves less', boxRoom(true, 'passthrough').sub < boxRoom(true, 'aggregate').sub && boxRoom(true, 'aggregate').sub < boxRoom(true, 'raw').sub);
check('the name keeps the whole width, the tag being on the second line', boxRoom(true, 'passthrough').name, croom.name);
check('a model\'s box has no tag and keeps its room whatever it is told', boxRoom(false, 'aggregate'), boxRoom(false));

print('\n--- a box at the mono font the page has ---');
check('the width a Mac\'s mono font advances, and one within 2% of it, is left as it is', [monoScaleOf(0.6), monoScaleOf(0.605), monoScaleOf(0.595)], [1, 1, 1]);
check('what is no font\'s advance is left as it is too', [monoScaleOf(0), monoScaleOf(NaN), monoScaleOf(Infinity), monoScaleOf(undefined), monoScaleOf(0.3), monoScaleOf(1.2)], [1, 1, 1, 1, 1, 1]);
check('Consolas, 1126 units of 2048, takes the characters narrower', Math.round(monoScaleOf(1126 / 2048) * 10000) / 10000, 0.9163);
var consolas = monoScaleOf(1126 / 2048);
check('a column box at Consolas keeps more of its name and of its second line', [boxRoom(true, 'passthrough', consolas), boxRoom(true, 'passthrough')], [{ name: 31, sub: 22 }, { name: 29, sub: 20 }]);
check('the scale is the default of boxRoom, and 1 until the page has measured', boxRoom(true, 'passthrough'), boxRoom(true, 'passthrough', 1));
check('a model\'s name is in the UI font and keeps its room, its mono line gains', [boxRoom(false, '', consolas).name, boxRoom(false, '', consolas).sub > boxRoom(false).sub], [boxRoom(false).name, true]);
check('at the Consolas scale the second line still stops 8px short of each tag', ROLES.filter(function (r) {
  var room = boxRoom(true, r, consolas), tagLeft = COLUMN_W - TAG.edge - tagWidth(r);
  return !(PAD_L + room.sub * SUB_PX * consolas <= tagLeft - TAG.gap && PAD_L + (room.sub + 1) * SUB_PX * consolas > tagLeft - TAG.gap);
}), []);
check('and the name keeps the box less its margins', (function () {
  var room = boxRoom(true, 'raw', consolas);
  return room.name * COLUMN_NAME_PX * consolas <= COLUMN_W - PAD_L - PAD_R && (room.name + 1) * COLUMN_NAME_PX * consolas > COLUMN_W - PAD_L - PAD_R;
})(), true);
var sf = monoScaleOf(1266 / 2048);
check('SF Mono, 1266 units of 2048, is outside the 2% and keeps its measured scale, a character fewer in a column box', [Math.round(sf * 1000) / 1000, boxRoom(true, 'passthrough', sf).name, boxRoom(true, 'passthrough', sf).sub],
  [1.03, boxRoom(true, 'passthrough', 1).name - 1, boxRoom(true, 'passthrough', 1).sub - 1]);
// measureMono against a stub page, in an eval of its own with the room it fills.
(function () {
  var measured = 0, advance = 549.8, throws = false;
  // Put back after, so a check added below meets the page it had.
  var had = { root: document.documentElement, make: document.createElement, style: globalThis.getComputedStyle };
  document.documentElement = {};
  document.createElement = function () {
    if (throws) throw new Error('no canvas');
    return { getContext: function () { return { measureText: function (s) { measured++; return { width: advance * s.length / 10 }; } }; } };
  };
  globalThis.getComputedStyle = function () { return { getPropertyValue: function () { return ' ui-monospace, Consolas, monospace'; } }; };
  function fresh() {
    return eval(lin.slice(lin.indexOf('const el = (name'), lin.indexOf('  /* Second line of a node box.'))
      + lin.slice(lin.indexOf('  let monoRead'), lin.indexOf('  /* `opts.folders` draws the canvas'))
      + '; ({ boxRoom: boxRoom, measureMono: measureMono })');
  }
  var page = fresh(), before = page.boxRoom(true, 'passthrough');
  page.measureMono();
  check('measuring a Consolas stub, the page\'s boxes pick up about 0.9163 by default', [before, page.boxRoom(true, 'passthrough')],
    [page.boxRoom(true, 'passthrough', 1), page.boxRoom(true, 'passthrough', 1126 / 2048 / 0.6)]);
  page.measureMono();
  check('and a second call does not measure again', measured, 1);
  throws = true;
  var broken = fresh();
  broken.measureMono();
  check('a canvas that throws leaves the Mac\'s widths', broken.boxRoom(true, 'passthrough'), broken.boxRoom(true, 'passthrough', 1));
  document.documentElement = had.root;
  document.createElement = had.make;
  globalThis.getComputedStyle = had.style;
})();
check('measuring costs one measureText for the page, never one a box', [(lin.match(/measureText\(/g) || []).length, /function measureMono\(\) \{\s*if \(monoRead\) return;/.test(lin), /function render\(d, opts = \{\}\) \{\s*measureMono\(\);/.test(lin)], [1, true, true]);
function drawn(role) {
  var g = roleTag(role);
  return { cls: g.attrs.class, role: g.attrs['data-role'], rect: g.kids[0], text: g.kids[1] };
}
var tag = drawn('aggregate');
check('a tag is a group of a rect and a word, and says its role', [tag.cls, tag.role, tag.rect.name, tag.text.name, tag.text.textContent], ['roletag', 'aggregate', 'rect', 'text', 'AGGREGATE']);
check('the rect is 14 tall at 25, its width the word\'s, 10px in from the right, radius 3, no fill of its own (the stylesheet\'s none)',
  [+tag.rect.attrs.x, +tag.rect.attrs.y, +tag.rect.attrs.width, +tag.rect.attrs.height, +tag.rect.attrs.rx, 'fill' in tag.rect.attrs], [173, 25, 67, 14, 3, false]);
check('its outline is the role\'s colour at 55%, inline, so an export keeps it', [tag.rect.attrs.style, +tag.rect.attrs['stroke-opacity']], ['stroke:var(--r-aggregate)', 0.55]);
check('the word is centred in it on the second line\'s baseline, in the role\'s text twin', [+tag.text.attrs.x, +tag.text.attrs.y, tag.text.attrs['text-anchor'], tag.text.attrs.class, tag.text.attrs.style],
  [206.5, 35.5, 'middle', 'roletaglabel', 'fill:var(--r-aggregate-text)']);
var mixed = drawn('mixed');
check('mixed, which has no colour of its own, is neutral in its outline and its word', [mixed.rect.attrs.style, mixed.text.attrs.style], ['stroke:var(--fg-dim)', 'fill:var(--fg-dim)']);
check('and so is a role a newer producer wrote', drawn('teleported').text.attrs.style, 'fill:var(--fg-dim)');
check('a role is read in any case', drawn('Raw').text.attrs.style, 'fill:var(--r-raw-text)');
check('a drawn box tells its role from its tag, and none from none', [tagOf({ querySelector: function () { return roleTag('window'); } }), tagOf({ querySelector: function () { return null; } })], ['window', '']);
ok('every role that has a tag has a text twin of its colour, named for it', Array.from(ROLE_TEXT).every(function (r) { return roleColor(r) === 'var(--r-' + r + ')'; }));

print('\n--- the +N pill ---');
check('24 by 18 for a short number', pillSize('+1'), { w: 24, h: 18 });
check('wider where the number is', [pillSize('+12').w, pillSize('+123').w], [26, 32]);
check('the reach past a box\'s edge is the widest pill and its 4px gap, whichever side', [
  badgeReach([{ hidden_up: 1 }]), badgeReach([{ hidden_down: 1234 }, { hidden_up: 3 }]), badgeReach([{ hidden_up: 12345 }]), badgeReach([{ name: 'a' }, { hidden_up: 0 }]), badgeReach([])],
  [28, 42, 48, 0, 0]);

print('\n--- the glow: a ring just outside the border ---');
var nd = { classList: { contains: function (c) { return c === 'rider'; } }, querySelector: function () { return { getAttribute: function (k) { return k === 'width' ? '202' : '22'; } }; } };
var g = glowOf(nd);
check('a ring 2px out, its radius two more than a slip\'s', [g.attrs.class, +g.attrs.x, +g.attrs.y, +g.attrs.width, +g.attrs.height, +g.attrs.rx], ['glow', -2, -2, 206, 26, 7]);
ok('3px wide at 16% of the accent, in the stylesheet, and no filter or shadow anywhere on the canvas',
  /\.glow \{ fill: none; stroke: rgba\(var\(--accent-rgb\), \.16\); stroke-width: 3; \}/.test(css) && !/filter:\s*drop-shadow/.test(css.slice(css.indexOf('.nd rect.box'), css.indexOf('.bdot {'))));
ok('hidden under Halloween, whose states have colours of their own', /html\[data-theme="halloween"\] \.glow \{ display: none; \}/.test(css));
