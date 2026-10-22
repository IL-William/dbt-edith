// The Reports panel's pure parts: which sections it lists and in what order,
// what is new on its button, the section it opens on, and when the section on
// screen is behind its file (0064).
// Run from the repository root: jsc web/tests/reports.js
var app = read('web/app.js');
eval(app.slice(app.indexOf('function reportsSections'), app.indexOf('async function loadReportsIndex')));

function check(label, got, want) {
  var g = JSON.stringify(got), w = JSON.stringify(want);
  print((g === w ? 'PASS  ' : 'FAIL  ') + label + (g === w ? '' : '\n        expected ' + w + '\n        got      ' + g));
}
function keys(sections) { return sections.map(function (s) { return s.key; }); }

print('--- the sections ---');
function empty(sections) { return sections.map(function (s) { return s.key + (s.empty ? ':empty' : ''); }); }
check('the job coverage alone, empty, before the index is read', empty(reportsSections(null)), ['jobs:empty']);
check('and while nothing was written or named, as where a source is added', empty(reportsSections({})), ['jobs:empty']);
check('collin once a run wrote a report', empty(reportsSections({ collin: { at: 5 } })), ['collin', 'jobs:empty']);
check('the jobs once a source is named', empty(reportsSections({ jobs: { sources: 1, at: 3 } })), ['jobs']);
check('in the order they are worked through, whatever the index says',
      keys(reportsSections({ jobs: { sources: 1, at: 3 }, collin: { at: 5 } })), ['collin', 'jobs']);
check('a key this page does not know is left out', keys(reportsSections({ later: { at: 1 } })), ['jobs']);
check('each in its tool\'s colour', reportsSections({ collin: { at: 5 } })[0].tool, 'collin');
check('an index read twice is not marked empty by the first', empty(reportsSections({}).concat(reportsSections({ jobs: { sources: 1, at: 3 } }))),
      ['jobs:empty', 'jobs']);
check('the macros while one of the project\'s is reached by no node and nothing explains it (0105)',
      empty(reportsSections({ macros: { unexplained: 2, at: 3 } })), ['jobs:empty', 'macros']);
check('or once a card asked for a macro\'s callers, empty: a click lights nothing',
      empty(reportsSections({}, 'macro.shop.cents')), ['jobs:empty', 'macros:empty']);
check('and lit by the macros alone when one is unexplained',
      empty(reportsSections({ macros: { unexplained: 1, at: 3 } }, 'macro.shop.cents')), ['jobs:empty', 'macros']);
check('last, after the coverage', keys(reportsSections({ macros: { unexplained: 1, at: 3 }, run: { at: 9 }, jobs: { sources: 1, at: 3 } })),
      ['run', 'jobs', 'macros']);
check('not before either', keys(reportsSections({}, '')), ['jobs']);

print('\n--- the last run\'s dot ---');
function tones(index) { return reportsSections(index).map(function (s) { return s.key + ':' + (s.tone || '-'); }); }
check('ok when nothing errored, failed or warned',
      tones({ run: { at: 9, command: 'build', error: 0, fail: 0, warn: 0, partial: 0, ok: 4, other: 0 } }), ['run:ok', 'jobs:-']);
