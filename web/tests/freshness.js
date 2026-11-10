// The manifest pill and its card: which state each payload
// lands in, and what the card is made to say about it. The card carries the
// words, so most of these assert on sentences rather than on numbers. Fixtures
// mirror the /api/freshness shape with invented paths.
// Run from the repository root: jsc web/tests/freshness.js
var src = read('web/app.js');
eval(src.slice(src.indexOf('function humanAge'), src.indexOf('function sendToTerminal')));

function check(label, got, want) {
  var g = JSON.stringify(got), w = JSON.stringify(want);
  print((g === w ? 'PASS  ' : 'FAIL  ') + label + (g === w ? '' : '\n        expected ' + w + '\n        got      ' + g));
}
function ok(label, cond) { check(label, !!cond, true); }
/* Everything the card would put on screen, as one string to search. */
function said(b) {
  var out = [b.head, b.when, b.desc, b.paused, b.done].concat(b.action ? [b.action.label, b.action.hint] : []);
  b.groups.forEach(function (g) {
    out.push(g.head);
    if (g.note) out.push(g.note);
    g.paths.forEach(function (p) { out.push(p.full); });
    if (g.more > 0) out.push('and ' + g.more + ' more');
  });
  b.branch.forEach(function (l) {
    out.push(l.parts.map(function (x) { return typeof x === 'string' ? x : x.code; }).join(''));
  });
  return out.join('\n');
}

var now = Math.floor(Date.now() / 1000);
function payload(over) {
  var base = {
    state: 'fresh', manifest_at: now - 720, age_secs: 720,
    edited: [], edited_n: 0, committed: [], committed_n: 0, gone: [], gone_n: 0,
    head: { sha: 'a1b2c3d', subject: 'Add the revenue mart', at: now - 900 },
    drift: { base: 'origin/main', behind: 0, fetched_at: now - 120 },
    advice: '',
  };
  for (var k in over) base[k] = over[k];
  return base;
}

print('--- nothing has changed since the parse ---');
var fresh = freshnessBadge(payload({}));
check('green', fresh.tone, 'ok');
check('the pill says manifest, and its age apart', [fresh.label, fresh.age], ['manifest', '12min']);
check('no drift segment when the branch is level', fresh.drift, '');
check('the card leads with a sentence, not a number', fresh.head, 'The lineage matches your files');
ok('and says what that means', said(fresh).indexOf('what your project says') >= 0);
ok('names the moment it was written', fresh.when.indexOf(', 12 min ago') >= 0);
// The card's ages are the board's ("2 min ago", "a moment ago"); the pill and the other screens keep humanAge's.
check('the card spells its ages as the board does, every unit a number, a space and the unit', [0, 9, 10, 45, 120, 12 * 60, 3 * 3600 + 300, 3 * 86400].map(cardAge),
  ['a moment ago', 'a moment ago', '10 s ago', '45 s ago', '2 min ago', '12 min ago', '3 h ago', '3 d ago']);
check('the minutes stop at 59 and the hours are floored', [3569, 3570, 3599, 3600, 86399, 86400].map(cardAge),
  ['59 min ago', '59 min ago', '59 min ago', '1 h ago', '23 h ago', '1 d ago']);
check('while humanAge keeps its compact form', [45, 120, 3 * 86400].map(humanAge), ['45s', '2min', '3d']);
check('nothing to list', fresh.groups.length, 0);
check('nothing to parse: no button', fresh.action, null);
check('the action row says so, with a check', [fresh.done, fresh.doneOk], ['Nothing to do: the graph is up to date.', true]);
check('no git paused note', fresh.paused, '');

print('\n--- behind the default branch: the advice replaces the sentence, as a line and no button ---');
var advised = freshnessBadge(payload({ drift: { base: 'origin/main', behind: 3, fetched_at: now - 300 }, advice: 'Pull origin/main, then parse again.' }));
check('no button', advised.action, null);
check('the line is the server\'s advice, without the check', [advised.done, advised.doneOk], ['Pull origin/main, then parse again.', false]);

