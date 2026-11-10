// The Jobs tab's pure parts: how a job and a step read, the coverage lists,
// the body the sources form sends, and the example it draws (0063).
// Run from the repository root: jsc web/tests/jobs.js
var app = read('web/app.js');
// The cron the words are built on, and the jobs slice itself.
eval(app.slice(app.indexOf('function cronFields'), app.indexOf('function zoneWall')));
eval(app.slice(app.indexOf('function jobsScheduleText'), app.indexOf('async function loadJobs')));

function check(label, got, want) {
  print((got === want ? 'PASS  ' : 'FAIL  ') + label + (got === want ? '' : '\n        expected ' + want + '\n        got      ' + got));
}

var DEFAULTS = [
  ['dag_id', 'dag_id'], ['schedule', 'schedule'], ['tasks', 'tasks'], ['task_id', 'task_id'], ['command', 'command'],
  ['depends_on', 'depends_on'], ['outlets', 'outlets'], ['sensors', 'sensors'], ['sensor_id', 'task_id'],
  ['external_dag_id', 'external_dag_id'], ['external_task_id', 'external_task_id'],
];

print('--- how a job starts ---');
check('a cron', jobsScheduleText({ kind: 'airflow', cron: '0 5 * * *' }), '0 5 * * *');
check('nothing starts it', jobsScheduleText({ kind: 'airflow' }), 'manual');
check('assets', jobsScheduleText({ kind: 'airflow', assets: ['a', 'b'] }), 'on a, b');
check('after a job read', jobsScheduleText({ kind: 'dbt_cloud', after: 'Daily build' }), 'after Daily build');
check('after a job no file defines',
      jobsScheduleText({ kind: 'dbt_cloud', after: '101', after_unread: true }), 'after job 101, which no file read defines');
check('several at once', jobsScheduleText({ kind: 'dbt_cloud', cron: '0 * * * *', ci: true, merge: true }), '0 * * * * · CI · on merge');
check('a schedule only the vars file knows',
      jobsScheduleText({ kind: 'dbt_cloud', note: 'a schedule only the vars file knows' }), 'a schedule only the vars file knows');
check('on a clock', jobsStart({ cron: '0 5 * * *', scheduled: true }), 'scheduled');
check('after a scheduled job', jobsStart({ after: 'Daily build', scheduled: true }), 'scheduled');
check('started by nothing', jobsStart({ scheduled: false }), 'manual');
check('CI, which no clock starts', jobsStart({ ci: true, scheduled: false }), 'other');
check('on an asset only a manual DAG updates', jobsStart({ assets: ['a'], scheduled: false }), 'other');
check('@once', jobsStart({ note: 'runs once, at its start date', scheduled: false }), 'other');
check('a templated schedule', jobsStart({ note: 'a schedule only known when the DAG is parsed', scheduled: false, schedule_unread: true }), 'unread');
check('on an asset no file read updates', jobsStart({ assets: ['raw'], assets_unread: ['raw'], scheduled: false, schedule_unread: true }), 'unread');
check('a dbt Cloud trigger on, its cron not read', jobsStart({ note: 'a schedule only the vars file knows', scheduled: true }), 'scheduled');
check('the one asset no file read updates', jobsScheduleText({ kind: 'airflow', assets: ['raw'], assets_unread: ['raw'] }),
      'on raw, which no file read updates');
check('one of several', jobsScheduleText({ kind: 'airflow', assets: ['raw', 'marts'], assets_unread: ['raw'] }),
      'on raw, marts; no file read updates raw');
var mixed = [
  { job: { name: 'by_hand', scheduled: false } }, { job: { name: 'ci', ci: true, scheduled: false } },
  { job: { name: 'daily', cron: '@daily', scheduled: true } }, { job: { name: 'jaffle_reports', scheduled: false } },
  { job: { name: 'templated', note: 'a schedule only known when the DAG is parsed', scheduled: false, schedule_unread: true } },
  { job: { name: 'after', after: 'daily', scheduled: true } },
];
check('scheduled first, then not read, manual last, each in the order sent',
      jobsByStart(mixed).map(function (g) { return g.job.name; }).join(' '), 'daily after templated ci by_hand jaffle_reports');
check('the order sent is left alone', mixed[0].job.name, 'by_hand');
check('no groups', jobsByStart(undefined).length, 0);
check('jobs listed bare, as the coverage reads them',
      jobsByStart([{ name: 'm', scheduled: false }, { name: 's', cron: '@daily', scheduled: true }], function (j) { return j; })
        .map(function (j) { return j.name; }).join(' '), 's m');

