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
// The corner, the note under the box and its button count the same drawing
// the same way: a hidden test is said apart, and no click draws it.
check('a capped selector with hidden tests counts what could be drawn',
  selectStatus({ matched: 6000, hidden_tests: 2000, nodes: new Array(3000), edges: new Array(10), truncated: true }),
  '3000 of 4000 drawn · 10 edges · 2000 tests hidden');
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
// Refused rather than cleaned: a name with the character dropped would be
// another name than the one resolved.
check('a quote is not typed', selectorCommand('a"b'), '');
check('nor a control character', selectorCommand('night\u001bly'), '');
check('nor a C1 control', selectorCommand('night\u0085ly'), '');
check('nor what the shell acts on in double quotes', selectorCommand('$(id)'), '');
check('nor a backtick, a backslash or a bang', [selectorCommand('`id`'), selectorCommand('a\\b'), selectorCommand('a!b')], ['', '', '']);
check('nor a typographic double quote, which PowerShell reads as one',
  [selectorCommand('a\u201d;calc;\u201cb'), lsCommand('a\u201eb', ''), lsCommand('a', 'b\u201d')], ['', '', '']);
check('a name in any script is typed as it is', selectorCommand('nuit_été'), 'dbt ls --selector "nuit_été" --output name');
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
// The corner's `3000 of 4210 drawn` went unread: a capped canvas looked like a
// lineage missing half its nodes.
var names = function (n) { return new Array(n).fill('x'); };
check('a capped canvas says so under the box',
  selectCapNote({ truncated: true, matched: 4210, nodes: new Array(3000), names: names(4210) }),
  ['Too large to draw whole: 3000 of 4210 on the canvas, those nearest what the selection names. '
    + 'Copy names lists every one; a narrower selection draws the rest.']);
// The names sent stop at 5000, and a selection can match more than that.
check('and does not say Copy names lists every one when the names were cut',
  /Copy names lists the first 5000 by name;/.test(selectCapNote({ truncated: true, matched: 22500, nodes: new Array(3000), names: names(5000) })[0]),
  true);
check('tests the eye keeps off are no part of what is left to draw',
  selectCapNote({ truncated: true, matched: 6000, hidden_tests: 2000, nodes: new Array(3000), names: names(5000) })[0]
    .indexOf('3000 of 4000 on') > 0, true);
check('counting what was selected, not what was drawn for a test\'s sake',
  selectCapNote({ truncated: true, matched: 50, nodes: [{}, {}, { context: true }] })[0].indexOf('2 of 50') > 0, true);
check('and a whole one says nothing', selectCapNote({ truncated: false, matched: 42, nodes: new Array(42) }), []);
check('first, and kept with the lost indirect_selection by the cap on warnings',
  selectWarnings(selectCapNote({ truncated: true, matched: 9, nodes: new Array(3) })
    .concat(selectNotes({ lost_indirect: true }), ['a', 'b', 'c', 'd'])).slice(0, 2).map(function (l) { return l.slice(0, 10); }),
  ['Too large ', 'This manif']);
// Past the cap zoom and pan slow down: drawing the rest is a click, said to
// cost something, and never what the next selection does.
var capped = { truncated: true, matched: 4210, nodes: new Array(3000), most: 20000 };
check('a capped canvas offers to draw the rest', capOffer(capped, 0).label, 'Draw all 4210');
check('as many as the server draws at most', capOffer(capped, 0).max, 20000);
check('and says what it costs', capOffer(capped, 0).hint, 'slower to zoom and pan');
check('and that it is this time only', /this time only/.test(capOffer(capped, 0).title), true);
check('a whole canvas offers nothing', capOffer({ truncated: false, matched: 42, nodes: new Array(42), most: 20000 }, 0), null);
check('past the most there is, it offers that much',
  capOffer({ truncated: true, matched: 25000, nodes: new Array(3000), most: 20000 }, 0).label, 'Draw 20000');
check('and nothing more once drawn at it',
  capOffer({ truncated: true, matched: 25000, nodes: new Array(20000), most: 20000 }, 20000), null);
// A server that lowers its ceiling is taken at its word.
check('the ceiling is the server\'s',
  capOffer({ truncated: true, matched: 16000, nodes: new Array(3000), most: 15000 }, 0).label, 'Draw 15000');
