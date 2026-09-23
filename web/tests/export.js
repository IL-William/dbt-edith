// What an exported graph says and what it may carry: the words of its header,
// its file name, the page around the picture, and the rules that keep a file
// read far from here both safe and faithful to the canvas.
// Run from the repository root: jsc web/tests/export.js
var lin = read('web/lineage.js');
eval(lin.slice(lin.indexOf('const MAT = {'), lin.indexOf('let svg, root')));
eval(lin.slice(lin.indexOf('function subtitle(n)'), lin.indexOf('  function init(')));
var Lineage = { nodeRoles: nodeRoles, roleColor: roleColor, matLabel: matLabel, nodeColor: nodeColor, subtitle: subtitle };

var src = read('web/app.js');
eval(src.slice(src.indexOf('function selectKindCounts'), src.indexOf('async function loadSidecar')));
eval(src.slice(src.indexOf('function legendEntries'), src.indexOf('function paintLegend')));
eval(src.slice(src.indexOf('function exportTitle'), src.indexOf('function exportLineage')));
var css = read('web/app.css');

function check(label, got, want) {
  var g = JSON.stringify(got), w = JSON.stringify(want);
  print((g === w ? 'PASS  ' : 'FAIL  ') + label + (g === w ? '' : '\n        expected ' + w + '\n        got      ' + g));
}
function ok(label, cond) { print((cond ? 'PASS  ' : 'FAIL  ') + label); }
function labels(entries) { return entries.map(function (e) { return e[0]; }); }

print('--- the legend, shared with the screen ---');
var modelSub = { mode: 'model', edges: [], nodes: [
  { name: 'b', kind: 'model', materialized: 'table' },
  { name: 'a', kind: 'model', materialized: 'view' },
  { name: 's', kind: 'source' },
  { name: 'c', kind: 'model', materialized: 'table' },
] };
check('what is on the canvas, sorted, each once', labels(legendEntries(modelSub)), ['source', 'table', 'view']);
check('in the canvas colour', legendEntries(modelSub)[1][1], nodeColor({ kind: 'model', materialized: 'table' }));
// Two roles meet on the third column, which makes it mixed; the other two have
// nothing feeding them, which makes them raw. Both only ever appear on a box.
var colSub = { mode: 'column', nodes: [{ name: 'x' }, { name: 'y' }, { name: 'z' }],
  edges: [[0, 2], [1, 2]], edge_kinds: ['rename', 'cast'] };
check('column mode explains the edges, and the raw and mixed badges', labels(legendEntries(colSub)),
  ['cast', 'mixed', 'raw', 'rename']);
check('a column graph with no roles falls back to materializations',
  labels(legendEntries({ mode: 'column', edges: [], nodes: [{ name: 'x', kind: 'model', materialized: 'view' }] })), ['view']);