print('\n--- how a job and a step are named ---');
check('a DAG', jobsJobLabel({ kind: 'airflow', name: 'shop_daily' }), 'DAG shop_daily');
check('a dbt Cloud job with its identifier', jobsJobLabel({ kind: 'dbt_cloud', name: 'Daily build', ident: 'daily' }), 'Job Daily build [[daily]]');
check('a task', jobsStepLabel({ kind: 'airflow' }, { name: 'marts', n: 2 }), 'task marts');
check('a task with no id', jobsStepLabel({ kind: 'airflow' }, { name: '', n: 2 }), 'task 2');
check('a task on a chip', jobsChipText({ kind: 'airflow', name: 'shop_daily' }, { name: 'marts', n: 2 }), 'shop_daily › marts');
check('a task with no id on a chip', jobsChipText({ kind: 'airflow', name: 'shop_daily' }, { name: '', n: 2 }), 'shop_daily › task 2');
check('a dbt Cloud step on a chip', jobsChipText({ kind: 'dbt_cloud', name: 'Daily build', steps: 3 }, { n: 2 }), 'Daily build › step 2 of 3');
check('chips, scheduled jobs first and each job\'s steps in order',
      jobsChips([
        { job: { kind: 'airflow', name: 'jaffle_reports', scheduled: false }, steps: [{ name: 'all', n: 1 }] },
        { job: { kind: 'airflow', name: 'daily', cron: '@daily', scheduled: true }, steps: [{ name: 'stage', n: 1 }, { name: 'marts', n: 2 }] },
      ]).map(function (c) { return c.text; }).join(' | '),
      'daily › stage | daily › marts | jaffle_reports › all');
check('a dbt Cloud step by its place', jobsStepLabel({ kind: 'dbt_cloud', steps: 3 }, { n: 2 }), 'step 2 of 3');
check('steps not resolved, counted over their jobs',
      jobsMaybeCount([{ steps: [{ n: 1 }, { n: 2 }] }, { steps: [{ n: 4 }] }]), 3);
check('none not resolved', jobsMaybeCount(undefined), 0);
var shopDaily = { source: 'airflow', file: 'shop_daily.yml', kind: 'airflow', name: 'shop_daily', ident: 'shop_daily' };
check('a task keeps its key when a task is added before it',
      jobsStepKey(shopDaily, { name: 'marts', n: 2, index: 7 }), jobsStepKey(shopDaily, { name: 'marts', n: 3, index: 8 }));
check('two tasks of a DAG, two keys',
      jobsStepKey(shopDaily, { name: 'marts', n: 2 }) !== jobsStepKey(shopDaily, { name: 'stage', n: 1 }), true);
var nightly = { source: 'cloud', file: 'jobs.yml', kind: 'dbt_cloud', name: 'Daily build', ident: 'daily' };
check('a dbt Cloud step by its place in its job',
      jobsStepKey(nightly, { n: 2 }) !== jobsStepKey(nightly, { n: 1 }), true);
check('two dbt Cloud jobs of one name in one file, by their identifier',
      jobsStepKey(nightly, { n: 1 }) !== jobsStepKey(Object.assign({}, nightly, { ident: 'daily_2' }), { n: 1 }), true);
check('the same file name in two sources',
      jobsStepKey(nightly, { n: 1 }) !== jobsStepKey(Object.assign({}, nightly, { source: 'cloud_eu' }), { n: 1 }), true);
check('the step dbt Cloud adds', jobsStepLabel({ kind: 'dbt_cloud', steps: 3 }, { n: 1, implicit: true }),
      'step 1 of 3, from run_generate_sources');

