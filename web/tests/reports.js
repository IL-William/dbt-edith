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
var badge = { text: '3 to investigate', title: 'collin\'s last run names 3 models or tables to look at.' };
check('collin\'s badge becomes news on the button', reportsNews(badge),
      [{ key: 'collin', tool: 'collin', text: '3 to investigate', title: badge.title }]);

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
