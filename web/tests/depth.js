// The depth control's pure parts (0158): a name cut in the middle, the
// selector the copy writes, what a level adds, the rows of a menu, and the
// number a button shows.
// Run from the repository root: jsc web/tests/depth.js
var src = read('web/app.js');
eval(src.slice(src.indexOf('function cutMiddle'), src.indexOf('function askDepth')));
// The holds of an ask: askDepth to depthKind, with a state and a repaint counter.
var S = { depthLevels: null, depthKey: '' };
var painted = 0;
function paintDepth() { painted++; }
var Lineage = { nodeColor: function () { return 'red'; } };
eval(src.slice(src.indexOf('function askDepth'), src.indexOf('function depthKind')));
var css = read('web/app.css');
var html = read('web/index.html');

function check(label, got, want) {
  var g = JSON.stringify(got), w = JSON.stringify(want);
  print((g === w ? 'PASS  ' : 'FAIL  ') + label + (g === w ? '' : '\n        expected ' + w + '\n        got      ' + g));
}
function ok(label, cond) { print((cond ? 'PASS  ' : 'FAIL  ') + label); }

print('--- a name cut in the middle ---');
var exact = 'a'.repeat(24);
check('24 characters are kept whole', cutMiddle(exact), exact);
check('25 are cut', cutMiddle('a'.repeat(25)), 'a'.repeat(12) + '…' + 'a'.repeat(11));
var long = 'customer_orders_by_payment_methods_daily';
check('a 40-character name keeps its start and its end', cutMiddle(long), 'customer_ord…thods_daily');
check('the cut is 24 characters', cutMiddle(long).length, 24);
check('a short name is itself', cutMiddle('customers'), 'customers');

print('\n--- the selector the copy writes ---');
check('both sides', depthSelector('model', 'customers', 2, 1), '2+customers+1');
check('a zero side is dropped', depthSelector('model', 'customers', 0, 3), 'customers+3');
check('the other zero side', depthSelector('model', 'customers', 2, 0), '2+customers');
check('both zero is the name', depthSelector('model', 'customers', 0, 0), 'customers');
check('a source is named by its method', depthSelector('source', 'raw.orders', 1, 0), '1+source:raw.orders');
check('a version is the name dbt reads', depthSelector('model', 'orders.v2', 0, 2), 'orders.v2+2');
check('a seed is a bare name', depthSelector('seed', 'raw_customers', 0, 1), 'raw_customers+1');

print('\n--- what a level adds ---');
check('one kind, plural', levelAdds({ kinds: [['view', 3]] }, false, false), '+ 3 views');
check('one node is named, not counted', levelAdds({ kinds: [['view', 1]], name: 'stg_customers' }, false, false), '+ stg_customers');
check('several kinds, the count of each', levelAdds({ kinds: [['view', 2], ['table', 1]] }, false, false), '+ 2 views, 1 table');
check('the last level, where the graph ends', levelAdds({ kinds: [['seed', 3]] }, true, false), '+ 3 seeds · all');
check('the last listed level of a graph that goes on', levelAdds({ kinds: [['seed', 3]] }, true, true), '+ 3 seeds');
check('a named node on the last level', levelAdds({ kinds: [['source', 1]], name: 'raw.orders' }, true, false), '+ raw.orders · all');
check('a long name is cut', levelAdds({ kinds: [['view', 1]], name: long }, false, false), '+ customer_ord…thods_daily');
check('a level the cap cut says so, and is never all', levelAdds({ kinds: [['column', 400]], cut: true }, true, true), '+ 400 columns or more');
check('even when the walk ended there', levelAdds({ kinds: [['column', 400]], cut: true }, true, false), '+ 400 columns or more');
check('words of the kinds', [['snapshot', 2], ['exposure', 2], ['analysis', 2], ['analysis', 1], ['source', 2], ['model', 1], ['column', 1]]
  .map(function (k) { return levelAdds({ kinds: [k] }, false, false); }),
  ['+ 2 snapshots', '+ 2 exposures', '+ 2 analyses', '+ 1 analysis', '+ 2 sources', '+ 1 model', '+ 1 column']);
check('materializations take model', [['incremental', 2], ['ephemeral', 1], ['materialized_view', 2], ['other', 1], ['other', 2]]
  .map(function (k) { return levelAdds({ kinds: [k] }, false, false); }),
  ['+ 2 incremental models', '+ 1 ephemeral model', '+ 2 materialized views', '+ 1 other node', '+ 2 other nodes']);
check('a custom materialization is a model of that kind', levelAdds({ kinds: [['dynamic_table', 2], ['unknown', 1]] }, false, false),
  '+ 2 dynamic_table models, 1 unknown model');
check('a label that is a property of every object is still a word', levelAdds({ kinds: [['constructor', 2]] }, false, false), '+ 2 constructor models');
check('a level that added nothing', levelAdds({ kinds: [] }, false, false), '+ nothing new');

