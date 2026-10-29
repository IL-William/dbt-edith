// Builds by job (0119): a cron read in the zone of its source, the last start
// and the one before it, and how a job's models stand against them. Invented
// names throughout.
// Run from the repository root: jsc web/tests/jobbuilds.js
var src = read('web/app.js');
eval(src.slice(src.indexOf('function locationRows'), src.indexOf('function catalogLocation')));
eval(src.slice(src.indexOf('function reportsSections'), src.indexOf('function reportsNews')));
eval(src.slice(src.indexOf('function jobsZoneIssue'), src.indexOf('function jobsProjectPathIssue')));
eval(src.slice(src.indexOf('function jobBuildsPick'), src.indexOf('function jobBuildsEnv')));

function check(label, got, want) {
  var g = JSON.stringify(got), w = JSON.stringify(want);
  print((g === w ? 'PASS  ' : 'FAIL  ') + label + (g === w ? '' : '\n        expected ' + w + '\n        got      ' + g));
}
function iso(ms) { return ms === null ? null : new Date(ms).toISOString().slice(0, 16) + 'Z'; }
function at(text) { return Date.parse(text); }
function prev(expr, tz, before) { return iso(cronPrevious(cronFields(expr), tz, at(before))); }

print('--- which crons are read ---');
function sets(expr) {
  var f = cronFields(expr);
  if (!f) return null;
  var arr = function (x) { return Array.from(x).sort(function (a, b) { return a - b; }); };
  return { m: arr(f.minute), h: arr(f.hour), d: arr(f.dom), mo: arr(f.month), w: arr(f.dow), da: f.domAny, wa: f.dowAny };
}
check('a plain daily one', sets('0 5 * * *').m.concat(sets('0 5 * * *').h), [0, 5]);
check('a step over the whole range', sets('*/20 * * * *').m, [0, 20, 40]);
check('a list and a range', sets('0 6,18 * * 1-3').h.concat(sets('0 6,18 * * 1-3').w), [6, 18, 1, 2, 3]);
check('a step from a start reaches the end of the range', sets('10/20 * * * *').m, [10, 30, 50]);
check('a range with a step', sets('0-30/10 * * * *').m, [0, 10, 20, 30]);
check('month and weekday names, in any case', [sets('0 0 * JAN,mar MON-wed').mo, sets('0 0 * JAN,mar MON-wed').w], [[1, 3], [1, 2, 3]]);
check('Sunday is 7 as well as 0', [sets('0 0 * * 7').w, sets('0 0 * * 5-7').w], [[0], [0, 5, 6]]);
check('a weekday step ends at 6, Sunday being 7 only where it is written', [sets('0 0 * * 1/2').w, sets('0 0 * * */3').w, sets('0 0 * * 5-7').w],
  [[1, 3, 5], [0, 3, 6], [0, 5, 6]]);
check('a day from the form of a question mark is any', [sets('0 0 ? * *').da, sets('0 0 * * ?').wa], [true, true]);
check('a cron whose hour is not every hour is placed on the wall clock (Airflow 2.7 and later)', ['0 5 * * *', '30 1 * * 1', '*/15 5 * * *', '0 5,6 * * *', '0 */2 * * *', '0 0-22 * * *'].map(function (e) { return cronFields(e).fixed; }),
  [true, true, true, true, true, true]);
check('only one that runs every hour counts elapsed time', ['0 * * * *', '*/15 * * * *', '@hourly', '5 */1 * * *', '0 0-23 * * *'].map(function (e) { return cronFields(e).fixed; }),
  [false, false, false, false, false]);