ok('the legend on screen paints the same entries',
  /function paintLegend\(sub\) \{[\s\S]{0,120}legendEntries\(sub\)/.test(src));

print('\n--- what the picture is called ---');
check('a selection is its expression',
  exportTitle({ mode: 'select', select: 'model_1 model_2 model_3', nodes: [{}], edges: [] }), 'model_1 model_2 model_3');
check('with its exclusion',
  exportTitle({ mode: 'select', select: 'a+', exclude: 'tag:x', nodes: [{}], edges: [] }), 'a+ --exclude tag:x');
check('a neighbourhood is its model',
  exportTitle({ mode: 'model', focus: 1, nodes: [{ name: 'stg_a' }, { name: 'fct_orders' }], edges: [] }), 'fct_orders');
check('a column is model.column', exportTitle({ mode: 'column', focus: 0, focus_column: 'amount',
  nodes: [{ name: 'amount', sub: 'fct_orders  ·  number' }], edges: [] }), 'fct_orders.amount');
check('nothing drawn has no name', exportTitle({ mode: 'select', nodes: [], edges: [] }), '');

var many = [];
for (var i = 1; i <= 30; i++) many.push('model_' + (i < 10 ? '0' : '') + i);
var long = shortTitle(many.join(' '), 60);
check('thirty terms keep the first ones and count the rest', long,
  'model_01 model_02 model_03 model_04 model_05 and 25 more');
ok('within sixty characters', long.length <= 60);
check('a short expression is kept whole', shortTitle('model_1 model_2 model_3', 60), 'model_1 model_2 model_3');
check('an exclusion that does not fit is named, never halved',
  shortTitle('a b --exclude ' + many.join(' '), 60), 'a b --exclude …');
check('one term longer than the limit is cut with an ellipsis',
  shortTitle('path:' + new Array(80).join('x'), 60).length, 60);
// Model names run long: a first term that cannot fit beside the count is the
// one cut, and the count survives.
var first = shortTitle('1+sat_web_shop__settled_customer_payment_monthly_detail+1 sat_b mdoel_typo', 60);
check('a first term too long is cut, not the count', first,
  '1+sat_web_shop__settled_customer_payment_monthly… and 2 more');
check('still within sixty', first.length, 60);

print('\n--- the command that checks it ---');
var tested = { mode: 'select', select: 'model_1 model_2', counts: { model: 2, test: 3 }, nodes: [{}], edges: [] };
check('a selection is what the dbt ls button types', exportCommand(tested), lsCommand('model_1 model_2', ''));
check('which reads', exportCommand(tested), 'dbt ls --select "model_1 model_2" --output name');
// Otherwise dbt ls prints the tests the checkbox kept off the canvas, and the
// reader's list is longer than the picture for no reason they can see.
check('a set with no test excludes them outright',
  exportCommand({ mode: 'select', select: 'model_1', exclude: 'tag:x', counts: { model: 1 }, nodes: [{}], edges: [] }),
  'dbt ls --select "model_1" --exclude "tag:x resource_type:test" --output name');
function hood(up, down, kind) {
  var nodes = [{ name: 'fct_orders', kind: kind || 'model', depth: 0 }];
  for (var u = 1; u <= up; u++) nodes.push({ name: 'up' + u, kind: 'model', depth: -u });
  for (var d = 1; d <= down; d++) nodes.push({ name: 'down' + d, kind: 'model', depth: d });
  return { mode: 'model', focus: 0, nodes: nodes, edges: [] };
}
check('a neighbourhood in dbt\'s own graph operators', exportCommand(hood(2, 2)),
  'dbt ls --select "2+fct_orders+2" --exclude "resource_type:test" --output name');
check('downstream only', exportCommand(hood(0, 1)),
  'dbt ls --select "fct_orders+1" --exclude "resource_type:test" --output name');
check('the model alone', exportCommand(hood(0, 0)),
  'dbt ls --select "fct_orders" --exclude "resource_type:test" --output name');
var withTest = hood(1, 0);
withTest.nodes.push({ name: 'not_null_fct_orders_id', kind: 'test', depth: 0 });
check('tests on the canvas are not excluded', exportCommand(withTest), 'dbt ls --select "1+fct_orders" --output name');
check('a source is not selected by a bare name', exportCommand(hood(1, 1, 'source')), '');
check('column lineage has no dbt equivalent',
  exportCommand({ mode: 'column', focus: 0, nodes: [{ name: 'amount', kind: 'model', depth: 0 }], edges: [] }), '');

print('\n--- the file name ---');
var day = '2026-09-23';
check('spaces become dashes', exportFileName('model_1 model_2 model_3', day),
  'lineage-model_1-model_2-model_3-2026-09-23.html');
check('so does selector punctuation, and + stays', exportFileName('tag:x,path:models/marts +stg_a', day),
  'lineage-tag-x-path-models-marts-+stg_a-2026-09-23.html');
check('nothing Windows forbids survives', exportFileName('a<>:"/\\|?*b', day), 'lineage-a-b-2026-09-23.html');
check('no dot or dash before the date', exportFileName('stg_a.', day), 'lineage-stg_a-2026-09-23.html');
check('a shortened title', exportFileName('a b --exclude …', day), 'lineage-a-b-exclude-2026-09-23.html');
ok('cut at sixty', exportFileName(new Array(100).join('ab'), day).length === 'lineage-'.length + 60 + '-2026-09-23.html'.length);
check('an empty title', exportFileName('', day), 'lineage-2026-09-23.html');
check('a day is padded', localDay(new Date(2026, 0, 5)), '2026-01-05');

print('\n--- moments, never ages ---');
var t = Date.UTC(2026, 8, 22, 6, 14) / 1000;
check('in UTC', exportStamp(t, 0), '2026-09-22 06:14 UTC+00:00');
check('two hours east', exportStamp(t, 120), '2026-09-22 08:14 UTC+02:00');
check('five and a half hours west', exportStamp(t, -330), '2026-09-22 00:44 UTC-05:30');
check('across midnight, padded', exportStamp(Date.UTC(2026, 0, 2, 1, 5) / 1000, -120), '2026-01-01 23:05 UTC-02:00');

print('\n--- the manifest, told to someone who was not there ---');
check('fresh', exportFreshness({ state: 'fresh' }), 'The manifest then matched the project\'s files.');
check('edited', exportFreshness({ state: 'edited', edited_n: 2 }),
  '2 files saved since the manifest was written had not been parsed, so the graph shows the project as it was before them.');
check('stale counts everything that moved', exportFreshness({ state: 'stale', committed_n: 3, gone_n: 1 }),
  '4 files had changed since the manifest was written, so some boxes may no longer match the code.');
check('one file', exportFreshness({ state: 'stale', committed_n: 1 }).indexOf('1 file had') === 0, true);
check('a branch behind its base', exportFreshness({ state: 'fresh', drift: { base: 'origin/main', behind: 3 } }),
  'The manifest then matched the project\'s files. The branch was 3 commits behind origin/main.');
check('nothing known, nothing said', exportFreshness(null), '');
ok('no age, which would be wrong by the time it is read', ['fresh', 'edited', 'stale'].every(function (s) {
  return !/ago|\d+(s|min|h|d)\b/.test(exportFreshness({ state: s, edited_n: 1, committed_n: 1,
    drift: { base: 'origin/main', behind: 1, fetched_at: 1 } }));
}));

print('\n--- the header ---');
var ctx = { project: 'jaffle_shop', dbtVersion: '1.9.4', manifestAt: '2026-09-22 08:14 UTC+02:00',
  branch: 'release/2.4', sha: 'a1b2c3d', cllSource: 'snowflake', fresh: { state: 'fresh' },
  version: '0.5.0', exportedAt: '2026-09-23 14:05 UTC+02:00' };
var drawn = [], all = [];
for (var k = 0; k < 2422; k++) {
  all.push('m' + k);
  if (k < 400) drawn.push({ name: 'm' + k, kind: 'model', depth: 0 });
}
var capped = { mode: 'select', select: 'big+', exclude: '', matched: 2422, counts: { model: 2422 }, truncated: true,
  nodes: drawn, edges: new Array(512), names: all, warnings: ['mdoel_x matches nothing'] };
var facts = exportFacts(capped, ctx);
check('the mode, in words', facts.kicker, 'Selection');
ok('a capped selection says so', facts.lines[0].indexOf('400 of 2422 drawn') >= 0);
check('and lists what it left out', facts.undrawn.length, 2022);
check('a mistyped name is not silently missing', facts.warnings, ['mdoel_x matches nothing']);
check('where it came from', facts.lines[1],
  'jaffle_shop  ·  dbt 1.9.4  ·  manifest written 2026-09-22 08:14 UTC+02:00  ·  branch release/2.4 at a1b2c3d');
check('when, and how true', facts.lines[2],
  'Exported 2026-09-23 14:05 UTC+02:00 with dbt-edith 0.5.0. The manifest then matched the project\'s files.');
ok('the command comes along', facts.command.indexOf('dbt ls --select "big+"') === 0);
var colFacts = exportFacts({ mode: 'column', focus: 0, focus_column: 'amount', truncated: false, edges: [[1, 0]],
  nodes: [{ name: 'amount', sub: 'fct_orders  ·  number', depth: 0 }, { name: 'amount', sub: 'stg_payments', depth: -1 }] }, ctx);
check('column mode, and how far it reaches', colFacts.lines[0], 'amount · 2 columns · 1 edges  ·  1 level up, 0 down');
// A synthetic cache looks exactly like a real one apart from its source.
ok('column mode names the cache it was drawn from', colFacts.lines[1].indexOf('column lineage from snowflake') >= 0);
ok('no other mode does', facts.lines[1].indexOf('column lineage') < 0);
var leaky = Object.assign({}, ctx, { manifest_path: '/Users/someone/p/target/manifest.json',
  root: '/Users/someone/p', sandbox: 'dev_someone', cll_file: '/Users/someone/p/target/column_lineage.json' });
ok('a field outside the list never reaches the file', JSON.stringify(exportFacts(capped, leaky)).indexOf('someone') < 0);

print('\n--- the frame of the picture ---');
var fr = frameOf({ x0: 0, x1: 1480, y0: -300, y1: 300 });
ok('room for a +N badge on either side', fr.x <= -27 && fr.x + fr.w >= 1480 + 27);
ok('and for a role tag above', fr.y <= -300 - 14);
var one = frameOf({ x0: 0, x1: 200, y0: -24, y1: 24 });
check('one box gets the smallest frame, not a giant one', [one.w, one.h], [1000, 560]);
check('centred on it', [one.x + one.w / 2, one.y + one.h / 2], [100, 0]);
var tightOne = frameOf({ x0: 0, x1: 200, y0: -24, y1: 24 }, 0, 0);
check('an image takes the tight frame, badges included', [tightOne.x, tightOne.y, tightOne.w, tightOne.h], [-40, -64, 280, 128]);

print('\n--- zooming ---');
var vb = [0, 0, 1000, 500];
var z = zoomViewBox(vb, 250, 100, 2, 120, 4000);
check('twice as close', [z[2], z[3]], [500, 250]);
check('the point under the pointer stays put', [(250 - z[0]) / z[2], (100 - z[1]) / z[3]], [0.25, 0.2]);
var close = zoomViewBox(vb, 500, 250, 1000, 120, 4000);
ok('no closer than the floor', Math.abs(close[2] - 120) < 1e-9);
ok('and the ratio kept', Math.abs(close[2] / close[3] - 2) < 1e-9);
check('no further than the ceiling', zoomViewBox(vb, 500, 250, 0.0001, 120, 4000)[2], 4000);

print('\n--- the image ---');
check('a small graph is pasted twice as sharp', pngScale(1000, 560), 2);
ok('a very wide one stays within what a canvas takes', 20000 * pngScale(20000, 3000) <= 16384);
ok('a large one stays within the memory budget', 6000 * 6000 * Math.pow(pngScale(6000, 6000), 2) <= 16e6 + 1);

print('\n--- the file ---');
var nasty = '</script><img src=x onerror=alert(1)> & "q" \'s';
var svgText = '<svg xmlns="http://www.w3.org/2000/svg" id="graph" viewBox="0 0 1000 560">'
  + '<g><g class="nd" data-id="model.p.a"><text class="t1">a</text></g></g></svg>';
function doc(text, frame) {
  return exportDocument({
    heading: text, noun: 'Nodes', svg: svgText, frame: frame || { x: 0, y: 0, w: 1000, h: 560 },
    legend: [['table', '#4ec78d'], [text, '#fff']],
    facts: { kicker: 'Selection', lines: [text], warnings: [text], undrawn: [text],
      command: 'dbt ls --select "' + text + '" --output name' },
    nodes: [{ id: text, name: text, sub: text, file: text }],
  });
}
var hostile = doc(nasty);
var plain = doc('model_1 model_2', { x: 0, y: 0, w: 1000, h: 3000 });
var charset = hostile.indexOf('<meta charset="utf-8">');
ok('the charset comes first, since a file has no header to say it', charset >= 0 && charset < 200);
ok('a policy forbids every fetch', hostile.indexOf('content="default-src \'none\';') > 0);
ok('no tag from the project', hostile.indexOf('<img') < 0 && hostile.indexOf('</script><') < 0);
ok('escaped in attributes too', hostile.indexOf('data-id="' + escapeHtml(nasty) + '"') > 0);
check('escapeHtml', escapeHtml('<a href="x">&\'</a>'), '&lt;a href=&quot;x&quot;&gt;&amp;&#39;&lt;/a&gt;');
check('exactly one script', hostile.split('<script').length - 1, 1);
function script(html) { return html.slice(html.indexOf('<script>') + 8, html.indexOf('</script>')); }
var expected = '(' + exportViewer + ')(' + zoomViewBox + ');';
ok('and it is the same text whatever is drawn', script(hostile) === expected && script(plain) === expected);
// Quoted values are blanked first: the hostile text is still in them, escaped,
// and `onerror=` inside a data-id is only text.
ok('no handler in any tag', !(hostile.match(/<[a-z][^>]*>/gi) || []).some(function (tag) {
  return /\son[a-z]+\s*=/i.test(tag.replace(/"[^"]*"/g, '""'));
}));
var bare = plain.replace('http://www.w3.org/2000/svg', '');
ok('no address, source, import or link besides the SVG namespace',
  !/https?:|src=|href=|url\(|@import|<link|<img/i.test(bare));
ok('the picture goes in as it came', plain.indexOf(svgText) > 0);
ok('a tall graph prints in portrait', plain.indexOf('size: portrait') > 0);
ok('a wide one in landscape', hostile.indexOf('size: landscape') > 0);
ok('no em dash', hostile.indexOf('—') < 0 && plain.indexOf('—') < 0);

print('\n--- the viewer runs far from here ---');
var viewer = String(exportViewer) + String(zoomViewBox);
ok('nothing in it would end its script early', !/<\/script|<script|<!--/i.test(viewer));
var named = ['S', 'api', 'toast', 'Lineage', 'escapeHtml', 'lsCommand', 'legendEntries', 'exportDocument',
  'exportCanvasCss', 'exportPageCss', 'exportFacts', 'exportTitle']
  .filter(function (w) { return new RegExp('(^|[^\\w$.])' + w + '\\b').test(viewer); });
check('it names nothing from app.js', named, []);
ok('nor its DOM helpers', !/\$\$?\(/.test(viewer));

print('\n--- the canvas looks the same in the file ---');
var copied = exportCanvasCss();
var rules = css.split('\n').filter(function (l) { return /^(#graph text|\.edge|\.nd|\.role)/.test(l); });
ok('the canvas rules are where this test looks for them', rules.length >= 15);
check('every one of them is copied as it is',
  rules.filter(function (l) { return copied.split('\n').indexOf(l) < 0; }), []);
function rootVars(text) {
  var at = text.indexOf(':root {');
  var block = text.slice(at, text.indexOf('}', at));
  var out = {}, re = /--([\w-]+):\s*([^;]+);/g, m;
  while ((m = re.exec(block))) out[m[1]] = m[2].trim();
  return out;
}
var appVars = rootVars(css), fileVars = rootVars(copied);
var used = (copied + exportPageCss('landscape')).match(/var\(--[\w-]+\)/g) || [];
var wrong = used.map(function (u) { return u.slice(6, -1); })
  .filter(function (v, n, a) { return a.indexOf(v) === n && fileVars[v] !== appVars[v]; });
check('every variable used is defined, with the value app.css gives it', wrong, []);
