// Job order in Reports (0126), drawn in the sections' own grammar (0137): what
// the answer of --job-order says on the page, the figures, the groups as folds
// in their order, the pairs by DAG, a pair's lines, the edges grouped by the
// model that reads and the ten drawn, and what the answer leaves out. A small
// stand-in for the page's elements holds what is drawn. Invented names throughout.
// Run from the repository root: jsc web/tests/joborder.js
var app = read('web/app.js');
eval(app.slice(app.indexOf('function reportsSections'), app.indexOf('async function loadReportsIndex')));
eval(app.slice(app.indexOf('function reportFoldKey'), app.indexOf('/* Draws one section')));
eval(app.slice(app.indexOf('function jobsScheduleText'), app.indexOf('async function loadJobs')));
eval(app.slice(app.indexOf('function jobOrderLine'), app.indexOf('async function jobOrderSection')));
eval(app.slice(app.indexOf('function jobsStepRow'), app.indexOf('function renderCoverage')));
// A top-level const the page holds: read from it, so the test follows the cap.
var CHECK_SHOWN = Number(/const CHECK_SHOWN = (\d+)/.exec(app)[1]);

function check(label, got, want) {
  var g = JSON.stringify(got), w = JSON.stringify(want);
  print((g === w ? 'PASS  ' : 'FAIL  ') + label + (g === w ? '' : '\n        expected ' + w + '\n        got      ' + g));
}

// Elements that hold a class, text, children, attributes and listeners.
function el(tag) {
  return {
    tag: tag, className: '', textContent: '', title: '', open: false, hidden: false, type: '', value: '', placeholder: '',
    dataset: {}, attrs: {}, children: [], on: {}, removed: false,
    append: function () {
      for (var i = 0; i < arguments.length; i++) {
        this.children.push(arguments[i]);
        if (typeof arguments[i] === 'object') arguments[i].parentElement = this;
      }
    },
    click: function () { if (this.on.click) this.on.click(); },
    dispatchEvent: function () {},
    scrollTop: 0, parentElement: null,
    querySelectorAll: function (sel) { return find(this, sel); },
    querySelector: function (sel) {
      if (sel === ':scope > summary') return this.children.filter(function (c) { return typeof c === 'object' && c.tag === 'summary'; })[0] || null;
      return find(this, sel)[0] || null;
    },
    replaceChildren: function () { this.children = []; this.append.apply(this, arguments); },
    addEventListener: function (type, fn) { this.on[type] = fn; },
    setAttribute: function (k, v) { this.attrs[k] = v; },
    getAttribute: function (k) { return this.attrs[k]; },
    closest: function (sel) {
      for (var at = this; at; at = at.parentElement) if (matches(at, sel)) return at;
      return null;
    },
    remove: function () { this.removed = true; },
  };
}
// The selectors the page's keep and put-back use: a class, a tag, [data-key], a list.
function matches(e, sel) {
  return sel.split(',').some(function (one) {
    one = one.trim();
    if (one === '[data-key]') return e.dataset && e.dataset.key !== undefined;
    if (one === '.reportbody details') return e.tag === 'details'; // the filter's own look at the folds
    if (one[0] === '.') return (e.className || '').split(' ').indexOf(one.slice(1)) >= 0;
    return e.tag === one;
  });
}
function find(root, sel, out) {
  out = out || [];
  root.children.forEach(function (c) {
    if (typeof c !== 'object') return;
    if (matches(c, sel)) out.push(c);
    find(c, sel, out);
  });
  return out;
}
function Event(type) { this.type = type; }
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
// The filter waits a moment after a key: timers kept here, run by flush().
var timers = [];
function setTimeout(f, ms) { timers.push({ f: f, ms: ms }); return timers.length; }
function clearTimeout(id) { if (id && timers[id - 1]) timers[id - 1] = null; }
function flush() { var due = timers.filter(Boolean); timers = []; due.forEach(function (t) { t.f(); }); }
var opened = [], copied = [], calls = [];
function openFromReport(id, how) { opened.push([id, how]); }
function copyText(t) { copied.push(t); }
function jobsRefusedFile(body, why) { calls.push('refused: ' + why); }
function jobsNoSource() { calls.push('no source'); }
function dot() { return jel('span', 'dot'); }
function cls(name) { return function (e) { return e.className === name || (e.className || '').split(' ').indexOf(name) >= 0; }; }

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

