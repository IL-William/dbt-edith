// The guided tour's steps, which it leaves out or words otherwise when a
// feature is off, and where its ring and card go. Headless Chrome walks the
// tour itself on Jaffle Shop (scripts/walk_tour.py).
// Run from the repository root: jsc web/tests/tour.js
var src = read('web/app.js');
eval(src.slice(src.indexOf('function keyLabel'), src.indexOf('function shortcutSheet')));
eval(src.slice(src.indexOf('function tourSteps'), src.indexOf('async function startTour')));
var page = read('web/index.html');

function check(label, got, want) {
  var g = JSON.stringify(got), w = JSON.stringify(want);
  print((g === w ? 'PASS  ' : 'FAIL  ') + label + (g === w ? '' : '\n        expected ' + w + '\n        got      ' + g));
}
function keys(steps) { return steps.map(function (s) { return s.key; }); }
function step(steps, key) { return steps.filter(function (s) { return s.key === key; })[0]; }

var model = { id: 'model.jaffle_shop.stg_orders', name: 'stg_orders', file: 'models/staging/stg_orders.sql', parents: 1, children: 2 };
var bare = { model: model, mac: true, jobs: 0, reports: [], cll: '', snowflake: false };
function ctx(over) {
  var c = {};
  for (var k in bare) c[k] = bare[k];
  for (var k2 in over) c[k2] = over[k2];
  return c;
}

print('--- which steps ---');
var all = tourSteps(bare);
check('with a model, every part of the page in the order of the plan', keys(all),
  ['welcome', 'editor', 'config', 'lineage', 'select', 'share', 'catalog', 'columns', 'jobs', 'reports', 'terminal', 'search', 'end']);
check('without one, the steps about a model are left out',
  keys(tourSteps(ctx({ model: null }))), ['welcome', 'columns', 'reports', 'terminal', 'search', 'end']);
check('  and the welcome says why, and what to run', /dbt parse/.test(step(tourSteps(ctx({ model: null })), 'welcome').text), true);
check('the welcome names the model and counts its neighbours',
  step(all, 'welcome').text.indexOf('`stg_orders`, a model of this project with 1 parent and 2 children') >= 0, true);
check('  in the singular for one child',
  step(tourSteps(ctx({ model: { id: 'x', name: 'x', file: 'x.sql', parents: 3, children: 1 } })), 'welcome').text.indexOf('3 parents and 1 child') >= 0, true);
check('the selection drawn is the one the step names', step(all, 'select').text.indexOf('`+stg_orders+`') >= 0, true);
check('the dbt_project.yml button rings its menu too, which the step opens',
  step(all, 'config').target, ['#crumbs .cfg', '.crumbmenu.config']);
check('Export, Copy image and Copy are rung together, after the selection they share',
  [step(all, 'share').target, keys(all).indexOf('share') === keys(all).indexOf('select') + 1], [['#export-btn', '#image-btn', '#select-copy'], true]);

print('\n--- a feature that is off says how to turn it on ---');
check('no job source: Add a source', /Add a source/.test(step(all, 'jobs').text), true);
check('  a source: the model\'s jobs', /run `stg_orders`/.test(step(tourSteps(ctx({ jobs: 2 })), 'jobs').text), true);
check('no report: the button is dimmed until one, and opens on the coverage meanwhile',
  [/dimmed until one has something/.test(step(all, 'reports').text), /opens on the coverage, where a source is added, and \{⌥⌘R\}/.test(step(all, 'reports').text)],
  [true, true]);
check('  reports: named', step(tourSteps(ctx({ reports: ['Last run', 'Job coverage'] })), 'reports').text
  .indexOf('Reports on the whole project: Last run, Job coverage.') === 0, true);
check('no column lineage, Snowflake off: where its switch is', /snowflake button in the top bar/.test(step(all, 'columns').text), true);
check('  Snowflake on: what it does, and no switch to find',
  [/fetches a column/.test(step(tourSteps(ctx({ snowflake: true })), 'columns').text),
    /snowflake button/.test(step(tourSteps(ctx({ snowflake: true })), 'columns').text)], [true, false]);
check('  a tool on screen: named, and how to use it',
  step(tourSteps(ctx({ cll: 'Collin' })), 'columns').text.indexOf('On screen: Collin. A click on a column') === 0, true);

print('\n--- keys as this machine reads them ---');
check('a Mac', /\{⌘K\} anywhere/.test(step(all, 'search').text), true);
check('elsewhere', /\{Ctrl\+K\} anywhere/.test(step(tourSteps(ctx({ mac: false })), 'search').text), true);
check('Escape', /\{Esc\} ends it/.test(step(all, 'welcome').text), true);

print('\n--- every step ---');
var every = all.concat(tourSteps(ctx({ model: null, jobs: 1, reports: ['Last run'], cll: 'Fusion', snowflake: true, mac: false })));
check('has a title and a text', every.filter(function (s) { return !s.title || !s.text; }).map(function (s) { return s.key; }), []);
// Keys are set apart from code: ⌘` holds a backtick, which as code would
// open a span that runs to the end of the text.
check('closes every brace and backtick it opens, a key holding a backtick included', every.filter(function (s) {
  var rest = s.text.replace(/\{[^{}]*\}/g, '');
  return /[{}]/.test(rest) || rest.split('`').length % 2 === 0;
}).map(function (s) { return s.key; }), []);
check('  the terminal\'s key among them', /\{⌘`\} brings it forward/.test(step(all, 'terminal').text), true);
check('writes no em dash', every.filter(function (s) { return /—/.test(s.title + s.text); }).map(function (s) { return s.key; }), []);
// A renamed id in the page would leave its step pointing at nothing, which
// draws a centred card and says nothing is wrong.
check('points at ids the page has, at least one per step, or at nothing for the welcome',
  all.filter(function (s) {
    if (s.key === 'welcome') return s.target !== '';
    var ids = [].concat(s.target).map(function (sel) { return (/#([\w-]+)/.exec(sel) || [])[1]; }).filter(Boolean);
    return !ids.length || ids.some(function (id) { return page.indexOf('id="' + id + '"') < 0; });
  }).map(function (s) { return s.key; }), []);
check('  the terminal by its tab, never the pane, which would start a shell',
  [step(all, 'terminal').target, /<button class="tab" data-dock="terminal"/.test(page)], ['#dock [data-dock=terminal]', true]);

print('\n--- tourRing ---');
var view = { width: 1400, height: 900 };
check('grows the target by four', tourRing({ left: 100, top: 10, right: 124, bottom: 34 }, view),
  { left: 96, top: 6, width: 32, height: 32 });
check('stays inside the view against its edges', tourRing({ left: 0, top: 500, right: 1400, bottom: 900 }, view),
  { left: 2, top: 496, width: 1396, height: 402 });

print('\n--- tourPlace ---');
var card = { width: 380, height: 160 };
check('below a button, centred on it', tourPlace({ left: 600, top: 6, width: 30, height: 30 }, card, view), { left: 425, top: 48 });
check('  clamped at the right edge', tourPlace({ left: 1360, top: 6, width: 30, height: 30 }, card, view), { left: 1012, top: 48 });
check('above a pane at the bottom', tourPlace({ left: 270, top: 500, width: 1130, height: 360 }, card, view), { left: 645, top: 328 });
check('inside a pane as tall as the window', tourPlace({ left: 270, top: 20, width: 1130, height: 870 }, card, view), { left: 645, top: 718 });