function fires(expr, from, n) {
  var f = cronFields(expr), out = [], t = at(from);
  for (var i = 0; i < n; i++) {
    var p = cronPrevious(f, 'UTC', t);
    if (p === null) { out.push(null); break; }
    out.push(['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'][new Date(p).getUTCDay()] + ' ' + iso(p).slice(8, 10));
    t = p - 1;
  }
  return out;
}
// 2026-10-27 is a Tuesday. What croniter fires for a step in each day field.
check('every other day of the month is the odd days: a step restricts', fires('0 0 */2 * *', '2026-10-27T12:00:00Z', 4), ['Tue 27', 'Sun 25', 'Fri 23', 'Wed 21']);
check('every other weekday is Sunday, Tuesday, Thursday, Saturday', fires('0 0 * * */2', '2026-10-27T12:00:00Z', 5), ['Tue 27', 'Sun 25', 'Sat 24', 'Thu 22', 'Tue 20']);
check('both restricted match either: the odd days or a Monday', fires('0 0 */2 * 1', '2026-10-27T12:00:00Z', 5), ['Tue 27', 'Mon 26', 'Sun 25', 'Fri 23', 'Wed 21']);
check('a day of the month alone when the weekday is any', fires('0 0 1 * */1', '2026-10-27T12:00:00Z', 2).length, 2);
check('the whole range of days beside a weekday with a star is the weekday alone', fires('0 0 1-31 * */3', '2026-10-27T12:00:00Z', 3), ['Sun 25', 'Sat 24', 'Wed 21']);
check('the whole range of days beside a plain weekday is every day', fires('0 0 1-31 * 1', '2026-10-27T12:00:00Z', 3), ['Tue 27', 'Mon 26', 'Sun 25']);
check('the presets', [sets('@hourly').m.concat(sets('@hourly').h.length), sets('@daily').h, sets('@midnight').h, sets('@weekly').w,
  sets('@monthly').d, sets('@quarterly').mo, sets('@yearly').mo, sets('@annually').d], [[0, 24], [0], [0], [0], [1], [1, 4, 7, 10], [1], [1]]);
check('a preset in capitals', sets('@DAILY') !== null, true);
['0 0 L * *', '0 0 15W * *', '0 0 * * 1#2', '0 ? * * *', '* * * * * *', '* * * *', '@once', '@continuous', '@reboot', '',
  '61 * * * *', '* 24 * * *', '0 0 0 * *', '0 0 * 13 *', '0 0 * * 8', '*/0 * * * *', '5-2 * * * *', 'a b c d e', '0 0 * * mon-xyz'].forEach(function (expr) {
  check('not read: ' + JSON.stringify(expr), cronFields(expr), null);
});
check('nothing at all', cronFields(undefined), null);

print('\n--- the last fire ---');
check('UTC: earlier today', prev('0 5 * * *', 'UTC', '2026-10-26T12:00:00Z'), '2026-10-26T05:00Z');
check('UTC: before today\'s, yesterday\'s', prev('0 5 * * *', 'UTC', '2026-10-26T04:59:00Z'), '2026-10-25T05:00Z');
check('at the instant counts', prev('0 5 * * *', 'UTC', '2026-10-26T05:00:00Z'), '2026-10-26T05:00Z');
check('and a second before does not', prev('0 5 * * *', 'UTC', '2026-10-26T04:59:59Z'), '2026-10-25T05:00Z');
check('an empty zone is UTC', iso(cronPrevious(cronFields('0 5 * * *'), '', at('2026-10-26T12:00:00Z'))), '2026-10-26T05:00Z');
check('every quarter hour', prev('*/15 * * * *', 'UTC', '2026-10-26T12:07:30Z'), '2026-10-26T12:00Z');
check('hourly, over midnight', prev('@hourly', 'UTC', '2026-10-26T00:00:00Z'), '2026-10-26T00:00Z');
check('a Monday', prev('0 9 * * 1', 'UTC', '2026-10-26T08:00:00Z'), '2026-10-19T09:00Z');
check('a Sunday written 7', prev('0 9 * * 7', 'UTC', '2026-10-26T12:00:00Z'), '2026-10-25T09:00Z');
check('weekdays only', prev('0 6 * * mon-fri', 'UTC', '2026-10-24T12:00:00Z'), '2026-10-23T06:00Z');
check('the 13th or a Friday, whichever is later', prev('0 0 13 * 5', 'UTC', '2026-10-26T12:00:00Z'), '2026-10-23T00:00Z');
check('the 13th alone', prev('0 0 13 * *', 'UTC', '2026-10-26T12:00:00Z'), '2026-10-13T00:00Z');
check('a month that has not begun', prev('0 0 1 */3 *', 'UTC', '2026-10-26T12:00:00Z'), '2026-10-01T00:00Z');
check('a day that is not in every year', prev('0 0 29 2 *', 'UTC', '2026-10-26T12:00:00Z'), '2024-02-29T00:00Z');
check('a cron that never fires has no last one', prev('0 0 31 2 *', 'UTC', '2026-10-26T12:00:00Z'), null);
check('a zone this browser does not know', prev('0 5 * * *', 'Europe/Lundon', '2026-10-26T12:00:00Z'), null);
check('no fields, no fire', cronPrevious(null, 'UTC', 0), null);

