// Overview ruler geometry: a mark must never escape the bar.
// Run from the repository root: jsc web/tests/diff.js
var src = read('web/app.js');
eval(src.slice(src.indexOf('function rulerMark'), src.indexOf('function decorateDiff')));

function check(label, ok) { print((ok ? 'PASS  ' : 'FAIL  ') + label); }
function inBar(m) { return m.top >= 0 && m.height > 0 && m.top + m.height <= 100.0001; }

var cases = [
  ['addition in the middle',       { editFrom: 50, editTo: 60, origFrom: 50, origTo: 50 }, 100, 'add'],
  ['pure deletion',                { editFrom: 20, editTo: 20, origFrom: 20, origTo: 30 }, 100, 'del'],
  ['chunk at the very end',        { editFrom: 99, editTo: 100, origFrom: 99, origTo: 99 }, 100, 'add'],
  ['chunk longer than the file',   { editFrom: 0, editTo: 500, origFrom: 0, origTo: 0 }, 100, 'add'],
  ['single-line file',             { editFrom: 0, editTo: 1, origFrom: 0, origTo: 0 }, 1, 'add'],
];
cases.forEach(function (c) {
  var m = rulerMark(c[1], c[2]);
  check(c[0] + ' stays inside the bar', inBar(m));
  check(c[0] + ' is of kind ' + c[3], m.kind === c[3]);
});

check('a zero total does not divide by zero', inBar(rulerMark({ editFrom: 0, editTo: 0 }, 0)));
check('a deletion stays visible', rulerMark({ editFrom: 5, editTo: 5 }, 1000).height >= 0.5);

print('\n--- regression: the diff panes must be able to scroll ---');
// height:100% on the editor does nothing if its parent pane has no definite
// height: CodeMirror then sizes itself to its content and nothing scrolls.
// These three rules form the chain.
var css = read('web/app.css');
[['.mergepane .CodeMirror-merge ', 'the merge container'],
 ['.mergepane .CodeMirror-merge-pane', 'the panes'],
 ['.mergepane .CodeMirror-merge .CodeMirror ', 'the editor']].forEach(function (r) {
  var i = css.indexOf(r[0]);
  var rule = i < 0 ? '' : css.slice(i, css.indexOf('}', i));
  check(r[1] + ' has a definite height', /height:\s*100%/.test(rule));
});
check('the container does not overflow because of its border',
      /\.mergepane \.CodeMirror-merge \{[^}]*box-sizing:\s*border-box/.test(css));

print('\n--- the inline layout: both sides in one list of lines (0165) ---');
load('web/vendor/diff_match_patch.js');
eval(src.slice(src.indexOf('function diffStateText'), src.indexOf('/* The merge addon paints')));

function same(label, got, want) {
  var g = JSON.stringify(got), w = JSON.stringify(want);
  print((g === w ? 'PASS  ' : 'FAIL  ') + label + (g === w ? '' : '\n        expected ' + w + '\n        got      ' + g));
}
function brief(lines) { return lines.map(function (l) { return l.kind[0] + ':' + l.text + '@' + l.a + '/' + l.b; }); }

same('identical sides are all unchanged lines, numbered alike',
  brief(inlineDiffLines('a\nb\n', 'a\nb\n')), ['s:a@1/1', 's:b@2/2']);
same('an added line has a working tree number and none in HEAD',
  brief(inlineDiffLines('a\nc\n', 'a\nb\nc\n')), ['s:a@1/1', 'a:b@null/2', 's:c@2/3']);
same('a removed line has a number in HEAD and none in the working tree',
  brief(inlineDiffLines('a\nb\nc\n', 'a\nc\n')), ['s:a@1/1', 'd:b@2/null', 's:c@3/2']);
same('a changed line in the middle is the old one then the new one',
  brief(inlineDiffLines('a\nb\nc\n', 'a\nB\nc\n')), ['s:a@1/1', 'd:b@2/null', 'a:B@null/2', 's:c@3/3']);
same('a new file is all additions',
  brief(inlineDiffLines('', 'x\ny\n')), ['a:x@null/1', 'a:y@null/2']);
same('a deleted file is all removals',
  brief(inlineDiffLines('x\ny\n', '')), ['d:x@1/null', 'd:y@2/null']);