print('\n--- a cron in words ---');
[
  ['@hourly', 'Every hour'], ['@daily', 'Every day at 00:00'], ['@midnight', 'Every day at 00:00'], ['@weekly', 'Sundays at 00:00'],
  ['@monthly', 'The 1st of each month at 00:00'], ['@quarterly', 'The 1st of January, April, July and October at 00:00'], ['@yearly', 'Every 1 January at 00:00'], ['@annually', 'Every 1 January at 00:00'],
  ['0 5 * * *', 'Every day at 05:00'], ['30 6 * * 1-5', 'Weekdays at 06:30'], ['0 6 * * 0,6', 'Weekends at 06:00'], ['0 6 * * 6,0', 'Weekends at 06:00'],
  ['0 6 * * 1', 'Mondays at 06:00'], ['0 6 * * mon', 'Mondays at 06:00'], ['0 6 * * 7', 'Sundays at 06:00'],
  ['0 6 * * 5,1,3', 'Mondays, Wednesdays and Fridays at 06:00'], ['0 6 * * 0-6', 'Every day at 06:00'],
  ['0 6,18 * * *', 'Every day at 06:00 and 18:00'], ['0 18,6 * * 1-5', 'Weekdays at 06:00 and 18:00'],
  ['15 0,6,12,18 * * *', 'Every day at 00:15, 06:15, 12:15 and 18:15'],
  ['0 5 1 * *', 'The 1st of each month at 05:00'], ['0 5 2 * *', 'The 2nd of each month at 05:00'], ['0 5 3 * *', 'The 3rd of each month at 05:00'],
  ['0 5 4 * *', 'The 4th of each month at 05:00'], ['0 5 11 * *', 'The 11th of each month at 05:00'], ['0 5 21 * *', 'The 21st of each month at 05:00'],
  ['0 5 22 * *', 'The 22nd of each month at 05:00'], ['0 5 23 * *', 'The 23rd of each month at 05:00'], ['0 5 28 * *', 'The 28th of each month at 05:00'],
  ['0 * * * *', 'Every hour'], ['30 * * * *', 'Every hour at :30'], ['0 */6 * * *', 'Every 6 hours'], ['30 */2 * * *', 'Every 2 hours at :30'],
  ['*/15 * * * *', 'Every 15 minutes'], ['*/1 * * * *', 'Every minute'], ['* * * * *', 'Every minute'], ['0 5 ? * *', 'Every day at 05:00'],
  ['  0 5 * * *  ', 'Every day at 05:00'],
].forEach(function (c) { check('cron ' + c[0], jobsCronWords(c[0]), c[1]); });
[
  ['both day fields restricted', '0 5 1 * 1'], ['a month restricted', '0 5 * 6 *'], ['an L', '0 5 L * *'], ['a hash', '0 5 * * 1#2'],
  ['six fields', '0 0 5 * * *'], ['templated', '{{ var.value.cron }}'], ['@once', '@once'], ['@continuous', '@continuous'],
  ['a range of hours', '0 9-17 * * *'], ['five hours listed', '0 1,2,3,4,5 * * *'], ['an hour step that does not divide the day', '0 */5 * * *'],
  ['an hour step of 1', '0 */1 * * *'], ['a step of 24', '0 */24 * * *'], ['a minute step that does not divide the hour', '*/7 * * * *'],
  ['a minute list', '0,30 * * * *'], ['a weekday step', '0 5 * * */2'], ['a day of the month step', '0 5 */2 * *'],
  ['a minute step with an hour', '*/15 6 * * *'], ['nothing', ''], ['a minute out of range', '75 5 * * *'],
  ['an hour step on a weekday', '0 */6 * * 1'], ['the 29th, which February skips', '0 5 29 * *'], ['the 30th', '0 5 30 * *'], ['the 31st', '0 5 31 * *'],
].forEach(function (c) { check('no words for ' + c[0], jobsCronWords(c[1]), ''); });

print('\n--- how a job starts, for its card ---');
function sched(job) { return jobsSchedule(job); }
check('a cron said in words, beside the cron', JSON.stringify(sched({ kind: 'airflow', cron: '0 5 * * *', scheduled: true })),
      JSON.stringify({ start: 'scheduled', words: 'Every day at 05:00', cron: '0 5 * * * UTC', zone: 'UTC', title: 'Runs on its own' }));
check('the zone follows the cron, UTC where the source names none', sched({ kind: 'airflow', cron: '0 5 * * *', timezone: 'Europe/London', scheduled: true }).cron, '0 5 * * * Europe/London');
check('a dbt Cloud cron is read in UTC', sched({ kind: 'dbt_cloud', cron: '0 2 * * *', timezone: 'Europe/London', scheduled: true }).cron, '0 2 * * * UTC');
check('no cron, no zone', sched({ kind: 'airflow', timezone: 'Europe/London', after: 'Nightly build', scheduled: true }).cron, '');
check('a cron that is no sentence has only its cron', JSON.stringify([sched({ cron: '0 5 1 * 1', scheduled: true }).words, sched({ cron: '0 5 1 * 1', scheduled: true }).cron]),
      JSON.stringify(['', '0 5 1 * 1 UTC']));
check('after a job', sched({ kind: 'dbt_cloud', after: 'Nightly build', scheduled: true }).words, 'After the job Nightly build');
check('after a job no file defines', sched({ kind: 'dbt_cloud', after: '101', after_unread: true, schedule_unread: true }).words,
      'After the job 101, which no file read defines');
check('on an asset', sched({ kind: 'airflow', assets: ['marts_ready'], scheduled: true }).words, 'On the asset marts_ready');
check('on assets', sched({ kind: 'airflow', assets: ['a', 'b'], scheduled: true }).words, 'On the assets a, b');
check('on the one asset no file read updates', sched({ kind: 'airflow', assets: ['raw'], assets_unread: ['raw'], schedule_unread: true }).words,
      'On the asset raw, which no file read updates');