print('\n--- in the zone of the source, across the clocks changing ---');
check('London in summer, 05:00 is 04:00Z', prev('0 5 * * *', 'Europe/London', '2026-10-24T12:00:00Z'), '2026-10-24T04:00Z');
check('London in summer, before it, the day before', prev('0 5 * * *', 'Europe/London', '2026-10-24T03:00:00Z'), '2026-10-23T04:00Z');
check('the day the clocks go back, 05:00 is 05:00Z', prev('0 5 * * *', 'Europe/London', '2026-10-25T12:00:00Z'), '2026-10-25T05:00Z');
check('the morning after, in winter', prev('0 5 * * *', 'Europe/London', '2026-10-26T12:00:00Z'), '2026-10-26T05:00Z');
check('across the change, the last fire before 05:00Z on the 25th was in summer time', prev('0 5 * * *', 'Europe/London', '2026-10-25T04:59:00Z'), '2026-10-24T04:00Z');
// A fixed time of day is placed on the wall clock, as Airflow places it.
check('a fixed time that happens twice fires once, at the later instant', prev('30 1 * * *', 'Europe/London', '2026-10-25T12:00:00Z'), '2026-10-25T01:30Z');
check('and not at the first while only that is past', prev('30 1 * * *', 'Europe/London', '2026-10-25T01:00:00Z'), '2026-10-24T00:30Z');
check('a fixed time the clocks skip fires shifted forward by the gap: 02:30 summer time', prev('30 1 * * *', 'Europe/London', '2026-03-29T12:00:00Z'), '2026-03-29T01:30Z');
check('and before that moment, the day before', prev('30 1 * * *', 'Europe/London', '2026-03-29T01:00:00Z'), '2026-03-28T01:30Z');
// Any other counts in elapsed time.
check('hourly counts hours: the skipped one is not fired', [prev('0 * * * *', 'Europe/London', '2026-03-29T01:30:00Z'), prev('0 * * * *', 'Europe/London', '2026-03-29T00:30:00Z')],
  ['2026-03-29T01:00Z', '2026-03-29T00:00Z']);
check('and the repeated hour is fired at both of its instants', [prev('0 * * * *', 'Europe/London', '2026-10-25T01:30:00Z'), prev('0 * * * *', 'Europe/London', '2026-10-25T00:59:00Z')],
  ['2026-10-25T01:00Z', '2026-10-25T00:00Z']);
check('every quarter hour does the same', prev('*/15 * * * *', 'Europe/London', '2026-10-25T01:20:00Z'), '2026-10-25T01:15Z');
check('two times a day are placed on the wall clock too: one 01:30 fire the night the clocks go back', [prev('30 1,13 * * *', 'Europe/London', '2026-10-25T12:00:00Z'),
  prev('30 1,13 * * *', 'Europe/London', '2026-10-25T01:00:00Z')], ['2026-10-25T01:30Z', '2026-10-24T12:30Z']);
check('and shifted by the gap the night they go forward', prev('30 1,13 * * *', 'Europe/London', '2026-03-29T12:00:00Z'), '2026-03-29T01:30Z');
check('Paris is an hour ahead of London in winter', prev('0 5 * * *', 'Europe/Paris', '2026-12-10T12:00:00Z'), '2026-12-10T04:00Z');
check('a zone behind UTC, the fire is on the day before in UTC', prev('0 22 * * *', 'America/New_York', '2026-12-10T12:00:00Z'), '2026-12-10T03:00Z');

print('\n--- a job\'s own clock ---');
var daily = { kind: 'airflow', scheduled: true, cron: '0 5 * * *', timezone: 'Europe/London' };
check('a cron in the zone of its source', jobClock(daily).tz + ' | ' + jobClock(daily).said, 'Europe/London | 0 5 * * * Europe/London');
check('UTC when the source names none', jobClock({ kind: 'airflow', scheduled: true, cron: '@daily' }).tz, 'UTC');
check('dbt Cloud\'s are UTC whatever else is on the job', jobClock({ kind: 'dbt_cloud', scheduled: true, cron: '0 2 * * *', timezone: 'Europe/London' }).tz, 'UTC');
check('no cron, no clock: manual, CI, an asset, another job', [{ kind: 'airflow' }, { kind: 'dbt_cloud', ci: true }, { kind: 'airflow', assets: ['a'] },
  { kind: 'dbt_cloud', after: 'nightly' }].map(jobClock), [null, null, null, null]);
check('a cron this cannot walk', jobClock({ kind: 'airflow', scheduled: true, cron: '0 0 L * *' }), null);
check('a start no file reads has no clock, whatever cron it carries', [jobClock({ kind: 'dbt_cloud', cron: '0 6 * * *', schedule_unread: true }),
  jobClock({ kind: 'dbt_cloud', cron: '0 6 * * *' })], [null, null]);
