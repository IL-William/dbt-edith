// Job order in Reports (0126): what the answer of --job-order says on the page,
// the groups in their order, a pair's lines, the edges and the ten drawn, and
// what the answer leaves out. A small stand-in for the page's elements holds
// what is drawn. Invented names throughout.
// Run from the repository root: jsc web/tests/joborder.js
var app = read('web/app.js');
eval(app.slice(app.indexOf('function jobsScheduleText'), app.indexOf('async function loadJobs')));
eval(app.slice(app.indexOf('function jobOrderLine'), app.indexOf('async function jobOrderSection')));

function check(label, got, want) {
  var g = JSON.stringify(got), w = JSON.stringify(want);
  print((g === w ? 'PASS  ' : 'FAIL  ') + label + (g === w ? '' : '\n        expected ' + w + '\n        got      ' + g));
}

// Elements that hold a class, text, children, attributes and listeners.
function el(tag) {
  return {
    tag: tag, className: '', textContent: '', title: '', open: false, dataset: {}, children: [], on: {}, removed: false,
    append: function () { for (var i = 0; i < arguments.length; i++) this.children.push(arguments[i]); },
    replaceChildren: function () { this.children = []; this.append.apply(this, arguments); },
    addEventListener: function (type, fn) { this.on[type] = fn; },
    remove: function () { this.removed = true; },
  };
}
var document = { createElement: el };
var jel = function (tag, className, textContent) { var e = el(tag); e.className = className; e.textContent = textContent || ''; return e; };
// The text an element shows, the strings appended to it included.
function text(e) {
  if (typeof e === 'string') return e;
  return e.textContent + e.children.filter(function (c) { return !c.removed; }).map(text).join('');
}
function all(e, test, out) {
  out = out || [];
  if (typeof e === 'string') return out;
  if (test(e)) out.push(e);
  e.children.forEach(function (c) { all(c, test, out); });
  return out;
}
var opened = [], copied = [], calls = [];
function openFromReport(id, how) { opened.push([id, how]); }
function copyText(t) { copied.push(t); }
function jobsRefusedFile(body, why) { calls.push('refused: ' + why); }
function jobsNoSource() { calls.push('no source'); }
function jobsStepList(title, refs) { calls.push(title + ' ' + JSON.stringify(refs)); return jel('details', 'jobs-sec', title); }

function side(job, label, schedule, extra) {
  return Object.assign({ kind: 'airflow', job: job, source: 'dags', label: label, schedule: schedule || 'scheduled 0 6 * * *',
    cron: '0 6 * * *', file: job + '.yml', line: 1 }, extra || {});
}
function edge(node, parent, through) {
  return { node: 'model.shop.' + node, node_name: node, parent: 'model.shop.' + parent, parent_name: parent,
    through: (through || []).map(function (t) { return { id: 'model.shop.' + t, name: t }; }) };
}
function pair(group, step, waits, edges, more) {
  return Object.assign({ group: group, step: step, waits_for: waits, edges: edges }, more || {});
}
var STAGE = side('shop_stage', 'task stage');
var MARTS = side('shop_marts', 'task build');
function answer(pairs, more) {
  var c = { none: 0, clock: 0, reversed: 0 };
  pairs.forEach(function (p) { c[p.group]++; });
  return Object.assign({ configured: true, checked: 2, counts: c, pairs: pairs, unresolved: [], left_out: [], cyclic: [] }, more || {});
}
function drawn(a) { var body = el('div'); calls = []; renderJobOrder(body, a); return body; }

