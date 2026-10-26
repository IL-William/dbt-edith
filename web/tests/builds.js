// The last build per environment (0117): where the dbt_artifacts tables are in
// each environment, what a model's runs add up to, and what the canvas and the
// reports make of it. Invented names throughout.
// Run from the repository root: jsc web/tests/builds.js
var src = read('web/app.js');
eval(src.slice(src.indexOf('function locationRows'), src.indexOf('function catalogLocation')));
eval(src.slice(src.indexOf('function reportsSections'), src.indexOf('function reportsNews')));
eval(src.slice(src.indexOf('function reportsDefaultSection'), src.indexOf('function reportsButtonTitle')));

function check(label, got, want) {
  var g = JSON.stringify(got), w = JSON.stringify(want);
  print((g === w ? 'PASS  ' : 'FAIL  ') + label + (g === w ? '' : '\n        expected ' + w + '\n        got      ' + g));
}

/* A package model as /api/node sends it: built into a sandbox, placed by the
   database variable of each .env file. */
function table(name, envs) {
  return {
    id: 'model.dbt_artifacts.' + name, name: name, kind: 'model', materialized: 'incremental',
    relation: 'ANALYTICS_DEV.dbt_artifacts.' + name,
    location: {
      written: { database: "{{ env_var('META_DB') }}", schema: 'dbt_artifacts', alias: '' },
      parsed: { database: 'ANALYTICS_DEV', schema: 'dbt_artifacts', alias: '' },
      built: { database: 'ANALYTICS_DEV', schema: 'dbt_artifacts', alias: name },
      envs: envs,
    },
  };
}
function placed(database) {
  return {
    place: { database: database, schema: 'dbt_artifacts', alias: '' },
    status: { database: { kind: 'env', vars: ['META_DB'] }, schema: { kind: 'literal' }, alias: { kind: 'literal' } },
  };
}
var missing = {
  place: { database: '', schema: 'dbt_artifacts', alias: '' },
  status: { database: { kind: 'missing', vars: ['META_DB'] }, schema: { kind: 'literal' }, alias: { kind: 'literal' } },
};
function pair(envs) { return [table('model_executions', envs), table('invocations', envs)]; }
var files = [
  { file: '.env.prod', name: 'PROD' }, { file: '.env.uat', name: 'UAT' }, { file: '.env.qa', name: 'QA' }, { file: '.env.ci', name: 'CI' },
];
var both = pair({ '.env.prod': placed('ANALYTICS_PROD'), '.env.uat': placed('ANALYTICS_PROD'), '.env.qa': missing });

print('--- where the tables are, per environment ---');
var targets = buildsTargets(both[0], both[1], files);
check('the manifest first, then each file, in order', targets.map(function (t) { return t.env; }), ['', '.env.prod', '.env.uat', '.env.qa', '.env.ci']);
check('the manifest\'s own build is where it built them', [targets[0].executions, targets[0].invocations, targets[0].reason],
  ['ANALYTICS_DEV.dbt_artifacts.model_executions', 'ANALYTICS_DEV.dbt_artifacts.invocations', '']);
check('a file that places them gives the relation the Location table shows', [targets[1].executions, targets[1].invocations],
  ['ANALYTICS_PROD.dbt_artifacts.model_executions', 'ANALYTICS_PROD.dbt_artifacts.invocations']);
check('a file that cannot place them says why, and asks nothing', [targets[3].executions, targets[3].reason],
  ['', 'META_DB is not defined in .env.qa']);
check('a file the package was never resolved against says so, and is not read as the built place',
  [targets[4].executions, targets[4].reason], ['', 'model_executions was not resolved against .env.ci']);
check('every environment is named as the page names it', targets.map(function (t) { return t.name; }), ['manifest', 'PROD', 'UAT', 'QA', 'CI']);
check('a manifest without the package places nothing', buildsTargets(null, null, [])[0].reason, 'model_executions is not in the manifest');
check('one table missing is enough', buildsTargets(both[0], null, [])[0].reason, 'invocations is not in the manifest');
var ephemeral = table('invocations', {});
ephemeral.relation = ''; ephemeral.materialized = 'ephemeral';
check('an ephemeral table is not in the warehouse', buildsTargets(both[0], ephemeral, [])[0].reason,
  'invocations is ephemeral, so it is not in the warehouse');

print('\n--- one query per pair of tables ---');
var queries = buildsQueries(targets);
check('two files placing them alike are asked once', queries.map(function (q) { return q.envs; }), [[''], ['.env.prod', '.env.uat']]);
check('the query names the two relations', queries[1].executions + ' | ' + queries[1].invocations,
  'ANALYTICS_PROD.dbt_artifacts.model_executions | ANALYTICS_PROD.dbt_artifacts.invocations');
