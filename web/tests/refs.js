// ref() and source() calls in a model, a versioned model's with its version,
// the names a file's parents are linked by, a name read off a unique_id, and
// what the Catalog says beside a version (0104). Fixtures are invented.
// Run from the repository root: jsc web/tests/refs.js
var src = read('web/app.js');
eval(src.slice(src.indexOf('const REF_RE'), src.indexOf("/* var('x')")));
eval(src.slice(src.indexOf('function versionChip'), src.indexOf('function renderCatalog')));
eval(src.slice(src.indexOf('function parentNames'), src.indexOf('async function markRefs')));
eval(src.slice(src.indexOf('function nameOfId'), src.indexOf('/* `look` asks the server')));

function check(label, got, want) {
  var g = JSON.stringify(got), w = JSON.stringify(want);
  print((g === w ? 'PASS  ' : 'FAIL  ') + label + (g === w ? '' : '\n        expected ' + w + '\n        got      ' + g));
}
function names(text) { return scanCalls(maskJinjaComments(text)).map(function (c) { return c.name; }); }
// The text each call's first range covers, which is what gets the link.
function linked(text) {
  return scanCalls(text).map(function (c) { return text.slice(c.ranges[0][0], c.ranges[0][1]); });
}

print('--- the shapes there were ---');
check('a model', names("select * from {{ ref('orders') }}"), ['orders']);
check('a model of a package', names("{{ ref('shop', 'orders') }}"), ['orders']);
check('a source', names("{{ source('crm', 'customers') }}"), ['crm.customers']);
check('a source needs its table', names("{{ source('crm') }}"), []);

print('\n--- a version ---');
check('v=', names("{{ ref('orders', v=2) }}"), ['orders.v2']);
check('version=', names("{{ ref('orders', version=2) }}"), ['orders.v2']);
check('a quoted version', names("{{ ref('orders', v='beta') }}"), ['orders.vbeta']);
check('spaces around the =', names("{{ ref( 'orders' , v = 3 ) }}"), ['orders.v3']);
check('a package, a model and a version', names("{{ ref('shop', 'orders', v=1) }}"), ['orders.v1']);
check('a dotted version', names("{{ ref('orders', v=1.5) }}"), ['orders.v1.5']);
check('no version is the bare name', names("{{ ref('orders') }} {{ ref('orders', v=1) }}"), ['orders', 'orders.v1']);
check('the link stays on the model name', linked("{{ ref('shop', 'orders', v=1) }}"), ['orders']);
check('an argument that is no version is no call', names("{{ ref('orders', w=1) }}"), []);
check('a version dbt reads as false is none, as dbt tests it',
  names("{{ ref('a', v=0) }} {{ ref('b', v=None) }} {{ ref('c', version=none) }} {{ ref('d', v='') }} {{ ref('e', v=0.0) }} {{ ref('f', v=False) }}"),
  ['a', 'b', 'c', 'd', 'e', 'f']);
check('a name or True is no version, whatever it holds once rendered',
  names("{{ ref('a', v=v) }} {{ ref('b', v=True) }} {{ ref('c', version=latest) }} {{ ref('d', v=1x) }} {{ ref('e', v=my.v) }} {{ ref('f', v=1.) }}"),
  ['a', 'b', 'c', 'd', 'e', 'f']);
check('a number is one, with a dot or without', names("{{ ref('a', v=2) }} {{ ref('b', v=2.1) }} {{ ref('c', v=10) }}"), ['a.v2', 'b.v2.1', 'c.v10']);
check("a quoted '0' is a string, which is true", names("{{ ref('orders', v='0') }}"), ['orders.v0']);
check('a version 10 is no zero', names("{{ ref('orders', v=10) }}"), ['orders.v10']);

print('\n--- the names a file\'s parents are linked by ---');
var v1 = { name: 'orders.v1', base: 'orders' }, v2 = { name: 'orders.v2', base: 'orders', latest: true };
check('a version is also its bare name', Object.keys(parentNames([v1])).sort(), ['orders', 'orders.v1']);
check('the bare name goes to the latest among the parents', parentNames([v1, v2]).orders.name, 'orders.v2');
check('in either order', parentNames([v2, v1]).orders.name, 'orders.v2');
check('never over a parent of that very name',
  parentNames([{ name: 'orders' }, v2]).orders.name, 'orders');
check('an unversioned parent is filed as before', Object.keys(parentNames([{ name: 'crm.customers' }, { name: 'stg' }])).sort(), ['crm.customers', 'stg']);
check('an answer is filed under the name asked',
  Object.keys(byAsked([{ name: 'orders.v2', asked: 'orders' }, { name: 'stg' }])).sort(), ['orders', 'stg']);

print('\n--- a name read off a unique_id ---');
check('the node the page holds', nameOfId('model.shop.orders.v2', { id: 'model.shop.orders.v2', name: 'orders.v2' }), 'orders.v2');
check('a version', nameOfId('model.shop.orders.v2', null), 'orders.v2');
check('a model', nameOfId('model.shop.orders', { id: 'model.shop.other', name: 'other' }), 'orders');
check('a source', nameOfId('source.shop.crm.customers'), 'crm.customers');
check('a generic test, without its hash', nameOfId('test.shop.not_null_orders_id.5a189d0695'), 'not_null_orders_id');
check('a singular test', nameOfId('test.shop.assert_positive'), 'assert_positive');
check('nothing', nameOfId(undefined), '');

print('\n--- the chip beside a version ---');
check('no chip on an unversioned model', versionChip({ name: 'orders' }), null);
check('the latest', versionChip({ name: 'orders.v2', version: '2', latest_version: '2', version_state: 'latest' }),
  { text: 'latest', title: "The latest version of orders: ref('orders') reaches v2." });
check('an old one', versionChip({ name: 'orders.v1', version: '1', latest_version: '2', version_state: 'old' }),
  { text: 'old · latest v2', title: "ref('orders') reaches v2; ref('orders', v=1) reaches this one." });
check('a prerelease', versionChip({ name: 'orders.v3', version: '3', latest_version: '2', version_state: 'prerelease' }).text,
  'prerelease · latest v2');
check('a version 0 is none of them, as dbt compares it',
  versionChip({ name: 'orders.v0', version: '0', latest_version: '1' }).text, 'version · latest v1');
check('no latest named', versionChip({ name: 'orders.v1', version: '1' }),
  { text: 'version', title: "the manifest names no latest version of orders; ref('orders', v=1) reaches this one." });