// The lists of a drawn section: the folds of a group, top level, in order.
function folds(body) { return all(body, function (e) { return e.tag === 'details' && e.className === 'reportfinding'; }); }
function listsOf(body) { return all(body, cls('reportlists'))[0]; }
function titleOf(d) { return d.children[0].children[0].textContent; }
function countOf(d) { return d.children[0].children[1].textContent; }

print('\n--- the figures ---');
function tiled(a) { return jobOrderTiles(a).map(function (t) { return [t.label, t.value, t.tone]; }); }
check('the steps looked at, a tile per kind found and the steps not resolved',
  tiled(answer([one, pair('clock', MARTS, [STAGE], []), pair('reversed', MARTS, [STAGE], [])], { unresolved: [lostStep] })),
  [['Steps looked at', '2', ''], ['Nothing orders them', '1', 'warn'], ['Only the clock orders them', '1', ''], ['Ordered the other way', '1', 'warn'],
    ['Steps not resolved', '1', '']]);
check('only the kinds with a pair', tiled(answer([one])).map(function (t) { return t[0]; }), ['Steps looked at', 'Nothing orders them']);
check('none while there is no pair and nothing unresolved', tiled(answer([])), []);
check('the steps not resolved alone are enough', tiled(answer([], { unresolved: [lostStep] })).map(function (t) { return t[0]; }), ['Steps looked at', 'Steps not resolved']);

print('\n--- the page, in the order it is read ---');
var full = drawn(answer([one], { cyclic: [{ source: 'dags', job: 'loop_a' }], left_out: [{ source: 'dags', job: 'manual_a' }],
  unresolved: [Object.assign({}, lostStep, { why: 'a templated command' })] }));
check('figures, the line, the warnings and the rest left out, then the filter and the lists',
  full.children.map(function (c) { return c.className; }), ['reporttiles', 'runnote', 'runnote', 'runnote', 'reportfind', 'reportlists']);
check('a warning in its colour, a plain note not', full.children.slice(2, 4).map(function (c) { return c.dataset.tone || ''; }), ['warn', '']);
check('the caveats come before the lists, not after', full.children.slice(2, 4).map(text),
  ['DAG loop_a has a depends_on cycle, which Airflow does not load: left out', '1 job nothing starts left out: manual_a']);
var filterInput = all(full, cls('reportfilter'))[0];
check('a filter, for the steps and the models', [filterInput.tag, filterInput.type, filterInput.placeholder], ['input', 'search', 'Filter the steps and models']);
var lists = listsOf(full);
check('a fold per group, then the steps not resolved', folds(full).map(titleOf), ['Nothing orders them', 'Steps not resolved, which could hide more pairs']);
check('the group warns, the steps do not', folds(full).map(function (d) { return d.dataset.tone || ''; }), ['warn', '']);
check('open while short', folds(full).map(function (d) { return d.open; }), [true, true]);

var mixedBody = drawn(mixed);
var mixedFolds = folds(mixedBody);
check('a fold each, its count in the summary', mixedFolds.map(function (d) { return [titleOf(d), countOf(d)]; }),
  [['Nothing orders them', '1'], ['Only the clock orders them', '1'], ['Ordered the other way', '1']]);
check('the ones the figures warn of warn', mixedFolds.map(function (d) { return d.dataset.tone || ''; }), ['warn', '', 'warn']);
check('the hint is the lever, one sentence, so no "why" behind it', mixedFolds.map(function (d) { return text(d.children[1]); }), [
  'No task, sensor or start of the job files puts the first step after the second',
  'The crons differ, so the clock may order them; nothing in the files does',
  'The files order them, the other way round']);
check('folded past 50', folds(drawn(answer(many)))[0].open, false);
check('and open at 50', folds(drawn(answer(many.slice(0, 50))))[0].open, true);

print('\n--- a pair ---');
check('a step is its job and itself', jobOrderSide(MARTS), 'DAG shop_marts › task build');
check('a dbt Cloud job as the Jobs tab names it', jobOrderSide(side('nightly', 'step 2/3', '', { kind: 'dbt_cloud' })), 'Job nightly › step 2/3');
var box = jobOrderPair(one);
check('the step should wait for the one that builds what it reads, in its own words when bare',
  text(box.children[0]), 'DAG shop_marts › task build should wait for DAG shop_stage › task stage');