print('\n--- saved but not committed: the medium state ---');
var edited = freshnessBadge(payload({
  state: 'edited', edited: ['models/marts/orders.sql', 'models/marts/schema.yml'], edited_n: 2,
  advice: 'Run dbt parse to see them.',
}));
check('amber', edited.tone, 'warn');
check('the card says whose changes they are', edited.head, 'Your saved edits are not in it yet');
ok('and what the graph is showing instead', said(edited).indexOf('as it was before them') >= 0);
ok('counts them', said(edited).indexOf('2 files saved since, not committed') >= 0);
ok('names one', said(edited).indexOf('models/marts/orders.sql') >= 0);
check('the group is keyed, for the git letters', edited.groups[0].key, 'edited');
check('the file name leads, the folder trails', edited.groups[0].paths[0].name, 'orders.sql');
check('folder', edited.groups[0].paths[0].dir, 'marts');
check('the button runs dbt parse in the terminal', edited.action, { label: 'Run dbt parse', hint: 'in the terminal below' });
check('and nothing is said as done', [edited.done, edited.doneOk], ['', false]);
check('where the button only types, the hint says Enter is left to you',
  freshnessBadge(payload({ state: 'edited', edited: ['a.sql'], edited_n: 1 }), true).action.hint,
  'typed in the terminal below; Enter is left to you');

print('\n--- one file only, so the count reads as one ---');
var one = freshnessBadge(payload({ state: 'edited', edited: ['models/a.sql'], edited_n: 1 }));
ok('singular', said(one).indexOf('1 file saved since') >= 0);

print('\n--- the branch moved under the manifest ---');
var stale = freshnessBadge(payload({
  state: 'stale', committed: ['models/stg/stg_customers.sql'], committed_n: 1,
  advice: 'Run dbt parse before trusting this lineage.',
}));
check('red', stale.tone, 'bad');
check('the card says what happened', stale.head, 'The files moved under this manifest');
check('explains how they got there, under that group', [stale.groups[0].key, stale.groups[0].note], ['committed', 'A checkout, a pull or a merge brought them in.']);
ok('and what that costs', said(stale).indexOf('no longer look like this') >= 0);
check('a stale manifest gets the button too', stale.action.label, 'Run dbt parse');
check('and the other groups carry no note', freshnessBadge(payload({ state: 'edited', edited: ['a.sql'], edited_n: 1 })).groups[0].note, undefined);

print('\n--- more changed than the card lists ---');
var many = freshnessBadge(payload({
  state: 'stale',
  committed: ['a.sql', 'b.sql', 'c.sql', 'd.sql', 'e.sql', 'f.sql'], committed_n: 31,
}));
ok('the rest are counted, not listed', said(many).indexOf('and 25 more') >= 0);

print('\n--- a model whose file was deleted ---');
var gone = freshnessBadge(payload({ state: 'stale', gone: ['models/marts/old_orders.sql'], gone_n: 1 }));
check('red', gone.tone, 'bad');
ok('names the vanished file', said(gone).indexOf('1 file the manifest names no longer exist') >= 0);
check('its group is keyed, so the card draws it struck through', gone.groups[0].key, 'gone');

print('\n--- a whole folder of models deleted ---');
var goneMany = freshnessBadge(payload({
  state: 'stale', gone: ['a.sql', 'b.sql', 'c.sql', 'd.sql', 'e.sql', 'f.sql'], gone_n: 9,
}));
ok('the rest are counted, not listed', said(goneMany).indexOf('and 3 more') >= 0);

print('\n--- behind the default branch, but the manifest is still true ---');
var behind = freshnessBadge(payload({
  drift: { base: 'origin/main', behind: 3, fetched_at: now - 300 },
  advice: 'Pull origin/main, then parse again.',
}));
check('drift never reddens a manifest that matches its files', behind.tone, 'ok');
check('the segment is the short form', behind.drift, 'main +3');
check('the card spells the short form out, the ref as code',
  behind.branch[1].parts, [{ code: 'origin/main' }, ' has 3 commits this branch does not have: the lineage is your branch, not production.']);
ok('says how current the count is', said(behind).indexOf('Remote refs last fetched 5min ago') >= 0);
check('and dims that line', behind.branch[2].dim, true);
check('the line with an age is flagged, for the card to rewrite in place', behind.branch[2].age, true);
check('names the commit you are on, the sha as code and the subject in curly quotes',
  behind.branch[0].parts, ['Sits at ', { code: 'a1b2c3d' }, ', \u201cAdd the revenue mart\u201d.']);
check('a commit without a subject ends the sentence', freshnessBadge(payload({ head: { sha: 'a1b2c3d' } })).branch[0].parts, ['Sits at ', { code: 'a1b2c3d' }, '.']);

