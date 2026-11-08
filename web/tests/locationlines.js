// The location in the space's card, as a list rather than a table of three
// columns (0160): one value where the stages agree, a line per run of equal
// stages where they do not, labelled by the last stage of the run. Fixtures are
// invented, in the shape of a Jaffle Shop manifest.
// Run from the repository root: jsc web/tests/locationlines.js
var src = read('web/app.js');
eval(src.slice(src.indexOf('function locationRows'), src.indexOf('function catalogLocation')));

function check(label, got, want) {
  var g = JSON.stringify(got), w = JSON.stringify(want);
  print((g === w ? 'PASS  ' : 'FAIL  ') + label + (g === w ? '' : '\n        expected ' + w + '\n        got      ' + g));
}
function ok(label, cond) { print((cond ? 'PASS  ' : 'FAIL  ') + label); }
function lines(loc, key, file, inSandbox) {
  var rows = locationRows(loc, file), r = rows.filter(function (x) { return x.key === key; })[0];
  return locationLines(r, file ? 'qa' : '', file || '', !!inSandbox);
}
function brief(ls) { return ls.map(function (l) { return [l.label, l.value, l.dim, l.cls]; }); }

print('--- one value where every stage agrees ---');
var same = { written: { database: '', schema: 'marts', alias: '' }, parsed: { database: '', schema: 'marts', alias: '' }, built: { database: '', schema: 'marts', alias: '' } };
var one = lines(same, 'schema');
check('a schema written, resolved and built the same is one line, with no label', brief(one), [['', 'marts', '', '']]);
check('whose title names the three stages', one[0].title.indexOf('written, resolved and built: '), 0);

print('\n--- where the stages differ ---');
var tpl = {
  written: { database: "{{ env_var('DB') }}", schema: 'marts', alias: '' },
  parsed: { database: 'jaffle_shop', schema: 'marts', alias: '' },
  built: { database: 'jaffle_shop', schema: 'marts', alias: 'orders' },
};
var db = lines(tpl, 'database');
check('a templated database: what was written, then what it came to, resolved and built as one run labelled by the last',
  brief(db), [['written', "{{ env_var('DB') }}", '', 'tpl'], ['built', 'jaffle_shop', '', '']]);
check('each title names the stages it stands for', [db[0].title.indexOf('the config as it appears'), db[1].title.indexOf('resolved and built: ')], [0, 0]);
var moved = { written: { database: 'a', schema: '', alias: '' }, parsed: { database: 'a', schema: '', alias: '' }, built: { database: 'b', schema: '', alias: '' } };
check('a model built elsewhere: what was written and resolved as one run, the built line marked moved and saying where from',
  [brief(lines(moved, 'database')), lines(moved, 'database')[1].title],
  [[['resolved', 'a', '', ''], ['built', 'b', '', 'moved']], 'dbt parsed a, built in b']);
check('in a sandbox that move is the expected one', lines(moved, 'database', '', true)[1].cls, 'moved expected');

print('\n--- nothing set, the target decides ---');
var dflt = lines(tpl, 'alias');
check('an alias set nowhere is the name built, with (default) after it, dimmed', brief(dflt), [['', 'orders', ' (default)', '']]);
check('and a key with nothing at any stage says default', brief(locationLines({ key: 'x', written: '', parsed: '', resolved: '', built: '', status: '', vars: [], branch: false, templated: false, redirected: false }, '', '', false)),
  [['', 'default', '', 'nul']]);
var project = { written: { database: '', schema: '', alias: '' }, parsed: { database: '', schema: 'staging', alias: '' }, built: { database: '', schema: 'staging', alias: '' } };
check('a schema from dbt_project.yml, not written in the file, is one value without a (default)', brief(lines(project, 'schema')), [['', 'staging', '', '']]);

print('\n--- an environment file ---');
var envd = {
  written: { database: "{{ env_var('DBT_DB') }}", schema: '', alias: '' },
  parsed: { database: 'dev_db', schema: '', alias: '' },
  built: { database: 'dev_db', schema: '', alias: '' },
  envs: { '.env.qa': { place: { database: 'QA_DB' }, status: { database: { kind: 'env', vars: ['DBT_DB'] } } } },
};
check('resolved with the file, three different values are three lines', brief(lines(envd, 'database', '.env.qa')),
  [['written', "{{ env_var('DBT_DB') }}", '', 'tpl'], ['resolved', 'QA_DB', '', ''], ['built', 'dev_db', '', '']]);
var unevaluated = {
  written: { database: "{{ var('x') }}", schema: '', alias: '' }, parsed: { database: 'jaffle_shop', schema: '', alias: '' }, built: { database: 'jaffle_shop', schema: '', alias: '' },
  envs: { '.env.qa': { place: { database: 'jaffle_shop' }, status: { database: { kind: 'unevaluated', vars: [] } } } },
};
check('an unevaluated mark is never merged away: the same text with another class stays its own line',
  brief(lines(unevaluated, 'database', '.env.qa')).map(function (l) { return [l[0], l[1], l[3]]; }),
  [['written', "{{ var('x') }}", 'tpl'], ['resolved', 'jaffle_shop', 'env-unevaluated'], ['built', 'jaffle_shop', '']]);
var missing = {
  written: { database: "{{ env_var('NOPE') }}", schema: '', alias: '' }, parsed: { database: '', schema: '', alias: '' }, built: { database: 'dev_db', schema: '', alias: '' },
  envs: { '.env.qa': { place: { database: '' }, status: { database: { kind: 'missing', vars: ['NOPE'] } } } },
};
check('a variable the file lacks is said as it is, in its own class', brief(lines(missing, 'database', '.env.qa')).map(function (l) { return [l[0], l[1], l[3]]; }),
  [['written', "{{ env_var('NOPE') }}", 'tpl'], ['resolved', 'missing NOPE', 'env-missing'], ['built', 'dev_db', '']]);

print('\n--- the words ---');
ok('no em or en dash in what it writes for the user', !/[–—]/.test(src.slice(src.indexOf('function locationLines'), src.indexOf('function splitRelation'))));
