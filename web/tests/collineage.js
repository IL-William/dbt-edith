// Column-mode logic: composite ids and the node subtitle.
// Run from the repository root: jsc web/tests/collineage.js
var app = read('web/app.js');
eval(app.slice(app.indexOf('function splitColId'), app.indexOf('/* Column-level lineage')));

eval(app.slice(app.indexOf('function sidecarLabel'), app.indexOf('function profileLink')));

var lin = read('web/lineage.js');
eval(lin.slice(lin.indexOf('function subtitle(n)'), lin.indexOf('  function init(')));

function check(label, got, want) {
  print((got === want ? 'PASS  ' : 'FAIL  ') + label + (got === want ? '' : '\n        expected ' + want + '\n        got      ' + got));
}

print('--- composite id ---');
var c = splitColId('model.shop.orders::order_id');
check('node extracted', c.node, 'model.shop.orders');
check('column extracted', c.column, 'order_id');

var m = splitColId('model.shop.orders');
check('model id: node', m.node, 'model.shop.orders');
check('model id: no column', m.column, '');

var src = splitColId('source.shop.crm.customers::customer_id');
check('source (several dots)', src.node, 'source.shop.crm.customers');
check('source: column', src.column, 'customer_id');

// A column name may contain a lone colon; only the first "::" separates.
var odd = splitColId('model.x.y::weird:name');
check('column containing a colon', odd.column, 'weird:name');

print('\n--- node subtitle ---');
check('column mode uses sub',
      subtitle({ sub: 'stg_crm__customers  ·  varchar', kind: 'model', materialized: 'view', schema: 'analytics', tests: 3 }),
      'stg_crm__customers  ·  varchar');
check('model mode without sub',
      subtitle({ sub: '', kind: 'model', materialized: 'incremental', schema: 'analytics', tests: 2 }),
      'incremental  ·  analytics  ·  2 tests');
check('source',
      subtitle({ kind: 'source', materialized: 'source', schema: 'raw', tests: 0 }),
      'source  ·  raw');
check('disabled model',
      subtitle({ kind: 'model', disabled: true, materialized: 'view', schema: 'x', tests: 0 }),
      'disabled  ·  x');
check('a single test is singular',
      subtitle({ kind: 'model', materialized: 'table', schema: 's', tests: 1 }),
      'table  ·  s  ·  1 test');

// The hover card passes an /api/node payload, where tests is a list of nodes
// rather than a count. Stringifying it put "[object Object]" on the line.
check('a list of tests is counted, not stringified',
      subtitle({ kind: 'model', materialized: 'incremental', schema: 'dbt_dev',
                 tests: [{ id: 'a' }, { id: 'b' }, { id: 'c' }] }),
      'incremental  ·  dbt_dev  ·  3 tests');
check('a list with one test is singular',
      subtitle({ kind: 'model', materialized: 'view', schema: 's', tests: [{ id: 'a' }] }),
      'view  ·  s  ·  1 test');
check('an empty list adds nothing',
      subtitle({ kind: 'model', materialized: 'view', schema: 's', tests: [] }),
      'view  ·  s');

print('\n--- Snowflake lineage switch ---');
check('no payload reads as off', sidecarLabel(null).text, 'Snowflake lineage: off');
check('off explains that nothing connects before a click',
      sidecarLabel({ enabled: false, state: 'off' }).title.indexOf('nothing connects before the first click') > 0, true);
var ready = sidecarLabel({ enabled: true, state: 'ready', profile: 'shop', target: 'dev', role: 'transformer',
                           python: '/work/shop/.venv/bin/python' });
check('ready is on', ready.text + ' / ' + ready.tone, 'Snowflake lineage: on / on');
check('ready names the connection and the Python that runs',
      ready.title, 'Click a column to fetch its lineage (profile shop, target dev, role transformer).\nPython: /work/shop/.venv/bin/python');
var failed = sidecarLabel({ enabled: true, state: 'failed', error: 'no profiles.yml at /home/me/.dbt/profiles.yml',
                            log: ['Traceback (most recent call last):', 'sf_lineage: no profiles.yml at /home/me/.dbt/profiles.yml'] });
check('failed is its own tone', failed.tone, 'failed');
check('failed shows the error first, then the last lines of the script',
      failed.title, 'no profiles.yml at /home/me/.dbt/profiles.yml\nTraceback (most recent call last):\nsf_lineage: no profiles.yml at /home/me/.dbt/profiles.yml');
