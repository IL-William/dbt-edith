// The Catalog: the words beside a node's name, its tabs, what the Columns filter
// counts, the halves of a Lineage cell and of a job chip, and the rules that
// give both tabs one heading style, the type column its dim colour, and the
// hover card's column list the colour it had. Fixtures are invented.
// Run from the repository root: jsc web/tests/catalog.js
var lin = read('web/lineage.js');
eval(lin.slice(lin.indexOf('function subtitle(n)'), lin.indexOf('  function init(')));
var Lineage = { subtitleParts: subtitleParts };

var app = read('web/app.js');
eval(app.slice(app.indexOf('function catalogMeta'), app.indexOf('function renderCatalog')));
eval(app.slice(app.indexOf('function columnCountText'), app.indexOf('function catalogColumns')));
eval(app.slice(app.indexOf('function relationParts'), app.indexOf('function relationBody')));
eval(app.slice(app.indexOf('function reachParts'), app.indexOf('/* Whether the Columns table has a Lineage column')));
eval(app.slice(app.indexOf('function jobsChipText'), app.indexOf('/* One chip per step that runs the node')));

var css = read('web/app.css');

function check(label, got, want) {
  var g = JSON.stringify(got), w = JSON.stringify(want);
  print((g === w ? 'PASS  ' : 'FAIL  ') + label + (g === w ? '' : '\n        expected ' + w + '\n        got      ' + g));
}
// The declarations of the first rule whose selector is exactly `sel`.
function rule(sel) {
  var plain = css.replace(/\/\*[\s\S]*?\*\//g, ''), at = 0;
  while (true) {
    var open = plain.indexOf('{', at);
    if (open < 0) return '';
    var prelude = plain.slice(at, open).trim().replace(/\s+/g, ' ');
    var close = plain.indexOf('}', open);
    if (prelude.split(', ').indexOf(sel) >= 0 || prelude.split(',').map(function (p) { return p.trim(); }).indexOf(sel) >= 0) return plain.slice(open + 1, close);
    at = close + 1;
  }
}

print('--- the words beside the name ---');
check('a table', catalogMeta({ kind: 'model', materialized: 'table', schema: 'main' }), 'table \u00b7 main');
check('a view', catalogMeta({ kind: 'model', materialized: 'view', schema: 'marts' }), 'view \u00b7 marts');
check('an incremental model says its strategy, as the hover card does',
  catalogMeta({ kind: 'model', materialized: 'incremental', strategy_shown: 'merge', schema: 'marts' }), 'merge \u00b7 marts');
check('a source', catalogMeta({ kind: 'source', materialized: 'source', schema: 'raw' }), 'source \u00b7 raw');
check('a disabled model keeps its materialization: its chip says it is disabled',
  catalogMeta({ kind: 'model', disabled: true, materialized: 'view', schema: 'marts' }), 'view \u00b7 marts');
check('the test count is Context\'s to say', catalogMeta({ kind: 'model', materialized: 'table', schema: 'main', tests: 3 }), 'table \u00b7 main');
check('no schema known', catalogMeta({ kind: 'model', materialized: 'table' }), 'table');
check('the same words as the hover card\'s lead', catalogMeta({ kind: 'seed', schema: 'main' }).split(' \u00b7 ')[0],
  subtitleParts({ kind: 'seed', schema: 'main' }).lead);

print('\n--- the tabs ---');
check('a model has Preview and Columns, counted', catalogTabList({ kind: 'model', columns: [1, 2, 3] }),
  [{ key: 'preview', label: 'Preview', count: 0 }, { key: 'columns', label: 'Columns', count: 3 }]);
check('a model with no column has no badge', catalogTabList({ kind: 'model', columns: [] })[1].count, 0);
check('a test has Preview alone', catalogTabList({ kind: 'test', columns: [] }), [{ key: 'preview', label: 'Preview', count: 0 }]);
// What the scripts, the tour and take_shots.py find the tabs by.
check('the tabs keep the ctab class beside the dock\'s tab, and say what they are',
  /className = 'tab ctab'/.test(app) && /setAttribute\('role', 'tablist'\)/.test(app) && /setAttribute\('aria-selected'/.test(app), true);

print('\n--- the filter\'s count ---');
check('nothing typed, every column typed', columnCountText(8, 8, 8, false), '8/8 typed');
check('nothing typed, some typed', columnCountText(8, 8, 7, false), '7/8 typed');
check('filtering', columnCountText(3, 8, 7, true), '3 of 8 \u00b7 7/8 typed');
check('filtering to none', columnCountText(0, 8, 7, true), '0 of 8 \u00b7 7/8 typed');
check('no type at all is the sentence beside the box, not a count', columnCountText(8, 8, 0, false), '');
check('filtering with no type', columnCountText(2, 8, 0, true), '2 of 8');
check('the count keeps its widest form while a word is typed, in ch', columnCountWidth(200, 180, true), '26ch');
check('with no type the widest form is the kept of total', columnCountWidth(200, 0, true), '10ch');
check('nothing typed keeps no room', columnCountWidth(200, 180, false), '');

print('--- a relation name breaks after its dots ---');
// What relationBody appends: a <wbr> between two pieces, never after the last.
function nameParts(t) { return relationParts(t).join('|'); }
check('a quoted name', nameParts('"jaffle_shop"."main"."customers"'), '"jaffle_shop".|"main".|"customers"');
check('an unquoted name', nameParts('analytics.dbt_alice.stg_orders'), 'analytics.|dbt_alice.|stg_orders');
check('a name with no dot is one piece, so no <wbr>', relationParts('customers'), ['customers']);
check('a trailing dot ends the last piece, no <wbr> after it', relationParts('main.'), ['main.']);
check('the pieces read back as the name, whatever it holds',
  ['"jaffle_shop"."main"."customers"', 'analytics.dbt_alice.stg_orders', 'customers', 'main.', 'a..b', ''].map(function (t) { return relationParts(t).join('') === t; }),
  [true, true, true, true, true, true]);

print('\n--- the halves of a Lineage cell ---');
// Nodes without a DOM: a tag and a text.
var document = { createElement: function (tag) { return { tag: tag }; } };
function drawn(parts) { return parts.map(function (p) { return typeof p === 'string' ? p : p.tag + ':' + p.textContent; }); }
check('upstream: the arrow, then the figure', drawn(reachParts('\u21902')), ['\u2190', 'b:2']);
check('downstream: the figure, then the arrow', drawn(reachParts('12\u2192')), ['b:12', '\u2192']);
check('while it is on its way', drawn(reachParts('\u2190\u2026')), ['\u2190', 'b:\u2026']);
check('and downstream', drawn(reachParts('\u2026\u2192')), ['b:\u2026', '\u2192']);

print('\n--- a job chip ---');
var job = { kind: 'airflow', name: 'shop_daily' };
check('its two halves', jobsChipParts(job, { name: 'marts', n: 2 }), ['shop_daily', 'marts']);
check('and the text the title and the tests know', jobsChipText(job, { name: 'marts', n: 2 }), 'shop_daily \u203a marts');
check('a dbt Cloud step', jobsChipParts({ kind: 'dbt_cloud', name: 'Daily build', steps: 3 }, { n: 2 }), ['Daily build', 'step 2 of 3']);

print('\n--- one look ---');
check('a heading is the same in both tabs', /letter-spacing:\s*\.05em/.test(rule('.cat-body h3')) && /font-size:\s*11px/.test(rule('.cat-body h3')), true);
check('a heading beside buttons is a row of its own, 10px between', /gap:\s*10px/.test(rule('.cat-body .sechead')), true);
check('the type column is dim, not the source colour', /color:\s*var\(--fg-mute\)/.test(rule('.cols .c-type')) && !/--source/.test(rule('.cols .c-type')), true);
check('the figure of a Lineage cell stands out, the arrow does not', [/var\(--fg-mute\)/.test(rule('.cols .c-lin')), /var\(--fg\)/.test(rule('.cols .c-lin b'))], [true, true]);
check('the Catalog\'s crumb is scoped, the editor\'s breadcrumb keeps its own', /margin:\s*3px 0 6px 11px/.test(rule('.cat-head .crumb')), true);
check('the dock\'s tab is the Catalog\'s too, in a row 32px high', /height:\s*32px/.test(rule('.cat-tabs')), true);
check('the hover card\'s column type is quiet too (the shell step), not the source colour', /color:\s*var\(--fg-mute\)/.test(rule('.hc-col .c-type')) && !/--source/.test(rule('.hc-col .c-type')), true);
check('the Location sizes are the Catalog\'s alone: the settings\' table and the clone dialog keep theirs',
  [/font-size:\s*10px/.test(rule('.loc th')), /padding:\s*4px 12px 4px 0/.test(rule('.loc td')), /padding:\s*5px 12px 5px 0/.test(rule('.cat-body .loc td'))], [true, true, true]);
check('the words beside the name give way before the name does',
  [/flex:\s*0 1000 auto/.test(rule('.cat-title .meta')), /flex:\s*0 0 auto/.test(rule('.cat-title .nm'))], [true, true]);
check('a test chip has no margin of its own: its box gaps them, so a row stays one height',
  [!/margin/.test(rule('.testchip')), /gap:\s*4px/.test(rule('.cols .testchips'))], [true, true]);
check('a job chip wraps rather than overflow', /min-height:\s*22px/.test(rule('.cat-body .jobchip')), true);
check('Sort is a tray of segments, the one in front raised', /box-shadow/.test(rule('.coltools .segbtn.active')), true);
check('a relation name breaks at its dots, inside a part only when the part alone is too long',
  [/word-break:\s*normal/.test(rule('.relcell .relname')), /overflow-wrap:\s*break-word/.test(rule('.relcell .relname'))], [true, true]);
check('in the Location table the name keeps its longest part whole and the buttons wrap under it',
  [/min-width:\s*min-content/.test(rule('.loc .relcell .relname')), /flex-wrap:\s*wrap/.test(rule('.loc .relcell'))], [true, true]);