check('one of several assets', sched({ kind: 'airflow', assets: ['raw', 'marts'], assets_unread: ['raw'], schedule_unread: true }).words,
      'On the assets raw, marts; no file read updates raw');
check('a note, capitalised', sched({ kind: 'dbt_cloud', note: 'a schedule only the vars file knows', scheduled: true }).words, 'A schedule only the vars file knows');
check('several, in the order cron, job, assets, note',
      sched({ cron: '0 * * * *', after: 'a', assets: ['x'], note: 'later', scheduled: true }).words, 'Every hour · After the job a · On the asset x · Later');
check('nothing starts it', JSON.stringify(sched({ kind: 'airflow' })),
      JSON.stringify({ start: 'manual', words: 'Manual', cron: '', zone: '', title: 'Nothing starts it: run by hand' }));
check('CI is a tag, not words', sched({ kind: 'dbt_cloud', ci: true }).words, '');
check('a merge is a tag, not words', sched({ kind: 'dbt_cloud', merge: true }).words, '');
check('an asset alone is started by something, never by a clock', sched({ assets: ['a'] }).title, 'Started by something, never by a clock');
check('a CI job has a tag that says it is never started by a clock', JSON.stringify(jobsTags({ ci: true })),
      JSON.stringify([{ text: 'CI', title: 'Started on each pull request, never by a clock' }]));
check('a merge job likewise', JSON.stringify(jobsTags({ merge: true })),
      JSON.stringify([{ text: 'Merge', title: 'Started on each merge, never by a clock' }]));
check('both, CI first', jobsTags({ ci: true, merge: true }).map(function (t) { return t.text; }).join(), 'CI,Merge');
check('a job started otherwise has no tag', jobsTags({ cron: '@daily' }).length, 0);
check('the zone only follows a string that is a cron', sched({ kind: 'airflow', cron: '@continuous' }).cron, '@continuous');
check('nor a templated one', sched({ kind: 'airflow', cron: '{{ var.value.cron }}' }).cron, '{{ var.value.cron }}');
check('but a preset has it', sched({ kind: 'airflow', cron: '@daily', timezone: 'Europe/London' }).cron, '@daily Europe/London');
check('not read says why', sched({ note: 'a schedule only known when the DAG is parsed', schedule_unread: true }).title.indexOf('May start on its own'), 0);
check('the zone apart, for the cell that draws it under the line', sched({ kind: 'airflow', cron: '0 5 * * *', timezone: 'Europe/London' }).zone, 'Europe/London');
check('no zone for a string that is no cron', sched({ kind: 'airflow', cron: '{{ var.value.cron }}' }).zone, '');
check('a CI job has no words and no cron, only its tag', JSON.stringify([sched({ kind: 'dbt_cloud', ci: true }).words, sched({ kind: 'dbt_cloud', ci: true }).cron]), JSON.stringify(['', '']));
check('nor a merge job', JSON.stringify([sched({ kind: 'dbt_cloud', merge: true }).words, sched({ kind: 'dbt_cloud', merge: true }).cron]), JSON.stringify(['', '']));

print('\n--- the text of a start\'s line ---');
var daily = { kind: 'airflow', cron: '0 5 * * *', timezone: 'Europe/London', scheduled: true };
check('the words, the cron and its zone', jobsLineText(daily), 'Every day at 05:00 0 5 * * * Europe/London');
check('the cron without its zone, where the zone is drawn apart', jobsLineText(daily, { zone: false }), 'Every day at 05:00 0 5 * * *');
check('a cron with no words has only itself', jobsLineText({ kind: 'airflow', cron: '0 5 1 * 1', scheduled: true }), '0 5 1 * 1 UTC');
check('manual says so', jobsLineText({ kind: 'airflow' }), 'Manual');
check('a CI job has no line', jobsLineText({ kind: 'dbt_cloud', ci: true }), '');
check('assets, in words', jobsLineText({ kind: 'airflow', assets: ['marts_ready'], scheduled: true }), 'On the asset marts_ready');

print('\n--- a chip\'s tooltip ---');
var chipStep = { shown: 'dbt build --select orders', n: 2 };
check('the job, its start in words with the zone, what it means, the command, the click',
      jobsChipTitle({ kind: 'airflow', name: 'shop_daily', cron: '0 5 * * *', scheduled: true }, chipStep),
      'DAG shop_daily: Every day at 05:00 · 0 5 * * * UTC\nRuns on its own\ndbt build --select orders\nclick: show it in the Jobs tab');
