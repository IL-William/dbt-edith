// Macro calls and the names a properties file declares: what the editor turns
// into links before the server says what each one reaches. Fixtures are invented.
// Run from the repository root: jsc web/tests/macros.js
var src = read('web/app.js');
eval(src.slice(src.indexOf('function maskJinjaComments'), src.indexOf('/* Explicit ref()')));
eval(src.slice(src.indexOf('function yamlIndent'), src.indexOf('/* ATX headings')));
eval(src.slice(src.indexOf('const DECLARING_LISTS'), src.indexOf('async function markRefs')));

function check(label, got, want) {
  var g = JSON.stringify(got), w = JSON.stringify(want);
  print((g === w ? 'PASS  ' : 'FAIL  ') + label + (g === w ? '' : '\n        expected ' + w + '\n        got      ' + g));
}
function calls(text) {
  return scanMacroCalls(maskJinjaComments(text)).map(function (h) { return h.call; });
}
function calls2(text, yaml) {
  return scanMacroCalls(maskJinjaComments(text), yaml).map(function (h) { return h.call; });
}
// Each range read back out of the text, which is what a mark will cover.
function spans(text, hits) {
  var out = [];
  hits.forEach(function (h) { h.ranges.forEach(function (r) { out.push(text.slice(r[0], r[1])); }); });
  return out;
}

print('--- the shapes a call takes ---');
check('a bare call', calls("select {{ cents('amount') }} from x"), ['cents']);
check('a package call', calls("{{ dbt_utils.star(from=ref('orders')) }}"), ['dbt_utils.star', 'ref']);
check('a space before the bracket', calls('{{ cents ("amount") }}'), ['cents']);
check('a call in a set', calls('{% set cols = shop_columns() %}'), ['shop_columns']);
check('a call in an if, and in a for', calls('{% if is_weekday() %}{% for c in shop_columns() %}{% endfor %}{% endif %}'),
  ['is_weekday', 'shop_columns']);
check('a call block names its macro', calls("{% call statement('main', fetch_result=True) %}x{% endcall %}"), ['statement']);
check('a call in an argument', calls('{{ cents(to_decimal(amount)) }}'), ['cents', 'to_decimal']);
check('whitespace control changes nothing', calls('{%- set x = cents(1) -%}{{- cents(2) -}}'), ['cents']);
check('a block over several lines', calls('{{ automate_dv.stage(\n    include_source_columns=true,\n    hashed_columns=hashes()\n) }}'),
  ['automate_dv.stage', 'hashes']);

print('\n--- what is not a call ---');
check('SQL outside the delimiters', calls('select coalesce(a, b), cents(c) from x'), []);
// Two names read like a package and its macro whatever they are: only the
// server knows `cols` is no package, and it answers nothing for these.
check('a method on a variable is still asked about', calls("{% do cols.append('x') %}{{ adapter.dispatch('hub', 'automate_dv')(a) }}"),
  ['cols.append', 'adapter.dispatch']);
check('a method after a call', calls('{{ load_result(x).table.columns(1) }}'), ['load_result']);
check('three names deep', calls('{{ a.b.c(1) }}'), []);
check('a filter', calls("{{ names | join(', ') }}{{ x|default(1) }}"), []);
check('a name with no bracket', calls('{{ this }}{{ target.name }}'), []);
check('the name a macro defines', calls('{% macro cents(amount, scale=2) %}{{ amount }}{% endmacro %}'), []);
check('but a default argument calls', calls('{%- macro cents(amount, scale=default_scale()) -%}{%- endmacro -%}'),
  ['default_scale']);
check('a test and a materialization define too',
  calls("{% test is_even(model, column_name) %}{% endtest %}{% materialization lake, adapter='snowflake' %}"), []);
