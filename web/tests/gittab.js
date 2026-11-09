// The Git tab's words: how far the branch is from its upstream, what each glyph
// would do, what the commit button says, and the letter a file wears (0165).
// Run from the repository root: jsc web/tests/gittab.js
var src = read('web/app.js');
eval(src.slice(src.indexOf('function aheadBehindWords'), src.indexOf('function paintCommit')));

function check(label, got, want) {
  var g = JSON.stringify(got), w = JSON.stringify(want);
  print((g === w ? 'PASS  ' : 'FAIL  ') + label + (g === w ? '' : '\n        expected ' + w + '\n        got      ' + g));
}
function repo(extra) {
  var info = { repo: true, branch: 'main', upstream: 'origin/main', ahead: 0, behind: 0,
               staged: [], unstaged: [], untracked: [], conflicted: [], merging: false };
  for (var k in extra) info[k] = extra[k];
  return info;
}
function files(n) { var out = []; for (var i = 0; i < n; i++) out.push({ path: 'f' + i + '.sql', code: 'M ' }); return out; }

print('--- the branch row ---');
check('level with its upstream: nothing', aheadBehindWords(repo({})), { parts: [], title: '' });
check('behind, in the warning tone, the title names the upstream',
  aheadBehindWords(repo({ behind: 1 })),
  { parts: [{ text: '1 behind', tone: 'behind' }], title: 'origin/main has 1 commit this branch does not have' });
check('ahead',
  aheadBehindWords(repo({ ahead: 2 })),
  { parts: [{ text: '2 ahead', tone: 'ahead' }], title: 'this branch has 2 commits origin/main does not have' });
check('both, each part with its tone',
  aheadBehindWords(repo({ ahead: 2, behind: 1 })).parts,
  [{ text: '2 ahead', tone: 'ahead' }, { text: '1 behind', tone: 'behind' }]);
check('no upstream, no words', aheadBehindWords(repo({ upstream: '', ahead: 3, behind: 3 })), { parts: [], title: '' });
check('no answer yet, no words', aheadBehindWords(null), { parts: [], title: '' });

print('\n--- what the three glyphs would do ---');
check('level', gitActionTitles(repo({})),
  { fetch: 'Fetch from origin', pull: 'Pull (fast-forward only)', push: 'Nothing to push' });
check('one to pull, two to push', gitActionTitles(repo({ behind: 1, ahead: 2 })),
  { fetch: 'Fetch from origin', pull: 'Pull 1 commit (fast-forward only)', push: 'Push 2 commits' });
check('three to pull, one to push', gitActionTitles(repo({ behind: 3, ahead: 1 })).pull, 'Pull 3 commits (fast-forward only)');
check('one to push', gitActionTitles(repo({ ahead: 1 })).push, 'Push 1 commit');
check('no upstream: the push publishes the branch, and says so',
  gitActionTitles(repo({ upstream: '' })).push, 'Push this branch to origin (sets its upstream)');
check('no repository: nothing to push', gitActionTitles({ repo: false, upstream: '' }).push, 'Nothing to push');

print('\n--- what the push dialog says beside its commits ---');
check('no upstream: the first push of the branch', pushNote(repo({ upstream: '' }), []),
  'No upstream yet, so this is the first push of this branch.');
check('upstream with commits: the list speaks, no note', pushNote(repo({ ahead: 2 }), [{ subject: 'a' }, { subject: 'b' }]), '');
check('upstream level: says so, not that there is no upstream', pushNote(repo({}), []),
  'origin/main already has every commit of main: nothing to push.');
check('only behind: the same, not that it is level', pushNote(repo({ behind: 1 }), []),
  'origin/main already has every commit of main: nothing to push.');

print('\n--- the commit button ---');
check('nothing staged', commitButtonText(repo({})), 'Stage a file to commit');
check('one file', commitButtonText(repo({ staged: files(1) })), 'Commit 1 file');
check('two files', commitButtonText(repo({ staged: files(2) })), 'Commit 2 files');
check('a merge with nothing conflicted finishes the merge', commitButtonText(repo({ merging: true })), 'Commit the merge');
check('a merge with staged files too', commitButtonText(repo({ merging: true, staged: files(3) })), 'Commit the merge');
check('a merge with a file still conflicted names the way out',
  commitButtonText(repo({ merging: true, conflicted: ['a.sql'] })), 'Resolve the conflicts first');
check('no repository, no answer yet', [commitButtonText(null), commitButtonText({ repo: false })], ['Commit', 'Commit']);
check('ready: files staged', commitReady(repo({ staged: files(1) })), true);
check('not ready: nothing staged', commitReady(repo({})), false);
check('ready: a merge with nothing conflicted', commitReady(repo({ merging: true })), true);
check('not ready: conflicts left', commitReady(repo({ merging: true, conflicted: ['a.sql'], staged: files(1) })), false);
check('not ready: no repository', commitReady({ repo: false, staged: [], conflicted: [] }), false);

print('\n--- the letter a file wears ---');
check('the tree\'s classes for modified, added, new', ['M', 'A', 'U'].map(function (l) { return gitLetterClass(l, false); }), ['m', 'a', 'u']);
check('deleted and renamed', ['D', 'R'].map(function (l) { return gitLetterClass(l, false); }), ['d', 'r']);
check('a conflicted file keeps its own class whatever the letter', gitLetterClass('U', true), 'x');
check('a letter nobody coloured is dim', gitLetterClass('T', false), 'q');

print('\n--- when a row given the keyboard scrolls the list ---');
check('a row inside the list does not', outOfView(100, 126, 50, 400), false);
check('one above it does', outOfView(40, 66, 50, 400), true);
check('one cut at the foot does', outOfView(380, 406, 50, 400), true);
check('one flush with both edges does not', outOfView(50, 400, 50, 400), false);

print('\n--- the markup the script reads ---');
var html = read('web/index.html');
check('the row of words sits between the branch and the glyphs',
  html.indexOf('id="branch-btn"') < html.indexOf('id="git-ahead-behind"') && html.indexOf('id="git-ahead-behind"') < html.indexOf('id="git-fetch"'), true);
check('the badges on pull and push are gone', /id="(behind|ahead)-n"/.test(html), false);
check('the keycap and the button share a row', /class="git-crow">[\s\S]*id="commit-btn"[\s\S]*id="commit-kbd"/.test(html), true);
