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

print('\n--- collin, run on demand ---');
var onlySnow = [found[0]];
var absent = lineageTools(onlySnow, 'column_lineage.snowflake.json', false, true, { found: false, state: 'idle' });
check('collin not installed is still clickable', absent.tools[1].available, true);
check('...and installs rather than picks', absent.tools[1].install, true);
check('...and says so', absent.tools[1].sub, 'install');
check('...with the install in its tooltip', absent.tools[1].title.indexOf('installs it into the terminal') > 0, true);
check('no collin payload at all reads as not installed',
      lineageTools(onlySnow, '', false, true).tools[1].install, true);
var installed = lineageTools(onlySnow, 'column_lineage.snowflake.json', false, true, { found: true, state: 'idle' });
check('collin installed with no cache runs on pick', installed.tools[1].sub + ' / ' + installed.tools[1].install,
      'runs on pick / false');
var pending = lineageTools(onlySnow, 'column_lineage.collin.json', false, true, { found: true, state: 'running' });
check('collin picked before its cache exists is checked', pending.tools[1].active, true);
check('...and says it is running', pending.tools[1].sub, 'running');
check('a collin cache is picked, not installed, even with collin gone',
      lineageTools(found, '', false, true, { found: false }).tools[1].install, false);

var noCacheYet = lineagePickLabel({ cll_edges: 0, cll_file: '' }, 'collin', false, null, { state: 'running' });
check('the button names Collin while its first run goes on',
      noCacheYet.name + ' / ' + noCacheYet.state + ' / ' + noCacheYet.tone, 'Collin / running / busy');
var rerun = lineagePickLabel({ cll_edges: 40, cll_file: '/p/target/column_lineage.collin.json', cll_source: 'collin' },
                             'collin', false, null, { state: 'running' });
check('a rerun keeps the count on screen and says it is running', rerun.detail + ' / ' + rerun.state, '40 col edges / running');
var firstFailed = lineagePickLabel({ cll_file: '' }, 'collin', false, null, { state: 'failed', error: 'no manifest' });
check('a failed first run says failed, with its reason in the tooltip',
      firstFailed.state + ' / ' + firstFailed.tone + ' / ' + firstFailed.title, 'failed / failed / no manifest');
check('Collin picked, nothing written yet and no run going says so',
      lineagePickLabel({ cll_file: '' }, 'collin', false, null, { state: 'idle' }).detail, 'no cache');
var rerunFailed = lineagePickLabel({ cll_edges: 40, cll_file: '/p/target/column_lineage.collin.json', cll_source: 'collin' },
                                   'collin', false, null, { state: 'failed', error: 'collin took longer than 600 s and was stopped' });
check('a failed rerun keeps the edges on screen and says it failed', rerunFailed.detail + ' / ' + rerunFailed.state,
      '40 col edges / failed');
check('...and why', rerunFailed.title.indexOf('The last run failed: collin took longer than 600 s') > 0, true);
check('another producer is not marked alpha',
      lineagePickLabel({ cll_edges: 3, cll_file: '/p/target/column_lineage.x.json', cll_source: 'x' }, '', false, null).badge, '');

check('another tool picked: collin says nothing', collinHint('fusion', { found: true, stale: true }), null);
check('running', collinHint('collin', { state: 'running' }).tone, 'busy');
check('failed shows collin\'s reason', collinHint('collin', { state: 'failed', error: 'no manifest' }).text, 'collin: no manifest');
check('behind the manifest says the next click runs it',
      collinHint('collin', { state: 'done', found: true, stale: true }).text,
      'older than the manifest: the next column click runs collin again');
var level = collinHint('collin', { state: 'done', found: true, stale: false },
                       [{ kind: 'rename', edges: 30 }, { kind: 'inferred', edges: 10 }]);
check('level with the manifest says it is alpha, and how much is a guess',
      level.text + ' / ' + level.tone, 'alpha, can be wrong · 25% guessed by name / alpha');
check('...with the caveat in its tooltip', level.title.indexOf('Alpha: collin is new and can be wrong.') === 0, true);
check('nothing inferred: alpha alone', collinHint('collin', { found: true }, [{ kind: 'cast', edges: 4 }]).text,
      'alpha, can be wrong');
check('a sliver inferred never reads as 0%',
      collinHint('collin', { found: true }, [{ kind: 'rename', edges: 999 }, { kind: 'inferred', edges: 1 }]).text,
      'alpha, can be wrong · 1% guessed by name');

