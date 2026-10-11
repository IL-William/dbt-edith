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
check('none before the index is read', keys(reportsSections(null)), []);
check('none while nothing was written or named', keys(reportsSections({})), []);
check('collin once a run wrote a report', keys(reportsSections({ collin: { at: 5 } })), ['collin']);
check('the jobs once a source is named', keys(reportsSections({ jobs: { sources: 1, at: 3 } })), ['jobs']);
check('in the order they are worked through, whatever the index says',
      keys(reportsSections({ jobs: { sources: 1, at: 3 }, collin: { at: 5 } })), ['collin', 'jobs']);
check('a key this page does not know is left out', keys(reportsSections({ later: { at: 1 } })), []);
check('each in its tool\'s colour', reportsSections({ collin: { at: 5 } })[0].tool, 'collin');

print('\n--- what is new ---');
check('nothing from collin, nothing new', reportsNews(null), []);
var badge = { text: '3 to investigate', count: 3, title: 'collin\'s last run names 3 models or tables to look at.' };
check('collin\'s badge becomes news on the button', reportsNews(badge),
      [{ key: 'collin', tool: 'collin', text: '3 to investigate', count: 3, title: badge.title }]);

print('\n--- where it opens ---');
var both = reportsSections({ collin: { at: 5 }, jobs: { sources: 1, at: 3 } });
check('on the first section with nothing new and nothing read yet', reportsDefaultSection(both, [], ''), 'collin');
check('on the section read last this session', reportsDefaultSection(both, [], 'jobs'), 'jobs');
check('on news before the last section read', reportsDefaultSection(both, reportsNews(badge), 'jobs'), 'collin');
check('not on a section that has gone', reportsDefaultSection(reportsSections({ jobs: { sources: 1, at: 3 } }), [], 'collin'), 'jobs');
check('news for a section that is not there opens the first', reportsDefaultSection(reportsSections({ jobs: { sources: 1, at: 3 } }), reportsNews(badge), ''), 'jobs');
check('nowhere when there is nothing', reportsDefaultSection([], reportsNews(badge), 'collin'), '');

print('\n--- the button\'s tooltip ---');
check('names what the reports hold and the key',
      reportsButtonTitle(both, [], '⌥⌘R'), 'Reports on the whole project: collin\'s report, Job coverage (⌥⌘R)');
check('then what is new, a line each',
      reportsButtonTitle(both, reportsNews(badge), 'Ctrl+Alt+R').split('\n')[1], badge.title);
check('says the key even with nothing to show', reportsButtonTitle([], [], 'Ctrl+Alt+R'), 'Reports on the whole project (Ctrl+Alt+R)');

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
check('listed first once dbt wrote a run', keys(reportsSections({ collin: { at: 5 }, run: { at: 9 } })), ['run', 'collin']);
var run = { at: 9, command: 'build', error: 1, fail: 2, warn: 3 };
check('its errors and failures are news', runBadge(run, 5),
      { text: '1 error, 2 failed', count: 3, title: 'dbt build ended with 1 error and 2 failed tests.' });
check('once only: a run already seen is not', runBadge(run, 9), null);
check('nor a run that went well', runBadge({ at: 9, command: 'run', error: 0, fail: 0, warn: 4 }, 0), null);
check('nor a file still being written, with no counts', runBadge({ at: 9 }, 0), null);
check('errors alone', runBadge({ at: 9, command: 'run', error: 2, fail: 0 }, 0).title, 'dbt run ended with 2 errors.');
check('a command the file does not name', runBadge({ at: 9, error: 1 }, 0).title, 'The last dbt command ended with 1 error.');
check('one failed test', runBadge({ at: 9, command: 'test', fail: 1 }, 0), { text: '1 failed', count: 1, title: 'dbt test ended with 1 failed test.' });
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
