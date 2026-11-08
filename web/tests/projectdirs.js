// The folder keys of dbt_project.yml: which key names a folder, and where its
// own text sits in its line (0036). The server places the chains; nothing here
// touches a disk. Fixtures are invented.
// Run from the repository root: jsc web/tests/projectdirs.js
var src = read('web/app.js');
eval(src.slice(src.indexOf('function yamlIndent'), src.indexOf('/* ATX headings')));
eval(src.slice(src.indexOf('const DECLARING_LISTS'), src.indexOf('async function markRefs')));
// One eval, since a const declared in one is gone outside it.
eval(src.slice(src.indexOf('const OUTLINE_MAX'), src.indexOf('function refreshOutline'))
  + src.slice(src.indexOf('function configMenuFor'), src.indexOf('/* dbt_project.yml at a key')));

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
// Without where each sits, which the excerpt reads and the next check pins.
function named(sets) { return sets.map(function (s) { return { name: s.name, value: s.value }; }); }
check('the configs of one level, as written', named(projectPathKeys(SETS)[1].sets), [
  { name: '+materialized', value: 'table' },
  { name: 'schema', value: 'core' },
  { name: '+tags', value: '["a", \'b\']' },
  { name: '+post-hook', value: 'grant select on {{ this }} to role x # not a comment' },
  { name: '+docs', value: '' },
  { name: '+pre-hook', value: '' },
  { name: '+sql_header', value: '' },
]);
check('and the line each starts on',
  projectPathKeys(SETS)[1].sets.map(function (s) { return [s.name, s.line]; }).slice(3), [
    ['+post-hook', 5], ['+docs', 6], ['+pre-hook', 8], ['+sql_header', 10],
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

print('\n--- a dbt_project.yml saved with CRLF endings ---');
// Read closed, the file comes from the disk as saved, and a key ending its line
// is followed by a \r. Every such folder used to vanish, and its configs, a
// block's lines and the flags below piled onto the levels above.
var NESTED = [
  'name: shop',
  'models:',
  '  +materialized: view',
  '  +post-hook:',
  '    - >',
  "      grant select on {{ this }} to role reporter",
  '  shop: # the project',
  '    +persist_docs:',
  '      relation: true',
  '    marts:',
  '      +schema: marts',
  '      finance:',
  '        +materialized: table',
  'flags:',
  '  send_anonymous_usage_stats: false',
  '',
].join('\n');
var CRLF = NESTED.replace(/\n/g, '\r\n');
// The excerpt of dbt_project.yml that reaches a file, as configMenu builds it
// once the file is read: one line of plain data per line shown. Each reads
// `line: text`, with ~ for a struck one and the note after it, | for the
// tagged one and ... for a gap before it.
function shown(text, file, roots) {
  var menu = configMenuFor(text, file, roots || ROOTS);
  return menu.lines.map(function (l) {
    return (l.gapBefore ? '... ' : '') + (l.line + 1) + ': ' + '  '.repeat(l.depth) + l.key + l.punct + l.value
      + (l.over ? ' ~' + (l.noted ? ' replaced on ' + l.over : '') : '') + (l.here ? ' | ' + menu.tag : '');
  });
}
// Jaffle Shop as the screenshots write it.
var JAFFLE = [
  "name: 'jaffle_shop'",
  'models:',
  '  jaffle_shop:',
  '    +materialized: table',
  "    +database: \"{{ env_var('JAFFLE_DB', 'jaffle_shop') }}\"",
  '    +schema: main',
  '    staging:',
  "      +tags: ['staging']",
  '      +materialized: view',
  '      +docs:',
  "        node_color: 'silver'",
  '    +docs:',
  "      node_color: 'gold'",
  '',
].join('\n');
check('a model one folder down: every line in order, the replaced ones struck, the folder tagged',
  shown(JAFFLE, 'models/staging/stg_payments.sql'), [
    '2: models:',
    '3:   jaffle_shop:',
    '4:     +materialized: table ~ replaced on 9',
    '5:     +database: "{{ env_var(\'JAFFLE_DB\', \'jaffle_shop\') }}"',
    '6:     +schema: main',
    '7:     staging: | stg_payments.sql is here',
    "8:       +tags: ['staging']",
    '9:       +materialized: view',
    '10:       +docs:',
    "11:         node_color: 'silver'",
    '12:     +docs:',
    "13:       node_color: 'gold' ~ replaced on 11",
  ]);
check('a model at the project\'s own level: a gap where the staging block was left out, nothing struck',
  shown(JAFFLE, 'models/customers.sql'), [
    '2: models:',
    '3:   jaffle_shop: | customers.sql is here',
    '4:     +materialized: table',
    '5:     +database: "{{ env_var(\'JAFFLE_DB\', \'jaffle_shop\') }}"',
    '6:     +schema: main',
    '... 12:     +docs:',
    "13:       node_color: 'gold'",
  ]);
check('the template is marked, the quotes kept and a trailing comment dropped',
  configMenuFor("name: s\nmodels:\n  s:\n    +schema: \"x_{{ y }}\" # why\n    +alias: 'a#b' # c\n", 'models/m.sql', ROOTS)
    .lines.map(function (l) { return [l.value, l.tpl]; }).slice(2), [['"x_{{ y }}"', true], ["'a#b'", false]]);
check('the guides follow the YAML depth, and the cursor of a line is its first character',
  configMenuFor(JAFFLE, 'models/customers.sql', ROOTS).lines.slice(0, 4).map(function (l) { return [l.depth, l.col]; }),
  [[0, 0], [1, 2], [2, 4], [2, 4]]);

var MERGED = [
  'name: s',
  'models:',
  '  +tags: [a]',
  '  +pre_hook: "select 1"',
  '  +meta:',
  '    owner: x',
  '    team: y',
  '  s:',
  '    +tags: [b]',
  '    +pre-hook: "select 2"',
  '    +meta:',
  '      owner: z',
  '    marts:',
  '      +docs: {node_color: gold}',
  '      +meta:',
  '        team: q',
  '',
].join('\n');
check('tags and hooks add up, pre_hook and pre-hook being one; meta is replaced key by key',
  shown(MERGED, 'models/marts/m.sql'), [
    '2: models:',
    '3:   +tags: [a]',
    '4:   +pre_hook: "select 1"',
    '5:   +meta:',
    '6:     owner: x ~ replaced on 12',
    '7:     team: y ~ replaced on 16',
    '8:   s:',
    '9:     +tags: [b]',
    '10:     +pre-hook: "select 2"',
    '11:     +meta:',
    '12:       owner: z',
    '13:     marts: | m.sql is here',
    '14:       +docs: {node_color: gold}',
    '15:       +meta:',
    '16:         team: q',
  ]);

print('\n--- what dbt replaces, key by key ---');
function over(text, file) {
  return configMenuFor(text, file, ROOTS).lines.filter(function (l) { return l.over; })
    .map(function (l) { return (l.line + 1) + '>' + l.over; });
}
check('a clobbered config is struck whole, its value lines too',
  over('name: s\nmodels:\n  s:\n    +post_hook:\n      - select 1\n      - select 2\n    +persist_docs:\n      relation: true\n    m:\n      +persist_docs:\n        relation: false\n', 'models/m/x.sql'),
  ['7>10', '8>10']);
check('and a key line holding a value, once', over('name: s\nmodels:\n  +schema: a\n  s:\n    +schema: b\n', 'models/x.sql'), ['3>5']);
check('tags and hooks are never struck', over(MERGED, 'models/marts/m.sql').some(function (o) { return /^(3|4|9|10)>/.test(o); }), false);
check('meta, key by key; the key line itself stays', over(MERGED, 'models/marts/m.sql'), ['6>12', '7>16']);
check('a flow style value is never half struck', over('name: s\nmodels:\n  s:\n    +docs: {node_color: gold}\n    m:\n      +docs:\n        node_color: red\n', 'models/m/x.sql'), []);
check('nor is a block one against a flow style one',
  over('name: s\nmodels:\n  s:\n    +docs:\n      node_color: gold\n    m:\n      +docs: {node_color: red}\n', 'models/m/x.sql'), []);
// The flow value nearest the file may hold the key, so no shallower line is named instead.
check('and no shallower level is struck in its place',
  over('name: s\nmodels:\n  +docs:\n    node_color: gold\n  s:\n    +docs:\n      node_color: red\n    m:\n      +docs: {node_color: blue}\n', 'models/m/x.sql'), []);
var GRANTS = [
  'name: s', 'models:', '  s:', '    +grants:', '      select: [a]', '      +insert: [b]', '      update: [c]',
  '    m:', '      +grants:', '        select: [d]', '        +insert: [e]', '        +update: [f]', '',
].join('\n');
check('grants: a key written without its plus replaces, with it extends',
  over(GRANTS, 'models/m/x.sql'), ['5>10']);
check('two settings of one config at a level are left alone',
  over('name: s\nmodels:\n  +schema: a\n  +schema: b\n  s:\n    +schema: c\n', 'models/x.sql'), []);
check('a config the file sets once, only above, is not struck', over(JAFFLE, 'models/customers.sql'), []);
check('a list value is shown whole, items at one level in, and struck whole',
  over('name: s\nmodels:\n  s:\n    +cluster_by:\n      - a\n      - b\n    m:\n      +cluster_by: [c]\n', 'models/m/x.sql'),
  ['4>8', '5>8', '6>8']);
check('its items read as values', configMenuFor('name: s\nmodels:\n  s:\n    +cluster_by:\n      - a\n', 'models/x.sql', ROOTS)
  .lines.slice(2).map(function (l) { return [l.key, l.value, l.depth]; }), [['+cluster_by', '', 2], ['', '- a', 3]]);
// Blank and comment-only lines are never shown, struck or not, outside a block value.
check('a # inside a | block is value, shown and struck with it',
  over('name: s\nmodels:\n  s:\n    +sql_header: |\n      set x = 1;\n      # a note\n\n      set y = 2;\n    m:\n      +sql_header: select 1\n', 'models/m/x.sql'),
  ['4>10', '5>10', '6>10', '8>10']);
check('the block text sits one level in from its key',
  configMenuFor('name: s\nmodels:\n  s:\n    +sql_header: |\n      set x = 1;\n', 'models/x.sql', ROOTS).lines.map(function (l) { return l.depth; }),
  [0, 1, 2, 3]);

check('latest_version_pointer goes key by key, snapshot_meta_column_names is replaced whole',
  over('name: s\nmodels:\n  +latest_version_pointer:\n    a: 1\n    b: 2\n  +snapshot_meta_column_names:\n    x: 1\n    y: 2\n  s:\n    +latest_version_pointer:\n      a: 3\n    +snapshot_meta_column_names:\n      x: 3\n', 'models/s/x.sql'),
  ['4>11', '6>12', '7>12', '8>12']);
check('a block\'s # line is a line of the value, shown as written',
  configMenuFor('name: s\nmodels:\n  s:\n    +h: |\n      # kept\n      select 1\n    +b: 2\n', 'models/x.sql', ROOTS)
    .lines.map(function (l) { return [l.line + 1, l.value]; }).slice(2), [[4, '|'], [5, '# kept'], [6, 'select 1'], [7, '2']]);
check('when both blocks are written dbt reads data_tests alone',
  reaching('name: s\ntests:\n  s:\n    +severity: warn\ndata_tests:\n  s:\n    +severity: error\n', 'tests/singular/a.sql'),
  ['data_tests', 'data_tests: s']);
check('and tests alone when that is all there is',
  reaching('name: s\ntests:\n  s:\n    +severity: warn\n', 'tests/singular/a.sql'), ['tests', 'tests: s']);

print('\n--- when there is nothing to show ---');
function reason(text, file, roots) { var m = configMenuFor(text, file, roots || ROOTS); return [m.note, m.lines.length]; }
check('too large', reason('x'.repeat(2 * 1024 * 1024 + 1), 'models/a.sql'), ['dbt_project.yml is too large to scan.', 0]);
check('not a resource', reason(JAFFLE, 'macros/a.sql'),
  ['Not a model, seed, snapshot, analysis or test under the paths dbt_project.yml sets.', 0]);
check('no key reaches it', reason('name: s\nseeds:\n  +tags: [a]\n', 'models/a.sql'), ['No key of dbt_project.yml reaches this file.', 0]);
check('the dialog is named after the file', configMenuFor(JAFFLE, 'models/staging/stg_payments.sql', ROOTS).title,
  'Where dbt_project.yml configures stg_payments.sql');

print('\n--- what the footer says wins ---');
check('a file of the project: its config() and properties YAML',
  [configMenuFor(JAFFLE, 'models/customers.sql', ROOTS).pkg, configFooter(false)],
  [false, 'A config() in the file, or in its properties YAML, wins over these lines.']);
check('a file of a package: these lines, applied last',
  [configMenuFor(JAFFLE, 'dbt_packages/utils/models/a.sql', ROOTS).pkg, /root project's lines/.test(configFooter(true)), /win over/.test(configFooter(true))],
  [true, true, true]);
check('the old claim is not made for a package', /config\(\) in the file/.test(configFooter(true)), false);
check('a package path needs a file under the package\'s folder',
  ['dbt_packages/utils', 'dbt_packages/utils/', 'models/dbt_packages/a/b.sql', 'dbt_modules/utils/models/a.sql'].map(inPackage),
  [false, false, false, true]);

print('\n--- where the dialog sits ---');
var PANE = { left: 100, right: 700 };
check('under its button, right edges level', configDialogAt({ right: 690, bottom: 40 }, PANE, { width: 460, height: 200 }, { height: 800 }),
  { left: 230, top: 44 });
check('held to the pane\'s right edge when the button sits past it', configDialogAt({ right: 760, bottom: 40 }, PANE, { width: 460, height: 200 }, { height: 800 }),
  { left: 232, top: 44 });
check('and to its left edge', configDialogAt({ right: 150, bottom: 40 }, PANE, { width: 460, height: 200 }, { height: 800 }),
  { left: 108, top: 44 });
check('and up from the window\'s bottom', configDialogAt({ right: 690, bottom: 700 }, PANE, { width: 460, height: 200 }, { height: 800 }).top, 592);

print('\n--- the same, in a file saved with CRLF endings ---');
check('the excerpt of a model, as the open tab reads it', shown(NESTED, 'models/marts/finance/orders.sql'), [
  '2: models:',
  '3:   +materialized: view ~ replaced on 13',
  '4:   +post-hook:',
  '5:     - >',
  '6:       grant select on {{ this }} to role reporter',
  '7:   shop:',
  '8:     +persist_docs:',
  '9:       relation: true',
  '10:     marts:',
  '11:       +schema: marts',
  '12:       finance: | orders.sql is here',
  '13:         +materialized: table',
]);
check('the same read from the disk, each line opening on its own line', configMenuFor(CRLF, 'models/marts/finance/orders.sql', ROOTS).lines,
  configMenuFor(NESTED, 'models/marts/finance/orders.sql', ROOTS).lines);
check('a lone \\r ends a line too, as CodeMirror reads it', editorText('a:\r  b: 1\r\n'), 'a:\n  b: 1\n');

print('\n--- the keys dbt would warn of (0103) ---');
var WARNED = [
  'models:',
  '  shop:',
  '    # moved under staging since the file was read',
  '    staging:',
  '      stagign:',
  '        +tags: [x]',
  '    merged:',
  '      <<: *shared',
  'tests:',
  '  +severity: warn',
  'on-run-end:',
  '  - stagign:',
  '      +tags: [x]',
].join('\n');
var paths = [
  { block: 'models', chain: ['shop', 'stagign'], line: 3 },
  { block: 'models', chain: ['shop', 'staging', 'stagign'], line: 5 },
  { block: 'models', chain: ['shop', 'merged', 'core'], line: 2 },
  { block: 'data_tests', written: 'tests', chain: [], line: 9 },
];
function lines(text, saved) { return unusedLines(text, paths, saved).map(function (m) { return [unusedSays(m.u), m.line]; }); }
check('each key by its chain, on the line the text has it now, a gone one unmarked', lines(WARNED, false),
      [['models.shop.staging.stagign', 4], ['data_tests', 8]]);
check('a path merged from an anchor at the server\'s line, while the text is the file', lines(WARNED, true),
      [['models.shop.stagign', 2], ['models.shop.staging.stagign', 4], ['models.shop.merged.core', 1], ['data_tests', 8]]);
check('nothing under a list item is a path', unusedLines(WARNED, [{ block: 'on-run-end', chain: ['0', 'stagign'], line: 0 }], false), []);
var SHARED = [
  'models:',
  '  shop:',
  '    base: &anc',
  '      +tags: [a]',
  '      inner:',
  '        +tags: [b]',
  '    aliased: *anc',
  '  flow: {p: {+tags: 1}, q: {+tags: 2}}',
].join('\n');
var shared = [
  { block: 'models', chain: ['shop', 'aliased'], line: 7 },
  { block: 'models', chain: ['shop', 'aliased', 'inner'], line: 7 },
  { block: 'models', chain: ['flow', 'p'], line: 8 },
  { block: 'models', chain: ['flow', 'q'], line: 8 },
];
function byLine(saved) { return unusedByLine(unusedLines(SHARED, shared, saved)).map(function (m) { return [m.line, m.paths.map(unusedSays)]; }); }
check('two paths on one line share one mark, in the order found', byLine(true),
      [[6, ['models.shop.aliased', 'models.shop.aliased.inner']], [7, ['models.flow.p', 'models.flow.q']]]);
check('a line with one path keeps its own mark', unusedByLine([{ line: 2, u: shared[0] }, { line: 5, u: shared[2] }]).map(function (m) { return m.line; }), [2, 5]);
check('the hover key names every path and tells a stale mark', [unusedHoverKey(shared.slice(0, 2), false), unusedHoverKey(shared.slice(0, 2), true)],
      ['unused:models.shop.aliased | models.shop.aliased.inner', 'unused:stale:models.shop.aliased | models.shop.aliased.inner']);
check('why: no fqn of its kind', unusedWhy(paths[0]),
      'No model\'s fqn begins with these keys, nor any disabled node\'s, so what the last one sets applies to nothing.');
check('why: a block with nothing of its kind, under the name it was written with', unusedWhy(paths[3]),
      'The manifest holds no data test and no disabled node, so what tests: sets applies to nothing. '
        + 'dbt reads tests: as data_tests:, its name since dbt 1.8.');
check('why: a config written without its +', unusedWhy({ block: 'seeds', chain: ['shop', 'docs'], line: 4 }),
      'No seed\'s fqn begins with these keys, nor any disabled node\'s, so what the last one sets applies to nothing. '
        + 'Without its +, dbt reads docs: as a folder; written +docs:, it is the config.');

print('\n--- the paths under the reason they apply to nothing ---');
var listed = [
  { block: 'models', chain: ['shop', 'stagign'], line: 3 },
  { block: 'models', chain: ['shop', 'staging', 'stagign'], line: 5 },
  { block: 'seeds', chain: ['shop', 'docs'], line: 8 },
  { block: 'models', chain: ['shop', 'merged', 'core'], line: 2 },
];
check('each reason once, the paths under it in the order they came', unusedByWhy(listed).map(function (g) { return g.rows.map(unusedSays); }),
      [['models.shop.stagign', 'models.shop.staging.stagign', 'models.shop.merged.core'], ['seeds.shop.docs']]);
check('the reason is the one the card says', unusedByWhy(listed)[1].why, unusedWhy(listed[2]));
check('a block of another kind is another reason, even with the same words after it', unusedByWhy(listed).length, 2);
check('none, none', unusedByWhy([]), []);
check('the keys a path has in common with the one before it, the first with none',
      unusedShared(listed.slice(0, 2)), [0, 2]);
check('across a block, only the keys that are the same from the start', unusedShared(listed), [0, 2, 0, 0]);
check('never every key: a path shows its own last one',
      unusedShared([{ block: 'models', chain: ['shop', 'a'], line: 1 }, { block: 'models', chain: ['shop', 'a'], line: 2 }]), [0, 2]);
check('a shorter path under a longer one keeps its last key',
      unusedShared([{ block: 'models', chain: ['shop', 'a', 'b'], line: 1 }, { block: 'models', chain: ['shop'], line: 2 }]), [0, 1]);
check('none for none', unusedShared([]), []);