check('the step is the first thing a row says', box.children[0].children[0].className, 'rstep');
var two = jobOrderPair(pair('none', MARTS, [STAGE, side('shop_again', 'task again')], []));
check('for one of several', text(two.children[0]), 'DAG shop_marts › task build should wait for one of DAG shop_stage › task stage, DAG shop_again › task again');
var same = jobOrderPair(pair('none', MARTS, [side('shop_marts', 'task stage')], []));
check('a step it waits for in its own job is named by its task alone',
  [jobOrderTargets(pair('none', MARTS, [side('shop_marts', 'task stage'), STAGE], [])), text(same.children[0])],
  [['task stage', 'DAG shop_stage › task stage'], 'DAG shop_marts › task build should wait for task stage']);
check('two sources holding a job of one name are two jobs',
  jobOrderTargets(pair('none', MARTS, [side('shop_marts', 'task stage', '', { source: 'other' })], [])), ['DAG shop_marts › task stage']);
var inFold = jobOrderPair(one, { inFold: true });
check('inside its DAG\'s fold, the task alone', text(inFold.children[0]), 'task build should wait for DAG shop_stage › task stage');
check('both clocks, with their zones as the answer words them, each said whose',
  text(jobOrderPair(pair('clock', side('shop_marts', 'task build', 'scheduled 0 7 * * *'), [side('shop_stage', 'task stage', 'scheduled 0 6 * * * Europe/London')], [])).children[1].children[0]),
  'this step: scheduled 0 7 * * *  ·  waits for: scheduled 0 6 * * * Europe/London');
check('with several to wait for, each under its job',
  jobOrderSchedules(pair('clock', MARTS, [side('a', 'task x', 'scheduled 0 5 * * *'), side('b', 'task y', 'manual')], [])),
  'this step: scheduled 0 6 * * *  ·  waits for: a: scheduled 0 5 * * *, b: manual');
check('the clocks and the file on one line, the clocks taking the free width', [box.children[1].className, box.children[1].children[0].className], ['rline rstepline', 'rsched muted']);
var shown = jobOrderPair(one, { inFold: true, shared: true });
check('the clocks left out when the fold says them once', shown.children[1].children.map(function (c) { return c.className; }), ['jobfile']);
var where = all(box, cls('jobfile'))[0];
copied = [];
where.on.click();
check('the file of the step is copied, as the Jobs tab does', [text(where), copied], ['dags · shop_marts.yml:1', ['shop_marts.yml:1']]);
check('and says why it is not opened', where.title, 'Outside the project, so it does not open here. Click to copy where it is.');

print('\n--- the pairs of one DAG, in a fold ---');
var other = side('shop_marts', 'task report');
var fold2 = jobOrderFolders([pair('none', MARTS, [STAGE], []), pair('none', other, [STAGE], []), pair('none', side('shop_ops', 'task clean'), [STAGE], [])], true);
check('a fold for a job with several pairs, a pair listed bare for one with a single', fold2.map(function (e) { return e.tag; }), ['details', 'ul']);
check('the fold is named by its DAG, with its count', text(fold2[0].children[0]), 'DAG shop_marts (2)');
check('and open as the group is', fold2[0].open, true);
check('the clocks the pairs all share are said once, over them', [fold2[0].children[1].className, text(fold2[0].children[1])],
  ['rline', 'this step: scheduled 0 6 * * *  ·  waits for: scheduled 0 6 * * *']);
var inner = all(fold2[0], function (e) { return e.tag === 'li'; });
check('each pair there by its task, with no clocks of its own', inner.map(function (li) { return [text(li.children[0]), li.children[1].children.length]; }),
  [['task build should wait for DAG shop_stage › task stage', 1], ['task report should wait for DAG shop_stage › task stage', 1]]);
check('the bare pair keeps its whole step and its clocks', text(all(fold2[1], function (e) { return e.tag === 'li'; })[0].children[0]),
  'DAG shop_ops › task clean should wait for DAG shop_stage › task stage');
var apart = jobOrderFolders([pair('clock', side('shop_marts', 'task build', 'scheduled 0 7 * * *'), [STAGE], []),
  pair('clock', side('shop_marts', 'task report', 'scheduled 0 8 * * *'), [STAGE], [])], false);
check('clocks that differ are said on each pair, and the fold says none', [apart[0].open, apart[0].children.length, jobOrderShared([
  pair('clock', side('a', 'x', 'scheduled 0 7 * * *'), [STAGE], []), pair('clock', side('a', 'y', 'scheduled 0 8 * * *'), [STAGE], [])])],
  [false, 2, '']);
