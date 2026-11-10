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
check('the jobs once a source is named, and the order of their steps with them (0126)', empty(reportsSections({ jobs: { sources: 1, at: 3 } })), ['jobs', 'joborder']);
check('in the order they are worked through, whatever the index says',
      keys(reportsSections({ jobs: { sources: 1, at: 3 }, collin: { at: 5 } })), ['collin', 'jobs', 'joborder']);
check('a key this page does not know is left out', keys(reportsSections({ later: { at: 1 } })), ['jobs']);
check('each in its tool\'s colour', reportsSections({ collin: { at: 5 } })[0].tool, 'collin');
check('an index read twice is not marked empty by the first', empty(reportsSections({}).concat(reportsSections({ jobs: { sources: 1, at: 3 } }))),
      ['jobs:empty', 'jobs', 'joborder']);
check('the macros while one of the project\'s is reached by no node and nothing explains it (0105)',
      empty(reportsSections({ macros: { unexplained: 2, at: 3 } })), ['jobs:empty', 'macros']);
check('or once a card asked for a macro\'s callers, empty: a click lights nothing',
      empty(reportsSections({}, 'macro.shop.cents')), ['jobs:empty', 'macros:empty']);
check('and lit by the macros alone when one is unexplained',
      empty(reportsSections({ macros: { unexplained: 1, at: 3 } }, 'macro.shop.cents')), ['jobs:empty', 'macros']);
check('last, after the coverage and the order of the steps', keys(reportsSections({ macros: { unexplained: 1, at: 3 }, run: { at: 9 }, jobs: { sources: 1, at: 3 } })),
      ['run', 'jobs', 'joborder', 'macros']);
check('nor before the index is read, nor while it holds no jobs, as the builds of a job are not (0119)',
      [keys(reportsSections(null)), keys(reportsSections({})), keys(reportsSections({ collin: { at: 5 } }))].map(function (k) { return k.indexOf('joborder'); }), [-1, -1, -1]);
check('quiet: listed, and it never lights the button (0126)',
      reportsSections({ jobs: { sources: 1, at: 3 } }).filter(function (s) { return s.key === 'joborder'; }).map(function (s) { return [!!s.quiet, !!s.empty, lights(s)]; }),
      [[true, false, false]]);
check('in the colour of the jobs', reportsSections({ jobs: { sources: 1, at: 3 } })[1].tool, 'jobs');
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
      ['run:bad', 'warnings:-', 'collin:-', 'jobs:-', 'joborder:-', 'macros:-']);
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
check('never on the order of the steps, last read or only one listed (0126)',
      [reportsDefaultSection(reportsSections({ jobs: { sources: 1, at: 3 }, collin: { at: 5 } }), [], 'joborder'), reportsDefaultSection(reportsSections({ jobs: { sources: 1, at: 3 } }), [], 'joborder')],
      ['collin', 'jobs']);
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
check('the order of the steps follows the coverage\'s entry, which it has none of its own (0126)',
      [reportsEntry('joborder'), reportsEntry('jobs'), reportsEntry('collin'), reportsStale({ sources: 1, at: 3 }, { jobs: { sources: 1, at: 4 } }, 'joborder'),
        reportsStale({ sources: 1, at: 3 }, { jobs: { sources: 1, at: 3 } }, 'joborder')], ['jobs', 'jobs', 'collin', true, false]);
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

print('\n--- the checks (0116) ---');
var checks = { checks: { count: 4, at: 3 } };
check('listed only while the index has them', [keys(reportsSections({})), keys(reportsSections(checks))],
      [['jobs'], ['jobs', 'checks']]);
check('in the rail as any section with something, not hollow, and quiet', [empty(reportsSections(checks)), reportsSections(checks)[1].quiet,
      reportsSections({ run: { at: 9 } })[0].quiet], [['jobs:empty', 'checks'], true, undefined]);
function lit(sections) { return sections.some(lights); }
check('the button stays dimmed with only them, and lit by any other section',
      [lit(reportsSections(checks)), lit(reportsSections(Object.assign({ run: { at: 9 } }, checks))), lit(reportsSections({}))], [false, true, false]);
check('not by a macro card\'s focus', keys(reportsSections({}, 'macro.shop.cents')), ['jobs', 'macros']);
check('last, after the macros', keys(reportsSections({ checks: { count: 1, at: 3 }, macros: { unexplained: 1, at: 3 }, warnings: { at: 4, manifest_at: 3, unused: 2 },
      run: { at: 9 }, collin: { at: 5 }, jobs: { sources: 1, at: 3 } })), ['run', 'warnings', 'collin', 'jobs', 'joborder', 'macros', 'checks']);