same('two empty sides hold no line', inlineDiffLines('', ''), []);
same('a blank line is a line of its own',
  brief(inlineDiffLines('a\n\nb\n', 'a\nb\n')), ['s:a@1/1', 'd:@2/null', 's:b@3/2']);
same('an added run that begins with the line after it slides down, as git draws it (the empty line comes after the text)',
  brief(inlineDiffLines('# T\n\nDoes it.\n\n', '# T\n\nDoes it.\n\nThe orders model.\n\n')),
  ['s:# T@1/1', 's:@2/2', 's:Does it.@3/3', 's:@4/4', 'a:The orders model.@null/5', 'a:@null/6']);
same('an added run that dmp places before the equal line it begins with slides past it',
  brief(inlineDiffLines('b\n\n', 'b\na\na\nb\nb\n\n')),
  ['s:b@1/1', 'a:a@null/2', 'a:a@null/3', 'a:b@null/4', 'a:b@null/5', 's:@2/6']);
same('and a removed one, the other way round',
  brief(inlineDiffLines('b\na\na\nb\nb\n\n', 'b\n\n')),
  ['s:b@1/1', 'd:a@2/null', 'd:a@3/null', 'd:b@4/null', 'd:b@5/null', 's:@6/2']);
same('a removed run slides down the same way',
  brief(inlineDiffLines('# T\n\nDoes it.\n\nThe orders model.\n\n', '# T\n\nDoes it.\n\n')),
  ['s:# T@1/1', 's:@2/2', 's:Does it.@3/3', 's:@4/4', 'd:The orders model.@5/null', 'd:@6/null']);
same('a run that cannot slide stays where it is',
  brief(inlineDiffLines('a\nc\n', 'a\nb\nc\n')), ['s:a@1/1', 'a:b@null/2', 's:c@2/3']);
same('a run at the end has nothing to slide over',
  brief(inlineDiffLines('a\n', 'a\nb\nb\n')), ['s:a@1/1', 'a:b@null/2', 'a:b@null/3']);
same('a run slides over several equal lines, the numbers following it',
  brief(inlineDiffLines('x\nx\n', 'x\nx\nx\nx\n')), ['s:x@1/1', 's:x@2/2', 'a:x@null/3', 'a:x@null/4']);
same('under autocrlf, one real change lists one removed and one added line, as git does',
  brief(inlineDiffLines('a\nb\nc\n', 'a\r\nB\r\nc\r\n')), ['s:a@1/1', 'd:b@2/null', 'a:B@null/2', 's:c@3/3']);
same('LF to CRLF only lists no changed line',
  brief(inlineDiffLines('a\nb\nc\n', 'a\r\nb\r\nc\r\n')), ['s:a@1/1', 's:b@2/2', 's:c@3/3']);
same('a CRLF file without a final newline, against LF with one',
  brief(inlineDiffLines('a\nb\n', 'a\r\nb')), ['s:a@1/1', 's:b@2/2']);
same('the head says why a file changed only in its line endings is listed',
  diffStateText({ before: 'a\nb\n', after: 'a\r\nb\r\n' }),
  'your working tree against HEAD · line endings differ (CRLF here, LF in HEAD)');
same('and the other way round, when HEAD is the CRLF one',
  diffStateText({ before: 'a\r\nb\r\n', after: 'a\nb\n' }),
  'your working tree against HEAD · line endings differ (LF here, CRLF in HEAD)');
same('no note when both end alike, or when a side has no line break or is missing',
  [diffStateText({ before: 'a\n', after: 'b\n' }), diffStateText({ before: 'a', after: 'a\r\n' }),
   diffStateText({ before_missing: true, before: '', after: 'a\r\n' })],
  ['your working tree against HEAD', 'your working tree against HEAD', 'a new file, nothing in HEAD']);
same('a final newline starts no line: a file without one reads the same',
  brief(inlineDiffLines('a\nb', 'a\nb\n')), ['s:a@1/1', 's:b@2/2']);
same('a last line added to a file without a final newline',
  brief(inlineDiffLines('a\nb', 'a\nb\nc')), ['s:a@1/1', 's:b@2/2', 'a:c@null/3']);

