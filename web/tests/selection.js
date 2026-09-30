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
// A named selector's answer is dbt's whatever the tests box says: a model drawn
// for a test's sake is no part of it, and a test kept off the canvas still is.
check('context is counted apart from the selection',
  selectStatus({ matched: 3, nodes: [{ name: 't1' }, { name: 't2' }, { name: 't3' }, { name: 'm', context: true }],
    edges: new Array(3), truncated: false }),
  '3 nodes · 3 edges · 1 for context');
check('a canvas of context alone leads with it, never with 0 nodes',
  selectStatus({ matched: 3, nodes: [{ context: true }, { context: true }], edges: new Array(1), truncated: false, hidden_tests: 3 }),
  '2 for context · 1 edge · 3 tests hidden');
check('tests the box keeps off the canvas are named',
  selectStatus({ matched: 7, nodes: new Array(2), edges: new Array(1), truncated: false, hidden_tests: 5 }),
  '2 nodes · 1 edge · 5 tests hidden');

print('\n--- a named selector ---');
check('its command carries nothing else', selectorCommand('nightly'), 'dbt ls --selector "nightly" --output name');
check('a quote cannot escape into it', selectorCommand('a"b'), 'dbt ls --selector "ab" --output name');
check('the box shows a selector as typed', selectionText({ selector: 'nightly', select: '' }), '--selector nightly');
check('and an expression with its exclusion', selectionText({ select: 'a+', exclude: 'tag:x' }), 'a+ --exclude tag:x');
check('an expression alone', selectionText({ select: 'a+' }), 'a+');
check('a refused selector is still found in the box', boxSelector('--selector ci'), 'ci');
check('pasted, with an equals sign', boxSelector('dbt ls --selector=ci --output name'), 'ci');
check('quoted', boxSelector('dbt ls --selector "ci"'), 'ci');
check('an expression names none', boxSelector('stg_orders+ --select x'), '');
var menu = [{ name: 'daily', description: 'Runs at night' }, { name: 'nightly', description: 'Every mart' },
  { name: 'marts', description: 'The nightly marts' }];
check('an empty filter keeps the whole list', filterSelectors(menu, '').map(function (s) { return s.name; }),
  ['daily', 'nightly', 'marts']);
check('a name that matches comes before a description that does',
  filterSelectors(menu, 'NIGHT').map(function (s) { return s.name; }), ['nightly', 'daily', 'marts']);
check('nothing matches', filterSelectors(menu, 'zzz'), []);
check('a manifest that kept indirect_selection has nothing to say', selectNotes({ lost_indirect: false }), []);
var lost = selectNotes({ lost_indirect: true });
check('one that lost it says so, once', lost.length, 1);
check('the menu says it about the manifest, not about one selector',
  selectorsNote({ lost_indirect: true })[0].indexOf('says so when drawn') > 0, true);
check('and says nothing when nothing was lost', selectorsNote({ selectors: [] }), []);
check('first, so the cap on warnings never drops it',
  selectWarnings(lost.concat(['a', 'b', 'c', 'd', 'e']))[0], lost[0]);
check('a selector of tests alone, with the tests box off, says why the canvas is empty',
  selectEmptyText({ matched: 311, hidden_tests: 311 }), 'This selector keeps only tests (311): open the tests eye to see them.');
check('the tests eye warns before it draws them', testsTitle(false).indexOf('hundreds') > 0, true);
check('and says how to hide them once they are drawn', /^Data tests shown.*Click to hide them\.$/.test(testsTitle(true)), true);
check('one that matched nothing says that', selectEmptyText({ matched: 0 }), 'Nothing matched.');
check('and some tests hidden among models is not the same thing',
  selectEmptyText({ matched: 5, hidden_tests: 2 }), 'Nothing matched.');

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
