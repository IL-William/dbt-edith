// What a resolved selector says on screen: the counts, the capped status line,
// the warnings, and the dbt command that checks all of it against dbt itself.
// Run from the repository root: jsc web/tests/selection.js
var src = read('web/app.js');
eval(src.slice(src.indexOf('function selectKindCounts'), src.indexOf('async function loadSidecar')));

function check(label, got, want) {
  var g = JSON.stringify(got), w = JSON.stringify(want);
  print((g === w ? 'PASS  ' : 'FAIL  ') + label + (g === w ? '' : '\n        expected ' + w + '\n        got      ' + g));
}

print('--- what the selection is made of ---');
check('biggest kind first', selectKindCounts({ source: 3, model: 39, test: 12 }),
  [['model', 39], ['test', 12], ['source', 3]]);
check('a tie is broken by name', selectKindCounts({ seed: 2, model: 2 }), [['model', 2], ['seed', 2]]);
check('nothing at all', selectKindCounts(undefined), []);

print('\n--- the summary beside the box ---');
check('nothing matched says so', selectSummary({ matched: 0, counts: {} }), 'nothing matched');
check('one of a kind is singular', selectSummary({ matched: 1, counts: { model: 1 } }), '1 model');
check('several kinds, biggest first',
  selectSummary({ matched: 54, counts: { model: 39, source: 3, test: 12 } }),
  '39 models  ·  12 tests  ·  3 sources');

print('\n--- the status line under the canvas ---');
check('nothing matched draws nothing', selectStatus({ matched: 0, counts: {}, nodes: [], edges: [] }), '');
check('what was drawn, and how it hangs together',
  selectStatus({ matched: 42, nodes: new Array(42), edges: new Array(57), truncated: false }),
  '42 nodes · 57 edges');
// The number that matters when a selection is larger than the canvas: a capped
// picture has to say it is capped, or it reads as the whole answer.
check('a capped canvas says what it left out',
  selectStatus({ matched: 2422, nodes: new Array(400), edges: new Array(512), truncated: true }),
  '400 of 2422 drawn · 512 edges');
check('one node and one edge are singular',
  selectStatus({ matched: 1, nodes: new Array(1), edges: new Array(1), truncated: false }),
  '1 node · 1 edge');
check('one node and no edge', selectStatus({ matched: 1, nodes: new Array(1), edges: [], truncated: false }),
  '1 node · 0 edges');

print('\n--- warnings ---');
check('nothing to say', selectWarnings([]), []);
check('nothing to say, and nothing sent', selectWarnings(undefined), []);
check('the same warning twice is one warning',
  selectWarnings(['nothing matches `a`', 'nothing matches `a`']), ['nothing matches `a`']);
check('four is the whole list', selectWarnings(['a', 'b', 'c', 'd']), ['a', 'b', 'c', 'd']);
check('more than four is counted instead',
  selectWarnings(['a', 'b', 'c', 'd', 'e', 'f']), ['a', 'b', 'c', 'd', 'and 2 more']);

print('\n--- the dbt command that checks this build ---');
check('the plain form', lsCommand('dim_customers+', ''),
  'dbt ls --select "dim_customers+" --output name');
check('an exclude half comes along', lsCommand('dim_customers+', 'tag:deprecated'),
  'dbt ls --select "dim_customers+" --exclude "tag:deprecated" --output name');
// The server drops quotes while parsing, so one can never come back in a
// selector; dropping them here too means the command is always runnable.
check('a quote cannot escape into the command', lsCommand('a"b', ''),
  'dbt ls --select "ab" --output name');

print('\n--- a +N badge writes itself into the expression ---');
check('upstream is a leading plus', expandTerm('dim_customers', 'stg_customers', 'up'),
  'dim_customers +stg_customers');
check('downstream is a trailing plus', expandTerm('dim_customers', 'fct_orders', 'down'),
  'dim_customers fct_orders+');
check('the same term is not added twice', expandTerm('a +b', 'b', 'up'), 'a +b');
check('spacing is tidied on the way', expandTerm('  a   b  ', 'c', 'up'), 'a b +c');
check('an empty box takes the first term', expandTerm('', 'a', 'down'), 'a+');

print('\n--- walking the history ---');
var hist = ['newest', 'older', 'oldest'];
check('no history goes nowhere', historyStep([], -1, 1), -1);
check('up from the empty line is the newest', historyStep(hist, -1, 1), 0);
check('up again is the one before', historyStep(hist, 0, 1), 1);
check('up stops at the oldest', historyStep(hist, 2, 1), 2);
check('down comes back', historyStep(hist, 1, -1), 0);
check('down past the newest is the empty line again', historyStep(hist, 0, -1), -1);
check('and stays there', historyStep(hist, -1, -1), -1);