var unread = { found: true, state: 'done', report: { models: 3248, parsed: 0,
  reasons: [{ why: 'no compiled_code and no compiled file', models: 3248 }] } };
var told = collinHint('collin', unread, [{ kind: 'inferred', edges: 68192 }]);
check('a run that read no SQL says so in the Columns tab', told.text, 'alpha, can be wrong · SQL read for 0 of 3248 models');
check('...and why, with what to do, in its tooltip', told.title.split('\n').slice(3).join(' | '),
      'collin read the SQL of 0 of 3248 models. | 3248 models: no compiled_code and no compiled file'
      + ' | Run dbt compile so target/compiled/ holds their SQL, then Regenerate.');
check('every model read: back to the share of edges',
      collinHint('collin', { found: true, report: { models: 4, parsed: 4 } }, [{ kind: 'rename', edges: 3 }, { kind: 'inferred', edges: 1 }]).text,
      'alpha, can be wrong · 25% guessed by name');
check('a reason with no remedy here gets no advice',
      collinReadLines({ models: 2, parsed: 1, reasons: [{ why: 'sql parser error: Expected ), found: EOF', models: 1 }] }).join(' | '),
      'collin read the SQL of 1 of 2 models. | 1 model: sql parser error: Expected ), found: EOF');
check('no report, nothing to add', collinReadLines(undefined).length, 0);
check('the button\'s tooltip carries the report too',
      lineagePickLabel({ cll_edges: 68192, cll_file: '/p/target/column_lineage.collin.json', cll_source: 'collin',
                         cll_kinds: [{ kind: 'inferred', edges: 68192 }] }, 'collin', false, null, unread)
        .title.indexOf('collin read the SQL of 0 of 3248 models.') > 0, true);

print('\n--- SQL read from target/compiled/ (collin 0.2.0) ---');
// The figures of collin's own measurement: a parse-only manifest, and files
// compiled for another target than it.
var foreign = { found: true, report: { models: 3248, parsed: 3238, from_files: 3238, set_aside: 1815,
  reasons: [{ why: 'no compiled_code and no compiled file', models: 10 }] } };
var aside = collinHint('collin', foreign, [{ kind: 'rename', edges: 9 }, { kind: 'inferred', edges: 1 }]);
check('a set aside file is not counted as read', aside.text, 'alpha, can be wrong · SQL read for 1423 of 3248 models');
check('...the tooltip says why, then what came from files, then what to do', aside.title.split('\n').slice(3).join(' | '),
      'collin read the SQL of 1423 of 3248 models. | 10 models: no compiled_code and no compiled file'
      + ' | 1815 models: a file in target/compiled/ that reads a table the manifest does not give the model,'
      + ' compiled for another target or before a ref moved, so set aside'
      + ' | 1423 models read from target/compiled/, as the manifest has no compiled SQL for them:'
      + ' a file there can be older than the manifest.'
      + ' | Run dbt compile with the target the manifest was written for, then Regenerate.');
var own = { models: 3472, parsed: 3472, from_files: 3470 };
check('every model read, some from files: the button gives the edges\' share',
      collinHint('collin', { found: true, report: own }, [{ kind: 'rename', edges: 3 }]).text, 'alpha, can be wrong');
check('...and the tooltip still says where the SQL came from', collinReadLines(own).join(' | '),
      '3470 models read from target/compiled/, as the manifest has no compiled SQL for them: a file there can be older than the manifest.');
check('nothing from files, every model read: nothing to add', collinReadLines({ models: 4, parsed: 4 }).length, 0);
check('a set aside file over its model',
      collinModelNote({ provenance: 'inferred', sql_file: 'compiled/shop/models/a.sql', set_aside: true }),
      "collin set aside this model's compiled file, target/compiled/shop/models/a.sql: it reads a table the manifest"
      + ' does not give this model, so it was compiled for another target or before a ref moved.'
      + ' Its edges are matched by column name.');
check('a model read from a file, with another fault, says both',
      collinModelNote({ provenance: 'per_column', sql_file: 'compiled/shop/models/b.sql' }),
      "This model's table is behind its compiled SQL: collin read the columns both have and matched the others by name."
      + ' collin read its SQL from target/compiled/shop/models/b.sql, as the manifest has none:'
      + ' that file can be older than the manifest.');