check('a zone this browser does not know', jobClock({ kind: 'airflow', scheduled: true, cron: '0 5 * * *', timezone: 'Mars/Olympus' }), null);
check('and why, for each', [jobClockWhy({ kind: 'airflow', scheduled: true, cron: '0 0 L * *' }), jobClockWhy({ kind: 'airflow', scheduled: true, cron: '0 5 * * *', timezone: 'Mars/Olympus' }),
  jobClockWhy(daily), jobClockWhy({ kind: 'airflow' }), jobClockWhy({ kind: 'dbt_cloud', cron: '0 6 * * *', schedule_unread: true }),
  jobClockWhy({ kind: 'dbt_cloud', cron: '0 6 * * *' })], ['0 0 L * * is a cron this cannot read.', 'Mars/Olympus is not a time zone this browser knows.', '', '',
  'Whether it runs on its cron is not read.', 'Nothing starts it on its cron.']);
check('a time in the zone of a job', zoneStamp(at('2026-10-24T04:00:00Z'), 'Europe/London'), '2026-10-24 05:00 Europe/London');
check('and in UTC without one', zoneStamp(at('2026-10-24T04:00:00Z'), ''), '2026-10-24 04:00 UTC');
check('the form refuses a zone the browser does not know', [jobsZoneIssue({ kind: 'airflow', timezone: 'Europe/Lundon' }),
  jobsZoneIssue({ kind: 'airflow', timezone: ' Europe/London ' }), jobsZoneIssue({ kind: 'airflow', timezone: '' }),
  jobsZoneIssue({ kind: 'dbt_cloud', timezone: 'Europe/Lundon' })], ['Europe/Lundon is not a time zone this browser knows: write it as Europe/London does.', '', '', '']);

print('\n--- how a job\'s models stand ---');
var now = at('2026-10-26T12:00:00Z'); // the last start of a 05:00 UTC job is today's, the one before yesterday's
function row(node, status, started, success, extra) {
  var r = { node: node, target: 'prod', status: status, started: started, last_success: success };
  for (var k in extra || {}) r[k] = extra[k];
  return r;
}
function rowsOf(list) { return { rows: buildsByNode(list) }; }
function mod(name, steps) { return { id: 'model.jaffle.' + name, steps: steps || [1] }; }
var utc = jobClock({ kind: 'airflow', scheduled: true, cron: '0 5 * * *' });
var job = { kind: 'airflow', scheduled: true, cron: '0 5 * * *', models: [mod('built_today'), mod('built_yesterday'), mod('missed_a_run'), mod('failed'), mod('never', [2])] };
var got = rowsOf([
  row('model.jaffle.built_today', 'success', '2026-10-26T05:00:00Z', '2026-10-26T05:10:00Z'),
  row('model.jaffle.built_yesterday', 'success', '2026-10-25T05:00:00Z', '2026-10-25T05:10:00Z'),
  row('model.jaffle.missed_a_run', 'success', '2026-10-24T05:00:00Z', '2026-10-24T05:10:00Z'),
  row('model.jaffle.failed', 'error', '2026-10-26T05:00:00Z', '2026-10-25T05:10:00Z', { message: 'Database Error: relation not found' }),
]);
var res = jobBuildVerdicts(job, got, utc, 7, now);
function by(r, id) { return r.items.find(function (i) { return i.id === 'model.jaffle.' + id; }); }
check('built since the last start is ok, and counted', [by(res, 'built_today').verdict, by(res, 'built_today').since], ['ok', true]);
check('built at the run before is ok too: the run may still be going', [by(res, 'built_yesterday').verdict, by(res, 'built_yesterday').since], ['ok', false]);
check('no success since the run before the last is late', by(res, 'missed_a_run').verdict, 'late');
check('a failed last attempt is an error, whatever the success before', by(res, 'failed').verdict, 'error');
check('and carries dbt\'s message', by(res, 'failed').worst.last.message, 'Database Error: relation not found');
check('no row in the window is not built', by(res, 'never').verdict, 'none');
check('the steps that run it are kept', by(res, 'never').steps, [2]);
check('counts', res.counts, { error: 1, late: 1, none: 1, ok: 2, since: 1, total: 5 });
check('the job\'s tone is its worst', res.tone, 'error');
check('judged by the clock, against the last start and the one before', [res.basis, iso(res.last), iso(res.before)], ['clock', '2026-10-26T05:00Z', '2026-10-25T05:00Z']);
check('worst first, then by id', res.items.map(function (i) { return i.id.split('.')[2]; }), ['failed', 'missed_a_run', 'never', 'built_today', 'built_yesterday']);
function toneOf(models, rows) { return jobBuildVerdicts({ kind: 'airflow', scheduled: true, cron: '0 5 * * *', models: models }, rowsOf(rows), utc, 7, now).tone; }
var fine = row('model.jaffle.built_today', 'success', '2026-10-26T05:00:00Z', '2026-10-26T05:10:00Z');
var later = row('model.jaffle.built_yesterday', 'success', '2026-10-25T05:00:00Z', '2026-10-25T05:10:00Z');
var missed = row('model.jaffle.missed_a_run', 'success', '2026-10-24T05:00:00Z', '2026-10-24T05:10:00Z');
check('tones: all built', toneOf([mod('built_today'), mod('built_yesterday')], [fine, later]), 'ok');
check('late outranks not built', toneOf([mod('missed_a_run'), mod('never')], [missed]), 'late');
check('not built outranks ok', toneOf([mod('built_today'), mod('never')], [fine]), 'none');
check('a job with no model is ok and empty', jobBuildVerdicts({ kind: 'airflow', scheduled: true, cron: '0 5 * * *', models: [] }, got, utc, 7, now).counts.total, 0);
check('an environment that has no rows for a model reads it not built', jobBuildVerdicts(job, { rows: new Map() }, utc, 7, now).counts.none, 5);

