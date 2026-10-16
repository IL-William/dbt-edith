// Whether a test keeps its failing rows, from the config the server sends:
// store_failures_as wins over store_failures, and keeps nothing as ephemeral.
// Run from the repository root: jsc web/tests/testconfig.js
var app = read('web/app.js');
eval(app.slice(app.indexOf('function testStoresFailures('), app.indexOf('function catalogTestConfig(')));

function check(label, got, want) {
  var g = JSON.stringify(got), w = JSON.stringify(want);
  print((g === w ? 'PASS  ' : 'FAIL  ') + label + (g === w ? '' : '\n        expected ' + w + '\n        got      ' + g));
}

function set(pairs) {
  return Object.keys(pairs).map(function (k) { return { key: k, value: pairs[k] }; });
}

check('nothing set keeps nothing', testStoresFailures([]), false);
check('no settings at all keeps nothing', testStoresFailures(undefined), false);
check('store_failures: true keeps them', testStoresFailures(set({ store_failures: 'true' })), true);
check('a where alone keeps nothing', testStoresFailures(set({ where: "status = 'x'" })), false);
check('store_failures_as: table keeps them', testStoresFailures(set({ store_failures_as: 'table' })), true);
check('store_failures_as: view keeps them', testStoresFailures(set({ store_failures_as: 'view' })), true);
check('ephemeral wins over store_failures: true',
  testStoresFailures(set({ store_failures: 'true', store_failures_as: 'ephemeral' })), false);
check('the as is read in any case', testStoresFailures(set({ store_failures_as: 'Ephemeral', store_failures: 'true' })), false);
