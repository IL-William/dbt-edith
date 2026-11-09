// Where the hover card lands next to what it describes: below by preference,
// above when there is no room, clamped rather than clipped when neither fits.
// Run from the repository root: jsc web/tests/hovercard.js
var src = read('web/app.js');
eval(src.slice(src.indexOf('function placeFloating'), src.indexOf('function hoverCardBody')));

function check(label, got, want) {
  var g = JSON.stringify(got), w = JSON.stringify(want);
  print((g === w ? 'PASS  ' : 'FAIL  ') + label + (g === w ? '' : '\n        expected ' + w + '\n        got      ' + g));
}
function atRect(left, top, width, height) {
  return { left: left, top: top, right: left + width, bottom: top + height };
}
var VIEW = { width: 1200, height: 800 };
var CARD = { width: 320, height: 200 };

print('--- room below ---');
var below = placeFloating(atRect(100, 100, 200, 48), CARD, VIEW);
check('sits under the anchor, one gap down', below.top, 156);
check('aligned on the anchor left edge', below.left, 100);
check('not flagged as flipped', below.above, false);

print('\n--- no room below, room above ---');
var flipped = placeFloating(atRect(100, 700, 200, 48), CARD, VIEW);
check('flips above the anchor', flipped.top, 492);
check('flagged as flipped', flipped.above, true);

print('\n--- no room either way ---');
var tight = placeFloating(atRect(10, 60, 200, 700), CARD, { width: 1200, height: 780 });
check('clamped into the viewport', tight.top, 576);
check('never flagged as flipped when clamped', tight.above, false);
check('stays on screen', tight.top >= 4 && tight.top + CARD.height <= 780, true);

print('\n--- a card taller than the viewport still starts on screen ---');
var tall = placeFloating(atRect(10, 10, 200, 20), { width: 320, height: 900 }, VIEW);
check('top is never negative', tall.top, 4);

print('\n--- horizontal clamping ---');
check('clamped at the right edge', placeFloating(atRect(1150, 100, 40, 20), CARD, VIEW).left, 876);
check('anchor off screen left pins to the margin', placeFloating(atRect(-300, 100, 40, 20), CARD, VIEW).left, 4);
check('a card wider than the viewport pins left, never negative',
  placeFloating(atRect(600, 100, 40, 20), { width: 1400, height: 100 }, VIEW).left, 4);

print('\n--- degenerate anchors ---');
var zero = placeFloating(atRect(500, 300, 0, 0), CARD, VIEW);
check('a zero-sized anchor still places below it', zero.top, 308);
check('and keeps its left', zero.left, 500);

print('\n--- the gap is a parameter ---');
check('a wider gap pushes the card further down', placeFloating(atRect(100, 100, 200, 48), CARD, VIEW, 20).top, 168);

print('\n--- a view that is not the window: the editor (0167) ---');
var EDITOR = { top: 100, left: 260, width: 700, height: 400 };      // rows 100..500, columns 260..960
check('below the anchor when it fits in the view',
  placeFloating(atRect(300, 150, 60, 18), CARD, EDITOR).top, 176);
var low = placeFloating(atRect(300, 440, 60, 18), CARD, EDITOR);
check('a low line opens above, not over what is under the view', low.top, 232);
check('flagged as flipped', low.above, true);
check('never lower than the view\'s bottom', low.top + CARD.height <= 100 + 400, true);
check('left edge is the view\'s, not the window\'s', placeFloating(atRect(100, 150, 40, 18), CARD, EDITOR).left, 264);
check('right edge is the view\'s', placeFloating(atRect(900, 150, 40, 18), CARD, EDITOR).left, 960 - 320 - 4);
check('clamped inside the view when neither side fits',
  placeFloating(atRect(300, 250, 60, 18), { width: 320, height: 380 }, EDITOR).top, 116);
check('the window is the default view: no top and no left',
  placeFloating(atRect(100, 700, 200, 48), CARD, { width: 1200, height: 800, top: 0, left: 0 }).top, 492);

print('\n--- the room a card may take ---');
var WIN = { width: 1200, height: 800 };
var inside = { left: 260, top: 100, right: 960, bottom: 500 };
check('no editor: the window', hoverView(WIN, null, CARD), { top: 0, left: 0, width: 1200, height: 800 });
check('the editor, when the card fits in it', hoverView(WIN, inside, CARD), { top: 100, left: 260, width: 700, height: 400 });
check('the editor cut by the window\'s edge', hoverView(WIN, { left: 260, top: 100, right: 1500, bottom: 900 }, CARD),
  { top: 100, left: 260, width: 940, height: 700 });
check('a card taller than the editor falls back to the window', hoverView(WIN, inside, { width: 330, height: 420 }),
  { top: 0, left: 0, width: 1200, height: 800 });
check('a card wider than the editor falls back to the window too', hoverView({ width: 840, height: 800 }, { left: 520, top: 100, right: 840, bottom: 500 }, { width: 330, height: 200 }),
  { top: 0, left: 0, width: 840, height: 800 });
check('an editor scrolled out of the window falls back too', hoverView(WIN, { left: 0, top: -300, right: 900, bottom: 60 }, CARD),
  { top: 0, left: 0, width: 1200, height: 800 });

print('\n--- what a test checks ---');
var col = { kind: 'test', name: 'not_null_orders_customer_id', test_name: 'not_null', column: 'customer_id' };
check('a column test names its generic and column', testFacts(col, 'column'), [{ text: 'not_null on customer_id' }]);
check('a generic on the whole model says so, with its package',
  testFacts({ kind: 'test', test_name: 'unique_combination_of_columns', namespace: 'dbt_utils' }, 'model'),
  [{ text: 'unique_combination_of_columns on the whole model' }, { text: 'generic from dbt_utils' }]);
check('a singular test at warn', testFacts({ kind: 'test', name: 'assert_positive', warn: true }, 'singular'),
  [{ text: 'A singular test: SQL of its own, no generic' }, { text: 'severity: warn', warn: true }]);
check('a model has none', testFacts({ kind: 'model', name: 'orders', test_name: 'x' }, 'model'), []);
check('nor does nothing', testFacts(null, 'column'), []);
