// Where a column is declared in a properties file (0187): the line the Catalog's
// glyph opens at, found under the entry of the node whose YAML declares it,
// and the file it opens. Fixtures are invented.
// Run from the repository root: jsc web/tests/columnline.js
var src = read('web/app.js');
eval(src.slice(src.indexOf('function yamlIndent'), src.indexOf('/* ATX headings')));
eval(src.slice(src.indexOf('const DECLARING_LISTS'), src.indexOf('async function markRefs')));
eval(src.slice(src.indexOf('function versionChip'), src.indexOf('function catalogMeta')));

function check(label, got, want) {
  var g = JSON.stringify(got), w = JSON.stringify(want);
  print((g === w ? 'PASS  ' : 'FAIL  ') + label + (g === w ? '' : '\n        expected ' + w + '\n        got      ' + g));
}
// The line a column lands on, 0-based, and what it says there.
function landing(text, c) {
  var at = columnLine(text, c);
  return at && [at.line, text.split('\n')[at.line].trim()];
}

var shop = [
  'version: 2',                                   // 0
  'models:',                                      // 1
  '  - name: customers',                          // 2
  '    description: |',                           // 3
  '      Columns of the table:',                  // 4
  '      - name: customer_id',                    // 5
  '    columns:',                                 // 6
  '      - name: "Customer_ID"',                  // 7
  '        description: The key',                 // 8
  '        tests:',                               // 9
  '          - unique',                           // 10
  '      - name: email',                          // 11
  '  - name: bare',                               // 12
  '    description: no columns here',             // 13
  '  - name: orders',                             // 14
  '    columns:',                                 // 15
  '      - name: order_id',                       // 16
  '      - name: status',                         // 17
  '        description: Where it stands',         // 18
  '    latest_version: 2',                        // 19
  '    versions:',                                // 20
  '      - v: 1',                                 // 21
  '        columns:',                             // 22
  '          - include: all',                     // 23
  '            exclude: [status]',                // 24
  '      - v: "2"',                               // 25
  '        columns:',                             // 26
  '          - include: all',                     // 27
  '          - name: order_id',                   // 28
  '            description: Renamed in v2',       // 29
  '      - v: 3.0',                               // 30
  '        columns:',                             // 31
  '          - name: total',                      // 32
  '      - v: 4',                                 // 33
  '        description: no columns of its own',   // 34
].join('\n');

print('--- a plain model ---');
check('its column, on the name line', landing(shop, { host: 'customers', column: 'email' }), [11, '- name: email']);
check('and the character is the name key\'s', columnLine(shop, { host: 'customers', column: 'email' }).ch, 8);
check('the case is folded, and quotes are no part of the name', landing(shop, { host: 'customers', column: 'customer_id' }),
  [7, '- name: "Customer_ID"']);
check('a description that lists "- name: customer_id" is no column',
  landing(shop, { host: 'customers', column: 'customer_id' })[0] !== 5, true);

print('\n--- how much of it is lit ---');
check('the whole item, its description and tests', [columnLine(shop, { host: 'customers', column: 'customer_id' }).end], [10]);
check('the last item runs to the end of the entry', [columnLine(shop, { host: 'customers', column: 'email' }).end], [11]);
check('an item with a description', [columnLine(shop, { host: 'orders', column: 'status' }).end], [18]);

print('\n--- a versioned model ---');
check('a version\'s own column wins over the entry\'s', landing(shop, { host: 'orders', version: '2', column: 'order_id' }),
  [28, '- name: order_id']);
check('a version that inherits through include: all lands on the entry\'s column',
  landing(shop, { host: 'orders', version: '2', column: 'status' }), [17, '- name: status']);
check('the entry\'s column for a version with no column list of its own',
  landing(shop, { host: 'orders', version: '4', column: 'order_id' }), [16, '- name: order_id']);
var plainV = 'models:\n  - name: m\n    versions:\n      - v: 2\n        columns:\n          - name: c\n';
check('v: 2 in the YAML and "2" in the manifest are one version', landing(plainV, { host: 'm', version: '2', column: 'c' }), [5, '- name: c']);
check('v: "2" quoted in the YAML is the same', landing(shop, { host: 'orders', version: '2', column: 'order_id' })[0], 28);
check('v: 3.0 is the 3 the manifest names', landing(shop, { host: 'orders', version: '3', column: 'total' }), [32, '- name: total']);
check('and the other way, 3.0 asked for', landing(shop, { host: 'orders', version: '3.0', column: 'total' }), [32, '- name: total']);
check('a version is not another: 1 does not read 2\'s column', landing(shop, { host: 'orders', version: '1', column: 'total' }),
  [22, 'columns:']);
check('the base name comes off a versioned node', [versionBase({ name: 'orders.v2', version: '2' })], ['orders']);
check('and off a prerelease of dotted text', [versionBase({ name: 'orders.v1.5', version: '1.5' })], ['orders']);
check('a node with no version keeps its name', [versionBase({ name: 'orders' })], ['orders']);
check('a name that does not end in its version is kept', [versionBase({ name: 'orders', version: '2' })], ['orders']);