print('--- the first line ---');
var one = pair('none', MARTS, [STAGE], [edge('orders', 'stg_orders', ['int_orders'])]);
var lostStep = { source: 'dags', job: 'shop_odd', label: 'task odd', shown: 'dbt run --selector nowhere', file: 'odd.yml', line: 4, why: 'no such selector' };
check('none, every step looked at: every step waits for the steps that build its parents', jobOrderLine(answer([])), 'Every step waits for the steps that build its parents');
check('none, no step builds a node: it does not claim they wait', jobOrderLine(answer([], { checked: 0 })), 'No step of a job that starts on its own builds a node');
check('none, some step not resolved: only what was resolved is said', jobOrderLine(answer([], { unresolved: [lostStep] })), 'No pair among the steps resolved');
check('none, but a source was not read: it does not claim so', jobOrderLine(answer([], { not_read: { sources: 1, of: 1, reason: 'nothing at this path' } })), 'No pair in what was read');
check('and that before the others', jobOrderLine(answer([], { checked: 0, unresolved: [lostStep], not_read: { sources: 1, of: 2, reason: 'x' } })), 'No pair in what was read');
check('a step not resolved beats nothing looked at, as the command line says it first', jobOrderLine(answer([], { checked: 0, unresolved: [lostStep] })), 'No pair among the steps resolved');
check('a pair is said whatever else was left', jobOrderLine(answer([one], { checked: 0, unresolved: [lostStep] })), '1 pair: 1 nothing orders them, 0 only the clock, 0 the other way');
check('one pair, with the three counts', jobOrderLine(answer([one])), '1 pair: 1 nothing orders them, 0 only the clock, 0 the other way');
check('several', jobOrderLine(answer([one, pair('clock', MARTS, [STAGE], []), pair('reversed', MARTS, [STAGE], [])])),
  '3 pairs: 1 nothing orders them, 1 only the clock, 1 the other way');

print('\n--- the groups ---');
var mixed = answer([pair('reversed', MARTS, [STAGE], []), pair('none', MARTS, [STAGE], []), pair('clock', MARTS, [STAGE], [])]);
check('in the order of the answer, whatever the pairs come in', jobOrderGroups(mixed).map(function (g) { return g.key; }), ['none', 'clock', 'reversed']);
check('only the ones with a pair', jobOrderGroups(answer([one])).map(function (g) { return g.key; }), ['none']);
check('each says what it means', jobOrderGroups(mixed).map(function (g) { return g.hint; }), [
  'No task, sensor or start of the job files puts the first step after the second',
  'The crons differ, so the clock may order them; nothing in the files does',
  'The files order them, the other way round']);
var many = [];
for (var i = 0; i < 51; i++) many.push(pair('none', side('dag_' + i, 'task t'), [STAGE], []));
check('open while short, folded past 50', [jobOrderGroups(answer(many.slice(0, 50)))[0].open, jobOrderGroups(answer(many))[0].open], [true, false]);
var body = drawn(mixed);
var secs = all(body, function (e) { return e.tag === 'details'; });
check('drawn, a fold each with its count in the title',
  secs.map(function (d) { return text(d.children[0]); }), ['Nothing orders them (1)', 'Only the clock orders them (1)', 'Ordered the other way (1)']);
check('and the hint on the title', secs[1].children[0].title, 'The crons differ, so the clock may order them; nothing in the files does');

print('\n--- a pair ---');
check('a step is its job and itself', jobOrderSide(MARTS), 'DAG shop_marts › task build');
check('a dbt Cloud job as the Jobs tab names it', jobOrderSide(side('nightly', 'step 2/3', '', { kind: 'dbt_cloud' })), 'Job nightly › step 2/3');
var box = jobOrderPair(one);
check('the step should wait for the one that builds what it reads', text(box.children[0]), 'DAG shop_marts › task build should wait for DAG shop_stage › task stage');
var two = jobOrderPair(pair('none', MARTS, [STAGE, side('shop_again', 'task again')], []));
check('for one of several', text(two.children[0]), 'DAG shop_marts › task build should wait for one of DAG shop_stage › task stage, DAG shop_again › task again');
check('both clocks, with their zones as the answer words them, each said whose',
  text(jobOrderPair(pair('clock', side('shop_marts', 'task build', 'scheduled 0 7 * * *'), [side('shop_stage', 'task stage', 'scheduled 0 6 * * * Europe/London')], [])).children[1]),
  'this step: scheduled 0 7 * * *  ·  waits for: scheduled 0 6 * * * Europe/London');
