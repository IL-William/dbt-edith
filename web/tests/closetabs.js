// Which tabs each close command takes, and which it leaves.
// Run from the repository root: jsc web/tests/closetabs.js
var src = read('web/app.js');
eval(src.slice(src.indexOf('function closeTargets'), src.indexOf('function renderTabs')));

function check(label, got, want) {
  var g = JSON.stringify(got), w = JSON.stringify(want);
  print((g === w ? 'PASS  ' : 'FAIL  ') + label + (g === w ? '' : '\n        expected ' + w + '\n        got      ' + g));
}

// Five tabs left to right, the third one right-clicked, two of them edited.
var order = ['a.sql', 'b.sql', 'c.sql', 'd.sql', 'e.sql'];
var open = new Map();
order.forEach(function (p) { open.set(p, { dirty: p === 'b.sql' || p === 'd.sql' }); });

print('--- closeTargets ---');
check('close takes the one clicked', closeTargets('close', 'c.sql', order, open), ['c.sql']);
check('others leaves it alone', closeTargets('others', 'c.sql', order, open),
  ['a.sql', 'b.sql', 'd.sql', 'e.sql']);
check('to the right takes what follows', closeTargets('right', 'c.sql', order, open), ['d.sql', 'e.sql']);
check('  nothing follows the last', closeTargets('right', 'e.sql', order, open), []);
check('  everything follows the first', closeTargets('right', 'a.sql', order, open),
  ['b.sql', 'c.sql', 'd.sql', 'e.sql']);
check('saved leaves the edited ones', closeTargets('saved', 'c.sql', order, open),
  ['a.sql', 'c.sql', 'e.sql']);
check('  the clicked tab is not spared when it is saved',
  closeTargets('saved', 'a.sql', order, open).indexOf('a.sql') >= 0, true);

print('\n--- one tab in the bar ---');
var one = ['only.sql'], oneOpen = new Map([['only.sql', { dirty: false }]]);
check('others takes none', closeTargets('others', 'only.sql', one, oneOpen), []);
check('to the right takes none', closeTargets('right', 'only.sql', one, oneOpen), []);
check('close takes it', closeTargets('close', 'only.sql', one, oneOpen), ['only.sql']);

print('\n--- a key that is gone ---');
// A tab closed while its menu was open must not resurrect itself as a target.
check('close finds nothing', closeTargets('close', 'gone.sql', order, open), []);
check('others takes nothing', closeTargets('others', 'gone.sql', order, open), []);
check('to the right takes nothing, never the whole bar',
  closeTargets('right', 'gone.sql', order, open), []);