function plain(n) { var out = []; for (var i = 0; i < n; i++) out.push({ kind: 'same', a: i + 1, b: i + 1 }); return out; }
var long = plain(40); long[20].kind = 'add'; long[20].a = null;
same('the 12 lines around a change are kept, and what is left of 8 is not worth a fold',
  inlineFolds(long, 12), []);
same('a file with no change at all folds whole when longer than 12', inlineFolds(plain(30), 12), [{ from: 0, to: 30 }]);
same('a short file folds nothing', inlineFolds(plain(12), 12), []);
var far = plain(60); far[5].kind = 'del'; far[50].kind = 'add';
same('two changes far apart leave a fold between them', inlineFolds(far, 12), [{ from: 18, to: 38 }]);

same('the runs of changed lines, for the ruler', inlineRuns([
  { kind: 'same' }, { kind: 'del' }, { kind: 'del' }, { kind: 'add' }, { kind: 'same' }, { kind: 'add' }]),
  [{ kind: 'del', from: 1, to: 3 }, { kind: 'add', from: 3, to: 4 }, { kind: 'add', from: 5, to: 6 }]);

var mixed = inlineDiffLines('a\nb\nc\n', 'a\nB\nc\n');
same('a working tree line finds its place among the inline lines, with the lines removed before it', inlineIndexFor(mixed, 1), 1);
same('a line with nothing removed before it is its own place', inlineIndexFor(mixed, 2), 3);
same('a line past the end lands on the last one the working tree has', inlineIndexFor(mixed, 99), 3);
same('a line a removal leaves out lands on the next', inlineIndexFor(inlineDiffLines('a\nb\nc\n', 'a\nc\n'), 1), 1);
same('a deleted file has no working tree line: the top', inlineIndexFor(inlineDiffLines('x\ny\n', ''), 4), 0);
same('no lines at all: the top', inlineIndexFor([], 0), 0);
same('an inline line finds its working tree line', workingLineFor(mixed, 2), 1);
same('a removed line takes the next working tree line', workingLineFor(mixed, 1), 1);
same('past the end it takes the last', workingLineFor(mixed, 99), 2);
same('no lines, line 0', workingLineFor([], 0), 0);

same('the head says what is compared', diffStateText({}), 'your working tree against HEAD');
same('a new file', diffStateText({ before_missing: true }), 'a new file, nothing in HEAD');
same('a deleted file', diffStateText({ after_missing: true }), 'deleted from your working tree');
same('and when the server cut it', diffStateText({ truncated: true }), 'your working tree against HEAD · truncated');

print('\n--- the layouts as the stylesheet and the page set them ---');
check('the side by side view wraps its lines', /MergeView\(pane, \{[^}]*lineWrapping:\s*true/.test(src), true);
check('and so does the inline one', /function mountInline[\s\S]*?CodeMirror\(pane, \{[^}]*lineWrapping:\s*true/.test(src), true);
check('the number gutters are tinted with the rows', /chunkClassLocation:\s*\['background', 'gutter'\]/.test(src), true);
check('the two panes meet at a one pixel rule',
  /\.CodeMirror-merge-2pane \.CodeMirror-merge-gap \{[^}]*width:\s*1px/.test(css), true);
check('an added row is the green tint, a removed row the red one',
  /CodeMirror-merge-editor \.CodeMirror-merge-l-chunk, \.diff-line-add \{[^}]*var\(--ok\) 11%/.test(css)
  && /CodeMirror-merge-left \.CodeMirror-merge-l-chunk, \.diff-line-del \{[^}]*var\(--bad\) 11%/.test(css), true);
check('a folded stretch says how many lines it holds', /collapsed-widget:before \{[^}]*attr\(data-n\)/.test(css), true);
check('a Markdown heading is bold text, not blue',
  /\.md-doc \.cm-header \{[^}]*color:\s*var\(--fg\)[^}]*font-weight:\s*700/.test(css), true);
check('and its inline code is the string colour, though CodeMirror calls it a comment',
  /\.md-doc\.cm-s-default \.cm-comment, \.md-doc \.cm-s-default \.cm-comment \{[^}]*var\(--syn-str\)/.test(css), true);
