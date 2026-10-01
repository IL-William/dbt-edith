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

print('\n--- a config is not a folder ---');
// The whole rule: a config written without its plus still has a value on its
// line, so it never opens a mapping.
check('a bare config with a value', chains('models:\n  shop:\n    materialized: table\n    schema: marts\n'),
  ['models', 'models: shop']);
check('a bare config holding a list', chains("models:\n  shop:\n    tags: ['a', 'b']\n"), ['models', 'models: shop']);
check('a bare config holding a mapping', chains('models:\n  shop:\n    docs:\n      node_color: gold\n'),
  ['models', 'models: shop']);
// dbt's own rule: a key is a config when it starts with `+` or is one of its
// config names, and a path segment otherwise. These are the ones a scanner
// could not tell from a folder by shape alone, because they open a mapping.
check('every config that opens a mapping', chains([
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
].join('\n')), ['models', 'models: shop']);
check('a config name is refused at any depth', chains('models:\n  shop:\n    staging:\n      schema:\n        x: 1\n'),
  ['models', 'models: shop', 'models: shop > staging']);
check('and a hook under either spelling', chains('models:\n  shop:\n    post_hook:\n      a: 1\n    pre-hook:\n      b: 2\n'),
  ['models', 'models: shop']);
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