check('under the dbt tool, with a title saying they are the tool\'s own checks and not dbt\'s warnings',
      [reportsSections(checks)[1].tool, /own checks/.test(reportsSections(checks)[1].title), /warn/.test(reportsSections(checks)[1].title)], ['dbt', true, true]);
check('no dot tone, which is the last run\'s alone',
      tones(Object.assign({ run: { at: 9, error: 1, fail: 0, warn: 0 } }, checks)), ['run:bad', 'jobs:-', 'checks:-']);
check('never the section the panel opens on: the first with something else, or the empty coverage',
      [reportsDefaultSection(reportsSections(Object.assign({ run: { at: 9 } }, checks)), [], ''), reportsDefaultSection(reportsSections(checks), [], ''),
        reportsDefaultSection(reportsSections(Object.assign({ macros: { unexplained: 1, at: 3 } }, checks)), [], '')],
      ['run', 'jobs', 'macros']);
check('nor the one read last',
      [reportsDefaultSection(reportsSections(Object.assign({ run: { at: 9 } }, checks)), [], 'checks'), reportsDefaultSection(reportsSections(checks), [], 'checks')], ['run', 'jobs']);
check('the tooltip is the one it would be without them',
      [reportsButtonTitle(reportsSections(checks), [], 'K'), reportsButtonTitle(reportsSections(Object.assign({ run: { at: 9 } }, checks)), [], 'K')],
      [reportsButtonTitle(reportsSections({}), [], 'K'), reportsButtonTitle(reportsSections({ run: { at: 9 } }), [], 'K')]);
check('stale once the manifest they were read in is behind', [reportsStale({ count: 4, at: 3 }, { checks: { count: 4, at: 3 } }, 'checks'),
      reportsStale({ count: 4, at: 3 }, { checks: { count: 4, at: 5 } }, 'checks'), reportsStale({ count: 4, at: 3 }, { checks: { count: 2, at: 3 } }, 'checks'),
      reportsStale({ count: 4, at: 3 }, {}, 'checks')], [false, true, true, true]);

print('\n--- the head of a section ---');
// reportHead only builds elements: a tiny fake DOM is all it needs.
var document = { createElement: function (tag) {
  return { tag: tag, children: [], dataset: {}, title: '', setAttribute: function () {},
           addEventListener: function () {}, append: function () { this.children.push.apply(this.children, arguments); } };
} };
function jel(tag, className, textContent) { var e = document.createElement(tag); e.className = className; e.textContent = textContent || ''; return e; }
function runEndWords() { return ''; }
eval(app.slice(app.indexOf('function reportHead'), app.indexOf('/* A fold of a section')));
function buttons(head) { return head.children[0].children.filter(function (c) { return c.tag === 'button'; }).map(function (c) { return c.textContent; }); }
check('Read again when the section can redraw itself, and no ×: a space is left by its tab or Escape (0140)',
      buttons(reportHead({ title: 'T', tool: 'dbt', again: function () {} })), ['Read again']);
check('no button of its own when it is not given a way to (the builds, whose own button asks Snowflake)',
      buttons(reportHead({ title: 'T', tool: 'snowflake', extra: [jel('button', '', 'Refresh')] })), ['Refresh']);

print('\n--- an explanation folded to its first sentence ---');
check('split at the first full stop followed by a space', leverSplit('No freshness. Neither warn_after nor error_after holds a count.'),
      { first: 'No freshness.', rest: 'Neither warn_after nor error_after holds a count.' });
check('a dotted name does not end the sentence', leverSplit('Reads stg_orders.csv before it. Then the rest.').first, 'Reads stg_orders.csv before it.');
check('a question or an exclamation ends it too', [leverSplit('Why? Because.').first, leverSplit('Look! Now.').first], ['Why?', 'Look!']);
check('one sentence has no rest, with or without its stop', [leverSplit('Only this.'), leverSplit('Only this')],
      [{ first: 'Only this.', rest: '' }, { first: 'Only this', rest: '' }]);
check('nothing at all', leverSplit(''), { first: '', rest: '' });
check('the rest keeps its own sentences', leverSplit('One. Two. Three.').rest, 'Two. Three.');