check('a CI job names its tag', jobsChipTitle({ kind: 'dbt_cloud', name: 'PR check', ci: true }, chipStep).split('\n')[0], 'Job PR check: CI');
check('a manual job says Manual', jobsChipTitle({ kind: 'airflow', name: 'shop_adhoc' }, chipStep).split('\n').slice(0, 2).join('|'),
      'DAG shop_adhoc: Manual|Nothing starts it: run by hand');
check('not read has the start unread', sched({ schedule_unread: true, note: 'x' }).start, 'unread');

print('\n--- what ties them to others ---');
check('a job\'s environment, sensors, triggers and what starts it, on one line',
      jobsMeta({ waits_for: ['crm › load'], triggers: ['Exports'], env: '3', target: 'prod', ci: true, merge: true }),
      'environment 3 · target prod · waits for crm › load · starts Exports when it ends · on each pull request · on each merge');
check('where it runs', jobsMeta({ env: '3', target: 'prod' }), 'environment 3 · target prod');
check('a CI job', jobsMeta({ env: '3', target: 'prod', ci: true }), 'environment 3 · target prod · on each pull request');
check('a job with nothing', jobsMeta({}), '');
function why(step, name) { return jobsWhy(step, name || 'customers').map(function (seg) {
  return seg.map(function (p) { return p.chip ? '[chip]' : p.code ? '`' + p.code + '`' : p.text; }).join('');
}); }
check('a step selects the node by its name', why({ via: ['customers'] }).join(' | '), 'selects [chip] by name');
check('a step selects it through a term', why({ via: ['+tag:daily'] }).join(' | '), 'selects [chip] through `+tag:daily`');
check('by name and through others', why({ via: ['customers', 'tag:a', '+b'] }).join(' | '), 'selects [chip] by name and through `tag:a`, `+b`');
check('a selector is a term', why({ via: ['--selector nightly'] }).join(' | '), 'selects [chip] through `--selector nightly`');
check('everything, with no chip', why({ via: ['no --select: everything'] }).join(' | '), 'selects everything (no --select)');
check('a step\'s neighbours, in reading order',
      why({ via: ['customers'], upstream: ['staging'], downstream: ['notify'], waited_by: ['reports › wait'],
        outlets: ['marts_ready'], feeds: ['shop_after'], warnings: ['said apart, in the warning box'] }).join(' | '),
      'selects [chip] by name | runs after the task `staging` | runs before the task `notify` | awaited by `reports › wait` | updates `marts_ready`, which starts `shop_after`');
check('tasks, in the plural', why({ upstream: ['a', 'b'], downstream: ['c', 'd'] }).join(' | '), 'runs after the tasks `a`, `b` | runs before the tasks `c`, `d`');
check('several awaiting', why({ waited_by: ['x › a', 'y › b'] }).join(' | '), 'awaited by `x › a`, `y › b`');
check('assets updated, none starting', why({ outlets: ['a', 'b'] }).join(' | '), 'updates `a`, `b`');
check('nothing to say', jobsWhy({}, 'customers').length, 0);
check('empty lists say nothing', jobsWhy({ via: [], upstream: [] }, 'customers').length, 0);
check('the header\'s counts', jobsHeadCounts({ groups: [{}, {}], maybe: [{ steps: [{}] }, { steps: [{}, {}] }] }), '2 run it · 3 not resolved');
check('one job runs it', jobsHeadCounts({ groups: [{}], maybe: [] }), '1 runs it');
check('only steps not resolved', jobsHeadCounts({ groups: [], maybe: [{ steps: [{}] }] }), '1 not resolved');
check('neither', jobsHeadCounts({ groups: [] }), '');
check('one source', jobsSourcesText(['orchestration/dags']), 'orchestration/dags');
check('two sources', jobsSourcesText(['dags', 'jobs.yml']), 'dags and jobs.yml');
check('three sources', jobsSourcesText(['a', 'b', 'c']), 'a, b and c');
check('no source', jobsSourcesText(undefined), '');
function line(view) { var l = jobsReadLine(view); return [l.read, l.unread]; }
check('the header names the sources read', JSON.stringify(line({ sources: ['airflow', 'dbt cloud'] })), JSON.stringify(['read from airflow and dbt cloud', '']));
check('and the one not read, with the reason',
      JSON.stringify(line({ sources: ['airflow'], not_read_names: ['dbt cloud'], not_read: { sources: 1, of: 2, reason: 'nothing at this path' } })),
      JSON.stringify(['read from airflow', 'dbt cloud not read (nothing at this path)']));
check('several not read, their reasons together',
      JSON.stringify(line({ sources: ['a'], not_read_names: ['b', 'c'], not_read: { sources: 2, of: 3, reason: 'nothing at this path, stopped at its bound on files or depth' } })),
      JSON.stringify(['read from a', 'b and c not read (nothing at this path, stopped at its bound on files or depth)']));