check('switched on but not started yet still reads as on',
      sidecarLabel({ enabled: true, state: 'off' }).text, 'Snowflake lineage: on');

print('\n--- the column lineage menu ---');
function cache(file, source, tool, mtime) {
  return { file: file, source: source, tool: tool, target: 'dev', generated_at: '2026-09-20T10:00:00Z', mtime: mtime };
}
// Newest first, as /api/meta sends them.
var found = [
  cache('column_lineage.snowflake.json', 'snowflake', 'snowflake', 300),
  cache('column_lineage.collin.json', 'collin', 'collin', 200),
  cache('column_lineage.json', 'snowflake', 'snowflake', 100),
  cache('column_lineage.synthetic.json', 'synthetic', undefined, 50),
];
var menu = lineageTools(found, 'column_lineage.collin.json', false, true);
check('three tools, in order', menu.tools.map(function (t) { return t.tool; }).join(' '), 'fusion collin snowflake');
check('a tool with no cache is greyed', menu.tools[0].available, false);
check('...and says which file it lacks', menu.tools[0].title.indexOf('column_lineage.fusion.json') >= 0, true);
check('...in the menu too', menu.tools[0].sub, 'no cache');
check('a tool stands for its newest cache', menu.tools[2].src.file, 'column_lineage.snowflake.json');
check('the one on screen is checked', menu.tools.map(function (t) { return t.active; }).join(' '), 'false true false');
check('Snowflake is never greyed while offered', menu.tools[2].available, true);
check('an older cache of a tool and another producer\'s stay pickable',
      menu.others.map(function (o) { return o.src.file; }).join(' '), 'column_lineage.json column_lineage.synthetic.json');
check('another producer is named, never "unknown"', sourceLabel(menu.others[1].src).name, 'synthetic');

var off = lineageTools(found, 'column_lineage.collin.json', false, false);
check('Snowflake off: absent, not greyed', off.tools.map(function (t) { return t.tool; }).join(' '), 'fusion collin');
check('Snowflake off: none of its caches either',
      off.others.map(function (o) { return o.src.file; }).join(' '), 'column_lineage.synthetic.json');

var live = lineageTools([found[1]], 'column_lineage.snowflake.json', true, true);
check('Snowflake picked before its first fetch is checked alone',
      live.tools.map(function (t) { return t.active; }).join(' '), 'false false true');
check('...and says what it does', live.tools[2].sub, 'fetch on click');
check('a cache that names no producer is named by its file',
      sourceLabel({ file: 'column_lineage.nightly.json', source: '', mtime: 0 }).name, 'nightly');

print('\n--- what the menu\'s button says ---');
var none = lineagePickLabel({ cll_edges: 0, cll_file: '' }, '', false, null);
check('nothing loaded', none.name + ' / ' + none.tool, 'column lineage: none / ');
var collinMeta = { cll_edges: 3412, cll_file: '/p/target/column_lineage.collin.json', cll_source: 'collin',
                   cll_target: 'qa', cll_dropped: 2 };
var fromCollin = lineagePickLabel(collinMeta, 'collin', false, null);
check('a tool\'s cache: its name and its count', fromCollin.name + ' · ' + fromCollin.detail, 'Collin · 3412 col edges');
check('...coloured as that tool', fromCollin.tool, 'collin');
check('...and the tooltip names the producer, the file and what was dropped', fromCollin.title,
      'column lineage from collin, target qa\n/p/target/column_lineage.collin.json\n2 row(s) dropped as unknown');
var synth = lineagePickLabel({ cll_edges: 80, cll_file: '/p/target/column_lineage.synthetic.json', cll_source: 'synthetic' }, '', false, null);
check('another producer keeps its own name, and no tool colour', synth.name + ' / ' + synth.tool, 'synthetic / ');
var fresh = lineagePickLabel({ cll_edges: 0, cll_file: '' }, 'snowflake', true, { enabled: true, state: 'ready' });
check('Snowflake picked, nothing fetched yet', fresh.name + ' · ' + fresh.detail + ' / ' + fresh.state, 'Snowflake · fetch on click / ');
var starting = lineagePickLabel({ cll_edges: 12, cll_file: '/p/target/column_lineage.snowflake.json', cll_source: 'snowflake' },
                                'snowflake', true, { enabled: true, state: 'starting' });
