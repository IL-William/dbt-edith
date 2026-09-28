// Find inside an editor, and the Catalog's column filter.
// Run from the repository root: jsc web/tests/find.js
var src = read('web/app.js');
eval(src.slice(src.indexOf('function escapeRegExp'), src.indexOf('const findPrefs')));
eval(src.slice(src.indexOf('function columnMatches'), src.indexOf('function focusColumnFilter')));

function check(label, got, want) {
  var g = JSON.stringify(got), w = JSON.stringify(want);
  print((g === w ? 'PASS  ' : 'FAIL  ') + label + (g === w ? '' : '\n        expected ' + w + '\n        got      ' + g));
}

var plain = { caseSensitive: false, wholeWord: false, regex: false };
function opts(o) { var r = {}; for (var k in plain) r[k] = plain[k]; for (var k2 in o) r[k2] = o[k2]; return r; }
// Every match of a query over some lines, as [line, from, to] triples.
function find(lines, query, o, cap) {
  var p = findPattern(query, opts(o || {}));
  return findMatches(lines, p.re, !!(o && o.wholeWord), cap || 10000).hits.map(function (h) { return [h.line, h.from, h.to]; });
}

print('--- findPattern ---');
check('nothing typed is no pattern', findPattern('', plain), { re: null, error: '' });
check('a plain query means its own dots', find(['a.b axb'], 'a.b'), [[0, 0, 3]]);
check('  and its own brackets', find(['sum(x) [y]'], 'sum(x) [y]'), [[0, 0, 10]]);
check('any case by default', find(['Select SELECT select'], 'select'), [[0, 0, 6], [0, 7, 13], [0, 14, 20]]);
check('match case finds only that case', find(['Select SELECT select'], 'select', { caseSensitive: true }), [[0, 14, 20]]);
check('a regular expression is one', find(['id_1 id_22 idx'], 'id_\\d+', { regex: true }), [[0, 0, 4], [0, 5, 10]]);
check('a broken one says why and matches nothing',
  [findPattern('[a', opts({ regex: true })).re, findPattern('[a', opts({ regex: true })).error.length > 0], [null, true]);
check('  while the same text, plain, is found', find(['x[a'], '[a'), [[0, 1, 3]]);

print('\n--- whole words ---');
check('letters, digits and _ make words', [isWordChar('a'), isWordChar('Z'), isWordChar('7'), isWordChar('_')], [true, true, true, true]);
check('an accented letter is one too', [isWordChar('é'), isWordChar('ü')], [true, true]);
check('punctuation and space are not', [isWordChar('.'), isWordChar(' '), isWordChar(''), isWordChar(undefined)], [false, false, false, false]);
check('id is not a whole word in order_id', find(['order_id'], 'id', { wholeWord: true }), []);
check('  but is in o.id and alone', find(['o.id, id'], 'id', { wholeWord: true }), [[0, 2, 4], [0, 6, 8]]);
check('.id is a whole word in o.id, as in VS Code', find(['o.id'], '.id', { wholeWord: true }), [[0, 1, 4]]);
check('donn is no word inside données', find(['données donn'], 'donn', { wholeWord: true }), [[0, 8, 12]]);
check('a rejected match does not hide the next one', find(['id_id id'], 'id', { wholeWord: true }), [[0, 6, 8]]);
check('whole words in a regular expression', find(['cust_id customer cust'], 'cust\\w*', { regex: true, wholeWord: true }),
  [[0, 0, 7], [0, 8, 16], [0, 17, 21]]);

print('\n--- empty matches ---');
check('^ alone finds nothing, and ends', find(['abc', 'def'], '^', { regex: true }), []);
check('x* skips the empty matches before the real one', find(['aaxx'], 'x*', { regex: true }), [[0, 2, 4]]);
check('an optional group matching nothing is skipped', find(['ab'], '(z)?', { regex: true }), []);

print('\n--- findMatches ---');
check('line by line, in order', find(['select a', 'from t', 'select b'], 'select'), [[0, 0, 6], [2, 0, 6]]);
check('^ is the start of each line', find(['select a', '  select b'], '^select', { regex: true }), [[0, 0, 6]]);
check('$ is the end of each line', find(['a,', 'b', 'c,'], ',$', { regex: true }), [[0, 1, 2], [2, 1, 2]]);
check('matches do not overlap', find(['aaaa'], 'aa'), [[0, 0, 2], [0, 2, 4]]);
check('no pattern, no match', findMatches(['a'], null, false, 10), { hits: [], capped: false });
var capped = findMatches(['aaaa'], findPattern('a', plain).re, false, 3);
check('past the cap it stops and says so', [capped.hits.length, capped.capped], [3, true]);
var exact = findMatches(['aaa'], findPattern('a', plain).re, false, 3);
check('exactly the cap is not past it', [exact.hits.length, exact.capped], [3, false]);
check('an empty document has nothing', find([''], 'a'), []);