check('none read: only the part not read', JSON.stringify(line({ not_read_names: ['x'], not_read: { sources: 1, of: 1, reason: 'nothing at this path' } })),
      JSON.stringify(['', 'x not read (nothing at this path)']));
check('all read: nothing not read', JSON.stringify(line({ sources: ['a'] })), JSON.stringify(['read from a', '']));
check('no source at all: nothing to say', JSON.stringify(line({})), JSON.stringify(['', '']));
check('two sources sharing a label are named once', jobsSourcesText(['dags', 'dags', 'jobs.yml']), 'dags and jobs.yml');

print('\n--- coverage ---');
var groups = jobsCoverageGroups([
  { folder: 'models/marts', name: 'a' }, { folder: 'models/marts', name: 'b' }, { folder: 'seeds', name: 'c' },
]);
function shape(gs) { return gs.map(function (g) { return g.folder + ':' + g.rows.map(function (r) { return r.name + (r.rest ? '@' + r.rest : ''); }).join(','); }).join(' '); }
check('rows by folder, in the order sent, a row alone under no folder', shape(groups), 'models/marts:a,b :c@seeds');
check('no rows', jobsCoverageGroups(undefined).length, 0);
// A folder per model: each would be a folder to unfold for one file, so each
// goes under the deepest folder it shares with another, and says the rest.
var perModel = jobsCoverageGroups([
  { folder: 'models/sales/orders', name: 'orders' }, { folder: 'models/sales/refunds', name: 'refunds' },
  { folder: 'models/crm/accounts', name: 'accounts' }, { folder: 'models/crm/contacts', name: 'contacts' },
  { folder: 'models/crm/contacts', name: 'contacts_v2' }, { folder: 'models/ops/shifts', name: 'shifts' },
]);
check('a folder of one is no level of its own, one of two stays one', shape(perModel),
      'models/sales:orders@orders,refunds@refunds models/crm:accounts@accounts models/crm/contacts:contacts,contacts_v2 models:shifts@ops/shifts');
check('a folder that holds two keeps them, with nothing left to say',
      shape(jobsCoverageGroups([{ folder: 'models/a', name: 'x' }, { folder: 'models/a', name: 'y' }])), 'models/a:x,y');
check('a row with no folder at all', shape(jobsCoverageGroups([{ folder: '', name: 'x' }])), ':x');
check('the server\'s order holds within a group', shape(jobsCoverageGroups([
  { folder: 'models/a/p', name: 'z' }, { folder: 'models/a/q', name: 'b' }])), 'models/a:z@p,b@q');

function checkJ(label, got, want) { check(label, JSON.stringify(got), JSON.stringify(want)); }
print('\n--- the figures over the coverage ---');
var nightly = { name: 'nightly', scheduled: true, cron: '0 6 * * *' };
var adhoc = { name: 'adhoc' };
var J = JSON.stringify;
function tiled(cov) { return jobsCoverageTiles(cov).map(function (t) { return [t.label, t.value, t.tone || '']; }); }
checkJ('covered of all by the line\'s own sum, the two kinds apart, the jobs',
      tiled({ total: 10, never: [1, 2], unscheduled: [3], jobs: [nightly, adhoc], empty: [], unresolved: [] }),
      [['In a scheduled job', '7 of 10', ''], ['Only in unscheduled jobs', '1', ''], ['In no job', '2', 'warn'], ['Jobs read', '2', '']]);
checkJ('in no job is not a warning at zero', tiled({ total: 2, never: [], unscheduled: [], jobs: [nightly], empty: [], unresolved: [] })[2], ['In no job', '0', '']);
checkJ('a schedule not read is a tile only when there is one, and comes out of the covered',
      tiled({ total: 10, never: [1], unscheduled: [2], unread: [3, 4], jobs: [nightly], empty: [], unresolved: [] }).slice(0, 4),
      [['In a scheduled job', '6 of 10', ''], ['Only in unscheduled jobs', '1', ''], ['In no job', '1', 'warn'], ['Schedule not read', '2', '']]);
checkJ('the jobs read say how many are scheduled', jobsCoverageTiles({ total: 1, never: [], unscheduled: [], jobs: [nightly, adhoc, nightly], empty: [], unresolved: [] }).filter(function (t) { return t.label === 'Jobs read'; })
  .map(function (t) { return [t.value, t.sub]; }), [['3', '2 scheduled']]);
checkJ('steps to look at: those that select nothing and those not resolved, only when there are some',
      [tiled({ total: 1, never: [], unscheduled: [], jobs: [], empty: [1], unresolved: [2, 3] }).pop(), tiled({ total: 1, never: [], unscheduled: [], jobs: [], empty: [], unresolved: [] }).length],
      [['Steps to look at', '3', ''], 4]);

