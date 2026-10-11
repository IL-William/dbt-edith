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

print('\n--- how a job and a step are named ---');
check('a DAG', jobsJobLabel({ kind: 'airflow', name: 'shop_daily' }), 'DAG shop_daily');
check('a dbt Cloud job with its identifier', jobsJobLabel({ kind: 'dbt_cloud', name: 'Daily build', ident: 'daily' }), 'Job Daily build [[daily]]');
check('a task', jobsStepLabel({ kind: 'airflow' }, { name: 'marts', n: 2 }), 'task marts');
check('a task with no id', jobsStepLabel({ kind: 'airflow' }, { name: '', n: 2 }), 'task 2');
check('a dbt Cloud step by its place', jobsStepLabel({ kind: 'dbt_cloud', steps: 3 }, { n: 2 }), 'step 2/3');
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
check('rows by folder, in the order sent', groups.map(function (g) { return g.folder + ':' + g.rows.length; }).join(' '), 'models/marts:2 seeds:1');
check('no rows', jobsCoverageGroups(undefined).length, 0);
check('the line over the lists', jobsCoverageLine({ total: 10, never: [1, 2], unscheduled: [3] }),
      '7 of 10 models, seeds and snapshots run in a scheduled job; 1 only in jobs nothing schedules, 2 in none.');
check('all covered', jobsCoverageLine({ total: 2, never: [], unscheduled: [] }), '2 of 2 models, seeds and snapshots run in a scheduled job.');

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