print('\n--- a success the same minute as the start ---');
var edge = jobBuildVerdicts({ kind: 'airflow', scheduled: true, cron: '0 5 * * *', models: [mod('a')] },
  rowsOf([row('model.jaffle.a', 'success', '2026-10-25T05:00:00Z', '2026-10-25T05:00:00Z')]), utc, 7, now);
check('at the earlier start exactly is not late', edge.items[0].verdict, 'ok');
var short = jobBuildVerdicts({ kind: 'airflow', scheduled: true, cron: '0 5 * * *', models: [mod('a')] },
  rowsOf([row('model.jaffle.a', 'success', '2026-10-25T04:59:00Z', '2026-10-25T04:59:00Z')]), utc, 7, now);
check('a minute before it is', short.items[0].verdict, 'late');

print('\n--- the same success, in two zones ---');
var london = jobClock({ kind: 'airflow', scheduled: true, cron: '0 5 * * *', timezone: 'Europe/London' });
var winterNow = at('2026-10-24T12:00:00Z'); // still summer time: 05:00 is 04:00Z
var early = row('model.jaffle.a', 'success', '2026-10-24T04:00:00Z', '2026-10-24T04:05:00Z');
var inLondon = jobBuildVerdicts({ kind: 'airflow', scheduled: true, cron: '0 5 * * *', models: [mod('a')] }, rowsOf([early]), london, 7, winterNow);
var inUtc = jobBuildVerdicts({ kind: 'airflow', scheduled: true, cron: '0 5 * * *', models: [mod('a')] }, rowsOf([early]), utc, 7, winterNow);
check('built after London\'s start of the day', [inLondon.items[0].since, iso(inLondon.last)], [true, '2026-10-24T04:00Z']);
check('but before the UTC one', [inUtc.items[0].since, iso(inUtc.last)], [false, '2026-10-24T05:00Z']);

print('\n--- a job with no clock of its own ---');
var manual = { kind: 'airflow', models: [mod('a'), mod('b'), mod('c')] };
var rows3 = rowsOf([row('model.jaffle.a', 'success', '2026-10-25T05:00:00Z', '2026-10-25T05:10:00Z'),
  row('model.jaffle.b', 'success', '2026-10-01T05:00:00Z', '2026-10-01T05:10:00Z')]);
var thr = jobBuildVerdicts(manual, rows3, null, 7, now);
check('late is no success within the days', [by(thr, 'a').verdict, by(thr, 'b').verdict, by(thr, 'c').verdict], ['ok', 'late', 'none']);
check('and nothing is counted as built since a start', [thr.basis, thr.last, thr.counts.since], ['threshold', null, 0]);
check('a threshold of 30 days forgives the older one', by(jobBuildVerdicts(manual, rows3, null, 30, now), 'b').verdict, 'ok');
var never = { kind: 'airflow', scheduled: true, cron: '0 0 31 2 *', models: [mod('a')] };
check('a cron that never fires has no start to hold a run to, so the threshold decides',
  [jobClock(never) !== null, jobBuildVerdicts(never, rows3, jobClock(never), 7, now).basis], [true, 'threshold']);

