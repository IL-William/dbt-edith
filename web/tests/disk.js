// The one stretch a tab replaces when its file changed on disk.
// Run from the repository root: jsc web/tests/disk.js
var src = read('web/app.js');
eval(src.slice(src.indexOf('function changedSpan'), src.indexOf('async function refreshOpenFiles')));

function check(label, got, want) {
  var g = JSON.stringify(got), w = JSON.stringify(want);
  print((g === w ? 'PASS  ' : 'FAIL  ') + label + (g === w ? '' : '\n        expected ' + w + '\n        got      ' + g));
}

function applied(cur, next) {
  var s = changedSpan(cur, next);
  return cur.slice(0, s.from) + s.text + cur.slice(s.to);
}

print('--- changedSpan ---');
var before = 'with orders as (\n    select *\n),\n\nfinal as (\n    select 1\n)\n';
var after = 'with customers as (\n    select *\n),\n\norders as (\n    select *\n),\n\nfinal as (\n    select 1\n)\n';
var span = changedSpan(before, after);
check('a CTE added on top is inserted, the old text untouched', span,
  { from: 5, to: 5, text: 'customers as (\n    select *\n),\n\n' });
check('  and gives back the new file', applied(before, after), after);
check('the same text replaces nothing', changedSpan('select 1', 'select 1'), { from: 8, to: 8, text: '' });
check('a line appended is inserted at the end', changedSpan('a\nb\n', 'a\nb\nc\n'), { from: 4, to: 4, text: 'c\n' });
check('a line removed is cut alone', changedSpan('a\nb\nc\n', 'a\nc\n'), { from: 2, to: 4, text: '' });
check('emptied, the whole text goes', changedSpan('abc', ''), { from: 0, to: 3, text: '' });
check('from empty, the whole text comes', changedSpan('', 'abc'), { from: 0, to: 0, text: 'abc' });

print('\n--- a repeat never counts twice ---');
// The head and the tail would both count the shared letters of "aa" and
// "aaa": the tail stops where the head ended, or the span runs backwards.
check('one letter more', changedSpan('aa', 'aaa'), { from: 2, to: 2, text: 'a' });
check('one letter less', changedSpan('aaa', 'aa'), { from: 2, to: 3, text: '' });
check('a line repeated', applied('x\nx\n', 'x\nx\nx\n'), 'x\nx\nx\n');

print('\n--- any pair gives back the new text ---');
var seed = 7;
function rand(n) { seed = (seed * 1103515245 + 12345) % 2147483648; return seed % n; }
function word() {
  var out = '', n = rand(12);
  for (var i = 0; i < n; i++) out += 'ab\n'.charAt(rand(3));
  return out;
}
var bad = 0;
for (var i = 0; i < 2000; i++) {
  var cur = word(), next = word(), s = changedSpan(cur, next);
  if (applied(cur, next) !== next || s.from > s.to) bad++;
}
check('2000 random pairs', bad, 0);