check('ok from a count of results that passed', tones({ run: { at: 9, error: 0, fail: 0, warn: 0, partial: 0, ok: 3, other: 0 } })[0], 'run:ok');
check('none for a run with no result at all (dbt build -s typo)', tones({ run: { at: 9, error: 0, fail: 0, warn: 0, partial: 0, ok: 0, other: 0 } })[0], 'run:-');
check('warn on a status the module has no word for', tones({ run: { at: 9, error: 0, fail: 0, warn: 0, partial: 0, ok: 5, other: 1 } })[0], 'run:warn');
check('warn on a warning', tones({ run: { at: 9, error: 0, fail: 0, warn: 2 } })[0], 'run:warn');
check('warn on a partial success alone', tones({ run: { at: 9, error: 0, fail: 0, warn: 0, partial: 1 } })[0], 'run:warn');
check('bad on an error', tones({ run: { at: 9, error: 1, fail: 0, warn: 0 } })[0], 'run:bad');
check('bad on a failed test, before a warning', tones({ run: { at: 9, error: 0, fail: 1, warn: 3 } })[0], 'run:bad');
check('none while the file is still being written and the index has its time alone', tones({ run: { at: 9 } })[0], 'run:-');
check('and no other section carries one',
      tones({ run: { at: 9, error: 1, fail: 1, warn: 1 }, warnings: { at: 1, manifest_at: 1, unused: 2 }, collin: { at: 5 },
        jobs: { sources: 1, at: 3 }, macros: { unexplained: 1, at: 3 } }),
      ['run:bad', 'warnings:-', 'collin:-', 'jobs:-', 'macros:-']);
function head(st) { return runEndTone(runEndCounts(st)); }
check('the head reads the run\'s own statuses by the same rule',
      [head({ ok: 4 }), head({ ok: 4, noop: 1, reused: 1 }), head({ ok: 4, partial: 1 }), head({ warn: 1, fail: 1 }), head({ ok: 2, error: 1 })],
      ['ok', 'ok', 'warn', 'bad', 'bad']);
check('and none for no result, or a skipped-only run', [head({}), head(undefined), head({ skipped: 3 })], ['', '', '']);
check('and warn for an unknown status, skipped or not', [head({ ok: 3, interrupted: 1 }), head({ skipped: 1, interrupted: 1 })], ['warn', 'warn']);
check('and ok with skipped beside what passed', head({ ok: 2, skipped: 1 }), 'ok');
check('the words of each tone', [runEndWords('ok'), runEndWords('warn'), runEndWords('bad'), runEndWords('')],
      ['All passed', 'Not everything passed', 'Ended with errors or failed tests', '']);

print('\n--- what is new ---');
check('nothing from collin, nothing new', reportsNews(null), []);
var badge = { text: '3 to investigate', short: '3', title: 'collin\'s last run names 3 models or tables to look at.' };
check('collin\'s badge becomes news on the button', reportsNews(badge),
      [{ key: 'collin', tool: 'collin', text: '3 to investigate', short: '3', title: badge.title }]);

print('\n--- where it opens ---');
var both = reportsSections({ collin: { at: 5 }, jobs: { sources: 1, at: 3 } });
check('on the first section with nothing new and nothing read yet', reportsDefaultSection(both, [], ''), 'collin');
check('on the section read last this session', reportsDefaultSection(both, [], 'jobs'), 'jobs');
check('on news before the last section read', reportsDefaultSection(both, reportsNews(badge), 'jobs'), 'collin');
check('not on a section that has gone', reportsDefaultSection(reportsSections({ jobs: { sources: 1, at: 3 } }), [], 'collin'), 'jobs');
check('news for a section that is not there opens the first', reportsDefaultSection(reportsSections({ jobs: { sources: 1, at: 3 } }), reportsNews(badge), ''), 'jobs');
check('nowhere when there is nothing', reportsDefaultSection([], reportsNews(badge), 'collin'), '');
check('not on the empty coverage read last, when another section has something',
      reportsDefaultSection(reportsSections({ collin: { at: 5 } }), [], 'jobs'), 'collin');
check('on the empty coverage when it is all there is', reportsDefaultSection(reportsSections({}), [], ''), 'jobs');
check('on the macros, not an empty coverage, once they have something (0070)',
      reportsDefaultSection(reportsSections({ macros: { unexplained: 1, at: 3 } }), [], ''), 'macros');

print('\n--- the button\'s tooltip ---');
check('names what the reports hold and the key',
      reportsButtonTitle(both, [], '⌥⌘R'), 'Reports on the whole project: collin\'s report, Job coverage (⌥⌘R)');
check('then what is new, a line each',
      reportsButtonTitle(both, reportsNews(badge), 'Ctrl+Alt+R').split('\n')[1], badge.title);