check('a model read from a file, nothing else to say',
      collinModelNote({ provenance: 'parsed', sql_file: 'compiled/shop/models/c.sql' }).indexOf('collin read its SQL from') === 0, true);
check('a parse error comes first, a file or not',
      collinModelNote({ provenance: 'inferred', parse_error: 'sql parser error', sql_file: 'compiled/shop/models/d.sql' })
        .indexOf('collin could not read this model') === 0, true);

print('\n--- what collin says about a model\'s columns ---');
var colNote = { provenance: 'parsed', columns: [
  { column: 'total', kind: 'lost', detail: 'phantom', names: ['base'] },
  { column: 'total', kind: 'lost', detail: 'phantom', names: ['base'] },
  { column: 'old_flag', kind: 'missing' },
  { column: 'new_flag', kind: 'unexpected' },
  { column: 'new_code', kind: 'unexpected' },
  { column: 'amount', kind: 'unbacked', detail: 'final', names: ['stg_payments'] },
  { column: 'status', kind: 'read_downstream', detail: 'warehouse',
    names: ['model.shop.b', 'model.shop.c', 'model.shop.d', 'model.shop.e', 'model.shop.f'], of: 7 },
  { column: 'id', kind: 'ambiguous', detail: 'every', names: ['model.shop.p', 'model.shop.q'] },
] };
var byCol = collinColumnNotes(colNote);
check('one entry per column, the same thing said once', byCol.get('total').length, 1);
check('a phantom says which CTE', byCol.get('total')[0],
      "collin lost this column's lineage: it is copied from the CTE base, which does not have it,"
      + ' the shape of a select * over a parent whose column list lacks it.');
check('names unreached', collinColumnText({ kind: 'lost', detail: 'names_unreached', names: ['x', 'y'] }),
      "collin lost this column's lineage: its expression reads x, y, which collin never placed.");
check('an unbacked read names the CTE and who has the name', byCol.get('amount')[0],
      'Read from the CTE final, which does not have it, so this compile cannot run as it is:'
      + ' usually a macro that read the warehouse at compile time and found a relation missing. stg_payments under it has the name.');
check('a downstream read names models, not ids, and how many more', byCol.get('status')[0],
      "Read by b, c, d, e, f and 2 more downstream, but absent from the warehouse's column list collin had for this model.");
check('an every-parent match says to check it', byCol.get('id')[0],
      'Matched by name to each of p, q: right for a union, wrong for a lookup join. Check it in the SQL.');
check('a kind from a later collin still says something',
      collinColumnText({ kind: 'novel' }), 'collin noted novel on this column, in target/collin.report.json.');
var tableCols = [{ name: 'Total' }, { name: 'old_flag' }, { name: 'amount' }, { name: 'status' }, { name: 'id' }];
check('the note points at the marks and names, by cause, the columns with no row',
      collinModelNote(colNote, tableCols),
      'collin flags 5 columns of this model, marked ! in the Lineage column.'
      + ' It also flags 2 columns this table has no row for. new_flag, new_code, each: Produced by the compiled SQL,'
      + ' but not in the warehouse: the table has not been rebuilt since the code changed.');
check('the model\'s own note comes first',
      collinModelNote({ provenance: 'per_column', columns: [{ column: 'a', kind: 'missing' }] }, [{ name: 'a' }]).indexOf("This model's table is behind") === 0, true);
check('no column notes, no extra sentence', collinModelNote({ provenance: 'parsed' }, tableCols), null);
check('no note at all', collinColumnNotes(undefined).size, 0);
var unparsed = collinColumnNotes({ provenance: 'inferred', parse_error: 'sql parser error',
  columns: [{ column: 'a', kind: 'missing' }, { column: 'b', kind: 'ambiguous', detail: 'order', names: ['model.shop.p'] }] });
check('an unparsed compile flags no column as missing from it', [...unparsed.keys()].join(' '), 'b');

print('\n--- updating collin ---');
check('no collin, no update row', collinUpdateRow({ found: false }, 'collin'), null);
check('another tool picked, no update row', collinUpdateRow({ found: true, outdated: true }, 'snowflake'), null);
var old = collinUpdateRow({ found: true, outdated: true, wants: '0.2.0', update: 'cargo install --force x', install: 'cargo install x' }, 'collin');
check('a collin with no version is offered the update, forced, as urgent', old.label + ' / ' + old.sub + ' / ' + old.command + ' / ' + old.urgent,
      'Update collin / needs 0.2.0 / cargo install --force x / true');