check('inside a string', calls('{{ log("cents(1) was here") }}'), ['log']);
check('a closer inside a string does not end the block', calls('{{ log("}}") ~ cents(1) }}'), ['log', 'cents']);
check('inside a Jinja comment', calls('{# {{ cents(1) }} #}'), []);
check('inside a raw block', calls('{% raw %}{{ cents(1) }}{% endraw %}{{ fx(2) }}'), ['fx']);
check('a block still being typed', calls('{{ cents(1'), ['cents']);
check('in a YAML hook, the Jinja only', calls('post-hook: "grant(x) {{ grant_select(this) }}"'), ['grant_select']);

print('\n--- where Jinja itself would disagree with a simpler scan ---');
// Jinja opens no comment inside a string, so masking from that {# on cut the
// string in half and read every quote after it the wrong way round.
check('a {# inside a string is text, not a comment',
  calls("{% set no = '{#' %}{{ h() }}{# c #}{% if 'x(' in s %}{{ k() }}{% endif %}"), ['h', 'k']);
check('a real comment after it is still skipped', calls("{% set no = '{#' %}{# {{ gone() }} #}{{ kept() }}"), ['kept']);
check('a {# inside a raw block is not a comment either',
  maskJinjaComments('{% raw %}{# x #}{% endraw %}{# y #}'), '{% raw %}{# x #}{% endraw %}       ');
// Jinja's lexer only ends a block once its brackets are shut.
check('the }} closing a nested dict does not end the block',
  calls("{{ config(meta={'owner': {'team': 'x'}}, post_hook=grant_select('r')) }}"), ['config', 'grant_select']);
check('nor does a %} inside brackets', calls("{% set m = {'a': '%}'} %}{{ after() }}"), ['after']);
// YAML hands Jinja a plain quote where a double-quoted scalar wrote \".
var HOOKS = '    +post-hook: "{{ grant_select(\\"reporting\\") }}"\n    +pre-hook: "{{ set_query_tag() }}"';
check('an escaped quote in a YAML scalar leaves the next hook found', calls2(HOOKS, true), ['grant_select', 'set_query_tag']);
check('and the offsets still land on the names', spans(HOOKS, scanMacroCalls(HOOKS, true)), ['grant_select', 'set_query_tag']);

print('\n--- ranges ---');
var text = "{{ cents(a) }} and {{ automate_dv.hub(b) }} and {{ cents(c) }}";
var hits = scanMacroCalls(text);
check('one entry per call, however often it appears', hits.map(function (h) { return h.call; }),
  ['cents', 'automate_dv.hub']);
check('every appearance is marked', hits[0].ranges.length, 2);
check('a range covers the macro\'s own name, not its package nor the bracket', spans(text, hits), ['cents', 'cents', 'hub']);
var multi = "{# a note\n   over two lines #}\nselect {{ cents(a) }}";
check('offsets survive a masked comment', spans(multi, scanMacroCalls(maskJinjaComments(multi))), ['cents']);

print('\n--- macroDefLine ---');
var FILE = [
  '{# {% macro cents(x) %} a copy kept for reference #}',   // 0
  '{% macro cents_rounded(x) %}',                           // 1
  '  {{ cents(x) }}',                                       // 2
  '{% endmacro %}',                                         // 3
  '',                                                       // 4
  '{%- macro  cents (amount) -%}',                          // 5
  '  {{ amount }} / 100',                                   // 6
  '{%- endmacro -%}',                                       // 7
].join('\n');
check('the definition, not a commented copy nor a longer name', macroDefLine(FILE, 'cents'), { line: 5, ch: 11 });
check('the column is where the name starts', FILE.split('\n')[5].slice(11, 16), 'cents');
check('a name that is only called is not defined here', macroDefLine(FILE, 'amount'), null);
check('the first macro of a file', macroDefLine(FILE, 'cents_rounded'), { line: 1, ch: 9 });
check('a {# inside a string above it does not hide the definition',
  macroDefLine("{% set re = '{#' %}\n{% macro audit() %}{% endmacro %}{# note #}", 'audit'), { line: 1, ch: 9 });