check('with several to wait for, each under its job',
  jobOrderSchedules(pair('clock', MARTS, [side('a', 'task x', 'scheduled 0 5 * * *'), side('b', 'task y', 'manual')], [])),
  'this step: scheduled 0 6 * * *  ·  waits for: a: scheduled 0 5 * * *, b: manual');
var where = all(box, function (e) { return e.className === 'jobfile'; })[0];
copied = [];
where.on.click();
check('the file of the step is copied, as the Jobs tab does', [text(where), copied], ['dags · shop_marts.yml:1', ['shop_marts.yml:1']]);

print('\n--- the edges ---');
var edges = all(box, function (e) { return e.className === 'joblink' && /reads/.test(text(e)); });
check('a model reads a parent, through the views between', edges.map(text), ['orders reads stg_orders through int_orders']);
check('with none between, nothing after the parent', text(jobOrderEdge(edge('orders', 'stg_orders'))), 'orders reads stg_orders');
check('and the nearest first, as many as there are', text(jobOrderEdge(edge('orders', 'stg_orders', ['int_a', 'int_b']))), 'orders reads stg_orders through int_a, int_b');
var links = all(box, function (e) { return e.className === 'jobs-model'; });
opened = [];
links.forEach(function (l) { l.on.click(); });
check('the model, its parent and what is between them open with the Jobs tab',
  opened, [['model.shop.orders', { dock: 'jobs' }], ['model.shop.stg_orders', { dock: 'jobs' }], ['model.shop.int_orders', { dock: 'jobs' }]]);
var thru = all(jobOrderEdge(edge('orders', 'stg_orders', ['int_a', 'int_b'])), function (e) { return e.className === 'jobs-model'; });
opened = [];
thru.forEach(function (l) { l.on.click(); });
check('the views and ephemerals between them open too', opened.slice(2, 4).map(function (o) { return o[0]; }), ['model.shop.int_a', 'model.shop.int_b']);
var lots = [];
for (var k = 0; k < 13; k++) lots.push(edge('m' + k, 'p' + k));
var big = jobOrderPair(pair('none', MARTS, [STAGE], lots));
var shown = function () { return all(big, function (e) { return e.className === 'joblink' && /reads/.test(text(e)) && !e.removed; }).length; };
check('ten edges, then a button for the rest', [shown(), all(big, function (e) { return e.tag === 'button' && /more/.test(e.textContent); }).map(text)], [10, ['+3 more']]);
all(big, function (e) { return /more/.test(e.textContent) && e.tag === 'button'; })[0].on.click();
check('which draws them', shown(), 13);
check('ten exactly has no button', all(jobOrderPair(pair('none', MARTS, [STAGE], lots.slice(0, 10))), function (e) { return /more/.test(e.textContent); }).length, 0);

print('\n--- what else the files say ---');
var noted = pair('none', MARTS, [STAGE], [], {
  later: [side('shop_stage', 'task stage')],
  idle_sensors: [{ id: 'wait_stage', waits: ['shop_stage › stage'] }, { waits: ['shop_stage'] }],
  group_sensors: [{ id: 'wait_group', waits: ['shop_stage'], task: 'task build' }],
});
check('in order, a line each', jobOrderNotes(noted), [
  'DAG shop_stage › task stage runs after it',
  'Note: sensor wait_stage of this DAG waits for shop_stage › stage; this task does not depend on it',
  'Note: a sensor of this DAG waits for shop_stage; this task does not depend on it',
  'Note: task build depends on sensor wait_group, which waits for shop_stage, whose tasks are not read']);