check('a file with a reason is never asked', buildsQueries([targets[3]]), []);
var quoted = table('model_executions', {});
quoted.relation = '"ANALYTICS_DEV"."dbt_artifacts"."model_executions"';
check('a quoted relation keeps its quotes', buildsTargets(quoted, both[1], [])[0].executions, '"ANALYTICS_DEV"."dbt_artifacts"."model_executions"');

print('\n--- the verdict ---');
var now = Date.parse('2026-10-22T12:00:00Z');
function run(status, started, success, extra) {
  var r = { node: 'model.jaffle.orders', target: 'prod', status: status, started: started, last_success: success };
  for (var k in extra || {}) r[k] = extra[k];
  return r;
}
function verdict(rows, days) { return buildVerdict(rows, days === undefined ? 7 : days, now).verdict; }
check('no row in the window is none', verdict([]), 'none');
check('and so is no list at all', verdict(undefined), 'none');
check('a success inside the threshold is ok', verdict([run('success', '2026-10-21T03:00:00Z', '2026-10-21T03:01:00Z')]), 'ok');
check('a success older than the threshold is stale', verdict([run('success', '2026-10-10T03:00:00Z', '2026-10-10T03:01:00Z')]), 'stale');
check('exactly at the threshold is still ok', verdict([run('success', '2026-10-15T12:00:00Z', '2026-10-15T12:00:00Z')]), 'ok');
check('a failed last attempt is an error, however recent the last success', verdict([run('error', '2026-10-22T03:00:00Z', '2026-10-21T03:01:00Z')]), 'error');
check('a failed test status counts too', verdict([run('fail', '2026-10-22T03:00:00Z', '')]), 'error');
check('a runtime error counts', verdict([run('runtime error', '2026-10-22T03:00:00Z', '')]), 'error');
check('a partial success counts: some batches of a microbatch failed', verdict([run('partial success', '2026-10-22T03:00:00Z', '2026-10-21T03:01:00Z')]), 'error');
check('a skipped last attempt does not turn a recent success into anything', verdict([run('skipped', '2026-10-22T03:00:00Z', '2026-10-21T03:01:00Z')]), 'ok');
check('a skipped last attempt after an old success is stale: the last success decides',
  verdict([run('skipped', '2026-10-22T03:00:00Z', '2026-10-01T03:01:00Z')]), 'stale');
check('skipped with no success in the window is stale, not none', verdict([run('skipped', '2026-10-22T03:00:00Z', '')]), 'stale');
check('the threshold is the user\'s', [verdict([run('success', '2026-10-10T03:00:00Z', '2026-10-10T03:01:00Z')], 30),
  verdict([run('success', '2026-10-20T03:00:00Z', '2026-10-20T03:01:00Z')], 1)], ['ok', 'stale']);
var prod = run('success', '2026-10-21T03:00:00Z', '2026-10-21T03:01:00Z');
var dev = run('error', '2026-10-22T09:00:00Z', '', { target: 'dev' });
var both2 = buildVerdict([prod, dev], 7, now);
check('of several targets the newest attempt speaks', [both2.verdict, both2.last.target], ['error', 'dev']);
check('and the newest success of any target is the last success', both2.success, '2026-10-21T03:01:00Z');
check('a success row that names no last success falls back on its own end',
  buildVerdict([run('success', '2026-10-21T03:00:00Z', '', { completed: '2026-10-21T03:02:00Z' })], 7, now).success, '2026-10-21T03:02:00Z');

print('\n--- how long ago ---');
check('under a minute', buildsAge('2026-10-22T11:59:30Z', now), 'just now');
check('minutes', buildsAge('2026-10-22T11:15:00Z', now), '45min ago');
check('hours', buildsAge('2026-10-22T07:00:00Z', now), '5h ago');
check('days', buildsAge('2026-10-19T12:00:00Z', now), '3d ago');
check('nothing known says nothing', buildsAge('', now), '');
check('a time ahead of the clock is just now', buildsAge('2026-10-23T12:00:00Z', now), 'just now');

print('\n--- what a box and a row say ---');
var v = buildVerdict([run('error', '2026-10-22T03:00:00Z', '2026-10-21T03:01:00Z', { completed: '2026-10-22T03:00:30Z', target: 'prod' })], 7, now);
check('the last attempt, then the last success', buildsSaid(v, 'PROD', now), 'PROD: error 8h ago (target prod), last success 1d ago');
check('a clean one says only the attempt', buildsSaid(buildVerdict([prod], 7, now), 'PROD', now), 'PROD: success 1d ago (target prod)');
check('none says what the window is', buildsSaid(buildVerdict([], 7, now), 'PROD', now), 'PROD: no run recorded in the last 90 days');
check('a stale one names its last success', buildsSaid(buildVerdict([run('success', '2026-10-10T03:00:00Z', '2026-10-10T03:01:00Z')], 7, now), 'UAT', now),
  'UAT: success 12d ago (target prod), last success 12d ago');