print('\n--- yamlDeclared ---');
var SOURCES = [
  'version: 2',                             // 0
  'sources:',                               // 1
  '  - name: crm',                          // 2
  '    database: "{{ env_var(\'RAW\') }}"',   // 3
  '    tables:',                            // 4
  '      - name: customers',                // 5
  '        identifier: CUSTOMERS_V2',       // 6
  '        columns:',                       // 7
  '          - name: id',                   // 8
  '      - name: "orders"   # the big one', // 9
  '      - description: moved here',        // 10
  '        name: refunds',                  // 11
  '  - tables:',                            // 12
  '      - name: events',                   // 13
  '    name: web',                          // 14
].join('\n');
function declared(text) {
  return yamlDeclared(text).map(function (d) { return d.kind + ':' + d.name; });
}
check('each table, named source.table, and never the source or a column', declared(SOURCES),
  ['source:crm.customers', 'source:crm.orders', 'source:crm.refunds', 'source:web.events']);
check('a range covers the name alone, quotes and comment left out', spans(SOURCES, yamlDeclared(SOURCES)),
  ['customers', 'orders', 'refunds', 'events']);

var MODELS = [
  'models:',                                // 0
  '  - name: stg_customers',                // 1
  '    columns:',                           // 2
  '      - name: customer_id',              // 3
  '        tests:',                         // 4
  '          - relationships:',             // 5
  '              to: ref(\'dim_customers\')', // 6
  'seeds:',                                 // 7
  '- name: country_codes',                  // 8
  'snapshots:',                             // 9
  '  - name: customers_snapshot',           // 10
  'exposures:',                             // 11
  '  - name: weekly_revenue',               // 12
  'macros:',                                // 13
  '  - name: cents',                        // 14
  '    arguments:',                         // 15
  '      - name: amount',                   // 16
].join('\n');
check('models, seeds at their key\'s column, snapshots, exposures and macros', declared(MODELS),
  ['node:stg_customers', 'node:country_codes', 'node:customers_snapshot', 'node:weekly_revenue', 'macro:cents']);

var PROJECT = [
  'name: shop',
  'models:',
  '  shop:',
  '    name: not_a_model',
  '    +materialized: view',
].join('\n');
check('the models: of dbt_project.yml is configs, and declares nothing', declared(PROJECT), []);
check('a names: list nested under something else declares nothing',
  declared('groups:\n  - name: finance\n    owner:\n      name: Ops'), []);
check('a tables: list outside sources: declares nothing',
  declared('exposures:\n  - name: board\n    tables:\n      - name: t\n'), ['node:board']);
check('a name written as Jinja is not a name', declared('models:\n  - name: "{{ var(\'m\') }}"'), []);
check('a table under a source with no name has nothing to be called',
  declared('sources:\n  - tables:\n      - name: t'), []);
check('an item whose dash line is only a comment keeps its list',
  declared('models:\n  - name: stg_orders\n  - # split later\n    name: stg_payments\n'), ['node:stg_orders', 'node:stg_payments']);
check('and so does a table',
  declared('sources:\n  - name: crm\n    tables:\n      - name: customers\n      - # legacy\n        name: orders'),
  ['source:crm.customers', 'source:crm.orders']);

print('\n--- yamlNameAt ---');
check('a plain value', yamlNameAt('  - name: orders', 4), { value: 'orders', from: 10, to: 16 });
check('a trailing comment is cut', yamlNameAt('name: orders  # yes', 0), { value: 'orders', from: 6, to: 12 });
check('a hash inside a word is kept', yamlNameAt('name: orders#2', 0), { value: 'orders#2', from: 6, to: 14 });
check('a tab before the comment cuts it too', yamlNameAt('name: orders\t# legacy', 0), { value: 'orders', from: 6, to: 12 });
check('single quotes', yamlNameAt("name: 'orders'", 0), { value: 'orders', from: 7, to: 13 });
check('a flow collection is no name', yamlNameAt('name: [a, b]', 0), null);
check('nor a block scalar', yamlNameAt('name: |', 0), null);
check('nor nothing', yamlNameAt('name:', 0), null);
