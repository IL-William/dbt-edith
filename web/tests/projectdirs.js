// The folder keys of dbt_project.yml: which key names a folder, and where its
// own text sits in its line (0036). The server places the chains; nothing here
// touches a disk. Fixtures are invented.
// Run from the repository root: jsc web/tests/projectdirs.js
var src = read('web/app.js');
eval(src.slice(src.indexOf('function yamlIndent'), src.indexOf('/* ATX headings')));
eval(src.slice(src.indexOf('const DECLARING_LISTS'), src.indexOf('async function markRefs')));

function check(label, got, want) {
  var g = JSON.stringify(got), w = JSON.stringify(want);
  print((g === w ? 'PASS  ' : 'FAIL  ') + label + (g === w ? '' : '\n        expected ' + w + '\n        got      ' + g));
}
// What the server would be asked about, `block: a > b`, in document order.
function chains(text) {
  return projectPathKeys(text).map(function (k) {
    return k.block + (k.chain.length ? ': ' + k.chain.join(' > ') : '');
  });
}
// Each range read back out of the text, which is what a mark will cover.
function spans(text) {
  return projectPathKeys(text).map(function (k) { return text.slice(k.ranges[0][0], k.ranges[0][1]); });
}

var PROJECT = [
  "name: 'jaffle_shop'",
  '',
  'model-paths: ["models"]',
  '',
  'seeds:',
  "  +tags: ['raw']",
  '  +docs:',
  "    node_color: '#cd7f32'",
  '',
  'models:',
  '  jaffle_shop:',
  '    +materialized: table',
  '    staging:',
  "      +tags: ['staging']",
  '      +docs:',
  "        node_color: 'silver'",
  '',
].join('\n');

print('--- the project file, as a project writes it ---');
check('a block, its project level and the folders under it', chains(PROJECT),
  ['seeds', 'models', 'models: jaffle_shop', 'models: jaffle_shop > staging']);
check('and each key is marked by its own name', spans(PROJECT), ['seeds', 'models', 'jaffle_shop', 'staging']);
// A `+docs:` holds a mapping, so only the chain rule keeps node_color out.
check('nothing under a plus config', chains('models:\n  shop:\n    +docs:\n      node_color: gold\n'),
  ['models', 'models: shop']);

print('\n--- a config is not a folder, by dbt\'s rule (0039) ---');
// dbt-core and Fusion alike: a key holding a mapping is a path segment unless
// it starts with a plus, whatever its name, and a config otherwise.
check('a bare config with a value', chains('models:\n  shop:\n    materialized: table\n    schema: marts\n'),
  ['models', 'models: shop']);
check('a bare config holding a list', chains("models:\n  shop:\n    tags: ['a', 'b']\n"), ['models', 'models: shop']);
// dbt looks for a folder named docs, and applies node_color to nothing, which
// Fusion refuses outright: the key is a folder, dashed when there is none.
check('a bare config holding a mapping is a folder to dbt', chains('models:\n  shop:\n    docs:\n      node_color: gold\n'),
  ['models', 'models: shop', 'models: shop > docs']);
check('every config name holding a mapping, without its plus', chains([
  'models:', '  shop:',
  '    docs:', '      node_color: gold',
  '    meta:', '      owner: team',
  '    grants:', '      select: [a]',
  '    persist_docs:', '      relation: true',
  '    quoting:', '      database: false',
  '    contract:', '      enforced: true',
  '    snapshot_meta_column_names:', '      dbt_valid_to: valid_to',
  '    column_types:', '      id: varchar',
  '',
].join('\n')), ['models', 'models: shop', 'models: shop > docs', 'models: shop > meta', 'models: shop > grants',
  'models: shop > persist_docs', 'models: shop > quoting', 'models: shop > contract',
  'models: shop > snapshot_meta_column_names', 'models: shop > column_types']);