check('one edge is one edge',
      lineagePickLabel({ cll_edges: 1, cll_file: '/p/target/column_lineage.collin.json', cll_source: 'collin' }, 'collin', false, null).detail,
      '1 col edge');
check('the script starting', starting.detail + ' / ' + starting.state + ' / ' + starting.tone, '12 col edges / starting / busy');
var broke = lineagePickLabel({ cll_edges: 0, cll_file: '' }, 'snowflake', true,
                             { enabled: true, state: 'failed', error: 'no snowflake-connector-python' });
check('a failed script says so, with the reason in the tooltip',
      broke.state + ' / ' + broke.tone + ' / ' + (broke.title.indexOf('no snowflake-connector-python') === 0), 'failed / failed / true');

print('\n--- why Snowflake is on or off ---');
check('no answer yet says nothing', featureNote(null, 'snowflake'), '');
check('followed from the adapter', featureNote({ snowflake: true, snowflake_set: false }, 'snowflake'),
      'on because the manifest\'s adapter is snowflake');
check('off from the adapter', featureNote({ snowflake: false, snowflake_set: false }, 'postgres'),
      'off because the manifest\'s adapter is postgres');
check('no adapter at all', featureNote({ snowflake: false, snowflake_set: false }, ''), 'off: the manifest names no adapter');
check('chosen', featureNote({ snowflake: false, snowflake_set: true }, 'snowflake'), 'off, as chosen for this project');

print('\n--- each tool has its own colour ---');
var css = read('web/app.css');
function toolColour(tool) {
  var m = css.match(new RegExp('\\[data-tool=' + tool + '\\] \\{ --tool: (#[0-9a-f]{6}); \\}'));
  return m ? m[1] : '';
}
var accent = (css.match(/--accent: (#[0-9a-f]{6});/) || [])[1];
var colours = lineageToolDefs().map(function (d) { return toolColour(d.tool); });
check('every tool has a colour', colours.filter(Boolean).length, 3);
check('no two tools share one', colours.filter(function (c, i) { return colours.indexOf(c) === i; }).length, 3);
check('none is the accent', colours.indexOf(accent), -1);
check('the environments\' colours are another attribute',
      /\[data-tone=(fusion|collin|snowflake)\]/.test(css), false);

print('\n--- what the Columns tab says beside the menu ---');
check('off says nothing: the switch already does', columnsHint({ enabled: false, state: 'off' }, false), null);
check('on with nothing fetched yet tells you what to do',
      columnsHint({ enabled: true, state: 'ready' }, false).text, 'click a column to fetch its lineage from Snowflake');
check('on with lineage already there stays quiet', columnsHint({ enabled: true, state: 'ready' }, true), null);
check('a failed script shows its own message, not a tooltip',
      columnsHint({ enabled: true, state: 'failed', error: 'no profiles.yml at /home/me/.dbt/profiles.yml' }, false).text,
      'no profiles.yml at /home/me/.dbt/profiles.yml');
check('a failed script with no message still says something',
      columnsHint({ enabled: true, state: 'failed', error: '' }, false).text, 'the Snowflake script could not start');
check('waiting on Snowflake says so', columnsHint({ enabled: true, state: 'busy' }, true).tone, 'busy');

print('\n--- what a failed click says ---');
var refused = connectionAdvice('251005: User is empty, but it must be provided', 'connect', '/Users/me/.dbt/profiles.yml');
check('a refused connection quotes Snowflake and points at the profile',
      refused.text + ' | ' + refused.ask + ' | ' + refused.file,
      'Snowflake refused the connection: 251005: User is empty, but it must be provided'
      + ' | check the user and account in | /Users/me/.dbt/profiles.yml');
check('a refused connection with no profile known says nothing about a file',
      connectionAdvice('could not connect', 'connect', '').file, undefined);
check('a query Snowflake rejected is not the profile\'s fault',
      connectionAdvice('Object does not exist', 'query', '/Users/me/.dbt/profiles.yml').file, undefined);
check('...and reads as itself', connectionAdvice('Object does not exist', 'query', '').text,
      'Snowflake: Object does not exist');
check('a request this build got wrong points nowhere',
      connectionAdvice('depth must be a whole number', 'request', '/Users/me/.dbt/profiles.yml').file, undefined);
