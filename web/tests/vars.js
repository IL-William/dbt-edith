// var() and env_var() calls in a model, and the line that says where a value
// came from. Fixtures are invented.
// Run from the repository root: jsc web/tests/vars.js
var src = read('web/app.js');
eval(src.slice(src.indexOf('function maskJinjaComments'), src.indexOf('/* Explicit ref()'))
  + src.slice(src.indexOf('const VAR_RE'), src.indexOf('/* automate_dv declares parents')));
eval(src.slice(src.indexOf('function yamlIndent'), src.indexOf('/* ATX headings')));

function check(label, got, want) {
  var g = JSON.stringify(got), w = JSON.stringify(want);
  print((g === w ? 'PASS  ' : 'FAIL  ') + label + (g === w ? '' : '\n        expected ' + w + '\n        got      ' + g));
}
function scan(text) { return scanVars(maskJinjaComments(text)); }
function found(text) {
  return scan(text).map(function (h) { return h.kind + ':' + h.name + (h.fallback ? '/' + h.fallback : ''); });
}

print('--- the four shapes ---');
check('var with one argument', found("select {{ var('prefix') }}"), ['var:prefix']);
check('var with a default', found("{{ var('days', 7) }}"), ['var:days/7']);
check('env_var with one argument', found("{{ env_var('DBT_WH') }}"), ['env_var:DBT_WH']);
check('env_var with a default', found("{{ env_var('DBT_TS', '1900-01-01') }}"), ["env_var:DBT_TS/'1900-01-01'"]);

print('\n--- quoting and spacing ---');
check('double quotes', found('{{ var("prefix") }}'), ['var:prefix']);
check('a space before the bracket', found("{{ var ('prefix') }}"), ['var:prefix']);
check('space inside the brackets', found("{{ var(  'prefix'  ) }}"), ['var:prefix']);

print('\n--- what must not match ---');
check('a namespaced macro is not dbt var()', found("{{ dbt_utils.var('x') }}"), []);
check('a macro whose name ends in var', found("{{ my_var('x') }}"), []);
check('ref is not a variable', found("{{ ref('orders') }}"), []);
check('source is not a variable', found("{{ source('raw', 'orders') }}"), []);
check('a call inside a jinja comment', found("{# {{ var('gone') }} #}"), []);
check('only the commented one is skipped',
  found("{# {{ var('gone') }} #}\nselect {{ var('kept') }}"), ['var:kept']);

print('\n--- ranges ---');
var text = "select {{ var('prefix') }} from {{ env_var('DBT_WH') }}";
var hits = scan(text);
check('two hits on one line', hits.length, 2);
// The whole call is the hover target, and a var()'s click target: a range
// covering only the quoted name is something you have to aim at.
check('the range covers the whole call', text.slice(hits[0].ranges[0][0], hits[0].ranges[0][1]), "var('prefix')");
check('and for the second call', text.slice(hits[1].ranges[0][0], hits[1].ranges[0][1]), "env_var('DBT_WH')");
check('the ranges do not overlap', hits[0].ranges[0][1] <= hits[1].ranges[0][0], true);
check('a call with a default is covered to its closing bracket',
  (function () { var t = "{{ var('days', 7) }}"; var h = scan(t)[0]; return t.slice(h.ranges[0][0], h.ranges[0][1]); })(),
  "var('days', 7)");

print('\n--- offsets survive masked comments and newlines ---');
var multi = "{# a note\n   over two lines #}\nwith x as (\n  select {{ var('prefix') }}\n)";
var one = scan(multi)[0];
check('the call is still found at its real offset', multi.slice(one.ranges[0][0], one.ranges[0][1]), "var('prefix')");

print('\n--- varNote ---');
check('read from the chosen file',
  varNote({ status: 'env', vars: ['DBT_WH'] }, '.env.dev'), 'read from .env.dev');
check('a default is not a file',
  varNote({ status: 'env', default_used: true, vars: ['DBT_TS'] }, '.env.dev'),
  'the default written in the call, not from a file');
check('missing names the file',
  varNote({ status: 'missing', vars: ['DBT_LAG'] }, '.env.dev'), 'DBT_LAG is not set in .env.dev');
check('missing with no environment selected',
  varNote({ status: 'missing', vars: ['DBT_LAG'] }, ''),
  'DBT_LAG needs an environment, and none is selected');
check('a placeholder is called one',
  varNote({ status: 'placeholder', vars: ['DBT_DB'] }, '.env.dev'),
  'DBT_DB is a placeholder in .env.dev, not a real value');
check('unevaluated covers the secret case',
  varNote({ status: 'unevaluated', vars: [] }, '.env.dev'),
  'not evaluated: a secret, or Jinja this does not read');
check('a redacted name never says the value, but does say it is set',
  varNote({ status: 'env', redacted: 'name', vars: ['SF_PW'] }, '.env.dev'),
  'set in .env.dev, hidden because the name reads as a credential');
check('a redacted and unset name says only that it is hidden',
  varNote({ status: 'missing', redacted: 'name', vars: ['SF_PW'] }, '.env.dev'),
  'hidden because the name reads as a credential');