print('\n--- a job that names a target ---');
var dev = row('model.jaffle.a', 'error', '2026-10-26T06:00:00Z', '', { target: 'dev' });
var prod = row('model.jaffle.a', 'success', '2026-10-26T05:00:00Z', '2026-10-26T05:10:00Z');
var both = rowsOf([prod, dev]);
check('every target, worst wins', jobBuildVerdicts({ kind: 'dbt_cloud', scheduled: true, cron: '0 5 * * *', models: [mod('a')] }, both, utc, 7, now).items[0].verdict, 'error');
check('only the target\'s own runs when it names one', jobBuildVerdicts({ kind: 'dbt_cloud', scheduled: true, cron: '0 5 * * *', target: 'prod', models: [mod('a')] }, both, utc, 7, now).items[0].verdict, 'ok');
check('a target nobody ran under is not built', jobBuildVerdicts({ kind: 'dbt_cloud', scheduled: true, cron: '0 5 * * *', target: 'qa', models: [mod('a')] }, both, utc, 7, now).items[0].verdict, 'none');

print('\n--- an environment kept to another target ---');
var kept = { rows: buildsByNode([prod]), target: 'prod', others: ['dev'] };
var clash = jobBuildVerdicts({ kind: 'dbt_cloud', scheduled: true, cron: '0 5 * * *', target: 'qa', models: [mod('a'), mod('b')] }, kept, utc, 7, now);
check('says so and gives no verdict, instead of every model not built', [clash.clash, clash.counts.total, clash.counts.none, clash.items.length],
  ['runs under target qa; this environment keeps prod', 0, 0, 0]);
check('the same target is judged', jobBuildVerdicts({ kind: 'dbt_cloud', scheduled: true, cron: '0 5 * * *', target: 'prod', models: [mod('a')] }, kept, utc, 7, now).items[0].verdict, 'ok');
check('a job naming none is judged on what the environment kept', jobBuildVerdicts({ kind: 'dbt_cloud', scheduled: true, cron: '0 5 * * *', models: [mod('a')] }, kept, utc, 7, now).clash, '');
check('a target only the vars file knows names none: no filter, no clash', [jobTarget({ target: '{{ env }}' }), jobTarget({ target: '{% if x %}a{% endif %}' }), jobTarget({ target: 'prod' }), jobTarget({})],
  ['', '', 'prod', '']);
check('so a templated job target judges every run and clashes with no environment', [jobBuildsTargetClash({ target: '{{ env }}' }, kept),
  jobBuildVerdicts({ kind: 'dbt_cloud', scheduled: true, cron: '0 5 * * *', target: '{{ env }}', models: [mod('a')] }, kept, utc, 7, now).items[0].verdict], ['', 'ok']);
check('an environment naming none is judged on the job\'s target', jobBuildsTargetClash({ target: 'qa' }, { rows: new Map() }), '');

print('\n--- the last two starts, once ---');
var clock = jobClock({ kind: 'airflow', scheduled: true, cron: '0 5 * * *' });
var once = jobFires(clock, now);
check('worked out and kept on the clock', [once === clock, iso(clock.last), iso(clock.before)], [true, '2026-10-26T05:00Z', '2026-10-25T05:00Z']);
clock.last = 1;
check('not worked out again for the same instant', jobFires(clock, now).last, 1);
check('but for another', iso(jobFires(clock, now + 86400000).last), '2026-10-27T05:00Z');
check('a job with no clock has none', jobFires(null, now), null);
check('a cron is parsed once', cronFields('0 5 * * *') === cronFields('0 5 * * *'), true);

print('\n--- built since the start, by every target ---');
var jobNone = { kind: 'airflow', scheduled: true, cron: '0 5 * * *', models: [mod('a')] };
var viaProd = row('model.jaffle.a', 'success', '2026-10-25T05:00:00Z', '2026-10-25T05:10:00Z', { target: 'prod' });
var viaDev = row('model.jaffle.a', 'success', '2026-10-26T09:00:00Z', '2026-10-26T09:05:00Z', { target: 'dev' });
var mixed = jobBuildVerdicts(jobNone, rowsOf([viaProd, viaDev]), utc, 7, now);
check('another target\'s build is not counted since the start, and the verdict is as before', [mixed.counts.since, mixed.items[0].since, mixed.items[0].verdict], [0, false, 'ok']);
var prodToday = row('model.jaffle.a', 'success', '2026-10-26T05:00:00Z', '2026-10-26T05:10:00Z', { target: 'prod' });
check('both targets built after it count', jobBuildVerdicts(jobNone, rowsOf([prodToday, viaDev]), utc, 7, now).counts.since, 1);
check('a job naming the target counts that target alone', jobBuildVerdicts(Object.assign({ target: 'dev' }, jobNone), rowsOf([viaProd, viaDev]), utc, 7, now).counts.since, 1);

