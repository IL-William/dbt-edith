// An incremental model's strategy and unique key: the box's second line with
// its struck word, what a screen reader hears of it, the hover card's line,
// and which columns a key names.
// Run from the repository root: jsc web/tests/strategy.js
var lin = read('web/lineage.js');
eval(lin.slice(lin.indexOf('function subtitle(n)'), lin.indexOf('  function init(')));
var Lineage = { adapterName: adapterName };

var app = read('web/app.js');
eval(app.slice(app.indexOf('const KEYED_STRATEGIES'), app.indexOf('/* A second line for HTML')));
eval(app.slice(app.indexOf('function article(word)'), app.indexOf('/* Marks a column for the Columns table')));

function check(label, got, want) {
  var g = JSON.stringify(got), w = JSON.stringify(want);
  print((g === w ? 'PASS  ' : 'FAIL  ') + label + (g === w ? '' : '\n        expected ' + w + '\n        got      ' + g));
}

// A <text> as writeSub fills it, without a DOM: the text it holds and the
// tspans appended to it, each with its class.
function fakeText() {
  return {
    textContent: '', kids: [],
    appendChild: function (k) { this.kids.push(k); },
  };
}
function fakeMake(name) {
  return { name: name, cls: '', textContent: '', setAttribute: function (k, v) { if (k === 'class') this.cls = v; } };
}
function written(p, max) {
  var t = fakeText();
  writeSub(t, p, max, fakeMake);
  return [t.textContent].concat(t.kids.map(function (k) { return (k.cls ? k.cls + ':' : '') + k.textContent; }));
}

var keyless = { kind: 'model', materialized: 'incremental', schema: 'marts', tests: 2,
  strategy_shown: 'append', strategy_named: 'merge', strategy_default: true, strategy_insert: 'snowflake__get_merge_sql' };
var keyed = { kind: 'model', materialized: 'incremental', schema: 'marts', tests: 3,
  strategy_shown: 'merge', strategy_default: true, unique_key: ['customer_id'] };

print('--- the second line ---');
check('the strategy takes the place of incremental', subtitle(keyed), 'merge  ·  marts  ·  3 tests');
check('the struck word in words, for a string', subtitle(keyless), 'append (not merge)  ·  marts  ·  2 tests');
check('in three parts, for what can strike it', subtitleParts(keyless),
  { lead: 'append', struck: 'merge', rest: 'marts  ·  2 tests' });
check('a default that could not be read keeps the materialization',
  subtitle({ kind: 'model', materialized: 'incremental', schema: 's', tests: 0 }), 'incremental  ·  s');
check('a disabled model strikes nothing', subtitleParts({ kind: 'model', disabled: true, materialized: 'incremental',
  strategy_shown: 'append', strategy_named: 'merge' }), { lead: 'disabled', struck: '', rest: '' });
check('column mode keeps its own line', subtitleParts({ sub: 'orders  ·  number', strategy_named: 'merge' }),
  { lead: 'orders  ·  number', struck: '', rest: '' });

print('\n--- the box, which cuts the line from its end ---');
check('the struck word in a tspan of its own', written(subtitleParts(keyless), 34),
  ['append ', 'struck:merge', '  ·  marts  ·  2 tests']);
var long = { lead: 'append', struck: 'merge', rest: 'a_long_schema_name  ·  12 tests' };
check('the test count goes first, never the strategy', written(long, 34),
  ['append ', 'struck:merge', '  ·  a_long_schema_na…']);
check('whole for the export', written(long, 0), ['append ', 'struck:merge', '  ·  a_long_schema_name  ·  12 tests']);
check('a line with nothing struck is clipped as before', written({ lead: 'delete+insert', struck: '', rest: 'a_long_schema_name  ·  12 tests' }, 34),
  ['delete+insert  ·  a_long_schema_n…']);
check('a cut in a separator ends before it', written({ lead: 'append', struck: 'merge', rest: 'jaffle_shop  ·  12 tests' }, 34),
  ['append ', 'struck:merge', '  ·  jaffle_shop…']);