check('and none of them with it', chains([
  'models:', '  shop:', '    +docs:', '      node_color: gold', '    +meta:', '      owner: team',
  '    +grants:', '      select: [a]', '    +contract:', '      enforced: true', '',
].join('\n')), ['models', 'models: shop']);
// The case that showed the old name list wrong: a folder named contract, its
// +tags applied by dbt-core 1.11 and Fusion 2.0.6 both.
check('a folder named after a config', chains('models:\n  shop:\n    contract:\n      +tags: [c]\n'),
  ['models', 'models: shop', 'models: shop > contract']);
check('at any depth', chains('models:\n  shop:\n    staging:\n      schema:\n        +tags: [s]\n'),
  ['models', 'models: shop', 'models: shop > staging', 'models: shop > staging > schema']);
check('a hook holding a mapping, under either spelling',
  chains('models:\n  shop:\n    post_hook:\n      a: 1\n    pre-hook:\n      b: 2\n'),
  ['models', 'models: shop', 'models: shop > post_hook', 'models: shop > pre-hook']);
check('a hook, which is a sequence', chains('models:\n  shop:\n    pre-hook:\n      - grant select\n'),
  ['models', 'models: shop']);

print('\n--- blocks whose keys are names, not paths ---');
check('vars', chains('vars:\n  shop:\n    where: x\n'), []);
check('sources', chains('sources:\n  shop:\n    raw:\n      +enabled: true\n'), []);
check('a nested models key is not the block', chains('vars:\n  models:\n    staging:\n      a: 1\n'), []);
check('every path block answers', chains([
  'seeds:', '  a:', '    b: 1', 'snapshots:', '  a:', '    b: 1', 'analyses:', '  a:', '    b: 1',
  'macros:', '  a:', '    b: 1', 'data_tests:', '  a:', '    b: 1', '',
].join('\n')),
  ['seeds', 'seeds: a', 'snapshots', 'snapshots: a', 'analyses', 'analyses: a', 'macros', 'macros: a',
    'data_tests', 'data_tests: a']);

print('\n--- the shapes a key takes ---');
check('a quoted key is unquoted in the chain', chains('models:\n  "shop":\n    \'01_staging\':\n      +tags: [a]\n'),
  ['models', 'models: shop', 'models: shop > 01_staging']);
check('and marked without its quotes', spans('models:\n  "shop":\n    +tags: [a]\n'), ['models', 'shop']);
check('a numeric folder stays a string', chains('models:\n  shop:\n    2024:\n      +tags: [a]\n'),
  ['models', 'models: shop', 'models: shop > 2024']);
// A tab in the indentation is refused by yamlIndent, so the line is dropped
// and nothing below it is claimed.
check('a tab in the indentation', chains('models:\n  shop:\n\tstaging:\n      +tags: [a]\n'), ['models', 'models: shop']);
check('a comment is not a key', chains('models:\n  # staging:\n  shop:\n    a: 1\n'), ['models', 'models: shop']);
// A block holding a sequence is no mapping of folders, so not even the block
// key is claimed: dbt would not read such a file either.
check('a key under a sequence item', chains('models:\n  - shop:\n      staging:\n        +tags: [a]\n'), []);
check('an empty file asks nothing', chains(''), []);

print('\n--- yamlKeyRange ---');
check('an unquoted key', yamlKeyRange('    staging:', 4), [4, 11]);
check('a space before the colon', yamlKeyRange('    staging :', 4), [4, 11]);
check('a double quoted key', yamlKeyRange('  "01 staging": x', 2), [3, 13]);
check('a single quoted key', yamlKeyRange("  '01 staging': x", 2), [3, 13]);
check('a key with a value on its line', yamlKeyRange('  materialized: table', 2), [2, 14]);
check('no colon, no key', yamlKeyRange('  - staging', 2), null);

print('\n--- the keys that reach a file (0038) ---');
var ROOTS = { models: ['models'], seeds: ['seeds'], snapshots: ['snapshots'], analyses: ['analyses'],
  macros: ['macros'], tests: ['tests', 'models'], data_tests: ['tests', 'models'] };