print('\n--- the dots of one environment ---');
var read = { readAt: now, byEnv: {
  '': { rows: buildsByNode([run('success', '2026-10-21T03:00:00Z', '2026-10-21T03:01:00Z')]) },
  '.env.prod': { error: 'Snowflake did not answer', rows: new Map() },
} };
var dots = buildsDots(read, '', 7, 'manifest', now);
check('a model with rows has its verdict', dots.of('model.jaffle.orders').verdict, 'ok');
check('a model without any is none', dots.of('model.jaffle.nobody').verdict, 'none');
check('and says no run was recorded', dots.of('model.jaffle.nobody').title, 'manifest: no run recorded in the last 90 days');
check('a model is judged once, however often a canvas asks', dots.of('model.jaffle.orders') === dots.of('model.jaffle.orders'), true);
check('an environment read with an error paints nothing', buildsDots(read, '.env.prod', 7, 'PROD', now), null);
check('and one not read yet', buildsDots(read, '.env.uat', 7, 'UAT', now), null);
check('and no read at all', buildsDots(null, '', 7, 'manifest', now), null);
check('rows are grouped by node, one per target', Array.from(buildsByNode([prod, dev, run('success', '', '', { node: 'model.jaffle.customers' })]).keys()),
  ['model.jaffle.orders', 'model.jaffle.customers']);

print('\n--- an environment kept to its target ---');
var staging = run('error', '2026-10-22T05:00:00Z', '', { target: 'stg' });
var other = run('success', '2026-10-21T03:00:00Z', '2026-10-21T03:01:00Z', { node: 'model.jaffle.customers', target: 'stg' });
var all = buildsByNode([prod, dev, staging, other]);
var onlyDev = buildsOfTarget(all, 'dev');
check('one target is kept', Array.from(onlyDev.get('model.jaffle.orders'), function (x) { return x.target; }), ['dev']);
check('a model left with none is dropped', Array.from(onlyDev.keys()), ['model.jaffle.orders']);
check('another target keeps its own nodes', Array.from(buildsOfTarget(all, 'stg').keys()), ['model.jaffle.orders', 'model.jaffle.customers']);
check('the answer shared by several files is not changed', all.get('model.jaffle.orders').length, 3);
check('a target nobody ran under keeps nothing', buildsOfTarget(all, 'uat').size, 0);
check('and an environment kept to it reads its verdict alone', buildsWorst(onlyDev.get('model.jaffle.orders'), 7, now).parts.length, 1);
var tf = buildsTargets(both[0], both[1], [{ file: '.env.prod', name: 'PROD', target: 'prod' }, { file: '.env.uat', name: 'UAT' }]);
check('a file that sets DBT_TARGET carries it, the manifest and the others none', tf.map(function (t) { return t.target; }), ['', 'prod', '']);
check('named with the target it is kept to, when it has one', tf.map(buildsEnvLabel), ['manifest', 'PROD · target prod', 'UAT']);

print('\n--- a list line under its environment ---');
check('the name is stripped from every target\'s line, and they are joined', buildsSaidBare(buildsWorst([prod, run('success', '2026-10-01T03:00:00Z', '2026-10-01T03:01:00Z', { target: 'stg' })], 7, now), 'PROD', now),
  'success 1d ago (target prod) · success 21d ago (target stg), last success 21d ago');
check('one line is stripped too', buildsSaidBare(buildVerdict([prod], 7, now), 'PROD', now), 'success 1d ago (target prod)');

print('\n--- the lists of the Reports section ---');
function m(name, pkg) { return { id: 'model.' + pkg + '.' + name, name: name, package: pkg }; }
var models = [m('orders', 'jaffle'), m('customers', 'jaffle'), m('payments', 'jaffle'), m('stg_old', 'jaffle'), m('model_executions', 'dbt_artifacts')];
function mine(name, status, started, success) { return run(status, started, success, { node: 'model.jaffle.' + name }); }
var got = { rows: buildsByNode([
  mine('orders', 'success', '2026-10-22T03:00:00Z', '2026-10-22T03:01:00Z'),
  mine('customers', 'error', '2026-10-22T03:00:00Z', '2026-10-20T03:01:00Z'),
  mine('stg_old', 'success', '2026-09-30T03:00:00Z', '2026-09-30T03:01:00Z'),
]) };
var lists = buildsVerdicts(got, models, 'jaffle', 7, now);
check('errors, stale, not built and fine', [lists.error, lists.stale, lists.none, lists.ok].map(function (l) { return l.map(function (x) { return x.name; }); }),
  [['customers'], ['stg_old'], ['payments'], ['orders']]);
