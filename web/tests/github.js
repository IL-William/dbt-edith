// The GitHub menu in the top bar: where its two rows go, and what an issue
// opened from it starts with. The build stamps are invented.
// Run from the repository root: jsc web/tests/github.js
var src = read('web/app.js');
eval(src.slice(src.indexOf('function githubLinks'), src.indexOf('function openGithubMenu')));

function check(label, got, want) {
  var g = JSON.stringify(got), w = JSON.stringify(want);
  print((g === w ? 'PASS  ' : 'FAIL  ') + label + (g === w ? '' : '\n        expected ' + w + '\n        got      ' + g));
}
var repo = 'https://github.com/IL-William/dbt-edith';
function body(l) {
  var at = l.issue.indexOf('?body=');
  return at < 0 ? null : decodeURIComponent(l.issue.slice(at + 6));
}
function lastLine(l) { var b = body(l).split('\n'); return b[b.length - 1]; }

print('--- the star ---');
var l = githubLinks(repo, '1.25.0', 'v1.24.0-2-g1a2b3c4, built 2026-10-14', 'macos');
check('opens the repository, where its Star button is', l.star, repo);

print('\n--- the issue ---');
check('a new issue on that repository', l.issue.indexOf(repo + '/issues/new?body='), 0);
check('the query is one encoded value: no space, newline, & or # left raw', /[\s&#]/.test(l.issue.slice(l.issue.indexOf('?') + 1)), false);
check('starts with a question its writer types under', body(l).split('\n')[0], '<!-- What happened, or what would you like? -->');
check('  with room left to answer it', body(l).split('\n').slice(1, 4), ['', '', '']);
check('ends with the version, the build and the system', lastLine(l), 'dbt-edith 1.25.0 (v1.24.0-2-g1a2b3c4, built 2026-10-14) on macOS');
check('windows written Windows', lastLine(githubLinks(repo, '1.25.0', 'built 2026-10-14', 'windows')), 'dbt-edith 1.25.0 (built 2026-10-14) on Windows');
check('Linux too', lastLine(githubLinks(repo, '1.25.0', 'built 2026-10-14', 'linux')), 'dbt-edith 1.25.0 (built 2026-10-14) on Linux');
check('another system by the name Rust gives it', lastLine(githubLinks(repo, '1.25.0', 'built 2026-10-14', 'freebsd')), 'dbt-edith 1.25.0 (built 2026-10-14) on freebsd');
check('no system named: said so rather than left blank', lastLine(githubLinks(repo, '1.25.0', '', '')), 'dbt-edith 1.25.0 on an unknown system');
check('no build: no empty brackets', lastLine(githubLinks(repo, '1.25.0', '', 'macos')), 'dbt-edith 1.25.0 on macOS');
check('no version: a question mark, not "undefined"', lastLine(githubLinks(repo, undefined, undefined, 'macos')), 'dbt-edith ? on macOS');
check('a system with + & # in it comes back whole', lastLine(githubLinks(repo, '1.25.0', '', 'a+b & #c')), 'dbt-edith 1.25.0 on a+b & #c');

print('\n--- only a build of this repository is named ---');
function stamp(b) { return lastLine(githubLinks(repo, '1.25.0', b, 'macos')); }
check('on a release tag', stamp('v1.25.0, built 2026-10-14'), 'dbt-edith 1.25.0 (v1.25.0, built 2026-10-14) on macOS');
check('past a tag, with changes', stamp('v1.24.0-2-g1a2b3c4-dirty, built 2026-10-14'), 'dbt-edith 1.25.0 (v1.24.0-2-g1a2b3c4-dirty, built 2026-10-14) on macOS');
check('a commit with no tag behind it', stamp('1a2b3c4-dirty, built 2026-10-14'), 'dbt-edith 1.25.0 (1a2b3c4-dirty, built 2026-10-14) on macOS');
check('the date alone, built from a copy with no git', stamp('built 2026-10-14'), 'dbt-edith 1.25.0 (built 2026-10-14) on macOS');
check('a copy built inside another repository keeps only the date', stamp('acme-release-2026.10-14-g1a2b3c4-dirty, built 2026-10-14'), 'dbt-edith 1.25.0 (built 2026-10-14) on macOS');
check('so does a tag of the user\'s own', stamp('my-demo-3-g1a2b3c4, built 2026-10-14'), 'dbt-edith 1.25.0 (built 2026-10-14) on macOS');
check('  even one shaped like a version with words after it', stamp('v1.2.3-acme-1-g1a2b3c4, built 2026-10-14'), 'dbt-edith 1.25.0 (built 2026-10-14) on macOS');
check('nothing recognisable: no brackets at all', stamp('acme'), 'dbt-edith 1.25.0 on macOS');