print('\n--- a source table ---');
var raw = [
  'version: 2',                                   // 0
  'sources:',                                     // 1
  '  - name: raw',                                // 2
  '    tables:',                                  // 3
  '      - name: orders',                         // 4
  '        columns:',                             // 5
  '          - name: id',                         // 6
  '            description: the key',             // 7
  '          - name: amount',                     // 8
  '      - name: refunds',                        // 9
  '        columns:',                             // 10
  '          - name: amount',                     // 11
  'seeds:',                                       // 12
  '  - name: country_codes',                      // 13
  '    columns:',                                 // 14
  '      - name: amount',                         // 15
].join('\n');
check('a source column, under its table', landing(raw, { host: 'raw.orders', column: 'amount' }), [8, '- name: amount']);
check('the same name under another table is that table\'s', landing(raw, { host: 'raw.refunds', column: 'amount' }), [11, '- name: amount']);
check('a seed\'s column is not the source\'s', landing(raw, { host: 'country_codes', column: 'amount' }), [15, '- name: amount']);
check('the item of a source column lights its description', [columnLine(raw, { host: 'raw.orders', column: 'id' }).end], [7]);

print('\n--- short of the column, the nearest place ---');
check('an undeclared column lands on the entry\'s columns:', landing(shop, { host: 'customers', column: 'created_at' }), [6, 'columns:']);
check('lit on that one line', columnLine(shop, { host: 'customers', column: 'created_at' }).end, 6);
check('an undeclared column of a version lands on that version\'s columns:',
  landing(shop, { host: 'orders', version: '2', column: 'nowhere' }), [26, 'columns:']);
check('a version with no columns: lands on the entry\'s', landing(shop, { host: 'orders', version: '4', column: 'nowhere' }), [15, 'columns:']);
check('a source table\'s undeclared column', landing(raw, { host: 'raw.orders', column: 'nowhere' }), [5, 'columns:']);
check('an entry without columns: lands on the entry', landing(shop, { host: 'bare', column: 'id' }), [12, '- name: bare']);
check('lit on that one line', columnLine(shop, { host: 'bare', column: 'id' }).end, 12);
check('a version the file does not declare falls back to the entry\'s columns:',
  landing(shop, { host: 'orders', version: '9', column: 'nowhere' }), [15, 'columns:']);
check('a version item with no columns and no entry columns lands on the version item',
  landing('models:\n  - name: m\n    versions:\n      - v: 1\n        description: x\n', { host: 'm', version: '1', column: 'c' }),
  [3, '- v: 1']);
check('a model the file does not declare: nothing, so the file opens at its top', columnLine(shop, { host: 'payments', column: 'id' }), null);
check('a node with no host has nothing to look for', columnLine(shop, { column: 'id' }), null);
check('a column named in a model\'s description is no column of another',
  landing(shop, { host: 'bare', column: 'customer_id' }), [12, '- name: bare']);

print('\n--- the file a column opens ---');
check('a source gives its YAML', columnYamlFile({ name: 'raw.orders', package: 'shop', file: 'models/staging/_sources.yml', yml: '' }, 'shop', false),
  'models/staging/_sources.yml');
check('a model gives its patch file', columnYamlFile({ name: 'orders', package: 'shop', file: 'models/orders.sql', yml: 'models/_models.yml' }, 'shop', false),
  'models/_models.yml');
check('a .yaml too', columnYamlFile({ name: 'orders', package: 'shop', file: 'models/orders.sql', yml: 'models/_models.yaml' }, 'shop', false),
  'models/_models.yaml');
check('a snapshot declared in YAML', columnYamlFile({ name: 'snap', package: 'shop', file: 'snapshots/snap.yml', yml: '' }, 'shop', false),
  'snapshots/snap.yml');
check('a model with no properties file has no YAML', columnYamlFile({ name: 'orders', package: 'shop', file: 'models/orders.sql', yml: '' }, 'shop', false), '');
check('a node of a package is not ours to edit', columnYamlFile({ name: 'stg', package: 'other', file: 'models/stg.sql', yml: 'models/stg.yml' }, 'shop', false), '');
check('a file under dbt_packages/ is a package\'s whatever the node says',
  columnYamlFile({ name: 'stg', package: 'shop', file: 'models/stg.sql', yml: 'dbt_packages/other/models/stg.yml' }, 'shop', false), '');
check('a site has no file to open', columnYamlFile({ name: 'orders', package: 'shop', file: 'models/orders.sql', yml: 'models/_models.yml' }, 'shop', true), '');
check('no project name known: the package is not held against it',
  columnYamlFile({ name: 'orders', package: 'shop', file: 'models/orders.sql', yml: 'models/_models.yml' }, '', false), 'models/_models.yml');

print('\n--- exact spelling, the project unknown, the flash\'s end ---');
var twin = 'models:\n  - name: m\n    columns:\n      - name: ID\n      - name: id\n        description: lower\n\n      # next\n      - name: other\n';
check('both spellings declared: the exact one lands on its own', landing(twin, { host: 'm', column: 'id' }), [4, '- name: id']);
check('and the upper one on its own', landing(twin, { host: 'm', column: 'ID' }), [3, '- name: ID']);
check('a spelling declared in neither case alone is still found folded', landing(twin, { host: 'm', column: 'Other' }), [8, '- name: other']);
check('a package file is not ours with the project empty, by its on-disk path',
  columnYamlFile({ name: 'stg', package: 'other', file: 'dbt_packages/other/models/stg.sql', yml: 'models/stg.yml' }, '', false), '');
check('the trailing blank and comment lines are left out of the flash', columnLine(twin, { host: 'm', column: 'id' }).end, 5);
check('a one-line item ends on itself', columnLine(twin, { host: 'm', column: 'ID' }).end, 3);