check('...and told why', old.title.indexOf('gives no version, so it is older than 0.2.0') > 0, true);
var older = collinUpdateRow({ found: true, outdated: true, version: '0.1.5 (abc)', wants: '0.2.0', update: 'u', install: 'i' }, 'collin');
check('an older version names both', older.sub + ' / ' + (older.title.indexOf('is 0.1.5 (abc).') > 0), '0.1.5, needs 0.2.0 / true');
var current = collinUpdateRow({ found: true, outdated: false, version: '0.2.0 (756b126)', wants: '0.2.0', update: 'u', install: 'i' }, 'collin');
check('a recent collin is offered a check, not forced, and not urgent', current.label + ' / ' + current.sub + ' / ' + current.command + ' / ' + current.urgent,
      'Check for a newer collin / 0.2.0 / i / false');
check('an old collin is said in the Columns tab before anything else',
      collinHint('collin', { found: true, outdated: true, version: '', wants: '0.2.0', stale: true }).text,
      'this collin gives no version, so it is older than 0.2.0: the column lineage menu updates it');
check('...or its version when it gives one',
      collinHint('collin', { found: true, outdated: true, version: '0.1.5 (abc)', wants: '0.2.0' }).text,
      'collin 0.1.5 is older than 0.2.0: the column lineage menu updates it');

print('\n--- what the Columns tab says over one model ---');
check('a model not in the report says nothing', collinModelNote(undefined), null);
check('its SQL not read, and why',
      collinModelNote({ provenance: 'inferred', parse_error: 'no compiled_code and no compiled file' }),
      "collin could not read this model's SQL: no compiled_code and no compiled file. Its edges are matched by column name.");
check('a compile set aside', collinModelNote({ provenance: 'inferred' }),
      "collin did not trust this model's compiled SQL: its edges are matched by column name.");
check('a table behind its compile',
      collinModelNote({ provenance: 'per_column' }).indexOf('read the columns both have and matched the others by name') > 0, true);
check('read, with issues', collinModelNote({ provenance: 'parsed', issues: 2 }),
      'collin noted 2 issues reading this model, listed in target/collin.report.json.');
check('read, nothing to say', collinModelNote({ provenance: 'parsed' }), null);

print('\n--- how much of collin\'s lineage is a guess ---');
check('no kinds, nothing counted', inferredShare(undefined).total, 0);
var share = inferredShare([{ kind: 'passthrough', edges: 7 }, { kind: 'inferred', edges: 3 }]);
check('inferred out of all', share.inferred + '/' + share.total, '3/10');
check('every edge a name match says so',
      collinCaveat([{ kind: 'inferred', edges: 68192 }]).split('\n')[1],
      'None of the 68192 edges was read from the SQL: every one is a match by column name.');
check('some of them', collinCaveat([{ kind: 'rename', edges: 5 }, { kind: 'inferred', edges: 2 }]).split('\n')[1],
      '2 of the 7 edges are a match by column name, not read from the SQL.');
check('none of them: the alpha line alone', collinCaveat([{ kind: 'rename', edges: 5 }]).split('\n').length, 1);
check('Collin and Snowflake are alpha in the menu, Fusion is not',
      lineageTools(found, '', false, true, { found: true }).tools.map(function (t) { return t.alpha; }).join(' '),
      'false true true');

print('\n--- what the menu\'s button says ---');
var none = lineagePickLabel({ cll_edges: 0, cll_file: '' }, '', false, null);
check('nothing loaded', none.name + ' / ' + none.tool, 'column lineage: none / ');
var collinMeta = { cll_edges: 3412, cll_file: '/p/target/column_lineage.collin.json', cll_source: 'collin',
                   cll_target: 'qa', cll_dropped: 2 };
var fromCollin = lineagePickLabel(collinMeta, 'collin', false, null);
check('a tool\'s cache: its name and its count', fromCollin.name + ' · ' + fromCollin.detail, 'Collin · 3412 col edges');
check('...coloured as that tool', fromCollin.tool, 'collin');
check('...marked alpha', fromCollin.badge, 'alpha');
check('...and the tooltip says it can be wrong before naming the producer, the file and what was dropped',
      fromCollin.title,
      'Alpha: collin is new and can be wrong. Check an edge against the compiled SQL before relying on it.\n\n'
      + 'column lineage from collin, target qa\n/p/target/column_lineage.collin.json\n2 row(s) dropped as unknown');
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