check('says the key even with nothing to show', reportsButtonTitle([], [], 'Ctrl+Alt+R').split('\n')[0], 'Reports on the whole project (Ctrl+Alt+R)');
// Dimmed rather than hidden, the button has to say what will fill it (0075).
check('  and then what will fill them',
      reportsButtonTitle(reportsSections({}), [], '⌥⌘R').split('\n').slice(1),
      ['Nothing yet: the last dbt run fills one, from target/run_results.json, a config path of dbt_project.yml '
        + 'that applies to nothing another, collin\'s report another, the job coverage once the Jobs tab has a source, '
        + 'which is added here, and a macro of the project no node reaches that nothing explains.']);
check('and leaves out a section with nothing in it yet',
      reportsButtonTitle(reportsSections({ collin: { at: 5 } }), [], '⌥⌘R'), 'Reports on the whole project: collin\'s report (⌥⌘R)');

print('\n--- the section on screen against its file ---');
check('nothing shown, nothing stale', reportsStale(null, { collin: { at: 5 } }, 'collin'), false);
check('the same stamp is not stale', reportsStale({ at: 5 }, { collin: { at: 5 } }, 'collin'), false);
check('a later run is', reportsStale({ at: 5 }, { collin: { at: 9 } }, 'collin'), true);
check('a report gone is', reportsStale({ at: 5 }, {}, 'collin'), true);
check('the jobs after a manifest reload are', reportsStale({ sources: 1, at: 3 }, { jobs: { sources: 1, at: 4 } }, 'jobs'), true);
check('and after a source is added', reportsStale({ sources: 1, at: 3 }, { jobs: { sources: 2, at: 3 } }, 'jobs'), true);
check('another section moving is not this one\'s news',
      reportsStale({ sources: 1, at: 3 }, { collin: { at: 9 }, jobs: { sources: 1, at: 3 } }, 'jobs'), false);

print('\n--- the last run ---');
check('listed first once dbt wrote a run', keys(reportsSections({ collin: { at: 5 }, run: { at: 9 } })), ['run', 'collin', 'jobs']);
var run = { at: 9, command: 'build', error: 1, fail: 2, warn: 3 };
check('its errors and failures are news', runBadge(run, 5),
      { text: '1 error, 2 failed', short: '3', title: 'dbt build ended with 1 error and 2 failed tests.' });
check('once only: a run already seen is not', runBadge(run, 9), null);
check('nor a run that went well', runBadge({ at: 9, command: 'run', error: 0, fail: 0, warn: 4 }, 0), null);
check('nor a file still being written, with no counts', runBadge({ at: 9 }, 0), null);
check('errors alone', runBadge({ at: 9, command: 'run', error: 2, fail: 0 }, 0).title, 'dbt run ended with 2 errors.');
check('a command the file does not name', runBadge({ at: 9, error: 1 }, 0).title, 'The last dbt command ended with 1 error.');
check('one failed test', runBadge({ at: 9, command: 'test', fail: 1 }, 0), { text: '1 failed', short: '1', title: 'dbt test ended with 1 failed test.' });
check('the run\'s news comes before collin\'s, in red',
      reportsNews(badge, runBadge(run, 0)).map(function (n) { return n.key + ':' + (n.tone || n.tool); }), ['run:bad', 'collin:collin']);
check('the panel opens on the run\'s news first',
      reportsDefaultSection(reportsSections({ run: { at: 9 }, collin: { at: 5 } }), reportsNews(badge, runBadge(run, 0)), 'collin'), 'run');

check('outcomes in words', ['ok', 'error', 'fail', 'warn', 'skipped', 'partial', 'noop', 'interrupted'].map(runStatusWord),
      ['ok', 'error', 'failed', 'warned', 'skipped', 'partly done', 'nothing to do', 'interrupted']);
check('times', [0.034, 2.25, 42.4, 125].map(runSecs), ['0.03 s', '2.3 s', '42 s', '2 min 5 s']);
check('a time that is not there says nothing', runSecs(undefined), '');
check('kinds as headings', [runKindPlural('model', true), runKindPlural('unit_test')], ['Models', 'unit tests']);