print('\n--- the rows of a menu ---');
var levels = { up: [{ kinds: [['view', 3]] }, { kinds: [['seed', 3]] }], down: [], up_more: false, down_more: false };
check('the focus alone, then each level, the last "all"', depthRows('up', 'customers', levels, 1), [
  { n: 0, sel: 'customers', full: 'customers', adds: 'alone', cur: false },
  { n: 1, sel: '1+customers', full: '1+customers', adds: '+ 3 views', cur: true },
  { n: 2, sel: '2+customers', full: '2+customers', adds: '+ 3 seeds · all', cur: false },
]);
var down = { up: [], down: [{ kinds: [['table', 2]] }, { kinds: [['view', 1]], name: 'daily' }], up_more: false, down_more: true };
check('downstream puts the number after the name, and a graph that goes on is never all',
  depthRows('down', 'orders', down, 2).map(function (r) { return [r.sel, r.adds, r.cur]; }), [
    ['orders', 'alone', false], ['orders+1', '+ 2 tables', false], ['orders+2', '+ daily', true]]);
check('a depth past the levels checks the last one', depthRows('up', 'customers', levels, 9).map(function (r) { return r.cur; }), [false, false, true]);
check('a depth of zero checks the focus', depthRows('up', 'customers', levels, 0).map(function (r) { return r.cur; }), [true, false, false]);
check('a box that holds text is read as a number', depthRows('up', 'customers', levels, '2').map(function (r) { return r.cur; }), [false, false, true]);
check('a side with no level has no rows', depthRows('down', 'customers', levels, 1), []);
check('nor has an answer without levels', depthRows('up', 'customers', null, 1), []);
var rows = depthRows('up', long, levels, 1);
check('a long name is cut in the row and whole in its full text', [rows[1].sel, rows[1].full], ['1+customer_ord…thods_daily', '1+' + long]);

print('\n--- the number a button shows ---');
check('within the levels, the box', shownDepth('2', [{}, {}, {}]), 2);
check('past them, the last level', shownDepth('3', [{}]), 1);
check('a side with none shows 0 though the box holds 1', shownDepth('1', []), 0);
check('levels not known, the box as it is', shownDepth('3', null), 3);
check('nonsense is 0', shownDepth('', null), 0);

print('\n--- an ask that failed ---');
askDepth('m|a');
failDepth('m|a');
check('a failure after the ask is held as failed, repainted once', [S.depthLevels.key, S.depthLevels.failed, S.depthLevels.levels, painted], ['m|a', true, null, 1]);
landDepth('m|a', { mode: 'model', focus: 0, nodes: [{ kind: 'model' }], levels: { up: [], down: [] } });
failDepth('m|a');
check('a failure after the levels landed keeps them', [!!S.depthLevels.levels, !!S.depthLevels.failed], [true, false]);
askDepth('m|b');
askDepth('m|a');
failDepth('m|a');
askDepth('m|a');
check('the same key asked again drops the failure', S.depthLevels, null);
askDepth('m|b');
failDepth('m|a');
check('a failure under a stale key is ignored', S.depthLevels, null);
painted = 0;
failDepth('m|a');
check('and repaints nothing', painted, 0);
askDepth('');
failDepth('');
check('the column draw failing holds the empty key failed', [S.depthLevels.key, S.depthLevels.failed], ['', true]);

print('\n--- where the failure is said and set ---');
ok('the menu says it on a failed hold', /held && held\.failed \? 'The levels could not be counted\.'/.test(src));
ok('the model\'s catch fails its ask', /failDepth\('m\|' \+ id\);\s+toast\('lineage: '/.test(src));
ok('the column\'s catch fails its ask before the canvas is painted again', /askDepth\(''\);\s+failDepth\(''\);\s+paintMode\(\);/.test(src));

print('\n--- the markup and the style ---');
ok('the inputs stay, hidden, with their range', /<input id="up" type="number" min="0" max="20" value="1" hidden>/.test(html)
  && /<input id="down" type="number" min="0" max="20" value="1" hidden>/.test(html));
ok('both buttons name what they open', /id="up-btn"[^>]*aria-haspopup="menu"[^>]*aria-expanded="false"/.test(html)
  && /id="down-btn"[^>]*aria-haspopup="menu"[^>]*aria-expanded="false"/.test(html));
ok('the kind bar is the .dot of a box', /id="depth-bar" class="dot"/.test(html) && !/fdot|fbar/.test(html + css));
ok('the narrow dock swaps the name for the bar', /#lineage-tools\.noname \.dsel \.fn \{ display: none; \}/.test(css)
  && /#lineage-tools\.noname \.dsel \.dot\.on \{ display: block; \}/.test(css));
ok('the menu rows are the board\'s grid', /grid-template-columns: minmax\(110px, max-content\) minmax\(0, 1fr\) 14px/.test(css));
ok('no colour is written in the depth control\'s rules', !/#[0-9a-fA-F]{3,8}\b/.test(css.slice(css.indexOf('.dsel {'), css.indexOf('/* --------------------------------------------------------------- lineage -- */'))));