print('\n--- one commit ahead reads as one ---');
var oneAhead = freshnessBadge(payload({ drift: { base: 'origin/main', behind: 1, fetched_at: now - 60 } }));
ok('singular', said(oneAhead).indexOf('has 1 commit this branch') >= 0);
check('segment', oneAhead.drift, 'main +1');

print('\n--- level with the default branch ---');
var level = freshnessBadge(payload({}));
check('said plainly rather than left out', level.branch[1].parts, ['This branch has everything ', { code: 'origin/main' }, ' has.']);

print('\n--- a clone that has never fetched ---');
var never = freshnessBadge(payload({ drift: { base: 'origin/main', behind: 0, fetched_at: 0 } }));
ok('says the comparison is worth little', said(never).indexOf('never been fetched') >= 0);
check('dim as well', never.branch[2].dim, true);
check('and carries no age to rewrite', never.branch[2].age, false);

print('\n--- no repository at all ---');
var norepo = freshnessBadge(payload({ head: {}, drift: {} }));
check('still green', norepo.tone, 'ok');
check('no drift segment', norepo.drift, '');
check('no This branch lines at all', norepo.branch, []);

print('\n--- no manifest ---');
var missing = freshnessBadge(payload({ state: 'missing', manifest_at: 0, age_secs: 0 }));
check('grey', missing.tone, 'none');
check('the label says what is wrong, with no age', [missing.label, missing.age], ['no manifest', '']);
check('so does the card', missing.head, 'No manifest.json yet');
check('no age is claimed', missing.when, '');
check('and the button writes one', missing.action.label, 'Run dbt parse');

print('\n--- the empty canvas, with a file open that has no node ---');
var path = 'models/marts/finance/orders_by_week.sql';
check('a fresh manifest: only the fact', noNodeHint(path, payload({})),
  'No node in the manifest comes from orders_by_week.sql.');
check('no answer from the badge yet: the same', noNodeHint(path, null),
  'No node in the manifest comes from orders_by_week.sql.');
ok('a stale one names the manifest', noNodeHint(path, payload({ state: 'stale' })).indexOf('older than the project\'s files') >= 0);
ok('and offers dbt parse as a maybe, since a macro has no node either',
  noNodeHint(path, payload({ state: 'edited' })).indexOf('if this one is new, run dbt parse') >= 0);
ok('no manifest at all', noNodeHint(path, payload({ state: 'missing' })).indexOf('no manifest.json yet') >= 0);
check('a seed is a node too', noNodeHint('seeds/fx_rates.csv', payload({})),
  'No node in the manifest comes from fx_rates.csv.');
check('a file that is never a node says nothing, so the plain hint stays', noNodeHint('README.md', payload({ state: 'stale' })), '');
check('nor does a properties file claim one', noNodeHint('models/marts/schema.yml', payload({ state: 'stale' })), '');
check('nor a .sql file that defines macros', noNodeHint('macros/cents_to_dollars.sql', payload({ state: 'stale' }), { macros: true }), '');

print('\n--- a repository whose own configuration names a program ---');
var paused = said(freshnessBadge(payload({ git_paused: 'filter.lfs.clean' })));
ok('the card says git is paused and names the key', paused.indexOf('git is paused here') >= 0 && paused.indexOf('(filter.lfs.clean)') >= 0);
ok('and that git in the terminal is unaffected', paused.indexOf('git in the terminal is unaffected') >= 0);
ok('a repository that is not paused says nothing of it', said(freshnessBadge(payload({}))).indexOf('paused') < 0);
check('the note is its own field, for the card to draw apart', freshnessBadge(payload({ git_paused: 'filter.lfs.clean' })).paused, gitPausedNote('filter.lfs.clean'));
check('the panel says the same words', gitPausedNote('filter.lfs.clean').indexOf('git is paused here: this repository\'s own configuration names a program (filter.lfs.clean)'), 0);

ok('a configuration that could not be listed says so, and gives git\'s reason', gitPausedNote('unreadable: invalid gitfile format').indexOf('could not be read (invalid gitfile format)') >= 0);

print('\n--- the same file, over a canvas still showing another model ---');
check('names the model on screen, which is easily taken for this one',
  noNodeHint(path, payload({}), { shown: 'orders_by_month' }),
  'No node in the manifest comes from orders_by_week.sql, so the lineage still shows orders_by_month.');
ok('and the stale manifest after it',
  noNodeHint(path, payload({ state: 'stale' }), { shown: 'orders_by_month' }).indexOf('still shows orders_by_month. The manifest is older') >= 0);