print('\n--- stepping ---');
var doc = ['ab ab', 'x', 'ab'];
var ab = findPattern('ab', plain).re;
var hits = findMatches(doc, ab, false, 100).hits;
function pos(line, ch) { return { line: line, ch: ch }; }
function step(lines, re, p, dir, whole) {
  var m = nextMatch(lines, re, !!whole, p, dir);
  return m && [m.line, m.from, m.to];
}
check('forward from the top lands on the first', step(doc, ab, pos(0, 0), 1), [0, 0, 2]);
check('forward from a match start stays on it', step(doc, ab, pos(0, 3), 1), [0, 3, 5]);
check('forward from the end of a match takes the next', step(doc, ab, pos(0, 5), 1), [2, 0, 2]);
check('forward past the last wraps to the first', step(doc, ab, pos(2, 2), 1), [0, 0, 2]);
check('backward takes the one before', step(doc, ab, pos(2, 0), -1), [0, 3, 5]);
check('backward within a line', step(doc, ab, pos(0, 3), -1), [0, 0, 2]);
check('backward from the first wraps to the last', step(doc, ab, pos(0, 0), -1), [2, 0, 2]);
check('one match wraps onto itself', step(['x ab x'], ab, pos(0, 4), 1), [0, 2, 4]);
check('  both ways', step(['x ab x'], ab, pos(0, 2), -1), [0, 2, 4]);
check('nothing to land on', [step(['x'], ab, pos(0, 0), 1), step(['x'], ab, pos(0, 0), -1)], [null, null]);
check('no pattern, no step', step(doc, null, pos(0, 0), 1), null);
check('a cursor past a shortened document still steps', step(['ab'], ab, pos(9, 0), 1), [0, 0, 2]);
var aa = findPattern('aa', plain).re;
check('a step lands where the count does, not mid-match', step(['aaa aa'], aa, pos(0, 1), 1), [0, 4, 6]);
check('whole words when stepping', step(['order_id o.id'], findPattern('id', plain).re, pos(0, 0), 1, true), [0, 11, 13]);
// Past the cap the counted list stops, the text does not.
var many = [];
for (var i = 0; i < 30; i++) many.push('a a');
var capped = findMatches(many, findPattern('a', plain).re, false, 10);
check('the count stops at the cap', [capped.hits.length, capped.capped], [10, true]);
check('  but a step past it finds the next match', step(many, findPattern('a', plain).re, pos(20, 1), 1), [20, 2, 3]);
check('  and a step back from there', step(many, findPattern('a', plain).re, pos(20, 2), -1), [20, 0, 1]);

print('\n--- where the selection is ---');
check('a selection that is a match is that match', hitAt(hits, pos(0, 3), pos(0, 5)), 1);
check('  one that only starts there is not', hitAt(hits, pos(0, 3), pos(0, 4)), -1);
check('  nor a bare cursor', hitAt(hits, pos(2, 0), pos(2, 0)), -1);
check('  nor anything past the last', hitAt(hits, pos(2, 2), pos(2, 4)), -1);
check('a match is a match', isMatch(doc, ab, false, pos(0, 3), pos(0, 5)), true);
check('  however far past the cap', isMatch(many, findPattern('a', plain).re, false, pos(25, 2), pos(25, 3)), true);
check('  but not half of one', isMatch(doc, ab, false, pos(0, 3), pos(0, 4)), false);
check('  nor a mid-match the count never had', isMatch(['aaa'], aa, false, pos(0, 1), pos(0, 3)), false);
check('  nor across lines', isMatch(doc, ab, false, pos(0, 3), pos(2, 2)), false);
check('  nor without a pattern', isMatch(doc, null, false, pos(0, 0), pos(0, 2)), false);

print('\n--- findLabel ---');
var three = { hits: [{}, {}, {}], capped: false };
check('nothing typed says nothing', findLabel('', three, -1, ''), '');
check('a broken pattern says so', findLabel('[', { hits: [], capped: false }, -1, 'Unterminated character class'), 'invalid pattern');
check('no match', findLabel('zz', { hits: [], capped: false }, -1, ''), 'No results');
check('one match, not on it', findLabel('a', { hits: [{}], capped: false }, -1, ''), '1 result');
check('several, not on one', findLabel('a', three, -1, ''), '3 results');
check('on the first of three', findLabel('a', three, 0, ''), '1 of 3');
check('capped, not on one', findLabel('a', { hits: [{}, {}], capped: true }, -1, ''), '2+ results');
check('capped, on a counted one', findLabel('a', { hits: [{}, {}], capped: true }, 1, '', true), '2 of 2+');
check('on one past the cap, which has no number', findLabel('a', { hits: [{}, {}], capped: true }, -1, '', true), '? of 2+');

print('\n--- the highlight agrees with the count ---');
// CodeMirror's StringStream, as much of it as an overlay touches.
function painted(line, query, o) {
  var p = findPattern(query, opts(o || {}));
  var mode = findOverlay(p.re, !!(o && o.wholeWord));
  var stream = { string: line, pos: 0, skipToEnd: function () { this.pos = this.string.length; } };
  var out = [], guard = 0;
  while (stream.pos < line.length && guard++ < 1000) {
    var from = stream.pos;
    var style = mode.token(stream);
    if (stream.pos <= from) return 'stuck at ' + from;
    if (style) out.push([0, from, stream.pos]);
  }
  return out;
}
[
  ['select a.id, b.id from order_id', 'id', {}],
  ['select a.id, b.id from order_id', 'id', { wholeWord: true }],
  ['id_id id', 'id', { wholeWord: true }],
  ['aaxx', 'x*', { regex: true }],
  ['abc', '^', { regex: true }],
  ['Select SELECT select', 'select', { caseSensitive: true }],
  ['données donn', 'donn', { wholeWord: true }],
].forEach(function (c) {
  check('painted as counted: ' + JSON.stringify(c[1]) + ' in ' + JSON.stringify(c[0]) + ' ' + JSON.stringify(c[2]),
    painted(c[0], c[1], c[2]), find([c[0]], c[1], c[2]));
});

print('\n--- columnMatches ---');
var col = { name: 'CUSTOMER_HK', description: 'The order placed' };
check('any part of the name', columnMatches(col, 'tomer'), true);
check('in any case', columnMatches(col, 'customer_hk'), true);
check('nothing typed keeps every column', columnMatches(col, ''), true);
check('the description is not searched', columnMatches(col, 'order'), false);
check('no match', columnMatches(col, 'amount'), false);