print('\n--- the folds, filtered on their data ---');
function fold(title, rows, more) {
  return Object.assign({ title: title, tone: '', lever: '', rows: rows, open: true, find: function (r) { return r.name; }, draw: function () { return []; } }, more || {});
}
var rowsA = [{ name: 'orders' }, { name: 'order_items' }, { name: 'customers' }];
var groupsA = [{ folds: [fold('Models', rowsA), fold('Sources', [{ name: 'raw_orders' }, { name: 'raw_users' }])] }];
function shapes(gs, q) { return reportFiltered(gs, q).map(function (g) { return g.folds.map(function (f) { return [f.title, f.hits.length, f.count]; }); }); }
check('unfiltered, every row and the total as the count', shapes(groupsA, ''), [[['Models', 3, '3'], ['Sources', 2, '2']]]);
check('filtered, the hits and "n of m", case-insensitively', shapes(groupsA, 'ORDER'), [[['Models', 2, '2 of 3'], ['Sources', 1, '1 of 2']]]);
check('filtered but nothing hidden, the plain count', shapes(groupsA, 'r'), [[['Models', 3, '3'], ['Sources', 2, '2']]]);
check('a badge is the count whenever nothing is hidden', shapes([{ folds: [fold('M', rowsA, { badge: 'high' })] }], 'r')[0][0][2], 'high');
check('a severity is not a badge: the count stays the count, and the fold carries the severity on',
  [shapes([{ folds: [fold('M', rowsA, { severity: 'info' })] }], '')[0][0][2], reportFiltered([{ folds: [fold('M', rowsA, { severity: 'info' })] }], '')[0].folds[0].severity], ['3', 'info']);
check('a fold with no hit is dropped while filtered', shapes(groupsA, 'customers'), [[['Models', 1, '1 of 3']]]);
check('and a group left with no fold', shapes(groupsA, 'nothing like this'), []);
check('blank spaces in the filter are not part of it', shapes(groupsA, '  order  ').length, 1);
check('the total is `of` when the server sent fewer rows', shapes([{ folds: [fold('Models', rowsA, { of: 2000 })] }], ''), [[['Models', 3, '2000']]]);
check('and "n of m" counts against it', shapes([{ folds: [fold('Models', rowsA, { of: 2000 })] }], 'order'), [[['Models', 2, '2 of 2000']]]);
check('a fold with a total of zero is dropped, whatever its rows', shapes([{ folds: [fold('Models', [], { of: 0 }), fold('Sources', rowsA)] }], ''), [[['Sources', 3, '3']]]);
check('a badge replaces the total unfiltered, and "n of m" wins while the filter hides some',
      [shapes([{ folds: [fold('M', rowsA, { badge: 'high' })] }], '')[0][0][2], shapes([{ folds: [fold('M', rowsA, { badge: 'high' })] }], 'order')[0][0][2]], ['high', '2 of 3']);
check('open as given', reportFiltered([{ folds: [fold('M', rowsA, { open: false })] }], '')[0].folds[0].open, false);
check('open by the rows it would list: the total unfiltered, the hits filtered',
      [reportFiltered([{ folds: [fold('M', rowsA, { of: 60, open: function (n) { return n <= 50; } })] }], '')[0].folds[0].open,
        reportFiltered([{ folds: [fold('M', rowsA, { of: 60, open: function (n) { return n <= 50; } })] }], 'order')[0].folds[0].open], [false, true]);
check('how many the server held back, said only unfiltered',
      [reportFiltered([{ folds: [fold('M', rowsA, { of: 2000 })] }], '')[0].folds[0].more, reportFiltered([{ folds: [fold('M', rowsA, { of: 2000 })] }], 'order')[0].folds[0].more], [1997, 0]);
check('a group with only a head stays, filtered or not',
      [reportFiltered([{ head: ['h'], folds: [] }], '').length, reportFiltered([{ head: ['h'], folds: [fold('M', rowsA)] }], 'zzz').length], [1, 1]);
check('the group\'s own keys travel with it', reportFiltered([{ group: 'rerun', label: 'Rerun', desc: 'd', folds: [fold('M', rowsA)] }], '')[0].label, 'Rerun');
check('the fold\'s own keys too, and the rows are not changed',
      [reportFiltered(groupsA, 'order')[0].folds[0].tone, groupsA[0].folds[0].rows.length], ['', 3]);

