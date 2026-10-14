// The Jobs tab's pure parts: how a job and a step read, the coverage lists,
// the body the sources form sends, and the example it draws (0063).
// Run from the repository root: jsc web/tests/jobs.js
var app = read('web/app.js');
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
check('a dbt Cloud step on a chip', jobsChipText({ kind: 'dbt_cloud', name: 'Daily build', steps: 3 }, { n: 2 }), 'Daily build › step 2/3');
check('chips, scheduled jobs first and each job\'s steps in order',
      jobsChips([
        { job: { kind: 'airflow', name: 'jaffle_reports', scheduled: false }, steps: [{ name: 'all', n: 1 }] },
        { job: { kind: 'airflow', name: 'daily', cron: '@daily', scheduled: true }, steps: [{ name: 'stage', n: 1 }, { name: 'marts', n: 2 }] },
      ]).map(function (c) { return c.text; }).join(' | '),
      'daily › stage | daily › marts | jaffle_reports › all');
check('a dbt Cloud step by its place', jobsStepLabel({ kind: 'dbt_cloud', steps: 3 }, { n: 2 }), 'step 2/3');
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
      'step 1/3, from run_generate_sources');

print('\n--- what ties them to others ---');
check('a job\'s sensors, triggers and environment',
      jobsJobLinks({ waits_for: ['crm › load'], triggers: ['Exports'], env: '3', target: 'prod' }).join(' | '),
      'waits for crm › load | starts Exports when it ends | environment 3, target prod');
check('a job with nothing', jobsJobLinks({}).length, 0);
check('a step\'s links, in reading order',
      jobsLinkRows({ via: ['+tag:daily'], upstream: ['stage'], downstream: ['notify'], waited_by: ['reports › wait'],
        outlets: ['marts_ready'], feeds: ['shop_after'], warnings: ['said apart, in the warning colour'] }).join(' | '),
      'via +tag:daily | after stage | before notify | awaited by reports › wait | updates marts_ready, which starts shop_after');
check('an asset nothing waits on', jobsLinkRows({ outlets: ['a'] }).join(), 'updates a');

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
check('the line over the lists', jobsCoverageLine({ total: 10, never: [1, 2], unscheduled: [3] }),
      '7 of 10 models, seeds and snapshots run in a scheduled job; 1 only in unscheduled jobs, 2 in no job.');
check('all covered', jobsCoverageLine({ total: 2, never: [], unscheduled: [] }), '2 of 2 models, seeds and snapshots run in a scheduled job.');
check('a schedule not read, counted apart', jobsCoverageLine({ total: 10, never: [1], unscheduled: [2], unread: [3, 4] }),
      '6 of 10 models, seeds and snapshots run in a scheduled job; 2 only in jobs whose schedule is not read, 1 only in unscheduled jobs, 1 in no job.');

print('\n--- what the form sends ---');
var sent = jobSourcesBody([
  { kind: 'airflow', path: ' ../orchestration/dags ', label: ' prod ', keys: { dag_id: 'pipeline.name', tasks: 'tasks', command: ' ', sensor_id: 'task_id' } },
  { kind: 'dbt_cloud', path: 'jobs.yml', label: '', keys: { dag_id: 'left over' } },
], DEFAULTS);
check('keys at the proposed shape are not sent, and dbt Cloud sends none', JSON.stringify(sent),
      '{"sources":[{"kind":"airflow","path":"../orchestration/dags","label":"prod","keys":{"dag_id":"pipeline.name"}},'
      + '{"kind":"dbt_cloud","path":"jobs.yml","label":"","keys":{}}]}');

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
