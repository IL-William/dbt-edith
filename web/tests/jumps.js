// The back and forward stacks: which entry a jump lands on once tabs have
// closed under it.
// Run from the repository root: jsc web/tests/jumps.js
var src = read('web/app.js');
eval(src.slice(src.indexOf('function nextJump'), src.indexOf('function jump(')));

function check(label, got, want) {
  var g = JSON.stringify(got), w = JSON.stringify(want);
  print((g === w ? 'PASS  ' : 'FAIL  ') + label + (g === w ? '' : '\n        expected ' + w + '\n        got      ' + g));
}
function openTabs() {
  var m = new Map();
  for (var i = 0; i < arguments.length; i++) m.set(arguments[i], {});
  return m;
}
var at = function (k, l) { return { key: k, line: l, ch: 0 }; };

print('--- nextJump ---');
var stack = [at('a.sql', 1), at('b.sql', 2), at('c.sql', 3)];

var r = nextJump(stack, openTabs('a.sql', 'b.sql', 'c.sql'));
check('takes the newest entry', r.at, at('c.sql', 3));
check('  and leaves the rest', r.rest.map(function (e) { return e.key; }), ['a.sql', 'b.sql']);
check('  without touching the stack it was given', stack.length, 3);

r = nextJump(stack, openTabs('a.sql', 'b.sql'));
check('skips a tab closed since', r.at, at('b.sql', 2));
check('  dropping the dead entry with it', r.rest.map(function (e) { return e.key; }), ['a.sql']);

r = nextJump(stack, openTabs('a.sql'));
check('skips as many as it has to', r.at, at('a.sql', 1));
check('  leaving nothing behind', r.rest, []);

r = nextJump(stack, openTabs());
check('every tab closed: nowhere to go', r.at, null);
check('  and the stack is spent, not stuck', r.rest, []);

check('an empty stack goes nowhere', nextJump([], openTabs('a.sql')).at, null);

print('\n--- a tab with no cursor ---');
// A diff or the profile is kept as a tab alone, with no line to restore.
check('kept, and said to have no line',
  nextJump([{ key: 'diff:a.sql' }], openTabs('diff:a.sql')).at, { key: 'diff:a.sql' });
check('  line is undefined, which is what jump() tests',
  nextJump([{ key: 'diff:a.sql' }], openTabs('diff:a.sql')).at.line === undefined, true);