print('\n--- a key row of the Checks ---');
check('a composite key keeps its columns, which nothing else on the row says', checkDetail({ key: ['order_id', 'line'], detail: 'key order_id, line: no not_null on line' }), 'key order_id, line: no not_null on line');
check('every column of it lacking the test is said once, not twice', checkDetail({ key: ['order_id', 'line'], detail: 'key order_id, line: no not_null on order_id, line' }), 'no not_null on any column of key order_id, line');
check('a single column key whose list is itself is as before', checkDetail({ key: ['id'], detail: 'key id: no not_null on id' }), 'no not_null on id');
check('a composite with only some columns missing keeps its detail whole', checkDetail({ key: ['a', 'b', 'c'], detail: 'key a, b, c: no not_null on b, c' }), 'key a, b, c: no not_null on b, c');
check('so two keys of one model missing the same column read apart',
      [checkDetail({ key: ['order_id', 'line'], detail: 'key order_id, line: no not_null on line' }), checkDetail({ key: ['sku', 'line'], detail: 'key sku, line: no not_null on line' })].join('|'),
      'key order_id, line: no not_null on line|key sku, line: no not_null on line');
check('a single column', checkDetail({ key: ['id'], detail: 'key id: no not_null on id' }), 'no not_null on id');
check('as sent when the key is not the one the text starts with', checkDetail({ key: ['id'], detail: 'key other: no not_null on other' }), 'key other: no not_null on other');
check('as sent without a key list', checkDetail({ detail: 'unique_key id' }), 'unique_key id');
check('a unique_key row is as sent', checkDetail({ key: ['id'], detail: 'unique_key id' }), 'unique_key id');
check('no detail, none', checkDetail({ id: 'model.shop.a' }), '');

print('\n--- the figures over the Warnings ---');
check('how many paths, and how many apply to nothing', warningsTiles({ checked: 12, unused: [{}, {}] }).map(function (t) { return [t.label, t.value, t.tone]; }),
      [['Config paths', '12', undefined], ['Apply to no resource', '2', 'warn']]);
check('no tone when every path applies', warningsTiles({ checked: 12, unused: [] })[1].tone, '');
check('none while the file could not be read', warningsTiles({ unread: { line: 3, message: 'x' }, checked: 0, unused: [] }), []);

print('\n--- the rail, by who reads the files ---');
var secs = reportsSections({ run: { at: 9 }, warnings: { unused: 2 }, collin: { at: 5 }, jobs: { sources: 1, at: 3 }, macros: { unexplained: 1, at: 3 },
  checks: { count: 4, at: 3 }, builds: {} });
check('dbt, collin, Jobs, Snowflake, each in the sections\' own order',
      reportsRailGroups(secs).map(function (g) { return [g.label, g.sections.map(function (s) { return s.key; })]; }),
      [['dbt', ['run', 'warnings', 'macros', 'checks']], ['collin', ['collin']], ['Jobs', ['jobs', 'joborder']], ['Snowflake', ['builds', 'jobbuilds']]]);
check('a group with no section is not listed', reportsRailGroups(reportsSections({})).map(function (g) { return g.label; }), ['Jobs']);
check('and none at all', reportsRailGroups([]), []);

print('\n--- the count beside a section ---');
function railN(key, index, collin, kept) { var c = reportsRailCount(key, index, collin, kept); return c && c.n; }
check('the last run: everything that did not pass', railN('run', { run: { error: 1, fail: 2, warn: 3, partial: 1, ok: 40, other: 1 } }), 8);
check('the warnings: the paths that apply to nothing', railN('warnings', { warnings: { unused: 5 } }), 5);
check('collin: what it names to look at', railN('collin', {}, { report: { to_investigate: 7 } }), 7);
check('the jobs: in no job or only in unscheduled ones', railN('jobs', { jobs: { never: 4, unscheduled: 3 } }), 7);
check('the macros: unexplained', railN('macros', { macros: { unexplained: 2 } }), 2);
check('the checks: the index\'s count', railN('checks', { checks: { count: 90 } }), 90);
check('the job order: what the section left when it was drawn', railN('joborder', {}, null, { joborder: 3 }), 3);
check('none until drawn, never fetched to count', railN('joborder', { jobs: {} }, null, {}), null);
check('the builds count nothing: a number across environments would match nothing on the page',
      ['builds', 'jobbuilds'].map(function (k) { return railN(k, { builds: {}, jobs: {} }, null, { builds: 5, jobbuilds: 2 }); }), [null, null]);
check('zero is none', [railN('run', { run: { error: 0, fail: 0, warn: 0, partial: 0, ok: 4, other: 0 } }), railN('warnings', { warnings: { unused: 0 } }), railN('collin', {}, { report: { to_investigate: 0 } })], [null, null, null]);
check('and a missing index', ['run', 'warnings', 'collin', 'jobs', 'macros', 'checks'].map(function (k) { return railN(k, null, null, null); }), [null, null, null, null, null, null]);
check('the checks say findings, as the index sums the rows of every check', reportsRailCount('checks', { checks: { count: 4 } }).title, '4 findings in this tool\'s own checks');
check('a title says what is counted, in the singular too', [reportsRailCount('warnings', { warnings: { unused: 1 } }).title, reportsRailCount('macros', { macros: { unexplained: 3 } }).title],
      ['1 config path of dbt_project.yml that applies to no resource', '3 macros no node reaches and nothing explains']);