var report = {
  command: 'build', selection: ['--select orders+'], dbt_version: '1.9.0', elapsed: 12.5,
  kinds: [{ kind: 'model', ran: 3, of: 40, statuses: { ok: 2, error: 1 } }, { kind: 'test', ran: 5, of: 5, statuses: { ok: 4, warn: 1 } }],
  older_than_manifest: false, other_target: 0, unknown: { items: [], of: 0 },
};
check('the head says the command with its selection', runAbout(report), 'dbt build --select orders+ · dbt 1.9.0 · 13 s');
check('...and copes with a file that names none', runAbout({ selection: [] }), 'a dbt command');
check('...nor a time of nothing', runAbout({ command: 'compile', selection: [], elapsed: 0 }), 'dbt compile');
check('a tile per kind, its outcomes worst first, then the time',
      runTiles(report).map(function (t) { return t.label + ' ' + t.value + ' (' + t.sub + ') ' + t.tone; }),
      ['Models 3 of 40 (1 error · 2 ok) bad', 'Tests 5 of 5 (1 warned · 4 ok) warn', 'Took 13 s (for the whole command) ']);
check('a partial run is said in counts, never listed',
      runNotes(report).map(function (n) { return n.text; }),
      ['It ran 3 of 40 models: a node it did not reach is not in it, which says nothing about that node.']);
var stale = JSON.parse(JSON.stringify(report));
stale.older_than_manifest = true; stale.other_target = 2; stale.unknown = { items: ['model.shop.gone'], of: 1 };
check('what to distrust comes first', runNotes(stale).filter(function (n) { return n.tone === 'warn'; }).length, 3);
check('...naming another target', runNotes(stale)[1].text,
      '2 results name tables the manifest names otherwise: this run was likely for another target.');
check('a compile overwrote the run', runNotes({ command: 'compile', kinds: [{ kind: 'model', ran: 3, of: 3, statuses: { ok: 3 } }], unknown: { of: 0 } })[0].text,
      'dbt compile wrote this file over the one before, so what the last run did is no longer in it.');
check('a run of nothing says so', runNotes({ command: 'run', kinds: [], unknown: { of: 0 } }).map(function (n) { return n.text; }), ['It ran no node.']);
check('a test run says what it did not do',
      runNotes({ command: 'test', kinds: [{ kind: 'test', ran: 5, of: 5, statuses: { ok: 5 } }], unknown: { of: 0 } })[0].text,
      'dbt test runs tests alone: no model was built by it.');
check('a failed test says how many rows it found', runEntryText({ status: 'fail', failures: 3, secs: 0.2 }), 'failed · 3 rows · 0.20 s');
check('an error without a time', runEntryText({ status: 'error', secs: 0 }), 'error');

print('\n--- the coverage\'s news, and a short button ---');
check('nothing to compare before the coverage was seen', jobsBadge({ never: 3, unscheduled: 1 }, undefined), null);
check('nor before the jobs were read', jobsBadge({ sources: 1, at: 3 }, 2), null);
check('the same or fewer is no news', [jobsBadge({ never: 3, unscheduled: 1 }, 4), jobsBadge({ never: 1, unscheduled: 1 }, 4)], [null, null]);
var more = jobsBadge({ never: 4, unscheduled: 2 }, 4);
check('more unscheduled since the last look', [more.text, more.short], ['2 more unscheduled', '+2']);
check('...said in full in the tooltip', more.title,
      '2 more models, seeds or snapshots than when you last looked are run by no scheduled job: 4 in no job, 2 only in unscheduled jobs.');
var three = reportsNews(badge, runBadge(run, 0), more);
check('three reports with news, in the sections\' order', three.map(function (n) { return n.key; }), ['run', 'collin', 'jobs']);
check('the coverage\'s in the warning colour', three[2].tone, 'warn');
check('several at once: their counts alone', reportsChipTexts(three), ['3', '3', '+2']);
check('one alone: its words', reportsChipTexts([three[2]]), ['2 more unscheduled']);
check('none: nothing', reportsChipTexts([]), []);