print('\n--- what the server quotes ---');
checkJ('a span between backticks is code, the rest text', jobsBackticks('the selector `tag:nothing` matches no node'),
  [{ text: 'the selector ', code: false }, { text: 'tag:nothing', code: true }, { text: ' matches no node', code: false }]);
checkJ('several, one after the other', jobsBackticks('`a` and `b`').map(function (p) { return [p.text, p.code]; }), [['a', true], [' and ', false], ['b', true]]);
checkJ('none, one text', jobsBackticks('a templated command'), [{ text: 'a templated command', code: false }]);
checkJ('an unmatched backtick stays', jobsBackticks('odd ` one'), [{ text: 'odd ` one', code: false }]);
checkJ('nothing at all', jobsBackticks(''), []);

print('\n--- what the form sends ---');
var sent = jobSourcesBody([
  { kind: 'airflow', path: ' ../orchestration/dags ', label: ' prod ', keys: { dag_id: 'pipeline.name', tasks: 'tasks', command: ' ', sensor_id: 'task_id' } },
  { kind: 'dbt_cloud', path: 'jobs.yml', label: '', keys: { dag_id: 'left over' } },
], DEFAULTS);
check('keys at the proposed shape are not sent, and dbt Cloud sends none', JSON.stringify(sent),
      '{"sources":[{"kind":"airflow","path":"../orchestration/dags","label":"prod","keys":{"dag_id":"pipeline.name"}},'
      + '{"kind":"dbt_cloud","path":"jobs.yml","label":"","keys":{}}]}');

var zoned = jobSourcesBody([
  { kind: 'airflow', path: 'dags', label: '', keys: {}, timezone: ' Europe/London ' },
  { kind: 'airflow', path: 'more', label: '', keys: {}, timezone: '  ' },
  { kind: 'dbt_cloud', path: 'jobs.yml', label: '', keys: {}, timezone: 'Europe/London' },
], DEFAULTS);
check('an Airflow source sends its time zone, trimmed, and an empty one none; dbt Cloud never', JSON.stringify(zoned),
      '{"sources":[{"kind":"airflow","path":"dags","label":"","keys":{},"timezone":"Europe/London"},'
      + '{"kind":"airflow","path":"more","label":"","keys":{}},{"kind":"dbt_cloud","path":"jobs.yml","label":"","keys":{}}]}');

var placed = jobSourcesBody([{ kind: 'dbt_cloud', path: 'jobs.yml', label: '', keys: {} }], DEFAULTS, 'project', '12-345');
check('where they are kept, and the version of the file the form was shown', JSON.stringify(placed),
      '{"sources":[{"kind":"dbt_cloud","path":"jobs.yml","label":"","keys":{}}],"place":"project","seen":"12-345"}');
check('a file that was not there is seen as an empty version',
      jobSourcesBody([], DEFAULTS, 'project', '').seen, '');
check('for me only sends no version', JSON.stringify(jobSourcesBody([], DEFAULTS, 'settings', undefined)),
      '{"sources":[],"place":"settings"}');

print('\n--- what the project\'s file may hold ---');
check('a path from the project, one folder up', jobsProjectPathIssue('../orchestration/dags'), '');
check('two folders up are the most', jobsProjectPathIssue('../../orchestration/dags'), '');
check('a path inside the project', jobsProjectPathIssue('jobs/airflow'), '');
check('an absolute path', jobsProjectPathIssue('/srv/dags'), 'an absolute path');
check('a Windows drive', jobsProjectPathIssue('C:\\dags'), 'an absolute path');
check('three folders up', jobsProjectPathIssue('../../../x'), 'more than two folders above the project');
check('up and down again is still too far', jobsProjectPathIssue('a/../../../../x'), 'more than two folders above the project');
check('a hidden folder', jobsProjectPathIssue('./.hidden/x'), 'a hidden folder or file');
check('a hidden folder deep in it', jobsProjectPathIssue('../orchestration/a/.ssh/b'), 'a hidden folder or file');
check('the dbt profile', jobsProjectPathIssue('../../.dbt/profiles.yml'), 'a hidden folder or file');
check('a profiles.yml by its name', jobsProjectPathIssue('../conf/profiles.yml'), 'a profiles.yml');
check('an empty path asks nothing', jobsProjectPathIssue('  '), '');

check('Save is held while a save is on its way', jobsSaveDisabled(true, ''), true);
check('and while a path cannot go in the file', jobsSaveDisabled(false, 'an absolute path'), true);
check('and free otherwise', jobsSaveDisabled(false, ''), false);
check('a save on its way holds it whatever is typed', jobsSaveDisabled(true, 'an absolute path'), true);

