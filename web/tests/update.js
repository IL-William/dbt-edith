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
check('the button names the release', installed.button, 'Update to 1.12.0');
ok('right under it: the terminal, Enter, and the restart',
  installed.step === 'It types the update into the terminal below. Press Enter there to run it, a minute or two, '
    + 'then stop dbt-edith where you started it and start it again: until then, 1.11.0 serves this page.');
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
ok('says why the running .exe steps aside', says(renamed, 'Windows will not replace a running .exe but lets it be renamed: dbt-edith.exe steps aside as dbt-edith.exe.old first'));
ok('that it goes on serving the page', says(renamed, 'goes on serving this page'));
ok('and that it comes back on a failure', says(renamed, 'comes back if the build fails'));

print('\n--- installed, not yet restarted (0095) ---');
var waiting = updateOffer({
  current: '1.11.0', latest: '1.12.0', newer: true, notes: notes, replaced: true,
  how: { command: 'git -C "C:/src/dbt-edith" fetch --tags --force origin; git -C "C:/src/dbt-edith" pull --ff-only origin main && ...', kind: 'install',
    source: 'C:/src/dbt-edith', exe: 'dbt-edith.exe', renames: true },
});
check('the pill says to restart, naming no version, since which went in is not known', waiting.label, 'restart to update');
check('nor does the menu\'s heading, whose newest release may have come out since', waiting.head,
  'dbt-edith 1.11.0 · a new one is installed');
ok('the menu says it is installed', waiting.installed);
check('and offers the command no more', waiting.command, '');
check('the step says how to restart', waiting.step,
  'A new dbt-edith is installed in this one\'s place. To run it, stop dbt-edith where you started it and start it '
    + 'again: until then, 1.11.0 serves this page.');
check('nothing left to say of the command', waiting.lines, []);
check('the notes stay', waiting.notes, notes);
check('a restart on the newest one offers nothing', updateOffer({ current: '1.12.0', latest: '1.12.0', replaced: true }), null);

print('\n--- no command, and why ---');
var carried = updateOffer({ current: '1.11.0', latest: '1.12.0', newer: true, notes: notes, why: 'elsewhere' });
check('still offered', carried.label, '1.12.0 available');
check('with no command', carried.command, '');
check('nor a step to follow', carried.step, '');
ok('carried from another machine: says where to rebuild it', says(carried, 'pull and rebuild it where it was built'));
var locked = updateOffer({ current: '1.11.0', latest: '1.12.0', newer: true, why: 'locked' });
ok('built in place on Windows: says to stop it first', says(locked, 'stop it, then pull and build it again'));
var shell = updateOffer({ current: '1.11.0', latest: '1.12.0', newer: true, why: 'shell' });
ok('another shell: says which ones it is typed for', says(shell, 'typed for sh, bash, zsh and PowerShell'));
var path = updateOffer({ current: '1.11.0', latest: '1.12.0', newer: true, why: 'path' });
ok('an odd path: says so, not that it was built elsewhere', says(path, 'a character a shell would read') && !says(path, 'where it was built'));

print('\n--- when a look was made, in the machine\'s own time ---');
// Built from local fields, so the answer is the same in every time zone.
function at(y, m, d, h, min) { return new Date(y, m - 1, d, h, min).getTime() / 1000; }
var now = at(2026, 10, 14, 8, 59);
check('today', whenLooked(at(2026, 10, 14, 7, 5), now), 'today at 07:05');
check('yesterday', whenLooked(at(2026, 10, 13, 21, 40), now), 'yesterday at 21:40');
check('before that, the date', whenLooked(at(2026, 10, 12, 9, 30), now), '12 Oct at 09:30');
check('across the new year', whenLooked(at(2026, 12, 31, 23, 59), at(2027, 1, 1, 0, 1)), 'yesterday at 23:59');
check('a look due later today', whenLooked(at(2026, 10, 14, 9, 0), now), 'today at 09:00');
check('a look due tomorrow', whenLooked(at(2026, 10, 15, 0, 0), at(2026, 10, 14, 9, 30)), 'tomorrow at 00:00');

print('\n--- what the menu says of the looks ---');
check('no answer from the server yet', updateLook(null, now), '');
check('switched off', updateLook({ current: '1.17.0', off: true }, now),
  'Started with --no-update-check, so no release is looked for.');