print('\n--- the warnings (0103) ---');
check('listed while a config path applies to nothing, after the run', keys(reportsSections({ collin: { at: 5 }, warnings: { at: 4, manifest_at: 3, unused: 2 }, run: { at: 9 } })),
      ['run', 'warnings', 'collin', 'jobs']);
check('in dbt\'s colour', reportsSections({ warnings: { at: 4, manifest_at: 3, unused: 2 } })[0].tool, 'dbt');
var warned = warningsBadge({ at: 3, manifest_at: 4, unused: 3 }, 0, { state: 'fresh' });
check('news from the first look, since dbt says it only as it parses', [warned.text, warned.short], ['3 unused paths', '3']);
check('...in full in the tooltip', warned.title,
      'dbt_project.yml has 3 config paths that apply to no resource, which dbt warns of as it parses.');
check('one path', warningsBadge({ unused: 1 }, 0).text, '1 unused path');
check('no news once seen', warningsBadge({ unused: 3 }, 3), null);
var rose = warningsBadge({ unused: 5 }, 3);
check('then only what was added', [rose.text, rose.short], ['2 unused paths', '+2']);
check('...said against the last look', rose.title,
      'dbt_project.yml has 5 config paths that apply to no resource, 2 more than when you last looked, which dbt warns of as it parses.');
check('none in the index, none on the button', warningsBadge(undefined, 0), null);
check('none while dbt_project.yml is newer than the manifest', warningsBadge({ at: 5, manifest_at: 4, unused: 3 }, 0, { state: 'fresh' }), null);
check('none while other files are', warningsBadge({ at: 3, manifest_at: 4, unused: 3 }, 0, { state: 'edited' }), null);
check('...the manifest behind, the marks grey too', [warningsStale({ at: 5, manifest_at: 4 }, null), warningsStale({ at: 3, manifest_at: 4 }, { state: 'stale' }),
      warningsStale({ at: 3, manifest_at: 4 }, { state: 'fresh' }), warningsStale(null, { state: 'stale' })], [true, true, false, false]);
check('...and when the freshness answer names a newer manifest than the index was read with, before the index arrives',
      [warningsStale({ at: 3, manifest_at: 4 }, { state: 'fresh', manifest_at: 6 }), warningsStale({ at: 3, manifest_at: 4 }, { state: 'fresh', manifest_at: 4 }),
      warningsStale({ at: 3, manifest_at: 4 }, { state: 'missing' })], [true, false, true]);
check('...so no chip in that gap', warningsBadge({ at: 3, manifest_at: 4, unused: 3 }, 0, { state: 'fresh', manifest_at: 6 }), null);
check('a file that could not be read is listed with no chip', [keys(reportsSections({ warnings: { at: 3, manifest_at: 4, unused: 0, unread: true } })),
      warningsBadge({ at: 3, manifest_at: 4, unused: 0, unread: true }, 0, { state: 'fresh' })], [['warnings', 'jobs'], null]);
var four = reportsNews(badge, runBadge(run, 0), more, warned);
check('the warnings\' news after the run\'s, in the sections\' order', four.map(function (n) { return n.key + ':' + (n.tone || n.tool); }),
      ['run:bad', 'warnings:dbt', 'collin:collin', 'jobs:warn']);
check('the panel opens on them when the run has none', reportsDefaultSection(reportsSections({ warnings: { unused: 3 }, collin: { at: 5 } }),
      reportsNews(badge, null, null, warned), ''), 'warnings');
check('stale once dbt_project.yml is saved again', reportsStale({ at: 4, manifest_at: 3, unused: 3 }, { warnings: { at: 6, manifest_at: 3, unused: 3 } }, 'warnings'), true);
check('or a new manifest is read', reportsStale({ at: 4, manifest_at: 3, unused: 3 }, { warnings: { at: 4, manifest_at: 8, unused: 3 } }, 'warnings'), true);
check('or the last path is gone', reportsStale({ at: 4, manifest_at: 3, unused: 3 }, {}, 'warnings'), true);

