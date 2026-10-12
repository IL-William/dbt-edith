// When a press on the lineage canvas becomes a pan, and when a double click is
// only the system's count running on from one.
// Run from the repository root: jsc web/tests/pan.js
var src = read('web/lineage.js');
eval(src.slice(src.indexOf('function pastSlop'), src.indexOf('function init(')));

function check(label, got, want) {
  var g = JSON.stringify(got), w = JSON.stringify(want);
  print((g === w ? 'PASS  ' : 'FAIL  ') + label + (g === w ? '' : '\n        expected ' + w + '\n        got      ' + g));
}

print('--- pastSlop ---');
check('still', pastSlop(0, 0), false);
check('a click that trembles 2 px each way stays a click', pastSlop(2, -2), false);
check('4 px either way is still inside Windows\' drag box', pastSlop(4, -4), false);
check('5 px across is a pan', pastSlop(5, 0), true);
check('5 px up is a pan', pastSlop(0, -5), true);
check('3 px each way stays a click, where |dx| + |dy| > 3 would not', pastSlop(3, 3), false);

print('\n--- panDouble ---');
check('a second pan from the spot the first ended on: no press since the first', panDouble(0), true);
check('one click right after a pan: its press is the only one since', panDouble(1), true);
check('two presses after a pan, however late, are a double click', panDouble(2), false);
check('no pan yet', panDouble(Infinity), false);