print('\n--- the source named at the head of a section ---');
function row(head) { return head.children[0]; }
var title = row(reportHead({ title: 'Job order', tool: 'jobs' })).children[0];
check('the title first', [title.className, title.textContent], ['ttl', 'Job order']);
var chip = row(reportHead({ title: 'Job order', tool: 'jobs' })).children[1];
check('then a chip, with the word the rail head says, and its source for the colour', [chip.className, chip.textContent, chip.dataset.tool], ['srcchip', 'Jobs', 'jobs']);
check('the same word as the rail\'s head, for every source',
  ['dbt', 'collin', 'jobs', 'snowflake'].map(function (t) { return [row(reportHead({ title: 'T', tool: t })).children[1].textContent, reportSourceLabel(t)]; }),
  [['dbt', 'dbt'], ['collin', 'collin'], ['Jobs', 'Jobs'], ['Snowflake', 'Snowflake']]);
check('and the rail groups carry it', reportsRailGroups(reportsSections({ run: { at: 9 }, collin: { at: 5 }, jobs: { sources: 1, at: 3 } })).map(function (g) { return [g.tool, g.label, reportSourceLabel(g.tool)]; }),
  [['dbt', 'dbt', 'dbt'], ['collin', 'collin', 'collin'], ['jobs', 'Jobs', 'Jobs']]);
check('no dot with no outcome to say: the chip is all', row(reportHead({ title: 'T', tool: 'dbt' })).children.filter(function (c) { return c.className === 'tooldot'; }).length, 0);
runEndWords = function (tone) { return tone === 'bad' ? 'Ended with errors or failed tests' : ''; };
var withDot = row(reportHead({ title: 'Last run', tool: 'dbt', tone: 'bad' })).children;
check('the dot stays for a run outcome, after the chip', [withDot[1].className, withDot[2].className, withDot[2].dataset.tone, withDot[2].title], ['srcchip', 'tooldot', 'bad', 'Ended with errors or failed tests']);
var withAbout = reportHead({ title: 'T', tool: 'dbt', about: 'read from run_results.json' });
check('what it was read from is a line of its own under the row, with a rule under the head',
  [withAbout.className, withAbout.children.map(function (c) { return c.className; }), withAbout.children[1].textContent], ['rhead', ['rhead-row', 'sub'], 'read from run_results.json']);
check('a section with nothing to say of that has no line, and none is made up', reportHead({ title: 'T', tool: 'dbt' }).children.length, 1);
check('a sentence about the section is a prose line; what was read stays the plain mono one',
  [reportHead({ title: 'T', tool: 'jobs', about: 'A sentence.', prose: true }).children[1].className, withAbout.children[1].className], ['sub prose', 'sub']);
// The two callers cannot be run here (they read the job files), so their source is pinned.
function callerSource(name) { var at = app.indexOf('async function ' + name); return app.slice(at, app.indexOf('\nasync function ', at + 1) < 0 ? undefined : app.indexOf('\nasync function ', at + 1)); }
check('Job coverage and Job order each ask for the prose line',
  ['jobsCoverageSection', 'jobOrderSection'].map(function (n) { return /reportHead\(\{[^}]*prose: true/.test(callerSource(n)); }), [true, true]);
var proseRule = /\.rhead \.sub\.prose \{[^}]*font:[^;}]*var\(--ui\)/.test(read('web/app.css'));
check('the prose line is drawn in the UI font', proseRule, true);
check('prose without a line says nothing', reportHead({ title: 'T', tool: 'jobs', prose: true }).children.length, 1);

print('\n--- a fold\'s count: a pill or text ---');
eval(app.slice(app.indexOf('function reportCounted'), app.indexOf('/* A fold as reportFiltered')));
check('a number, a thousands separator (comma or space) and "n of m" are pills',
  ['12', '0', '1,234', '1 234', '3 of 12', '1,204 of 3,000'].map(reportCounted), [true, true, true, true, true, true]);
check('the number 7, not a string, is a pill', reportCounted(7), true);
check('a phrase, words after the number, nothing, or "of" first stay text',
  ['used by 3 models, 2 tests', '3 models', '', undefined, 'of 3'].map(reportCounted), [false, false, false, false, false]);