function notes(r, fresh) { return warningsNotes(r, fresh).map(function (n) { return n.tone + ': ' + n.text; }); }
check('nothing to distrust, nothing said', notes({ at: 3, manifest_at: 4, unused: [{}], checked: 9 }, { state: 'fresh' }), []);
check('a project file newer than the manifest',
      notes({ at: 5, manifest_at: 4, unused: [{}], checked: 9 }, { state: 'fresh' }),
      ['warn: dbt_project.yml was saved after the manifest was written: a key added or moved since may name '
        + 'nodes the manifest does not hold yet, until dbt parses again.']);
check('other files newer than the manifest',
      notes({ at: 3, manifest_at: 4, unused: [{}], checked: 9 }, { state: 'edited' }),
      ['warn: Files changed since the manifest was written: a folder added since is in no manifest, '
        + 'so its key reads unused until dbt parses again.']);
check('a file that could not be read, and nothing else',
      notes({ at: 5, manifest_at: 4, unused: [], checked: 0, unread: { line: 12, message: 'a tag such as !!str' } }, { state: 'edited' }),
      ['warn: dbt_project.yml could not be read here, at line 12: a tag such as !!str. Nothing is worked out from it until it can be.']);
check('every path used', notes({ at: 3, manifest_at: 4, unused: [], checked: 9 }, null),
      [': Each of the 9 config paths of dbt_project.yml applies to a resource.']);

print('\n--- commands that build nothing ---');
['compile', 'show', 'docs generate', 'run-operation'].forEach(function (c) {
  check('dbt ' + c + ' wrote over the last run', runNotes({ command: c, kinds: [], unknown: { of: 0 } })[0].text,
        'dbt ' + c + ' wrote this file over the one before, so what the last run did is no longer in it.');
});
check('a clone is a run', runNotes({ command: 'clone', kinds: [{ kind: 'model', ran: 2, of: 2, statuses: { ok: 2 } }], unknown: { of: 0 } }).length, 0);
check('kinds dbt writes besides nodes read as headings', [runKindPlural('sql_operation', true), runKindPlural('macro', true)], ['Sql operations', 'Macros']);

print('\n--- the warnings and the macros in one panel (0103, 0105) ---');
check('both listed, the warnings first and the macros after the coverage',
      keys(reportsSections({ macros: { unexplained: 1, at: 3 }, warnings: { at: 4, manifest_at: 3, unused: 2 }, run: { at: 9 } })),
      ['run', 'warnings', 'jobs', 'macros']);
check('the macros alone with something, the warnings out',
      empty(reportsSections({ macros: { unexplained: 1, at: 3 } })), ['jobs:empty', 'macros']);
check('a card\'s focus lists them empty beside the warnings, which are not empty',
      empty(reportsSections({ warnings: { at: 4, manifest_at: 3, unused: 2 } }, 'macro.shop.cents')), ['warnings', 'jobs:empty', 'macros:empty']);
check('opens on the warnings, not on a macros section a click listed',
      reportsDefaultSection(reportsSections({ warnings: { at: 4, manifest_at: 3, unused: 2 } }, 'macro.shop.cents'), [], 'macros'), 'warnings');
check('on the first with something over every section, the coverage\'s emptiness passed',
      reportsDefaultSection(reportsSections({ macros: { unexplained: 1, at: 3 } }, 'macro.shop.cents'), [], ''), 'macros');
check('on the first there is when all are empty, a focus listing the macros last',
      reportsDefaultSection(reportsSections({}, 'macro.shop.cents'), [], 'macros'), 'jobs');
check('the tooltip names both', reportsButtonTitle(reportsSections({ macros: { unexplained: 1, at: 3 }, warnings: { unused: 1 } }), [], 'K').split('\n')[0],
      'Reports on the whole project: Warnings, Macros (K)');
