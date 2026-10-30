// The warning Edith's menu gives when the terminal's shell reads the current
// folder, and the freshness badge typing instead of running while it does
// (0130). Pure functions, sliced from web/app.js up to sendToTerminal.
// Run from the repository root: jsc web/tests/pathwarn.js
var src = read('web/app.js');
eval(src.slice(src.indexOf('function pathWarning'), src.indexOf('function sendToTerminal')));

function check(label, got, want) {
  var g = JSON.stringify(got), w = JSON.stringify(want);
  print((g === w ? 'PASS  ' : 'FAIL  ') + label + (g === w ? '' : '\n        expected ' + w + '\n        got      ' + g));
}
function ok(label, cond) { check(label, !!cond, true); }

check('a shell that does not says nothing', pathWarning(''), []);
check('an unheard answer says nothing', pathWarning(undefined), []);
check('an unknown reason says nothing', pathWarning('other'), []);
// A line as the menu draws it: text, the entry in code marks for the test.
function flat(line) { return line.map(function (p) { return typeof p === 'string' ? p : '[' + p.code + ']'; }).join(''); }
function lines(said) { return said.map(flat); }
var relRaw = pathWarning('relative', 'bin'), emptyRaw = pathWarning('empty'), cmdRaw = pathWarning('cmd');
var rel = lines(relRaw), empty = lines(emptyRaw), cmd = lines(cmdRaw);
ok('a relative entry is named as written, in a part of its own', rel[0].indexOf('a relative entry ([bin]), which') >= 0);
ok('so no markup character reaches the text', relRaw[0].every(function (p) { return typeof p === 'string' ? p.indexOf('`') < 0 : true; }));
ok('read from the current folder, the program in the project, not at its root',
  rel[0].indexOf('reads from the current folder') >= 0 && rel[0].indexOf('a program in the project') >= 0 && rel[0].indexOf('root') < 0);
check('an entry with markup stays one part of text', pathWarning('relative', '<b>%X%\\bin')[0][2].code, '<b>%X%\\bin');
ok('a relative entry unnamed still reads', lines(pathWarning('relative'))[0].indexOf('a relative entry, which') >= 0);
ok('an empty entry is read by the shell, whichever it is', empty[0].indexOf('empty entry, which the shell reads as') >= 0 && empty[0].indexOf('bash') < 0);
ok('cmd.exe is named, not the PATH', cmd[0].indexOf('cmd.exe') >= 0 && cmd[0].indexOf('PATH holds') < 0);
check('cmd.exe keeps its sentence', cmd[0], 'cmd.exe looks in the current folder before the PATH, so a program committed at the project\'s root could run there in place of git, dbt or cargo.');
[rel, empty, cmd].forEach(function (said) {
  ok('each names the three programs: ' + said[0].slice(0, 24), said[0].indexOf('git, dbt or cargo') >= 0);
  ok('and is one sentence: ' + said[0].slice(0, 24), said[0].split('. ').length === 1 && said[0].slice(-1) === '.');
});
check('a PATH reason adds the limit of the check', rel.length + ' ' + empty.length, '2 2');
ok('which says it reads the PATH dbt-edith hands the shell, its startup files may change it',
  rel[1].indexOf('PATH dbt-edith hands the shell') >= 0 && rel[1].indexOf('startup files') >= 0);
check('cmd.exe reads the folder whatever the PATH holds, so no such limit', cmd.length, 1);
ok('no em dash', rel.concat(empty, cmd).join('').indexOf(String.fromCharCode(0x2014)) < 0);

check('the badge runs dbt parse where nothing reads the folder', freshnessRuns(''), true);
check('the badge runs before the answer is heard', freshnessRuns(undefined), true);
['relative', 'empty', 'cmd'].forEach(function (why) {
  check('the badge only types it for ' + why, freshnessRuns(why), false);
});