check('on its way', updateLook({ current: '1.17.0', checking: true, checked: at(2026, 10, 14, 0, 30) }, now),
  'Looking for a newer release on GitHub…');
var nine = at(2026, 10, 14, 9, 0);
check('up to date', updateLook({ current: '1.17.0', latest: '1.17.0', checked: now, next: nine }, now),
  '1.17.0 is the newest release, as GitHub listed it today at 08:59. The next look is today at 09:00.');
check('a newer one', updateLook({ current: '1.13.2', latest: '1.17.0', newer: true, checked: now, next: nine }, now),
  'GitHub listed 1.17.0 today at 08:59. The next look is today at 09:00.');
check('past 09:00 in Paris, the next is at midnight there, said in the machine\'s time',
  updateLook({ current: '1.17.0', latest: '1.17.0', checked: at(2026, 10, 14, 9, 0), next: at(2026, 10, 15, 0, 0) }, at(2026, 10, 14, 9, 30)),
  '1.17.0 is the newest release, as GitHub listed it today at 09:00. The next look is tomorrow at 00:00.');
check('a build ahead of every release', updateLook({ current: '1.18.0', latest: '1.17.0', checked: now, next: nine }, now),
  'This build is ahead of 1.17.0, the newest release GitHub listed today at 08:59. The next look is today at 09:00.');
check('no answer, ever', updateLook({ current: '1.17.0', failed: now, error: 'Could not resolve host: github.com', next: at(2026, 10, 14, 9, 59) }, now),
  'The look today at 08:59 got no answer from GitHub: Could not resolve host: github.com. The next look is today at 09:59.');
check('no answer, after an older one that still stands',
  updateLook({ current: '1.13.2', latest: '1.13.2', checked: at(2026, 10, 13, 9, 30), failed: now, error: 'x', next: at(2026, 10, 14, 9, 59) }, now),
  'The look today at 08:59 got no answer from GitHub: x. 1.13.2 is the newest release, as GitHub listed it yesterday at 09:30. '
  + 'The next look is today at 09:59.');
check('not looked for yet, and due', updateLook({ current: '1.17.0', next: now }, now),
  'Not looked for yet. The next look is due now.');

print('\n--- the releases since the running one ---');
var tag = 'https://github.com/IL-William/dbt-edith/releases/tag/v';
check('no list while up to date', releaseRows({ current: '1.17.0', latest: '1.17.0' }), null);
check('no list without titles: the single link stays', releaseRows({ current: '1.13.2', latest: '1.17.0', newer: true, notes: tag + '1.17.0' }), null);
var three = releaseRows({ current: '1.14.1', latest: '1.16.0', newer: true, releases: [
  { version: '1.16.0', title: 'Give a tour of the page', notes: tag + '1.16.0' },
  { version: '1.15.1', title: 'Give each dock tab a glyph', notes: tag + '1.15.1' },
  { version: '1.15.0', title: 'Sort a node\'s jobs by how they start', notes: tag + '1.15.0' },
] });
check('counts them', three.head, '3 releases since yours');
check('newest first, each to its notes', three.rows.map(function (r) { return r.version + ' ' + r.href; }),
  ['1.16.0 ' + tag + '1.16.0', '1.15.1 ' + tag + '1.15.1', '1.15.0 ' + tag + '1.15.0']);
check('the whole title in the tooltip', three.rows[2].tip, 'dbt-edith 1.15.0: Sort a node\'s jobs by how they start');
check('nothing older', three.older, null);
var one = releaseRows({ current: '1.16.1', latest: '1.17.0', newer: true, releases: [{ version: '1.17.0', title: 'Show the strategy', notes: tag + '1.17.0' }] });
check('one release, said so', one.head, '1 release since yours');
var capped = releaseRows({ current: '1.0.0', latest: '1.25.0', newer: true, older: 5,
  older_notes: 'https://github.com/IL-William/dbt-edith/releases',
  releases: [{ version: '1.25.0', title: 'A', notes: tag + '1.25.0' }] });
check('counts the ones left out', capped.head, '6 releases since yours');
check('and links every release for them', capped.older, { label: 'and 5 older', href: 'https://github.com/IL-William/dbt-edith/releases', tip: 'Every release, on GitHub' });