// The keys reaching `file`, as `block: a > b`, broadest first.
function reaching(text, file, roots) {
  var at = fileFqn(file, roots || ROOTS, projectNameIn(text));
  return configLevels(projectPathKeys(text), at).map(function (k) {
    return k.block + (k.chain.length ? ': ' + k.chain.join(' > ') : '');
  });
}
var LAYERS = [
  'name: shop',
  'models:',
  '  +materialized: view',
  '  shop:',
  '    +persist_docs:',
  '      relation: true',
  '    main_layer:',
  '      +schema: core',
  '    other_layer:',
  '      +schema: other',
  '  audit:',
  '    +enabled: false',
  '  helpers:',
  '    +schema: helpers',
  '    staging:',
  '      +tags: [h]',
  'seeds:',
  '  shop:',
  '    +quote_columns: false',
  'data_tests:',
  '  shop:',
  '    main_layer:',
  '      +severity: warn',
  '    singular:',
  '      +store_failures: true',
  '',
].join('\n');

check('a model three folders below the only key written', reaching(LAYERS, 'models/main_layer/sub/sub_sub/orders.sql'),
  ['models', 'models: shop', 'models: shop > main_layer']);
check('a model in a folder nothing names', reaching(LAYERS, 'models/elsewhere/orders.sql'), ['models', 'models: shop']);
check('a model at the root of model-paths', reaching(LAYERS, 'models/orders.py'), ['models', 'models: shop']);
// `models: shop: main_layer:` is a folder above it; data_tests' own is for its tests.
check('never through data_tests', reaching(LAYERS, 'models/main_layer/orders.sql').indexOf('data_tests: shop > main_layer'), -1);
check('a seed, through seeds only', reaching(LAYERS, 'seeds/countries.csv'), ['seeds', 'seeds: shop']);
check('a singular test, through data_tests', reaching(LAYERS, 'tests/singular/orders_balance.sql'),
  ['data_tests', 'data_tests: shop', 'data_tests: shop > singular']);
check('a key named after the model itself',
  reaching('name: shop\nmodels:\n  shop:\n    marts:\n      orders:\n        +materialized: table\n', 'models/marts/orders.sql'),
  ['models', 'models: shop', 'models: shop > marts', 'models: shop > marts > orders']);
check('and not its namesake folder elsewhere',
  reaching('name: shop\nmodels:\n  shop:\n    marts:\n      orders:\n        +materialized: table\n', 'models/marts/orders_daily.sql'),
  ['models', 'models: shop', 'models: shop > marts']);
// dbt reads the first level as a project or a package, so `staging:` there
// names neither and configures nothing, even though a folder has its name.
check('a first key that is not the project', reaching(
  'name: shop\nmodels:\n  staging:\n    +schema: s\n', 'models/staging/orders.sql'), ['models']);

print('\n--- a package ---');
check('reached by the block and by its own name, never by the project\'s',
  reaching(LAYERS, 'dbt_packages/audit/models/runs.sql'), ['models', 'models: audit']);
check('a folder inside it', reaching(LAYERS, 'dbt_packages/helpers/models/staging/stg_a.sql'),
  ['models', 'models: helpers', 'models: helpers > staging']);
check('and under the old directory name', reaching(LAYERS, 'dbt_modules/audit/models/runs.sql'), ['models', 'models: audit']);
check('the package conventional roots, not the project\'s',
  fileFqn('dbt_packages/audit/models/a/runs.sql', { models: ['transform'] }, 'shop'),
  { blocks: ['models'], fqn: ['audit', 'a', 'runs'] });

print('\n--- fileFqn ---');
check('the fqn dbt builds', fileFqn('models/a/b/orders.sql', ROOTS, 'shop'),
  { blocks: ['models'], fqn: ['shop', 'a', 'b', 'orders'] });
check('a dot inside the name', fileFqn('models/orders.v2.sql', ROOTS, 'shop'),
  { blocks: ['models'], fqn: ['shop', 'orders.v2'] });
check('a snapshot', fileFqn('snapshots/scd/orders_snap.sql', ROOTS, 'shop'),
  { blocks: ['snapshots'], fqn: ['shop', 'scd', 'orders_snap'] });