print('\n--- the example ---');
check('the proposed shape', jobsExample('airflow', {}, DEFAULTS), [
  '# one file per DAG',
  'dag_id: sales_daily',
  'schedule: "0 6 * * *"',
  'tasks:',
  '  - task_id: build_sales',
  '    command: dbt run --select tag:sales',
  '    depends_on: [stage_sales]',
  'sensors:',
  '  - task_id: wait_crm',
  '    external_dag_id: crm_daily',
  '    external_task_id: crm_load',
].join('\n'));
check('a source\'s own keys, nested where a dotted path says', jobsExample('airflow',
  { dag_id: 'pipeline.name', schedule: 'pipeline.cron', tasks: 'steps', command: 'dbt' }, DEFAULTS), [
  '# one file per DAG',
  'pipeline:',
  '  name: sales_daily',
  '  cron: "0 6 * * *"',
  'steps:',
  '  - task_id: build_sales',
  '    dbt: dbt run --select tag:sales',
  '    depends_on: [stage_sales]',
  'sensors:',
  '  - task_id: wait_crm',
  '    external_dag_id: crm_daily',
  '    external_task_id: crm_load',
].join('\n'));
check('a key nested inside a task', jobsExample('airflow', { command: 'run.cmd' }, DEFAULTS).split('\n').slice(4, 8).join('\n'), [
  '  - task_id: build_sales',
  '    run:',
  '      cmd: dbt run --select tag:sales',
  '    depends_on: [stage_sales]',
].join('\n'));
check('dbt Cloud is its own schema', jobsExample('dbt_cloud', { dag_id: 'x' }, DEFAULTS).split('\n')[1], 'jobs:');

print('\n--- when a save closes the sources form ---');
var READ = { kind: 'airflow', files: 2, jobs: 2, steps: 5 };
check('counted in a DAG\'s words', jobsReadCounts(READ), '2 DAGs, 5 tasks');
check('counted in a dbt Cloud job\'s words', jobsReadCounts({ kind: 'dbt_cloud', jobs: 1, steps: 1 }), '1 job, 1 step');
check('a clean read', jobsReadClean([READ]), true);
check('a file without a DAG beside the DAGs found is still clean', jobsReadClean([Object.assign({}, READ, { files: 3, skipped: 1 })]), true);
check('no source is not clean', jobsReadClean([]), false);
check('a source the walk refused is not clean', jobsReadClean([READ, { kind: 'airflow', error: 'not a .yml or .yaml file' }]), false);
check('a refused file is not clean',
      jobsReadClean([Object.assign({}, READ, { refused: [{ file: 'a.yml', line: 3, message: 'a tab in the indent' }] })]), false);
check('a walk stopped at a bound is not clean', jobsReadClean([Object.assign({}, READ, { capped: true })]), false);
check('keys that find no job are not clean', jobsReadClean([{ kind: 'airflow', files: 2, jobs: 0, steps: 0, skipped: 2 }]), false);

var SENT = { sources: [{ kind: 'airflow', path: '../jobs', label: '', keys: { tasks: 'jobs' } }] };
var copy = function (v) { return JSON.parse(JSON.stringify(v)); };
check('a clean read that was kept closes the form', jobsSaveCloses({ saved: true, sources: [READ] }, SENT, copy(SENT)), true);
check('a read kept only until dbt-edith stops leaves it open', jobsSaveCloses({ saved: false, sources: [READ] }, SENT, copy(SENT)), false);
check('a read to correct leaves it open', jobsSaveCloses({ saved: true, sources: [Object.assign({}, READ, { capped: true })] }, SENT, copy(SENT)), false);
var edited = copy(SENT);
edited.sources.push({ kind: 'dbt_cloud', path: '', label: '', keys: {} });
check('a form edited while it read stays open', jobsSaveCloses({ saved: true, sources: [READ] }, SENT, edited), false);

check('the toast of a clean save', jobsSaveToast({ saved: true, sources: [READ, { kind: 'dbt_cloud', jobs: 1, steps: 2 }] }).join(' | '),
      'Saved and read: 2 DAGs, 5 tasks; 1 job, 2 steps. | ok');
check('the toast of a read not kept', jobsSaveToast({ saved: false, sources: [READ] }).join(' | '),
      'Read, but kept only until dbt-edith stops: 2 DAGs, 5 tasks. | err');
check('the toast of a read to correct', jobsSaveToast({ saved: true, sources: [Object.assign({}, READ, { jobs: 0 })] }).join(' | '),
      'Saved, but not every source read cleanly: Orchestration says why. | err');
check('the toast with no source left', jobsSaveToast({ saved: true, sources: [] }).join(' | '), 'Saved: no job source left. | ok');