check('none for a pair with nothing to add', jobOrderNotes(one), []);
check('drawn under the clocks', all(jobOrderPair(noted), function (e) { return e.className === 'joblink'; }).map(text).slice(1, 3),
  ['DAG shop_stage › task stage runs after it', 'Note: sensor wait_stage of this DAG waits for shop_stage › stage; this task does not depend on it']);

print('\n--- what the answer leaves out ---');
var left = answer([], { cyclic: [{ source: 'dags', job: 'loop_a' }], left_out: [{ source: 'dags', job: 'manual_a' }, { source: 'dags', job: 'manual_b' }],
  not_read: { sources: 1, of: 2, reason: 'nothing at this path' } });
check('a loop, the jobs nothing starts, a source not read', jobOrderCaveats(left), [
  { warn: true, text: 'DAG loop_a has a depends_on cycle, which Airflow does not load: left out' },
  { warn: false, text: '2 jobs nothing starts left out: manual_a, manual_b' },
  { warn: true, text: 'Not read: 1 of 2 sources (nothing at this path)' }]);
check('one job, said in the singular', jobOrderCaveats(answer([], { left_out: [{ source: 'dags', job: 'manual_a' }] }))[0].text, '1 job nothing starts left out: manual_a');
var seven = [];
for (var j = 1; j <= 7; j++) seven.push({ source: 'dags', job: 'manual_' + j });
check('five names then the count of the rest', jobOrderCaveats(answer([], { left_out: seven }))[0].text,
  '7 jobs nothing starts left out: manual_1, manual_2, manual_3, manual_4, manual_5, +2 more');
check('two sources holding one name are told apart', jobOrderCaveats(answer([], { left_out: [{ source: 'a', job: 'daily' }, { source: 'b', job: 'daily' }] }))[0].text,
  '2 jobs nothing starts left out: a: daily, b: daily');
check('and a loop likewise', jobOrderCaveats(answer([], { cyclic: [{ source: 'a', job: 'daily' }, { source: 'b', job: 'daily' }] })).map(function (c) { return c.text.split(' has')[0]; }),
  ['DAG a: daily', 'DAG b: daily']);
check('the sources, in the command line\'s words', [jobOrderUnread({ sources: 1, of: 1, reason: 'x' }), jobOrderUnread({ sources: 2, of: 2, reason: 'x' }), jobOrderUnread({ sources: 1, of: 3, reason: 'x' })],
  ['the source (x)', 'all 2 sources (x)', '1 of 3 sources (x)']);
check('nothing left out, nothing said', jobOrderCaveats(answer([])), []);
var full = drawn(answer([one], { cyclic: [{ source: 'dags', job: 'loop_a' }],
  unresolved: [{ source: 'dags', job: 'shop_odd', label: 'task odd', shown: 'dbt run {{ x }}', file: 'odd.yml', line: 4, why: 'a templated command' }] }));
check('the steps not resolved are listed by the coverage\'s own list, an Airflow task by its id',
  calls, ['Steps not resolved, which could hide more pairs ' + JSON.stringify([{ source: 'dags', job: 'shop_odd', label: 'task odd', shown: 'dbt run {{ x }}',
    file: 'odd.yml', line: 4, why: 'a templated command', name: 'odd' }])]);
check('and drawn in order: the line, the group, the steps, the loop', full.children.map(function (c) { return c.className; }), ['jobs-line', 'jobs-sec', 'jobs-sec', 'jobs-warn']);

print('\n--- no source, or a file that does not read ---');
drawn({ sources_refused: 'line 2: a source wants a path', configured: false });
check('the file\'s reason, before anything else', calls, ['refused: line 2: a source wants a path']);
drawn({ configured: false, counts: { none: 0, clock: 0, reversed: 0 }, pairs: [], unresolved: [], left_out: [], cyclic: [] });
check('no source named', calls, ['no source']);
var none = drawn(answer([]));
check('a source read with no pair says so, and draws nothing else', none.children.map(text), ['Every step waits for the steps that build its parents']);
