// What a static site's page says of itself: the header's facts, the tab's title, the node an address names.
// Run from the repository root: jsc web/tests/sitepage.js
var src = read('web/app.js');
eval(src.slice(src.indexOf('function siteStamp'), src.indexOf('function paintSiteStamp')));
// What the Columns hint of a site reads of the cache: the share of its edges collin matched by name.
eval(src.slice(src.indexOf('function collinRead('), src.indexOf('function collinModelNote')));
eval(src.slice(src.indexOf('function inferredShare'), src.indexOf('/* How old a cache is')));

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

// The Code tab says what the manifest holds, and what it does not.
var code = { raw: 'select * from {{ ref(\'a\') }}', compiled: 'select * from db.a', lang: 'sql', file: 'models/b.sql', url: null };
check('the source is Jinja over SQL', siteCodeBody(code, 'source'), { text: code.raw, mode: 'dbt-sql' });
check('the compiled text is SQL', siteCodeBody(code, 'compiled'), { text: 'select * from db.a', mode: 'text/x-sql' });
check('a Python model is plain text, source and compiled', [siteCodeBody({ raw: 'x', compiled: 'y', lang: 'python' }, 'source').mode, siteCodeBody({ raw: 'x', compiled: 'y', lang: 'python' }, 'compiled').mode],
  ['text/plain', 'text/plain']);
check('a node dbt did not compile says so, and names the remedy', siteCodeBody({ raw: 'x', compiled: null, lang: 'sql' }, 'compiled'),
  { note: 'dbt did not compile this node when it wrote the manifest.', run: ['Run ', 'dbt-edith --site', ' after dbt build, dbt compile or dbt docs generate.'] });
check('its source is still shown', siteCodeBody({ raw: 'x', compiled: null, lang: 'sql' }, 'source').text, 'x');
check('a compiled text that is empty is a text, not a missing one', siteCodeBody({ raw: 'x', compiled: '', lang: 'sql' }, 'compiled').text, '');
check('no source is said, not shown as an empty editor', siteCodeBody({ raw: '', compiled: 'select 2', lang: 'sql' }, 'source'), { note: 'The manifest holds no source for this node.' });
check('no language is SQL', siteCodeBody({ raw: 'x', compiled: 'y' }, 'compiled').mode, 'text/x-sql');
check('the server\'s hint about types is unchanged', siteNoTypes(false), 'no types: run dbt compile --write-catalog to pull them from Snowflake');
check('a site\'s says which step writes the catalog, and not Snowflake', [siteNoTypes(true).includes('dbt-edith --site'), siteNoTypes(true).includes('dbt docs generate'), siteNoTypes(true).includes('--write-catalog'), siteNoTypes(true).includes('Snowflake')],
  [true, true, true, false]);

// The column lineage a site carries: older than the manifest, said in the Columns tab and on the status line, and nowhere else.
var kinds = [{ kind: 'passthrough', edges: 6 }, { kind: 'inferred', edges: 2 }];
var fresh = { cll: true, cll_tool: 'collin', cll_stale: false, cll_kinds: kinds };
check('no column lineage, nothing to say', [siteCllHint({}), siteCllHint({ cll: false }), siteCllHint(null)], [null, null, null]);
check('a cache older than the manifest says so, in its own tone', siteCllHint(Object.assign({}, fresh, { cll_stale: true })),
  { text: 'older than the manifest: it may not match this commit', tone: 'stale', title: siteStaleSaid() });
check('and says it whatever tool wrote it', siteCllHint({ cll: true, cll_tool: 'fusion', cll_stale: true }).tone, 'stale');
check('collin\'s cache says how far to trust it, as the server\'s page does', siteCllHint(fresh).text, 'alpha, can be wrong \u00b7 25% guessed by name');
check('and its tooltip says why', siteCllHint(fresh).title.split('\n').slice(0, 2),
  ['Alpha: collin is new and can be wrong. Check an edge against the compiled SQL before relying on it.', '2 of the 8 edges are a match by column name, not read from the SQL.']);
check('a cache read whole has the alpha line alone', siteCllHint({ cll: true, cll_tool: 'collin', cll_kinds: [{ kind: 'rename', edges: 3 }] }).text, 'alpha, can be wrong');
check('another tool\'s fresh cache has nothing to say', siteCllHint({ cll: true, cll_tool: 'fusion', cll_kinds: kinds }), null);
check('the sentence names what writes it again', siteStaleSaid().includes('--run-collin') && siteStaleSaid().includes('older than the manifest') && siteStaleSaid().includes('may not match'), true);
check('the status line says it in a few words, after a stale cache only', [siteStaleTail(fresh), siteStaleTail({}), siteStaleTail(null), siteStaleTail(Object.assign({}, fresh, { cll_stale: true }))],
  ['', '', '', ' \u00b7 column lineage older than the manifest']);
check('no column lineage in the meta, however stale it says', siteStaleTail({ cll: false, cll_stale: true }), '');