check('shared: nothing for no pair', jobOrderShared([]), '');
check('the same DAG of two sources is two folds',
  jobOrderFolders([pair('none', side('d', 'a'), [STAGE], []), pair('none', side('d', 'b'), [STAGE], []), pair('none', side('d', 'c', '', { source: 'more' }), [STAGE], []),
    pair('none', side('d', 'e', '', { source: 'more' }), [STAGE], [])], true).map(function (e) { return text(e.children[0]); }), ['DAG d (2)', 'DAG d (2)']);

print('\n--- the edges ---');
check('a model reads a parent, through the views between', text(jobOrderEdge(jobOrderEdgeRows([edge('orders', 'stg_orders', ['int_orders'])])[0])), 'orders reads stg_orders through int_orders');
check('with none between, nothing after the parent', text(jobOrderEdge(jobOrderEdgeRows([edge('orders', 'stg_orders')])[0])), 'orders reads stg_orders');
check('and the nearest first, as many as there are', text(jobOrderEdge(jobOrderEdgeRows([edge('orders', 'stg_orders', ['int_a', 'int_b'])])[0])), 'orders reads stg_orders through int_a, int_b');
var severalParents = jobOrderEdgeRows([edge('orders', 'stg_orders', ['int_a']), edge('orders', 'stg_items', ['int_a', 'int_b']), edge('payments', 'stg_pay')]);
check('edges are grouped by the model that reads, first seen first', severalParents.map(function (r) { return [r.node_name, r.edges.length]; }), [['orders', 2], ['payments', 1]]);
check('several parents on one line, what is between them once, in brackets', text(jobOrderEdge(severalParents[0])), 'orders reads stg_orders, stg_items (through int_a, int_b)');
check('several parents with nothing between them', text(jobOrderEdge(jobOrderEdgeRows([edge('orders', 'stg_orders'), edge('orders', 'stg_items')])[0])), 'orders reads stg_orders, stg_items');
var links = all(box, cls('jobs-model'));
opened = [];
links.forEach(function (l) { l.on.click(); });
check('the model, its parent and what is between them open with the Jobs tab',
  opened, [['model.shop.orders', { dock: 'jobs' }], ['model.shop.stg_orders', { dock: 'jobs' }], ['model.shop.int_orders', { dock: 'jobs' }]]);