print('\n--- a model judged once for every job that runs it ---');
var shared = new Map();
var j1 = { kind: 'airflow', scheduled: true, cron: '0 5 * * *', models: [mod('a')] }, j2 = { kind: 'airflow', scheduled: true, cron: '0 5 * * *', models: [mod('a')], target: 'prod' };
var r1 = jobBuildVerdicts(j1, kept, utc, 7, now, shared), r2 = jobBuildVerdicts(j2, kept, utc, 7, now, shared), r3 = jobBuildVerdicts(j1, kept, utc, 7, now, shared);
check('one entry per model and target used', Array.from(shared.keys()).map(function (k) { return k.replace('\u0000', '|'); }), ['|model.jaffle.a', 'prod|model.jaffle.a']);
check('and the same judgement shared', [r1.items[0].v === r3.items[0].v, r1.items[0].v === r2.items[0].v], [true, false]);
check('which gives the answer it gave unshared', [r2.items[0].verdict, r2.counts.since], [jobBuildVerdicts(j2, kept, utc, 7, now).items[0].verdict, 1]);

print('\n--- the start, in words ---');
function sinceOf(cron, tz, nowText) {
  var c = jobClock({ kind: 'airflow', scheduled: true, cron: cron, timezone: tz });
  var r = jobBuildVerdicts({ kind: 'airflow', models: [mod('a')] }, rowsOf([]), c, 7, at(nowText));
  r.counts.since = 2;
  r.counts.total = 5;
  return jobSinceText(r, c.tz);
}
check('a daily job names the time', sinceOf('0 5 * * *', 'Europe/London', '2026-10-26T12:00:00Z'), '2 of 5 since 05:00');
check('an hourly one too', sinceOf('0 * * * *', 'UTC', '2026-10-26T12:30:00Z'), '2 of 5 since 12:00');
check('a clock change in the day is still a daily job', sinceOf('0 5 * * *', 'Europe/London', '2026-10-26T12:00:00Z').indexOf('Mon'), -1);
check('a weekly one names the day', sinceOf('0 9 * * 1', 'Europe/London', '2026-10-28T12:00:00Z'), '2 of 5 since Mon 26 Oct 09:00');
check('a monthly one too', sinceOf('0 6 1 * *', 'UTC', '2026-10-28T12:00:00Z'), '2 of 5 since Thu 1 Oct 06:00');
check('a weekday job over a weekend', sinceOf('0 6 * * mon-fri', 'UTC', '2026-10-25T12:00:00Z'), '2 of 5 since Fri 23 Oct 06:00');

print('\n--- jobs that share a name ---');
function jb(name, source, ident, file, line) { return { name: name, source: source, ident: ident || '', file: file || name + '.yml', line: line || 1 }; }
var alone = jb('shop_daily', 'orchestration'), tw1 = jb('Daily build', 'cloud-a', 'daily'), tw2 = jb('Daily build', 'cloud-b', 'daily');
var tags = jobsTwinTags([alone, tw1, tw2]);
check('a name that is its own has no tag', tags.has(alone), false);
check('twins in two sources are told apart by the source', [tags.get(tw1), tags.get(tw2)], ['cloud-a', 'cloud-b']);
var s1 = jb('Daily build', 'cloud', 'daily'), s2 = jb('Daily build', 'cloud', 'daily_eu'), d1 = jb('shop', 'dags', '', 'a/shop.yml', 3), d2 = jb('shop', 'dags', '', 'b/shop.yml', 3);
var tags2 = jobsTwinTags([s1, s2, d1, d2]);
check('in one source, by the identifier the Jobs tab writes, or by the file', [tags2.get(s1), tags2.get(s2), tags2.get(d1), tags2.get(d2)],
  ['cloud · [[daily]]', 'cloud · [[daily_eu]]', 'dags · a/shop.yml', 'dags · b/shop.yml']);
