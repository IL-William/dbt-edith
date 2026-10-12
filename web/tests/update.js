// The status bar's offer of a newer dbt-edith (0068): when it shows, what it
// names, and what its menu says about the command the server composed. The
// fixtures mirror the /api/update shape with invented paths.
// Run from the repository root: jsc web/tests/update.js
var src = read('web/app.js');
eval(src.slice(src.indexOf('function updateOffer'), src.indexOf('function openUpdateMenu')));

function check(label, got, want) {
  var g = JSON.stringify(got), w = JSON.stringify(want);
  print((g === w ? 'PASS  ' : 'FAIL  ') + label + (g === w ? '' : '\n        expected ' + w + '\n        got      ' + g));
}
function ok(label, cond) { check(label, !!cond, true); }
function says(o, words) { return o.lines.join('\n').indexOf(words) >= 0; }

var notes = 'https://github.com/IL-William/dbt-edith/releases/tag/v1.12.0';

print('--- nothing to offer ---');
check('no answer yet', updateOffer(null), null);
check('the look is on its way', updateOffer({ current: '1.11.0', checking: true }), null);
check('the newest is the one running', updateOffer({ current: '1.11.0', latest: '1.11.0' }), null);
check('switched off', updateOffer({ current: '1.11.0', off: true }), null);

print('\n--- installed by cargo, from a clone on this machine ---');
var installed = updateOffer({
  current: '1.11.0', latest: '1.12.0', newer: true, notes: notes,
  how: { command: 'git -C "/src/dbt-edith" pull --ff-only origin main && cargo install --locked --path "/src/dbt-edith"',
    kind: 'install', source: '/src/dbt-edith', exe: 'dbt-edith' },
});
check('the pill names the release', installed.label, '1.12.0 available');
check('the menu says both versions', installed.head, 'dbt-edith 1.12.0 · this is 1.11.0');
check('the command is the server\'s, untouched', installed.command,
  'git -C "/src/dbt-edith" pull --ff-only origin main && cargo install --locked --path "/src/dbt-edith"');
check('the notes link is the release page', installed.notes, notes);
ok('names the branch and the clone it pulls', says(installed, 'It pulls main into /src/dbt-edith and installs it again with cargo install'));
ok('says the page is still the old one until a restart', says(installed, 'Then stop dbt-edith and start it again: until then, 1.11.0 serves this page.'));
ok('says nothing of Windows elsewhere', !says(installed, 'Windows'));

print('\n--- built in place ---');
var built = updateOffer({
  current: '1.11.0', latest: '1.12.0', newer: true, notes: notes,
  how: { command: 'git -C "/src/dbt-edith" pull --ff-only origin main && ...', kind: 'build', source: '/src/dbt-edith',
    exe: 'dbt-edith' },
});
ok('says it builds rather than installs', says(built, 'builds it again with cargo build --release'));

print('\n--- installed by cargo, on Windows ---');
var renamed = updateOffer({
  current: '1.11.0', latest: '1.12.0', newer: true, notes: notes,
  how: { command: 'git -C "C:/src/dbt-edith" pull --ff-only origin main && ...', kind: 'install',
    source: 'C:/src/dbt-edith', exe: 'dbt-edith.exe', renames: true },
});
ok('says why the running .exe steps aside', says(renamed, 'dbt-edith.exe steps aside as dbt-edith.exe.old first, since Windows will not replace a running .exe'));
ok('and that it comes back on a failure', says(renamed, 'comes back if the build fails'));

print('\n--- no command, and why ---');
var carried = updateOffer({ current: '1.11.0', latest: '1.12.0', newer: true, notes: notes, why: 'elsewhere' });
check('still offered', carried.label, '1.12.0 available');
check('with no command', carried.command, '');
ok('carried from another machine: says where to rebuild it', says(carried, 'pull and rebuild it where it was built'));
var locked = updateOffer({ current: '1.11.0', latest: '1.12.0', newer: true, why: 'locked' });
ok('built in place on Windows: says to stop it first', says(locked, 'stop it, then pull and build it again'));
var shell = updateOffer({ current: '1.11.0', latest: '1.12.0', newer: true, why: 'shell' });
ok('another shell: says which ones it is typed for', says(shell, 'typed for sh, bash, zsh and PowerShell'));
var path = updateOffer({ current: '1.11.0', latest: '1.12.0', newer: true, why: 'path' });
ok('an odd path: says so, not that it was built elsewhere', says(path, 'a character a shell would read') && !says(path, 'where it was built'));
