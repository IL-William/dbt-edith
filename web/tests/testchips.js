// The Tests cell in Catalog > Columns: which chips show, and what +N hides.
// Fixtures mirror the /api/node column shape with invented names.
// Run from the repository root: jsc web/tests/testchips.js
var src = read('web/app.js');
eval(src.slice(src.indexOf('function testChips'), src.indexOf('function catalogColumns')));

function check(label, got, want) {
  var g = JSON.stringify(got), w = JSON.stringify(want);
  print((g === w ? 'PASS  ' : 'FAIL  ') + label + (g === w ? '' : '\n        expected ' + w + '\n        got      ' + g));
}
function test(label, name) {
  return { id: 'test.shop.' + (name || label), label: label, name: (name || label) + '_orders_status' };
}
function labels(list) { return list.map(function (t) { return t.label; }); }

// not_null, unique and accepted_values: 29 characters between them, the shape
// nearly every documented column has.
var short = [test('not_null'), test('unique'), test('accepted_values'),
  test('relationships', 'rel_statuses'), test('relationships', 'rel_legacy')];
// dbt_expectations names, which are longer than the Description column.
var long = [test('expect_column_value_lengths_to_be_between'),
  test('expect_column_values_to_not_match_like_pattern'),
  test('not_null'), test('unique')];

print('--- a cell that fits shows everything ---');
check('one test is one chip', labels(testChips([short[0]]).shown), ['not_null']);
check('nothing is hidden', testChips([short[0]]).hidden.length, 0);
check('three short ones fit', testChips(short.slice(0, 3)).shown.length, 3);

print('\n--- past three, the tail goes behind the chip ---');
check('three and two', [testChips(short).shown.length, testChips(short).hidden.length], [3, 2]);
check('the hidden ones are the tail, in order', labels(testChips(short).hidden), ['relationships', 'relationships']);
check('the shown ones keep their order', labels(testChips(short).shown), ['not_null', 'unique', 'accepted_values']);
check('ten tests read as three and a +7',
  [testChips(short.concat(short)).shown.length, testChips(short.concat(short)).hidden.length], [3, 7]);

print('\n--- long names are cut by width, not by count ---');
// Three of these wrap the row to three lines while three short ones sit on one,
// which is the whole reason the budget is in characters.
check('one long name fills the budget', testChips(long).shown.length, 1);
check('and the rest are counted', testChips(long).hidden.length, 3);
check('the first chip still names something', labels(testChips(long).shown), ['expect_column_value_lengths_to_be_between']);
check('a single very long test is never replaced by +1', testChips([long[0]]).hidden.length, 0);

print('\n--- two tests of one generic are two tests ---');
// They read the same on the chip, which is why the card names them: the payload
// carries one entry per test node, so the count behind +N is the real one.
var twice = [test('expression_is_true', 'not_sentinel'), test('expression_is_true', 'not_future'), test('not_null')];
check('both survive, and all three fit', testChips(twice).shown.length, 3);
check('the card can tell them apart', testChips(twice).shown.map(function (t) { return t.name; }),
  ['not_sentinel_orders_status', 'not_future_orders_status', 'not_null_orders_status']);

print('\n--- a column with no test at all ---');
// /api/node skips an empty list, so the cell is asked about a missing field.
check('a missing list is not a crash', testChips(undefined).shown.length, 0);
check('nor is an empty one', testChips([]).hidden.length, 0);

print('\n--- a caller may still set its own caps ---');
check('a tighter count', testChips(short, 1).shown.length, 1);
check('a tighter budget', testChips(short, 3, 10).shown.length, 1);
check('a budget wide enough for every short name', testChips(short, 5, 200).hidden.length, 0);

print('\n--- which tests the Columns table can show at all ---');
// A test naming a column is a chip in that row; one naming none guards the whole
// table and has no row, which is the split the Preview tab is built on.
var guards = [
  { id: 'test.shop.a', test_name: 'not_null', column: 'order_id', name: 'not_null_orders_order_id' },
  { id: 'test.shop.b', test_name: 'unique_combination_of_columns', column: '', name: 'dbt_utils_unique_combination_of_columns_orders' },
  { id: 'test.shop.c', test_name: '', column: '', name: 'test_orders_revenue_reconciles' },
  { id: 'test.shop.d', test_name: 'relationships', column: 'customer_id', name: 'relationships_orders_customer_id' },
];
check('the model keeps the ones naming no column',
  splitTests(guards).model.map(function (t) { return t.id; }), ['test.shop.b', 'test.shop.c']);
check('the columns keep the rest',
  splitTests(guards).column.map(function (t) { return t.id; }), ['test.shop.a', 'test.shop.d']);
check('a singular test has no generic name to show', splitTests(guards).model[1].test_name, '');
check('nothing is lost between the two',
  splitTests(guards).model.length + splitTests(guards).column.length, guards.length);
check('a node with no test at all splits into two empty lists',
  [splitTests(undefined).model.length, splitTests(undefined).column.length], [0, 0]);