var l1 = jb('shop', 'dags', '', 'shop.yml', 1), l2 = jb('shop', 'dags', '', 'shop.yml', 9);
check('and by the line when one file holds both', [jobsTwinTags([l1, l2]).get(l1), jobsTwinTags([l1, l2]).get(l2)], ['dags · shop.yml:1', 'dags · shop.yml:9']);
var p1 = jb('daily', 'prod', '', 'daily.yml', 1), p2 = jb('daily', 'prod', '', 'daily.yml', 1), p3 = jb('other', 'prod');
var tags3 = jobsTwinTags([p3, p1, p2]);
check('two sources of one label over one file fall back to the place in the list', [tags3.get(p1), tags3.get(p2), tags3.has(p3)],
  ['prod · daily.yml:1 · #2', 'prod · daily.yml:1 · #3', false]);
check('no jobs, no tags', jobsTwinTags([]).size, 0);

print('\n--- the jobs as the server sends them ---');
var ids = ['model.jaffle.a', 'model.jaffle.b', 'model.jaffle.c'];
var sent = [{ name: 'daily', by_step: [{ n: 1, name: 'stage', models: [0, 1] }, { n: 2, name: 'marts', models: [1, 2] }] }, { name: 'idle' }];
var made = jobBuildsJobs(ids, sent);
check('a model carries the steps that run it', made[0].models, [{ id: 'model.jaffle.a', steps: [1] }, { id: 'model.jaffle.b', steps: [1, 2] }, { id: 'model.jaffle.c', steps: [2] }]);
check('and the steps are named', made[0].step_names, [{ n: 1, name: 'stage' }, { n: 2, name: 'marts' }]);
check('a job running no model has none', [made[1].models, made[1].step_names], [[], []]);
check('nothing at all', jobBuildsJobs([], undefined), []);

print('\n--- the section is listed from the index ---');
function sec(list) { return list.map(function (s) { return s.key + (s.empty ? ':empty' : '') + (s.quiet ? ':quiet' : ''); }); }
check('with the builds and the jobs, after the builds, quiet like them',
  sec(reportsSections({ builds: true, jobs: { sources: 1, at: 3 } })), ['jobs', 'joborder:quiet', 'builds:quiet', 'jobbuilds:quiet']);
check('never lighting the button', reportsSections({ builds: true, jobs: { sources: 1, at: 3 } }).filter(lights).map(function (s) { return s.key; }), ['jobs']);
check('in Snowflake\'s colour', reportsSections({ builds: true, jobs: { sources: 1, at: 3 } })[3].tool, 'snowflake');
check('without a source named, not at all', sec(reportsSections({ builds: true })), ['jobs:empty', 'builds:quiet']);
check('without the builds, not at all', sec(reportsSections({ jobs: { sources: 1, at: 3 } })), ['jobs', 'joborder:quiet']);
check('nor from an index of its own key', sec(reportsSections({ jobbuilds: true })), ['jobs:empty']);
check('after the macros and the checks', reportsSections({ builds: true, jobs: { sources: 1, at: 3 }, checks: { count: 2, at: 3 } }).map(function (s) { return s.key; }),
  ['jobs', 'joborder', 'checks', 'builds', 'jobbuilds']);

print('--- which environment a row opens on ---');
var col = function (key, tone) { return { key: key, tone: tone }; };
var cols = [col('', 'ok'), col('CI', 'late'), col('DEV', 'error'), col('QA', 'none')];
check('the pick made on the row wins', jobBuildsPick(cols, 'QA', 'CI'), 'QA');
check('the last pick anywhere comes next', jobBuildsPick(cols, undefined, 'CI'), 'CI');
check('the worst otherwise, an error over the rest', jobBuildsPick(cols, undefined, null), 'DEV');
check('late over not built over built', [
  jobBuildsPick([col('A', 'ok'), col('B', 'none'), col('C', 'late')], null, null),
  jobBuildsPick([col('A', 'ok'), col('B', 'none')], null, null),
  jobBuildsPick([col('A', 'ok'), col('B', 'ok')], null, null)], ['C', 'B', 'A']);
check('a tie takes the first column', jobBuildsPick([col('A', 'late'), col('B', 'late'), col('C', 'error'), col('D', 'error')], null, null), 'C');
check('the manifest column has an empty key and can be picked', jobBuildsPick(cols, '', 'CI'), '');
check('a row pick with nothing to list falls through to the last', jobBuildsPick(cols, 'UAT', 'CI'), 'CI');
check('a last pick with nothing to list falls through to the worst', jobBuildsPick(cols, null, 'UAT'), 'DEV');
check('nothing listable gives null', jobBuildsPick([], 'CI', 'DEV'), null);
