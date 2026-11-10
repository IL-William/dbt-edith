// What a search result says, and where the match falls inside a line.
// Run from the repository root: jsc web/tests/grep.js
var src = read('web/app.js');
// A bare DOM for grepHitText: an element keeps its class, its children and its text.
function fakeEl(tag, className, text) {
  var el = { tag: tag, className: className, text: text || '', kids: [] };
  el.classList = { add: function (c) { el.className += (el.className ? ' ' : '') + c; } };
  el.appendChild = function (k) { el.kids.push(k); };
  return el;
}
function jel(tag, className, text) { return fakeEl(tag, className, text); }
function markedText(host, parts) { host.kids.push(parts.join('|')); }
eval(src.slice(src.indexOf('function splitMatch'), src.indexOf('let grepTimer')));

function check(label, got, want) {
  var g = JSON.stringify(got), w = JSON.stringify(want);
  print((g === w ? 'PASS  ' : 'FAIL  ') + label + (g === w ? '' : '\n        expected ' + w + '\n        got      ' + g));
}

print('--- splitMatch ---');
check('splits around the match', splitMatch('select customerid from t', 'customerid'),
  ['select ', 'customerid', ' from t']);
check('the match keeps the line\'s own case', splitMatch('select CustomerId from t', 'customerid'),
  ['select ', 'CustomerId', ' from t']);
check('a query in another case still matches', splitMatch('select customerid', 'CUSTOMERID'),
  ['select ', 'customerid', '']);
check('at the very start', splitMatch('customerid, x', 'customerid'), ['', 'customerid', ', x']);
check('no match leaves the text whole', splitMatch('select 1', 'nope'), ['select 1', '', '']);
check('an empty query leaves the text whole', splitMatch('select 1', ''), ['select 1', '', '']);

print('\n--- grepSummary ---');
check('nothing typed says nothing', grepSummary(null, ''), '');
check('too short asks for more', grepSummary(null, 'cu'), 'three letters or more');
check('waiting says so', grepSummary(null, 'customer'), 'searching…');
check('no match names the query', grepSummary({ files: [], total: 0 }, 'customer'), 'no match for \u201ccustomer\u201d');
check('one match is singular',
  grepSummary({ files: [{ path: 'a.sql', hits: [{}] }], total: 1 }, 'customer'),
  '1 match in 1 file');
check('several are plural',
  grepSummary({ files: [{ path: 'a.sql' }, { path: 'b.sql' }], total: 29 }, 'customer'),
  '29 matches in 2 files');
check('a capped search says it is partial',
  grepSummary({ files: [{ path: 'a.sql' }], total: 5, capped: true }, 'customer'),
  '5 matches in 1 file · showing the first found');
check('skipped files are reported, not hidden',
  grepSummary({ files: [{ path: 'a.sql' }], total: 5, skipped: 17 }, 'customer'),
  '5 matches in 1 file · 17 files skipped');

print('\n--- grepLine ---');
check('leading whitespace is dropped', grepLine('        select amount', 'amount'), ['select ', 'amount', '']);
check('a match near the start is untouched', grepLine('  sum(amount) as total', 'amount'), ['sum(', 'amount', ') as total']);
check('a match ending at column 24 is untouched', grepLine('select * from stg_amount_x', 'amount'), ['select * from stg_', 'amount', '_x']);
check('a match far in is cut at a word edge, with an ellipsis',
  grepLine('select customer_id, sum(case when x then amount else 0 end)', 'amount'),
  ['\u2026then ', 'amount', ' else 0 end)']);
check('the cut falls after the first of the edge characters found going back',
  grepLine('    customer_payments.total_amount as customer_lifetime_value', 'amount'),
  ['\u2026total_', 'amount', ' as customer_lifetime_value']);
check('with no edge in reach, the cut is 8 characters before the match',
  grepLine('abcdefghijklmnopqrstuvwxyz0123456789amount tail', 'amount'),
  ['\u202623456789', 'amount', ' tail']);
check('a cut never falls between the halves of an emoji',
  grepLine('\u{1F600}'.repeat(10) + 'x' + 'amount', 'amount'), ['\u2026' + '\u{1F600}'.repeat(4) + 'x', 'amount', '']);
check('the match keeps its own case', grepLine('select customer_id, sum(case when x then AMOUNT end)', 'amount'),
  ['\u2026then ', 'AMOUNT', ' end)']);
check('no match leaves the text whole, trimmed', grepLine('   select 1', 'nope'), ['select 1', '', '']);
check('a long query near the start keeps the whole line',
  grepLine('ab' + 'x'.repeat(30) + ' rest', 'x'.repeat(30)), ['ab', 'x'.repeat(30), ' rest']);
check('a line the server already cut at the front never gets two ellipses',
  grepLine('\u2026select customer_id, sum(case when x then amount else 0 end)', 'amount'),
  ['\u2026then ', 'amount', ' else 0 end)']);
check('a server-cut line whose match is near its start keeps its one ellipsis',
  grepLine('\u2026sum(amount) as total', 'amount'), ['\u2026sum(', 'amount', ') as total']);

print('\n--- grepHitText ---');
function drawn(parts) {
  var host = fakeEl('span', 'tx');
  grepHitText(host, parts);
  return { cls: host.className, kids: host.kids.map(function (k) {
    return typeof k === 'string' ? k
      : k.tag + '.' + k.className + ':' + (k.kids.length ? k.kids.map(function (j) { return j.className + ':' + j.text; }).join(',') : k.text);
  }) };
}
check('a match is drawn as before, match and after, each in its own box',
  drawn(['select ', 'amount', ' from t']),
  { cls: 'tx split', kids: ['span.tx-a:tx-in:select ', 'mark.:amount', 'span.tx-b: from t'] });
check('a tab or a run of spaces is one space, so the rtl box cannot reorder the words around it',
  drawn(['select\tcol,\tsum(', 'amount', ' as   x\ty']),
  { cls: 'tx split', kids: ['span.tx-a:tx-in:select col, sum(', 'mark.:amount', 'span.tx-b: as x y'] });
check('nothing before the match leaves no empty box',
  drawn(['', 'amount', ' from t']), { cls: 'tx split', kids: ['mark.:amount', 'span.tx-b: from t'] });
check('nothing after the match leaves no empty box',
  drawn(['sum(', 'amount', '']), { cls: 'tx split', kids: ['span.tx-a:tx-in:sum(', 'mark.:amount'] });
check('a row with no match keeps its one text and its ellipsis at the end',
  drawn(['select 1', '', '']), { cls: 'tx', kids: ['select 1||'] });

print('\n--- the row\'s styles ---');
var css = read('web/app.css');
function rule(selector) {
  var at = css.indexOf('\n' + selector + ' {');
  return at < 0 ? '' : css.slice(at, css.indexOf('}', at));
}
check('the box before the match is cut at its left', /direction: rtl/.test(rule('.ghit .tx-a')) && /text-overflow: ellipsis/.test(rule('.ghit .tx-a')), true);
check('its text is isolated left-to-right so punctuation is not reordered',
  /direction: ltr/.test(rule('.ghit .tx-in')) && /unicode-bidi: isolate/.test(rule('.ghit .tx-in')), true);
check('the left-to-right mark is generated content, never in the text',
  /content: '\\200e'/.test(rule('.ghit .tx-in::after')), true);
check('the match never shrinks', /flex: 0 0 auto/.test(rule('.ghit .tx.split mark')), true);
check('the text after the match gives way first, at its end',
  /flex: 1 1 0/.test(rule('.ghit .tx-b')) && /text-overflow: ellipsis/.test(rule('.ghit .tx-b')), true);