check('a value hidden for what it carries says so, not that the name gave it away',
  varNote({ status: 'env', redacted: 'value', vars: ['REPORTS_URL'] }, '.env.dev'),
  'set in .env.dev, hidden because the value carries a password');
check('a plain literal has nothing to add', varNote({ status: 'literal', vars: [] }, ''), '');

print('\n--- varDefLine: where a click on var() lands ---');
var project = [
  "name: 'shop'",
  "version: '1.0.0'",
  '',
  'vars:',
  '  # the start of the history',
  "  start_date: '2020-01-01'",
  '  days: 7',
  '  "quoted name": x',
  '  regions:',
  '    - eu',
  '    - us',
  '  shop_pkg:',
  '    days: 30',
  '  empty:',
  '',
  'models:',
  '  shop:',
  '    days: 1',
].join('\n');
check('a plain key, at its column', varDefLine(project, 'start_date'), { line: 5, ch: 2 });
check('a number value', varDefLine(project, 'days'), { line: 6, ch: 2 });
check('a quoted key, unquoted', varDefLine(project, 'quoted name'), { line: 7, ch: 2 });
check('a list var, at its key', varDefLine(project, 'regions'), { line: 8, ch: 2 });
check('a var with no value', varDefLine(project, 'empty'), { line: 13, ch: 2 });
// Without the global key, `days` is left under a package's scope and under
// models:, neither of which is the var the card shows.
check('a name under a package scope or models: alone is not found',
  varDefLine(project.replace('  days: 7\n', ''), 'days'), null);
check('a list item is not a key', varDefLine(project, 'eu'), null);
check('a name the block lacks', varDefLine(project, 'nowhere'), null);
check('no vars: block at all', varDefLine("name: 'shop'\nmodels:\n  shop:\n    days: 1", 'days'), null);
check('a vars key nested elsewhere is not the block',
  varDefLine("models:\n  vars:\n    days: 1", 'days'), null);
check('two keys of one name: the last, which YAML keeps and the card reads',
  varDefLine('vars:\n  days: 7\n  days: 9', 'days'), { line: 2, ch: 2 });
check('four spaces of indent', varDefLine('vars:\n    days: 7', 'days'), { line: 1, ch: 4 });

print('\n--- varLanding: the buffer, the disk only while they agree ---');
check('a clean buffer lands on its key', varLanding(project, 'days', { line: 99 }), { line: 6, ch: 2 });
check('a key the outline cannot place lands on the line the server read',
  varLanding('vars: {days: 7}', 'days', { line: 1 }), { line: 0, ch: 0 });
// An unsaved buffer is passed with no row: the disk no longer describes it.
check('an unsaved buffer with the key renamed lands nowhere',
  varLanding(project.replace('  days: 7', '  lookback_days: 7'), 'days', null), null);
check('an unsaved buffer with a key added finds it',
  varLanding(project.replace('  days: 7', '  days: 7\n  added: 1'), 'added', null), { line: 7, ch: 2 });

print('\n--- linkUnder: which link a click or a pause lands on ---');
function classes(list) { return { contains: function (c) { return list.indexOf(c) >= 0; } }; }
check('a var() alone', linkUnder(classes(['cm-varlink'])), 'var');
check('an env_var() alone has its card and nothing to open', linkUnder(classes(['cm-varlink', 'env'])), 'env');
check("a ref() inside a var()'s default is the ref", linkUnder(classes(['cm-varlink', 'cm-reflink'])), 'ref');
check("a macro call inside a var()'s default is the macro", linkUnder(classes(['cm-varlink', 'cm-macrolink'])), 'macro');
check("and inside an env_var()'s", linkUnder(classes(['cm-varlink', 'env', 'cm-reflink'])), 'ref');
check('a folder key', linkUnder(classes(['cm-dirlink'])), 'dir');
check('plain text', linkUnder(classes(['cm-string'])), '');
check('no classes at all', linkUnder(undefined), '');
var defaulted = "{{ var('payments_table', ref('stg_payments')) }}";
var both = scan(defaulted)[0].ranges[0];
var name = defaulted.indexOf('stg_payments');
check("the var mark does cover the ref's name, which is why the order matters",
  both[0] < name && name + 'stg_payments'.length <= both[1], true);

print('\n--- outsideVarNames: a var\'s own name is the var\'s ---');
var both2 = "select * from {{ ref('orders') }} where {{ var('orders') }} > 1 and x = {{ var('t', ref('orders')) }}";
function at(word, n) { var i = -1; for (var k = 0; k <= n; k++) i = both2.indexOf(word, i + 1); return [i, i + word.length]; }
var vocabHits = [{ name: 'orders', ranges: [at('orders', 0), at('orders', 1), at('orders', 2)] }];
check('the name inside var() quotes is left to the var',
  outsideVarNames(both2, vocabHits)[0].ranges, [at('orders', 0), at('orders', 2)]);
check('a hit with no range left is dropped', outsideVarNames("{{ var('orders') }}", [{ name: 'orders', ranges: [[8, 14]] }]), []);
check('the name is where the quotes are', scan("{{ var( 'days' ) }}")[0].nameAt, [9, 13]);

