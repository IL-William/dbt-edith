// The query history's pure parts: the role menu, the state of a query, and how
// a row reads. Its Snowsight link is in snowsight.js.
// Run from the repository root: jsc web/tests/history.js
var app = read('web/app.js');
eval(app.slice(app.indexOf('function historyRoles'), app.indexOf('async function loadHistory')));

function check(label, got, want) {
  print((got === want ? 'PASS  ' : 'FAIL  ') + label + (got === want ? '' : '\n        expected ' + want + '\n        got      ' + got));
}

print('--- the role menu ---');
var targets = [
  { name: 'dev', role: 'transformer', same_login: true },
  { name: 'prod', role: 'reporter', same_login: true },
  { name: 'qa', role: 'TRANSFORMER', same_login: true },
  { name: 'ci', role: 'ci_runner', same_login: false, why: 'it signs in as another user or account' },
  { name: 'dyn', same_login: true, why: 'its role is a Jinja expression' },
];
var menu = historyRoles(targets, 'dev', 'TRANSFORMER');
check('the target\'s role comes first, as null so the script resolves it',
      JSON.stringify(menu.offered[0]), '{"role":null,"label":"transformer","targets":["dev","qa"]}');
check('a role two targets share is offered once', menu.offered.length, 2);
check('another target with the same login', JSON.stringify(menu.offered[1]), '{"role":"reporter","label":"reporter","targets":["prod"]}');
check('every role', menu.all.role, '*');
check('another login and an unknown role are greyed with their reason',
      menu.greyed.map(function (g) { return g.label + ': ' + g.why; }).join(' | '),
      'ci_runner: it signs in as another user or account | unknown role: its role is a Jinja expression');
var bare = historyRoles([{ name: 'dev', same_login: true, why: 'it names no role' }], 'dev', 'PUBLIC');
check('a target with no role reads the session\'s', bare.offered[0].label, 'PUBLIC');
check('before the first answer, the menu still has an entry', historyRoles(null, '', '').offered[0].label, 'the target\'s role');

print('\n--- a query\'s state ---');
check('success', queryState({ status: 'success' }), 'ok');
check('failed', queryState({ status: 'failed_with_error', error: 'SQL compilation error' }), 'failed');
check('cancelled is only in the message',
      queryState({ status: 'failed_with_error', error: 'SQL execution canceled' }), 'cancelled');
check('an incident is a failure', queryState({ status: 'failed_with_incident' }), 'failed');
check('waking a warehouse is running', queryState({ status: 'resuming_warehouse' }), 'running');
check('blocked waits', queryState({ status: 'blocked' }), 'queued');

print('\n--- how a row reads ---');
check('milliseconds', queryDuration(840), '840 ms');
check('seconds', queryDuration(12345), '12.3 s');
check('minutes', queryDuration(185000), '3 min 5 s');
check('hours', queryDuration(2 * 3600000 + 5 * 60000), '2 h 5 min');
check('no time, nothing', queryDuration(undefined), '');
var utc = function () { return 0; };
var now = Date.parse('2026-10-08T15:00:00Z');
check('today is the time alone', historyTime(Date.parse('2026-10-08T09:12:03.123Z'), now, utc), '09:12:03');
check('another day names it', historyTime(Date.parse('2026-10-07T23:59:59Z'), now, utc), '2026-10-07 23:59:59');
check('on the reader\'s clock', historyTime(Date.parse('2026-10-07T23:30:00Z'), now, function () { return 120; }), '01:30:00');
check('dbt\'s leading comment is skipped',
      sqlHead('/* {"app": "dbt", "node_id": "model.shop.orders"} */\n\n  create or replace   view x as (\n select 1)'),
      'create or replace view x as (');
check('line comments too', sqlHead('-- a note\n-- another\nselect *\nfrom t'), 'select *');
check('an unclosed comment shows nothing rather than half of it', sqlHead('/* never closed'), '');
check('no text', sqlHead(undefined), '');
