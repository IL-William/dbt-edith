// Where a data test is declared in a properties file (0041): the line a test
// opens at, found under the entry of the node whose YAML declares it, since
// the manifest gives a test no line. Fixtures are invented.
// Run from the repository root: jsc web/tests/testline.js
var src = read('web/app.js');
eval(src.slice(src.indexOf('function yamlIndent'), src.indexOf('/* ATX headings')));
eval(src.slice(src.indexOf('const DECLARING_LISTS'), src.indexOf('async function markRefs')));

function check(label, got, want) {
  var g = JSON.stringify(got), w = JSON.stringify(want);
  print((g === w ? 'PASS  ' : 'FAIL  ') + label + (g === w ? '' : '\n        expected ' + w + '\n        got      ' + g));
}
// The line a test lands on, 0-based, and what it says there.
function landing(text, t) {
  var at = testLine(text, t);
  return at && [at.line, text.split('\n')[at.line].trim()];
}

var shop = [
  'version: 2',                                   // 0
  'models:',                                      // 1
  '  - name: customers',                          // 2
  '    description: |',                           // 3
  '      Columns are checked:',                   // 4
  '      - not_null on every key',                // 5
  '    columns:',                                 // 6
  '      - name: customer_id',                    // 7
  '        tests:',                               // 8
  '          - unique',                           // 9
  '          - not_null  # the key',              // 10
  '  - name: orders',                             // 11
  '    data_tests:',                              // 12
  '      - dbt_utils.unique_combination_of_columns:', // 13
  '          arguments:',                         // 14
  '            combination_of_columns: [order_id, status]', // 15
  '      - dbt_utils.expression_is_true:',        // 16
  '          arguments:',                         // 17
  '            expression: "amount >= 0"',        // 18
  '          column_name: amount',                // 19
  '    columns:',                                 // 20
  '      - name: "Customer_ID"',                  // 21
  '        data_tests:',                          // 22
  '          - not_null',                         // 23
  '          - relationships:',                   // 24
  '              arguments:',                     // 25
  '                to: ref(\'customers\')',       // 26
  '                field: customer_id',           // 27
  '      - name: status',                         // 28
  '        data_tests:',                          // 29
  '          - accepted_values:',                 // 30
  '              arguments:',                     // 31
  '                values: [placed, shipped]',    // 32
  '          - accepted_values:',                 // 33
  '              name: status_is_not_legacy',     // 34
  '              arguments:',                     // 35
  '                values: [placed, shipped, returned]', // 36
  '          - name: status_is_set',              // 37
  '            test_name: not_null',              // 38
  '      - name: amount',                         // 39
  '        data_tests:',                          // 40
  '          - positive_amount',                  // 41
].join('\n');

print('--- a column test, under its own model ---');
check('a bare generic, the first of its column', landing(shop, { host: 'customers', column: 'customer_id', test_name: 'unique' }), [9, '- unique']);
check('a comment after it is no part of its name', landing(shop, { host: 'customers', column: 'customer_id', test_name: 'not_null' }),
  [10, '- not_null  # the key']);
check('the same column in another model is that model\'s, whatever its case or quotes',
  landing(shop, { host: 'orders', column: 'customer_id', test_name: 'not_null' }), [23, '- not_null']);
check('a generic with arguments, by its key', landing(shop, { host: 'orders', column: 'customer_id', test_name: 'relationships' }),
  [24, '- relationships:']);
check('a description that lists "- not_null" is no test', landing(shop, { host: 'customers', column: 'customer_id', test_name: 'not_null' })[0] !== 5, true);

print('\n--- two of one generic on one column ---');
check('the one named as dbt names it', landing(shop, { host: 'orders', column: 'status', test_name: 'accepted_values', name: 'status_is_not_legacy' }),
  [33, '- accepted_values:']);
check('else the first, since only their arguments differ',
  landing(shop, { host: 'orders', column: 'status', test_name: 'accepted_values', name: 'accepted_values_orders_status__placed__shipped' }),
  [30, '- accepted_values:']);
check('an item naming its generic under test_name', landing(shop, { host: 'orders', column: 'status', test_name: 'not_null', name: 'status_is_set' }),
  [37, '- name: status_is_set']);

print('\n--- a test on the whole model ---');
check('through its package, matched by its bare name', landing(shop, { host: 'orders', test_name: 'unique_combination_of_columns' }),
  [13, '- dbt_utils.unique_combination_of_columns:']);
check('one naming a column in its arguments still sits in the model\'s tests',
  landing(shop, { host: 'orders', column: 'amount', test_name: 'expression_is_true' }), [16, '- dbt_utils.expression_is_true:']);

print('\n--- short of the item, the nearest place ---');
check('a generic the column does not list: the column\'s tests', landing(shop, { host: 'orders', column: 'amount', test_name: 'unique' }),
  [40, 'data_tests:']);
check('a column the model does not declare: the model\'s tests', landing(shop, { host: 'orders', column: 'nowhere', test_name: 'unique' }),
  [12, 'data_tests:']);
check('a model with no tests at all: the model', landing('models:\n  - name: bare\n    description: x\n', { host: 'bare', test_name: 'unique' }),
  [1, '- name: bare']);
check('a model the file does not declare: nothing, so the file opens at its top', testLine(shop, { host: 'payments', test_name: 'unique' }), null);
check('a singular test has no host, and nothing to look for', testLine(shop, { test_name: '', name: 'assert_x' }), null);

print('\n--- sources and seeds ---');
var raw = [
  'version: 2',                                   // 0
  'sources:',                                     // 1
  '  - name: raw',                                // 2
  '    tables:',                                  // 3
  '      - name: orders',                         // 4
  '        tests:',                               // 5
  '          - dbt_expectations.expect_table_row_count_to_be_between:', // 6
  '              arguments: {min_value: 1}',      // 7
  '        columns:',                             // 8
  '          - name: id',                         // 9
  '            tests: [unique]',                  // 10
  '          - name: amount',                     // 11
  '            tests:',                           // 12
  '              - not_null',                     // 13
  'seeds:',                                       // 14
  '  - name: country_codes',                      // 15
  '    columns:',                                 // 16
  '      - name: amount',                         // 17
  '        tests:',                               // 18
  '          - unique',                           // 19
].join('\n');
check('a source test, under its table', landing(raw, { host: 'raw.orders', column: 'amount', test_name: 'not_null' }), [13, '- not_null']);
check('and one on the table', landing(raw, { host: 'raw.orders', test_name: 'expect_table_row_count_to_be_between' }),
  [6, '- dbt_expectations.expect_table_row_count_to_be_between:']);
check('a flow list is one line: its key', landing(raw, { host: 'raw.orders', column: 'id', test_name: 'unique' }), [10, 'tests: [unique]']);
check('a seed\'s test, under its seed', landing(raw, { host: 'country_codes', column: 'amount', test_name: 'unique' }), [19, '- unique']);
check('the source\'s column is not the seed\'s', landing(raw, { host: 'raw.orders', column: 'amount', test_name: 'unique' }), [12, 'tests:']);

print('\n--- how much of it is lit ---');
check('an item with its arguments, all of it', [testLine(shop, { host: 'orders', column: 'customer_id', test_name: 'relationships' }).end], [27]);
check('a bare item, its line', [testLine(shop, { host: 'customers', column: 'customer_id', test_name: 'unique' }).end], [9]);
check('the nearest thing found, its line alone', [testLine(shop, { host: 'orders', column: 'amount', test_name: 'unique' }).end], [40]);