check('and none sent, no offer', capOffer({ truncated: true, matched: 4210, nodes: new Array(3000) }, 0), null);
// A named selector's hidden tests are in `matched`, and no click draws them.
check('all is what can be drawn',
  capOffer({ truncated: true, matched: 6000, hidden_tests: 2000, nodes: new Array(3000), most: 20000 }, 0).label, 'Draw all 4000');
check('and what fits is offered whole',
  capOffer({ truncated: true, matched: 22000, hidden_tests: 3000, nodes: new Array(3000), most: 20000 }, 0).label, 'Draw all 19000');
check('the offer says whether it draws it all', [capOffer(capped, 0).all,
  capOffer({ truncated: true, matched: 25000, nodes: new Array(3000), most: 20000 }, 0).all], [true, false]);
// `source:*+` on a large project: the starts alone take most of the cap, and
// the canvas is sources with few of what they feed. The note says so, and
// what draws the rest, before Copy names, which a narrow bar cuts first.
var filled = { truncated: true, matched: 4210, nodes: new Array(3000), names: names(4210), most: 20000, starts: 2650 };
check('starts that fill the cap are said, with the button that draws the rest',
  selectCapNote(filled, capOffer(filled, 0)),
  ['Too large to draw whole: 3000 of 4210 on the canvas, 2650 of them what the selection names. '
    + 'Draw all or a narrower selection draws the rest of their lineage; Copy names lists every one.']);
check('from half the canvas',
  /1500 of them what the selection names/.test(selectCapNote(Object.assign({}, filled, { starts: 1500 }), capOffer(filled, 0))[0]), true);
check('below it, the note is the one before',
  selectCapNote(Object.assign({}, filled, { starts: 1499 }), capOffer(filled, 0)),
  selectCapNote({ truncated: true, matched: 4210, nodes: new Array(3000), names: names(4210) }));
check('and with no starts sent, too',
  selectCapNote(Object.assign({}, filled, { starts: undefined }), capOffer(filled, 0)),
  selectCapNote({ truncated: true, matched: 4210, nodes: new Array(3000), names: names(4210) }));
var huge = { truncated: true, matched: 25000, nodes: new Array(3000), names: names(5000), most: 20000, starts: 2900 };
check('a button that cannot draw it all draws more of it',
  /^Draw 20000 or a narrower selection draws more of their lineage; Copy names lists the first 5000 by name\.$/
    .test(selectCapNote(huge, capOffer(huge, 0))[0].split('names. ')[1]), true);
var atMost = { truncated: true, matched: 25000, nodes: new Array(20000), names: names(5000), most: 20000, starts: 12000 };
check('and once drawn at the most there is, only a narrower selection is left',
  /names\. A narrower selection draws more of their lineage;/.test(selectCapNote(atMost, capOffer(atMost, 20000))[0]), true);
check('a whole canvas says nothing, starts or not', selectCapNote({ truncated: false, matched: 42, nodes: new Array(42), starts: 40 }, null), []);
// A raise holds for the line and the tests box it was asked for, nothing else.
var raised = { expr: 'source:stripe+', tests: 0, max: 20000 };
check('the same line keeps it', raiseFor(raised, 'source:stripe+', 0), 20000);
check('another line is cut again', raiseFor(raised, 'source:stripe+1', 0), 0);
check('so is the tests eye', raiseFor(raised, 'source:stripe+', 1), 0);
check('and an emptied box', raiseFor(raised, '', 0), 0);
check('nothing asked, nothing lifted', raiseFor(null, 'source:stripe+', 0), 0);
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
check('and counts them while it is open', [testsLabel(0), testsLabel(311)], ['tests', 'tests · 311']);
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
// The server refuses a quote in a selector, and so does this: a command
// with the character dropped would check another selection than the one run.
check('a quote is not typed', lsCommand('a"b', ''), '');
check('nor a control character, on either half', [lsCommand('a\rb', ''), lsCommand('a', 'c\u001b[2Jd')], ['', '']);
check('nor what the shell acts on in double quotes', [lsCommand('$(id)', ''), lsCommand('a', '`b`')], ['', '']);

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
