// What a static site's page says of itself: the header's facts, the tab's title, the node an address names.
// Run from the repository root: jsc web/tests/sitepage.js
var src = read('web/app.js');
eval(src.slice(src.indexOf('function siteStamp'), src.indexOf('function paintSiteStamp')));

function check(label, got, want) {
  var g = JSON.stringify(got), w = JSON.stringify(want);
  print((g === w ? 'PASS  ' : 'FAIL  ') + label + (g === w ? '' : '\n        expected ' + w + '\n        got      ' + g));
}
var fmt = function (d) { return 'at ' + d.toISOString(); };
var meta = { environment: 'prod', commit: '0123456789abcdef0123456789abcdef01234567',
  commit_url: 'https://github.com/acme/shop/commit/0123456789abcdef0123456789abcdef01234567',
  generated_at: '2030-01-02T03:04:05Z', dbt_version: '1.9.0', project: 'shop' };

check('the header names the environment, a short sha with its link, the time and dbt', siteStamp(meta, fmt), {
  env: 'prod', sha: '0123456', url: meta.commit_url, when: 'at 2030-01-02T03:04:05.000Z', dbt: 'dbt 1.9.0' });
check('with nothing known there is nothing to say', siteStamp({}, fmt), { env: '', sha: '', url: '', when: '', dbt: '' });
check('a commit without a host has a sha and no link', siteStamp({ commit: 'abcdef1234' }, fmt).url, '');
check('a link that is not https is no link', siteStamp({ commit: 'abcdef1', commit_url: 'javascript:alert(1)' }, fmt).url, '');
check('a link to another scheme is no link', siteStamp({ commit: 'abcdef1', commit_url: 'http://github.com/a/b/commit/abcdef1' }, fmt).url, '');
check('a time that is not one is left out', siteStamp({ generated_at: 'yesterday' }, fmt).when, '');
check('the time is the reader\'s own clock by default', typeof siteStamp(meta).when, 'string');

check('the title is the project, the environment and the tool', siteTitle(meta), 'shop · prod · dbt-edith');
check('without an environment', siteTitle({ project: 'shop' }), 'shop · dbt-edith');
check('with nothing', siteTitle({}), 'dbt-edith');

check('an address names a node', siteHashId('#model.shop.orders'), 'model.shop.orders');
check('its escapes are read', siteHashId('#' + encodeURIComponent('model.shop.café orders')), 'model.shop.café orders');
check('no address names none', siteHashId(''), '');
check('a bad escape names none', siteHashId('#%E0%A4%A'), '');
check('a node is written as it is read', siteHashId(siteHash('model.shop.orders_😀')), 'model.shop.orders_😀');