var thru = all(jobOrderEdge(jobOrderEdgeRows([edge('orders', 'stg_orders', ['int_a', 'int_b'])])[0]), cls('jobs-model'));
opened = [];
thru.forEach(function (l) { l.on.click(); });
check('the views and ephemerals between them open too', opened.slice(2, 4).map(function (o) { return o[0]; }), ['model.shop.int_a', 'model.shop.int_b']);
var lots = [];
for (var k = 0; k < 13; k++) lots.push(edge('m' + k, 'p' + k));
var big = jobOrderPair(pair('none', MARTS, [STAGE], lots));
var readers = function () { return all(big, function (e) { return e.className === 'joblink' && / reads /.test(text(e)) && !e.removed; }).length; };
check('ten models, then a button for the rest', [readers(), all(big, function (e) { return e.tag === 'button' && /more/.test(e.textContent); }).map(text)], [10, ['+3 more']]);
all(big, function (e) { return /more/.test(e.textContent) && e.tag === 'button'; })[0].on.click();
check('which draws them', readers(), 13);
check('ten exactly has no button', all(jobOrderPair(pair('none', MARTS, [STAGE], lots.slice(0, 10))), function (e) { return /more/.test(e.textContent); }).length, 0);
var grouped = [];
for (var g = 0; g < 30; g++) grouped.push(edge('one_model', 'parent_' + g));
check('thirty parents of one model are one line, not thirty',
  [all(jobOrderPair(pair('none', MARTS, [STAGE], grouped)), function (e) { return e.className === 'joblink'; }).length,
    all(jobOrderPair(pair('none', MARTS, [STAGE], grouped)), function (e) { return e.tag === 'button' && /more/.test(e.textContent); }).length], [1, 0]);

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
check('drawn under the clocks', all(jobOrderPair(noted), cls('joblink')).map(text).slice(0, 2),
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

print('\n--- the steps not resolved, in the coverage\'s own list ---');
var lostFold = folds(full)[1];
check('an Airflow task by its id, the reason under it, the command in full, the file copied',
  all(lostFold, function (e) { return e.tag === 'li'; }).map(function (li) { return [text(li.children[0]), li.children.slice(1, -1).map(text).join(''), li.children.slice(-1)[0].children.map(function (c) { return c.className; })]; }),
  [['shop_odd › odd', ' · a templated command', ['rcmd', 'jobfile']]]);
check('the command is not cut: it is the whole of it', all(lostFold, cls('rcmd')).map(text), ['dbt run --selector nowhere']);
check('a dbt Cloud step keeps its place', text(jobsStepRow({ job: 'nightly', n: 2, shown: 'dbt build', source: 's', file: 'f.yml', line: 3, why: '' }).children[0]), 'nightly › step 2');
var namedStep = Object.assign({}, lostStep, { name: 'odd' });
var stepSpec = jobsStepList('Steps', [namedStep], 'A lever.');
check('as a fold: titled, with its lever, open while short, found by job, task, command and reason',
  [stepSpec.title, stepSpec.lever, stepSpec.open(20), stepSpec.open(21), stepSpec.find(namedStep)],
  ['Steps', 'A lever.', true, false, 'shop_odd odd dbt run --selector nowhere no such selector']);
check('with no name, its place', jobsStepList('S', []).find({ job: 'nightly', n: 2, shown: 'dbt build', why: 'x' }), 'nightly step 2 dbt build x');
check('the server\'s backticks drawn as code, not as backticks', (function () {
  var li = jobsStepRow(Object.assign({}, lostStep, { name: 'odd', why: 'the selector `tag:nothing` matches no node' }));
  return [text(li.children[0]), li.children.slice(1, 5).map(function (c) { return typeof c === 'string' ? c : c.className + ':' + c.textContent; })];
})(), ['shop_odd › odd', ['muted: · ', 'the selector ', 'rquote:tag:nothing', ' matches no node']]);
check('the command and the file on one line, pinned', jobsStepRow(Object.assign({}, lostStep, { name: 'odd' })).children.slice(-1)[0].className, 'rline rstepline');

print('\n--- the filter ---');
var typed = drawn(answer([one, pair('clock', side('shop_ops', 'task clean'), [STAGE], [edge('refunds', 'stg_refunds')])]));
var input = all(typed, cls('reportfilter'))[0];
function filtered(q) { input.value = q; input.on.input(); flush(); return listsOf(typed); }
check('every fold before anything is typed', folds(typed).map(function (d) { return [titleOf(d), countOf(d)]; }), [['Nothing orders them', '1'], ['Only the clock orders them', '1']]);
check('a name of a model an edge reads finds the pair, the other fold gone', folds(filtered('refunds')).map(function (d) { return [titleOf(d), countOf(d)]; }), [['Only the clock orders them', '1']]);
check('a parent too', folds(filtered('stg_orders')).map(titleOf), ['Nothing orders them']);
check('what is between them too', folds(filtered('int_orders')).map(titleOf), ['Nothing orders them']);
check('a step by its job', folds(filtered('SHOP_OPS')).map(titleOf), ['Only the clock orders them']);
check('nothing found says so', text(filtered('no such thing')), 'Nothing here matches the filter.');
check('and emptying it brings them all back', folds(filtered('')).length, 2);

print('\n--- no source, or a file that does not read ---');
drawn({ sources_refused: 'line 2: a source wants a path', configured: false });
check('the file\'s reason, before anything else', calls, ['refused: line 2: a source wants a path']);
drawn({ configured: false, counts: { none: 0, clock: 0, reversed: 0 }, pairs: [], unresolved: [], left_out: [], cyclic: [] });
check('no source named', calls, ['no source']);
var none = drawn(answer([]));
check('a source read with no pair says so, and draws nothing else: no figures, no filter', none.children.map(text), ['', 'Every step waits for the steps that build its parents', '']);
check('the figures empty, the lists empty', [none.children[0].className, none.children[0].children.length, none.children[2].children.length], ['reporttiles', 0, 0]);

print('\n--- the pieces every section is made of (0137) ---');
var tiles = reportTiles([{ label: 'Steps', value: '4', sub: 'looked at', tone: 'warn' }, { label: 'Pairs', value: '2' }]);
check('a tile each: label, value, what is under it', tiles.children.map(function (t) { return t.children.map(text); }), [['Steps', '4', 'looked at'], ['Pairs', '2', '']]);
check('the tone only where there is one', tiles.children.map(function (t) { return t.dataset.tone || ''; }), ['warn', '']);
check('no tile, an empty box', [reportTiles([]).className, reportTiles([]).children.length], ['reporttiles', 0]);

var lever = reportLever('No freshness. Neither warn_after nor error_after holds a count.');
check('a lever says its first sentence, and keeps it as its key', [text(lever).slice(0, 'No freshness.'.length), lever.dataset.key, lever.className], ['No freshness.', 'No freshness.', 'rlever']);
var tail = all(lever, cls('rtail'))[0];
check('the last word of the sentence goes with the button, so that "why" is never alone on a line', [lever.textContent, tail.textContent, tail.children.map(text)], ['No ', 'freshness.', ['why']]);
var more = all(lever, cls('rmore'))[0], why = all(lever, cls('rmorebtn'))[0];
check('the rest behind "why", hidden', [text(more), more.hidden, text(why), why.attrs['aria-expanded'], why.type], [' Neither warn_after nor error_after holds a count.', true, 'why', 'false', 'button']);
why.on.click();
check('which a click unfolds', [more.hidden, why.attrs['aria-expanded']], [false, 'true']);
why.on.click();
check('and folds again', [more.hidden, why.attrs['aria-expanded']], [true, 'false']);
var short = reportLever('One sentence only.');
check('a lever of one sentence is its text alone', [short.textContent, short.children.length], ['One sentence only.', 0]);
check('a lever of one sentence has no "why"', all(short, cls('rmorebtn')).length, 0);
check('another class for the intro of a section', reportLever('Reads it. Then more.', 'runnote').className, 'runnote');

var fold = reportFold({ title: 'Models', count: '3 of 9', open: true, tone: 'warn', lever: 'Why. And so.' });
check('a fold: its title and count in the summary, then the lever', [fold.tag, fold.className, text(fold.children[0]), fold.children[1].className], ['details', 'reportfinding', 'Models3 of 9', 'rlever']);
check('open as asked, in its tone', [fold.open, fold.dataset.tone], [true, 'warn']);
check('no lever, none drawn', reportFold({ title: 'T', count: '1', open: false }).children.length, 1);
check('shut by the reader, it is marked as theirs', (function () { var f = reportFold({ title: 'T', count: '1', open: false }); f.children[0].on.click(); return f.dataset.touched; })(), '1');

var rowsL = [];
for (var q = 0; q < 8; q++) rowsL.push('row ' + q);
var host = jel('details', 'reportfinding');
var rowFn = function (r) { return jel('li', '', r); };
var capped = reportList(rowsL, rowFn, host, 5, 40);
check('the first rows, a button for the rest, and how many the server held back',
  capped.children.map(text), ['row 0', 'row 1', 'row 2', 'row 3', 'row 4', 'Show all 8', 'and 40 more']);
all(capped, function (e) { return e.className === 'btn sm showall'; })[0].on.click();
check('Show all draws the rest, is remembered on the fold, and the button goes', [capped.children.length, capped.children.map(text).slice(-2), host.dataset.all],
  [9, ['row 7', 'and 40 more'], '1']);
check('under the cap, no button', reportList(rowsL, rowFn, jel('details', ''), 8, 0).children.length, 8);
check('nothing held back, no "and more"', reportList(rowsL, rowFn, jel('details', ''), Infinity, 0).children.map(text).slice(-1), ['row 7']);

var listsHost = el('div');
var seenArgs = [];
function foldSpec(title, rows, more) {
  return Object.assign({ title: title, tone: '', rows: rows, open: false, find: function (r) { return r; },
    draw: function (hits, d, rest) { seenArgs.push([hits.length, rest]); return [jel('ul', 'rentries', hits.join(','))]; } }, more || {});
}
drawReportLists(listsHost, [{ folds: [foldSpec('A', ['x', 'y', 'xz'], { of: 10 })] }], '');
check('a fold drawn with its hits and the rows the server held back', [listsHost.children.map(titleOf), seenArgs], [['A'], [[3, 7]]]);
check('and its list inside it', text(listsHost.children[0].children[1]), 'x,y,xz');
seenArgs = [];
drawReportLists(listsHost, [{ folds: [foldSpec('A', ['x', 'y', 'xz'], { of: 10 })] }], 'x');
check('filtered: the hits, "n of m", nothing said of the rest', [titleOf(listsHost.children[0]), countOf(listsHost.children[0]), seenArgs], ['A', '2 of 10', [[2, 0]]]);
drawReportLists(listsHost, [{ folds: [foldSpec('A', ['x'])] }], 'nothing like it');
check('nothing found says so', listsHost.children.map(text), ['Nothing here matches the filter.']);
drawReportLists(listsHost, [{ folds: [] }], '');
check('and nothing to show, with no filter, says nothing', listsHost.children.length, 0);
drawReportLists(listsHost, [{ group: 'rerun', label: 'Rerun dbt', desc: 'What dbt wrote is behind.', folds: [foldSpec('A', ['x'])] }, { folds: [foldSpec('B', ['y'])] }], '');
check('a group with a label is a section under its heading, the fold inside it', [listsHost.children[0].tag, listsHost.children[0].className, listsHost.children[0].dataset.group,
  listsHost.children[0].children.map(function (c) { return c.tag; }), text(listsHost.children[0].children[0])], ['section', 'reportgroup', 'rerun', ['h3', 'p', 'details'], 'Rerun dbt']);
check('one without is bare, after it', listsHost.children.map(function (c) { return c.tag; }), ['section', 'details']);
var headEl = jel('h3', 'buildsenv', 'QA');
drawReportLists(listsHost, [{ head: [headEl, jel('p', 'runnote', 'Not read.')], folds: [] }, { head: headEl, folds: [] }], '');
check('a group with a head brings it, and stays with no fold; a single element is as good as a list', listsHost.children.map(text), ['QA', 'Not read.', 'QA']);

var drawnWith = [];
var fbox = reportFilterBox('Filter the things', function (q2) { drawnWith.push(q2); });
var fin = fbox.children[0];
check('a filter box: a search input with its placeholder', [fbox.className, fin.tag, fin.type, fin.placeholder, fin.attrs['aria-label'], fin.className],
  ['reportfind', 'input', 'search', 'Filter the things', 'Filter the things', 'reportfilter']);
fin.value = 'orders';
fin.on.input();
check('typing draws nothing at once', drawnWith, []);
flush();
check('the lists are drawn with what is typed once it has waited', [drawnWith, timers.length], [['orders'], 0]);
fin.value = 'o'; fin.on.input(); fin.value = 'or'; fin.on.input(); fin.value = 'ord'; fin.on.input();
flush();
check('keys typed in a row draw once, with the last', drawnWith, ['orders', 'ord']);
check('80 ms is the wait', (function () { fin.on.input(); var ms = timers.filter(Boolean)[0].ms; flush(); return ms; })(), 80);
fin.on.input(); fin.redraw('put back');
check('a put-back draws at once and the wait that was pending is dropped', [drawnWith.slice(-2), fin.value, timers.filter(Boolean).length], [['ord', 'put back'], 'put back', 0]);

var secBody = el('div');
reportLists(secBody, [{ folds: [foldSpec('A', ['x'])] }], 'Filter');
check('lists with something to filter: the filter, then the lists, drawn', [secBody.children.map(function (c) { return c.className; }), folds(secBody).length], [['reportfind', 'reportlists'], 1]);
var secEmpty = el('div');
reportLists(secEmpty, [{ folds: [foldSpec('A', [], { of: 0 })] }], 'Filter');
check('with none, no filter', secEmpty.children.map(function (c) { return c.className; }), ['reportlists']);
var alsoQs = [];
var secAlso = el('div');
reportLists(secAlso, [], 'Filter', function (q3) { alsoQs.push(q3); });
check('what else the filter drives gets it too, and a filter though there are no folds', [secAlso.children.map(function (c) { return c.className; }), alsoQs], [['reportfind', 'reportlists'], ['']]);
all(secAlso, cls('reportfilter'))[0].value = 'zz';
all(secAlso, cls('reportfilter'))[0].on.input();
flush();
check('typing drives it', alsoQs, ['', 'zz']);
check('a section with no fold never says nothing matches: its own table says what is left', secAlso.children[1].children.length, 0);

// A filter hiding all of a table's rows is that table's to say; one hiding
// every fold of a section is the lists'.
var alsoHost = el('div');
drawReportLists(alsoHost, [], 'zz');
check('no group, a word typed: nothing is said', alsoHost.children.length, 0);

print('\n--- the place a section is left in, and put back ---');
function leverFold(title, levers) {
  var d = reportFold({ title: title, count: '1', open: true });
  levers.forEach(function (t) { d.append(reportLever(t)); });
  return d;
}
function sectionView(levers) {
  var view = el('div');
  view.className = 'reports-view';
  var body = jel('div', 'reportbody');
  body.append(leverFold('Models', levers));
  view.append(body);
  return view;
}
var LEVERS = ['Alpha is first. More about alpha.', 'Beta is second. More about beta.'];
var left = sectionView(LEVERS);
all(left, cls('rmorebtn'))[0].click();
left.children[0].scrollTop = 120;
var place = keepReportPlace(left);
check('two levers kept under their own sentence, the open one and the closed one',
  [place.whys, place.scroll], [{ 'Alpha is first.': true, 'Beta is second.': false }, 120]);
var back = sectionView(LEVERS);
restoreReportPlace(back, place);
var btns = all(back, cls('rmorebtn'));
check('put back: the first unfolded and saying so, the second as it was', [btns.map(function (b) { return b.attrs['aria-expanded']; }), btns.map(text),
  all(back, cls('rmore')).map(function (m) { return m.hidden; }), back.children[0].scrollTop], [['true', 'false'], ['less', 'why'], [false, true], 120]);
check('a click again folds it, and says why again', (function () { btns[0].click(); return [btns[0].textContent, btns[0].attrs['aria-expanded']]; })(), ['why', 'false']);
check('the open folds are kept by their title', keepReportPlace(sectionView(LEVERS)).open, { Models: true });

print('\n--- a fold the reader opened keeps its place through a filter ---');
// A section put back as it was left marks the folds the reader touched, so that
// the first key typed in a fresh filter box does not close them again.
function foldSection(titles) {
  var view = el('div');
  view.className = 'reports-view';
  var body = jel('div', 'reportbody');
  var specs = titles.map(function (title) {
    return { title: title, tone: '', rows: ['row of ' + title], open: false, find: function (r) { return r; },
      draw: function (hits) { return [jel('ul', 'rentries', hits.join(','))]; } };
  });
  reportLists(body, [{ folds: specs }], 'Filter');
  view.append(body);
  return view;
}
var firstView = foldSection(['Alpha', 'Beta']);
var alpha = folds(firstView)[0];
alpha.children[0].click();          // the reader's click, which marks it theirs
alpha.open = true;
var leftAt = keepReportPlace(firstView);
check('the folds the reader touched are kept', leftAt.touched, { Alpha: true });
var secondView = foldSection(['Alpha', 'Beta']);
restoreReportPlace(secondView, leftAt);
check('put back: open, and theirs', [folds(secondView)[0].open, folds(secondView)[0].dataset.touched, folds(secondView)[1].dataset.touched || ''], [true, '1', '']);
var typedIn = all(secondView, cls('reportfilter'))[0];
typedIn.value = 'row';
typedIn.on.input();
flush();
check('the first key typed does not close it', folds(secondView).map(function (d) { return [titleOf(d), d.open]; }), [['Alpha', true], ['Beta', false]]);
typedIn.value = 'beta'; typedIn.on.input(); flush();
typedIn.value = ''; typedIn.on.input(); flush();
check('nor does a filter that hid it for a while', folds(secondView).map(function (d) { return [titleOf(d), d.open]; }), [['Alpha', true], ['Beta', false]]);

print('\n--- the pairs are drawn up to a cap, the rest behind Show all ---');
var pairsMany = [];
for (var k = 0; k < CHECK_SHOWN + 20; k++) pairsMany.push(pair('none', side('dag_' + (k % 7), 'task t' + k), [STAGE], []));
var bigBody = drawn(answer(pairsMany));
var bigFold = folds(bigBody)[0];
var drawnBefore = all(bigFold, cls('rstep')).length;
check('only the cap is drawn, a button names the whole', [drawnBefore, text(all(bigFold, cls('showall'))[0])], [CHECK_SHOWN, 'Show all ' + (CHECK_SHOWN + 20)]);
check('each DAG says its whole count, drawn or not', all(bigFold, function (e) { return e.tag === 'summary' && e.parentElement.className === 'checkfolder'; }).map(text).slice(0, 2).map(function (s) { return /\((\d+)\)$/.exec(s)[1]; }), ['18', '17']);
var showAll = all(bigFold, cls('showall'))[0];
showAll.click();
check('Show all draws the rest, is remembered on the fold, and the button goes',
  [all(bigFold, cls('rstep')).length, bigFold.dataset.all, showAll.parentElement.removed], [CHECK_SHOWN + 20, '1', true]);
check('under the cap, no button', all(folds(drawn(answer(pairsMany.slice(0, CHECK_SHOWN))))[0], cls('showall')).length, 0);
