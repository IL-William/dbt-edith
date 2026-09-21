// The manifest freshness badge and its hover card: which state each payload
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
  var out = [b.head, b.when, b.desc, b.advice, b.hint].concat(b.facts);
  b.groups.forEach(function (g) {
    out.push(g.head);
    g.paths.forEach(function (p) { out.push(p.full); });
    if (g.more > 0) out.push('and ' + g.more + ' more');
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
check('the age is the label', fresh.label, 'manifest 12min');
check('no drift segment when the branch is level', fresh.drift, '');
check('the card leads with a sentence, not a number', fresh.head, 'The lineage matches your files');
ok('and says what that means', said(fresh).indexOf('what your project actually says') >= 0);
ok('names the moment it was written', fresh.when.indexOf('12min ago') >= 0);
check('nothing to list', fresh.groups.length, 0);
check('with nothing to advise, the hint names the command', fresh.hint, 'Click to run dbt parse in the terminal.');

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
check('the file name leads, the folder trails', edited.groups[0].paths[0].name, 'orders.sql');
check('folder', edited.groups[0].paths[0].dir, 'marts');
ok('the advice is the action, not the diagnosis again', said(edited).indexOf('Run dbt parse to see them.') >= 0);
check('and the hint follows it rather than repeating it', edited.hint, 'Click to run it in the terminal.');

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
ok('explains how they got there', said(stale).indexOf('A checkout, a pull or a merge brought them in') >= 0);
ok('and what that costs', said(stale).indexOf('no longer look like this') >= 0);

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
ok('the card spells the short form out', said(behind).indexOf('origin/main has 3 commits this branch does not have') >= 0);
ok('and says why that matters', said(behind).indexOf('your branch and not production') >= 0);
ok('the advice is to pull first', said(behind).indexOf('Pull origin/main, then parse again.') >= 0);
ok('says how current the count is', said(behind).indexOf('Remote refs last fetched 5min ago') >= 0);
ok('names the commit you are on', said(behind).indexOf('This branch sits at a1b2c3d, "Add the revenue mart"') >= 0);

print('\n--- one commit ahead reads as one ---');
var oneAhead = freshnessBadge(payload({ drift: { base: 'origin/main', behind: 1, fetched_at: now - 60 } }));
ok('singular', said(oneAhead).indexOf('has 1 commit this branch') >= 0);
check('segment', oneAhead.drift, 'main +1');

print('\n--- level with the default branch ---');
var level = freshnessBadge(payload({}));
ok('said plainly rather than left out', said(level).indexOf('has everything origin/main has') >= 0);

print('\n--- a clone that has never fetched ---');
var never = freshnessBadge(payload({ drift: { base: 'origin/main', behind: 0, fetched_at: 0 } }));
ok('says the comparison is worth little', said(never).indexOf('never been fetched') >= 0);

print('\n--- no repository at all ---');
var norepo = freshnessBadge(payload({ head: {}, drift: {} }));
check('still green', norepo.tone, 'ok');
check('no drift segment', norepo.drift, '');
ok('no fetch line', said(norepo).indexOf('Remote refs') < 0);
ok('no commit line', said(norepo).indexOf('This branch sits at') < 0);

print('\n--- no manifest ---');
var missing = freshnessBadge(payload({ state: 'missing', manifest_at: 0, age_secs: 0 }));
check('grey', missing.tone, 'none');
check('the label says what is wrong', missing.label, 'no manifest');
check('so does the card', missing.head, 'No manifest.json yet');
check('no age is claimed', missing.when, '');
