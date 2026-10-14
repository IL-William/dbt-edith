// Search palette: nodes, macros and plain files in one list, without duplicates.
// Run from the repository root: jsc web/tests/palette.js
var src = read('web/app.js');
// A single eval: a `const` declaration does not leak from one eval to the next,
// so `base` has to live in the same scope as `mergeHits`.
eval(src.slice(src.indexOf('const base = (path)'), src.indexOf('const dirOf'))
   + src.slice(src.indexOf('function mergeHits'), src.indexOf('/* One box for everything')));

function check(label, ok) { print((ok ? 'PASS  ' : 'FAIL  ') + label); }

var nodes = [
  { id: 'model.p.stg_x', name: 'stg_x', file: 'models/stg/stg_x.sql' },
  { id: 'source.p.raw.y', name: 'raw.y', file: '' },
];
var paths = [
  'models/stg/stg_x.sql',                                  // already represented by a node
  'tests/generic/data_quality/assert_ids_ordered.sql',     // generic test: not a node
  '.env',                                                  // not a node at all
  'macros/project_ops/audit.sql',
];
var out = mergeHits(nodes, [], paths);

check('nodes come first', out[0].kind === 'node' && out[1].kind === 'node');
check('a model .sql is not listed twice', out.filter(function (h) {
  return h.sub === 'models/stg/stg_x.sql'; }).length === 1);
check('a generic test can be found', out.some(function (h) {
  return h.kind === 'file' && h.label === 'assert_ids_ordered.sql'; }));
check('.env can be found', out.some(function (h) { return h.label === '.env'; }));
check('expected total', out.length === 5);
check('a file shows its name, not its path', out[2].label.indexOf('/') === -1);
check('a node without a file hides nothing', mergeHits([{ name: 'n', file: '' }], [], ['a.sql']).length === 2);
check('no node: files only', mergeHits([], [], paths).length === 4);
check('no file: nodes only', mergeHits(nodes, [], []).length === 2);

var macros = [
  { call: 'cents_to_dollars', name: 'cents_to_dollars', file: 'macros/cents_to_dollars.sql' }, // named after its file
  { call: 'cents_rounded', name: 'cents_rounded', file: 'macros/money.sql' },         // one of several in it
  { call: 'automate_dv.hub', name: 'hub', file: 'dbt_packages/automate_dv/macros/tables/hub.sql' },
  { call: 'test_is_positive', name: 'test_is_positive', file: 'tests/generic/is_positive.sql' },
];
var withMacros = mergeHits(nodes, macros, paths.concat(['macros/cents_to_dollars.sql', 'macros/money.sql',
  'dbt_packages/automate_dv/macros/tables/hub.sql',
  'tests/generic/is_positive.sql']));
check('macros come after the nodes, before the files', withMacros.map(function (h) { return h.kind; }).join() ===
  'node,node,macro,macro,macro,macro,file,file,file,file');
check('a macro reads as the project calls it', withMacros[4].label === 'automate_dv.hub');
check('a file named after its macro is not listed twice', !withMacros.some(function (h) {
  return h.kind === 'file' && (h.path === 'macros/cents_to_dollars.sql' || h.path.indexOf('/hub.sql') > 0); }));
check('nor one named after its generic test', !withMacros.some(function (h) {
  return h.kind === 'file' && h.path === 'tests/generic/is_positive.sql'; }));
check('a file holding more than the macro found stays', withMacros.some(function (h) {
  return h.kind === 'file' && h.path === 'macros/money.sql'; }));