check('an analysis', fileFqn('analyses/q1.sql', ROOTS, 'shop'), { blocks: ['analyses'], fqn: ['shop', 'q1'] });
check('several model-paths, each its own root',
  fileFqn('transform/staging/a.sql', { models: ['models', 'transform'] }, 'shop'),
  { blocks: ['models'], fqn: ['shop', 'staging', 'a'] });
// One root inside another: the file belongs to the one that holds it closest.
check('the longest root wins',
  fileFqn('models/seeds/a.csv', { models: ['models'], seeds: ['models/seeds'] }, 'shop'),
  { blocks: ['seeds'], fqn: ['shop', 'a'] });
check('a macro is no node', fileFqn('macros/cents.sql', ROOTS, 'shop'), null);
check('nor a compiled file', fileFqn('target/compiled/shop/models/orders.sql', ROOTS, 'shop'), null);
check('a root is a whole segment', fileFqn('models_old/orders.sql', ROOTS, 'shop'), null);
check('no project name, no fqn', fileFqn('models/orders.sql', ROOTS, null), null);

print('\n--- what each level sets ---');
var SETS = [
  'models:',
  '  shop:',
  '    +materialized: table   # why: big',
  "    schema: 'core'",
  '    +tags: ["a", \'b\']',
  '    +post-hook: "grant select on {{ this }} to role x # not a comment"',
  '    +docs:',
  '      node_color: gold',
  '    +pre-hook:',
  '      - select 1',
  '    +sql_header: |',
  '      set x = 1;',
  '    staging:',
  '      +enabled: true',
  '',
].join('\n');
check('the configs of one level, as written', projectPathKeys(SETS)[1].sets, [
  { name: '+materialized', value: 'table' },
  { name: 'schema', value: 'core' },
  { name: '+tags', value: '["a", \'b\']' },
  { name: '+post-hook', value: 'grant select on {{ this }} to role x # not a comment' },
  { name: '+docs', value: '' },
  { name: '+pre-hook', value: '' },
  { name: '+sql_header', value: '' },
]);
check('a config dbt has not been heard to name, by its shape',
  projectPathKeys('models:\n  shop:\n    +materialized: view\n    my_flag: true\n')[1].sets.map(function (s) { return s.name; }),
  ['+materialized', 'my_flag']);
check('a folder is not a config of its parent', projectPathKeys(SETS)[1].sets.some(function (s) { return s.name === 'staging'; }), false);
check('where each key sits', projectPathKeys(SETS).map(function (k) { return [k.line, k.col]; }), [[0, 0], [1, 2], [12, 4]]);
check('where a quoted key sits', projectPathKeys('models:\n  "shop":\n    +tags: [a]\n').map(function (k) { return [k.line, k.col]; }),
  [[0, 0], [1, 3]]);
check('the apostrophe of a word opens no quote', yamlLineValue("  +description: don't # gone", 2), "don't");

print('\n--- projectNameIn ---');
check('plain', projectNameIn('name: shop\nversion: 1\n'), 'shop');
check('quoted, with a comment', projectNameIn("name: 'shop'  # the project\n"), 'shop');
check('a nested name is not the project', projectNameIn('models:\n  name: x\n'), null);
check('none', projectNameIn('version: 1\n'), null);

print('\n--- the card of a key placed nowhere ---');
function note(chain, roots) { return missingKeyNote({ block: 'models', chain: chain, roots: roots || ['models'] }); }
check('a folder that is not there', note(['shop', 'stagign']),
  'No such folder under models. dbt reports a config path that matches nothing as a warning when it parses the project.');
check('a config without its plus', note(['shop', 'docs']),
  'Without its +, dbt reads docs: as a folder, finds none under models, and applies nothing under it. Written +docs:, it is the config.');
check('a block with no readable path', note(['shop'], []), 'models has no readable path in this file, so there is nothing to look under.');
check('the levels reaching a model in a folder named contract',
  reaching('name: shop\nmodels:\n  shop:\n    contract:\n      +tags: [c]\n', 'models/contract/orders.sql'),
  ['models', 'models: shop', 'models: shop > contract']);