check('the export measures the line as it reads', subLine(subtitleParts(keyless)), 'append merge  ·  marts  ·  2 tests');

print('\n--- what a screen reader hears ---');
check('the strike in words', strategySaid(keyless, 'snowflake'), "append, Snowflake's default merge with no unique key");
check('configured', strategySaid({ strategy_shown: 'append', strategy_named: 'delete+insert' }, 'snowflake'),
  'append, configured delete+insert with no unique key');
check('the default, named', strategySaid(keyed, 'snowflake'), "Snowflake's default merge");
check('configured and plain', strategySaid({ strategy_shown: 'merge' }, 'snowflake'), 'merge');
check('nothing on a view', strategySaid({ kind: 'model', materialized: 'view' }, 'snowflake'), '');
check('an adapter as its makers write it', [adapterName('duckdb'), adapterName('bigquery'), adapterName('materialize'), adapterName('')],
  ['DuckDB', 'BigQuery', 'Materialize', 'the adapter']);

print('\n--- the hover card ---');
function facts(n, adapter) { return strategyFacts(n, adapter || 'snowflake'); }
check('an insert, worded by the macro that inserts', facts(keyless), [{
  text: "append in effect: merge, Snowflake's default, with no unique key, so Snowflake runs a plain insert into", warn: true }]);
check("dbt's own merge, configured", facts({ kind: 'model', materialized: 'incremental', strategy_shown: 'append',
  strategy_named: 'merge', strategy_insert: 'default__get_merge_sql' }, 'postgres'),
  [{ text: 'append in effect: merge with no unique key, on FALSE, so every row is inserted', warn: true }]);
check('a delete+insert that deletes nothing', facts({ kind: 'model', materialized: 'incremental', strategy_shown: 'append',
  strategy_named: 'delete+insert', strategy_insert: 'default__get_delete_insert_merge_sql' }),
  [{ text: 'append in effect: delete+insert with no unique key, so nothing is deleted before the insert', warn: true }]);
check('the key it works on, and where the strategy comes from', facts(keyed),
  [{ text: "merge on customer_id, Snowflake's default" }]);
check('a key of several columns', facts({ kind: 'model', materialized: 'incremental', strategy_shown: 'delete+insert',
  unique_key: ['order_id', 'line_number'] }), [{ text: 'delete+insert on order_id, line_number' }]);
check('no key and no known macro', facts({ kind: 'model', materialized: 'incremental', strategy_shown: 'merge' }, 'duckdb'),
  [{ text: 'merge, no unique key' }]);
check('an append says nothing of a key', facts({ kind: 'model', materialized: 'incremental', strategy_shown: 'append',
  unique_key: ['id'] }), [{ text: 'append' }]);
check('a default that could not be read says so', facts({ kind: 'model', materialized: 'incremental' }),
  [{ text: "Snowflake's default strategy, which could not be read from its macros", muted: true }]);
check('nothing for a view that inherited one', facts({ kind: 'model', materialized: 'view', strategy: 'merge' }), []);

print('\n--- which columns a key names ---');
var cols = [{ name: 'ORDER_ID' }, { name: 'line_number' }, { name: 'Amount' }];
check('ignoring case and quotes, in the key\'s order',
  keyColumns(['line_number', '"order_id"', "order_id || '-' || line_number", 'missing'], cols).map(function (k) {
    return [k.entry, k.column && k.column.name];
  }),
  [['line_number', 'line_number'], ['"order_id"', 'ORDER_ID'], ["order_id || '-' || line_number", null], ['missing', null]]);
check('names compared as Snowflake compares them', [columnKey('`Amount`'), columnKey(' [x] '), columnKey('"a b"')],
  ['amount', 'x', 'a b']);
check('no key, no columns', keyColumns(undefined, cols), []);
check('a view, an ephemeral', [article('view'), article('ephemeral')], ['a view', 'an ephemeral']);
