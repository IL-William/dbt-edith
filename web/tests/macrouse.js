// What a macro's card, its palette row and the Reports panel say of who
// calls it, from the `used` the server sends (0105). Fixtures are invented.
// Run from the repository root: jsc web/tests/macrouse.js
var src = read('web/app.js');
eval(src.slice(src.indexOf('const USE_KINDS'), src.indexOf('function fillMacroCard')));

function check(label, got, want) {
  var g = JSON.stringify(got), w = JSON.stringify(want);
  print((g === w ? 'PASS  ' : 'FAIL  ') + label + (g === w ? '' : '\n        expected ' + w + '\n        got      ' + g));
}

print('--- a macro some node reaches ---');
var cents = macroUseText({ by: [{ kind: 'model', n: 9 }, { kind: 'test', n: 1 }], through: 3, callers: ['round_amount'], callers_of: 1 });
check('the head counts each kind, one and several', cents.head, 'used by 9 models, 1 test');
check('then how many come through other macros, and which call it', cents.notes,
  ['3 of them through other macros', 'called by round_amount']);
check('the palette says the total', cents.short, 'used by 10');
check('every one through macros says so',
  macroUseText({ by: [{ kind: 'model', n: 2 }], through: 2, callers: ['a'], callers_of: 1 }).notes[0], 'all of them through other macros');
check('nothing through macros, no caller: no note', macroUseText({ by: [{ kind: 'model', n: 1 }] }).notes, []);
check('a hook reads as one', macroUseText({ by: [{ kind: 'operation', n: 1 }] }).head, 'used by 1 hook');
check('a call dbt did not record is said so', macroUseText({ by: [{ kind: 'model', n: 3 }], written: 1 }).notes,
  ['1 of them only where dbt recorded no call, read from their SQL']);
check('every one of them', macroUseText({ by: [{ kind: 'model', n: 1 }], written: 1 }).notes,
  ['all of them only where dbt recorded no call, read from their SQL']);
check('analyses', macroUseText({ by: [{ kind: 'analysis', n: 2 }] }).head, 'used by 2 analyses');
check('more callers than named are counted',
  macroUseText({ by: [{ kind: 'model', n: 4 }], through: 4, callers: ['a', 'b', 'c'], callers_of: 5 }).notes[1], 'called by a, b, c and 2 more');

print('\n--- a macro no node reaches: no reference found, and why ---');
function why(used, adapter) { var t = macroUseText(used, adapter); return [t.head, t.short].concat(t.notes); }
check('nothing lists it', why({ why: 'none' }), ['no reference found', 'no reference found',
  'nothing in the manifest calls it: dbt run-operation or another package may']);
check('only macros no node reaches list it', why({ why: 'macros', callers: ['orphan'], callers_of: 1 })[2],
  'called only by orphan, which no node reaches either');
check('a generic test', why({ why: 'test' })[2], 'a generic test no enabled test uses');
check('a dispatch variant names the manifest\'s adapter', why({ why: 'dispatch' }, 'duckdb')[2],
  'a variant adapter.dispatch did not pick for duckdb, the one this manifest was parsed for');
check('and the adapter when the page has none', why({ why: 'dispatch' })[2],
  'a variant adapter.dispatch did not pick for the adapter, the one this manifest was parsed for');
check('generate_schema_name, or a macro of dbt\'s own overridden', why({ why: 'dbt' })[2],
  'dbt calls it itself by its name, and records no call');
check('what something dbt runs calls', why({ why: 'dbt', from: 'dbt.get_binding_char' })[2],
  'called by dbt.get_binding_char, which dbt runs itself');
check('a materialization counts the models of its name',
  why({ why: 'run', run: { key: 'materialized', name: 'tagged_view', nodes: 3 } })[2],
  'dbt runs it for each model materialized tagged_view, and 3 are');
check('one', why({ why: 'run', run: { key: 'materialized', name: 'tagged_view', nodes: 1 } })[2],
  'dbt runs it for each model materialized tagged_view, and 1 is');
check('none', why({ why: 'run', run: { key: 'materialized', name: 'tagged_view', nodes: 0 } })[2],
  'dbt runs it for each model materialized tagged_view, and none is');
check('an incremental strategy', why({ why: 'run', run: { key: 'incremental_strategy', name: 'custom', nodes: 2 } })[2],
  'dbt runs it for each incremental model with strategy custom, and 2 are');
check('a snapshot strategy', why({ why: 'run', run: { key: 'strategy', name: 'mine', nodes: 1 } })[2],
  'dbt runs it for each snapshot with strategy mine, and 1 is');
check('never the word unused, nor dead', ['none', 'macros', 'test', 'dispatch', 'run', 'dbt'].some(function (w) {
  return /unused|dead/.test(why({ why: w, callers: ['x'], callers_of: 1, run: { key: 'strategy', name: 'x', nodes: 0 } }).join(' '));
}), false);
check('an empty answer is no reference found', macroUseText(undefined).head, 'no reference found');

print('\n--- the report\'s groups ---');
var groups = macroGroups([
  { id: 'macro.shop.grant_select', name: 'grant_select', call: 'grant_select', file: 'macros/ops.sql', used: { why: 'none' } },
  { id: 'macro.shop.purge', name: 'purge', call: 'purge', used: { why: 'none' } },
  { id: 'macro.shop.helper', name: 'helper', call: 'helper', file: 'macros/ops.sql',
    used: { why: 'macros', callers: ['a', 'b', 'c'], callers_of: 4 } },
  { id: 'macro.shop.materialization_tagged_view_default', name: 'materialization_tagged_view_default',
    call: 'materialization_tagged_view_default', used: { why: 'run', run: { key: 'materialized', name: 'tagged_view', nodes: 1 } } },
  { id: 'macro.shop.schema_helper', name: 'schema_helper', call: 'schema_helper', used: { why: 'dbt', from: 'generate_schema_name' } },
]);
check('one group per reason, in the order sent', groups.map(function (g) { return g.why + ':' + g.items.length; }),
  ['none:2', 'macros:1', 'run:1', 'dbt:1']);
check('each with its title', groups.map(function (g) { return g.title; }),
  ['Nothing in the manifest calls them', 'Called only by macros no node reaches', 'Run by dbt for the nodes that name them',
    'Called by dbt itself']);
check('a macro not in the project has no file to open', groups[0].items[1].file, '');
check('the callers beside a macro only macros call', groups[1].items[0].extra, 'called by a, b, c and 1 more');
check('the models beside a materialization', groups[2].items[0].extra, '1 model materialized tagged_view');
check('the macro dbt runs beside its helper', groups[3].items[0].extra, 'called by generate_schema_name');
check('snapshots counted as such', runSaid({ key: 'strategy', name: 'mine', nodes: 2 }).count, '2 snapshots with strategy mine');
check('nothing beside the others', groups[0].items[0].extra, '');
check('no list, no group', macroGroups([]), []);

print('\n--- how a node in the list reaches the macro ---');
check('listing it', userHow({}), '');
check('through a macro', userHow({ via: 'round_amount' }), ' · through round_amount');
check('read from its SQL', userHow({ how: 'written' }), ' · read from its SQL, dbt recorded no call');
check('both, as the card counts it', userHow({ how: 'written', via: 'outer' }),
  ' · through outer, read from its SQL, dbt recorded no call');