check('the project\'s own models are counted, not the package\'s', lists.total, 4);
check('with the packages, all five', buildsVerdicts(got, models, '', 7, now).total, 5);
check('an environment not read lists nothing', buildsVerdicts(null, models, 'jaffle', 7, now).total, 0);
check('and one that failed', buildsVerdicts({ error: 'no', rows: new Map() }, models, 'jaffle', 7, now).total, 0);
var older = buildsVerdicts({ rows: buildsByNode([mine('orders', 'success', '2026-10-01T03:00:00Z', '2026-10-01T03:01:00Z'),
  mine('stg_old', 'success', '2026-09-30T03:00:00Z', '2026-09-30T03:01:00Z')]) }, models, 'jaffle', 7, now);
check('the stalest comes first', older.stale.map(function (x) { return x.name; }), ['stg_old', 'orders']);

print('\n--- several targets, one box ---');
var stg = run('success', '2026-10-21T03:00:00Z', '2026-10-21T03:01:00Z', { target: 'stg' });
var worst = buildsWorst([prod, stg], 7, now);
check('all fine is ok', worst.verdict, 'ok');
var stale = run('success', '2026-10-01T03:00:00Z', '2026-10-01T03:01:00Z', { target: 'stg' });
check('one stale target makes the model stale', buildsWorst([prod, stale], 7, now).verdict, 'stale');
check('an error outranks stale, whichever target is newer', buildsWorst([stale, run('error', '2026-09-01T03:00:00Z', '', { target: 'dev' }), prod], 7, now).verdict, 'error');
check('the worst target speaks for the model', buildsWorst([prod, stale], 7, now).last.target, 'stg');
check('the targets are kept by name', buildsWorst([prod, stale], 7, now).parts.map(function (p) { return p.last.target; }), ['prod', 'stg']);
check('the tooltip lists each', buildsSaid(buildsWorst([prod, stale], 7, now), 'PROD', now),
  'PROD: success 1d ago (target prod)\nPROD: success 21d ago (target stg), last success 21d ago');
check('one target says one line', buildsSaid(buildsWorst([prod], 7, now), 'PROD', now), 'PROD: success 1d ago (target prod)');
check('none stays none', buildsWorst([], 7, now).verdict, 'none');
var multi = buildsDots({ byEnv: { '': { rows: buildsByNode([prod, run('error', '2026-10-22T09:00:00Z', '', { target: 'dev' })]) } } }, '', 7, 'manifest', now);
check('the dot of a model with a failing target is an error', multi.of('model.jaffle.orders').verdict, 'error');
check('and its title says both', multi.of('model.jaffle.orders').title.split('\n').length, 2);
var counted2 = buildsVerdicts({ rows: buildsByNode([mine('orders', 'success', '2026-10-21T03:00:00Z', '2026-10-21T03:01:00Z'),
  run('error', '2026-10-22T03:00:00Z', '', { node: 'model.jaffle.orders', target: 'dev' })]) }, models, 'jaffle', 7, now);
check('the lists count the worst target too', counted2.error.map(function (x) { return x.name; }), ['orders']);

print('\n--- the section is listed from the index ---');
function sec(list) { return list.map(function (s) { return s.key + (s.empty ? ':empty' : '') + (s.quiet ? ':quiet' : ''); }); }
check('with the flag, last of the sections, and quiet: a click\'s result is no file\'s news',
  sec(reportsSections({ builds: true })), ['jobs:empty', 'builds:quiet']);
check('it never lights the button', reportsSections({ builds: true }).filter(lights).map(function (s) { return s.key; }), []);
check('in Snowflake\'s colour', reportsSections({ builds: true })[1].tool, 'snowflake');
check('without the flag, not at all', sec(reportsSections({})), ['jobs:empty']);
check('after the macros and the checks', reportsSections({ builds: true, checks: { count: 2, at: 3 }, macros: { unexplained: 1, at: 3 } }).map(function (s) { return s.key; }),
  ['jobs', 'macros', 'checks', 'builds']);
check('it does not open the panel in place of the section that is there',
  reportsDefaultSection(reportsSections({ builds: true, macros: { unexplained: 1, at: 3 } }), [], ''), 'macros');
check('and with nothing else, the coverage stays the opening one', reportsDefaultSection(reportsSections({ builds: true }), [], ''), 'jobs');
