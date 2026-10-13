// What the status bar says about Python, before and after the terminal's shell
// reports the venv it has at a prompt (0069).
// Run from the repository root: jsc web/tests/venv.js
var src = read('web/app.js');
eval(src.slice(src.indexOf('function venvKey'), src.indexOf('boot().catch')));

function check(label, got, want) {
  var g = JSON.stringify(got), w = JSON.stringify(want);
  print((g === w ? 'PASS  ' : 'FAIL  ') + label + (g === w ? '' : '\n        expected ' + w + '\n        got      ' + g));
}

print('--- one spelling per folder ---');
check('Git Bash and Windows name one folder', venvKey('/c/Users/me/proj/env', true), venvKey('C:\\Users\\me\\proj\\env', true));
check('so do forward slashes and another case', venvKey('C:/USERS/me/proj/env/', true), venvKey('c:\\Users\\Me\\proj\\env', true));
check('Cygwin\'s spelling too', venvKey('/cygdrive/d/work/env', true), venvKey('D:\\work\\env', true));
check('a verbatim path too', venvKey('\\\\?\\C:\\Users\\me\\env', true), venvKey('/c/Users/me/env', true));
check('another folder stays another', venvKey('/c/Users/me/proj/env2', true) === venvKey('C:\\Users\\me\\proj\\env', true), false);
check('elsewhere a path keeps its case', venvKey('/Users/Me/proj/env/', false), '/Users/Me/proj/env');
check('and a one-letter folder is no drive', venvKey('/w/Proj/env', false) === venvKey('/w/proj/env', false), false);

// The project's two venvs, neither active when dbt-edith started: the Windows
// shape of the report that asked for this.
var project = {
  source: 'project', name: 'core-env', path: 'C:\\p\\core-env', python: '3.12.4', dbt: 'dbt-core 1.10.0, dbt-snowflake 1.10.0',
  others: ['fusion-env'],
  found: [
    { name: 'core-env', path: 'C:\\p\\core-env', python: '3.12.4', dbt: 'dbt-core 1.10.0, dbt-snowflake 1.10.0' },
    { name: 'fusion-env', path: 'C:\\p\\fusion-env', python: '3.12.4', dbt: 'dbt-fusion 2.0.0rc196' },
  ],
};

print('\n--- until the terminal says anything ---');
var before = venvStatus(project, undefined);
check('the best one found, inactive', before.text, 'venv (inactive) core-env · py 3.12.4');
check('coloured as found but not active', before.state, 'off');
check('and how to activate it', before.title.split('\n'), [
  'C:\\p\\core-env', 'dbt: dbt-core 1.10.0, dbt-snowflake 1.10.0',
  'found in the project, not activated when dbt-edith started; source its activate script in the terminal',
  'also found: fusion-env',
]);

print('\n--- the terminal activates one ---');
var fusion = venvStatus(project, '/c/p/fusion-env');
check('the one the shell has, with what was read of it', fusion.text, 'venv fusion-env · py 3.12.4');
check('coloured as active', fusion.state, 'on');
check('and where it came from', fusion.title.split('\n'), [
  'C:\\p\\fusion-env', 'dbt: dbt-fusion 2.0.0rc196', 'activated in the terminal', 'also found: core-env',
]);
check('the first one, under any spelling', venvStatus(project, 'C:/P/CORE-ENV').text, 'venv core-env · py 3.12.4');

print('\n--- and deactivates it ---');
var off = venvStatus(project, '');
check('back to the best one found, inactive', off.text, 'venv (inactive) core-env · py 3.12.4');
check('and coloured so again', off.state, 'off');
check('said of the terminal now', off.title.split('\n')[2], 'not activated in the terminal; source its activate script there');

print('\n--- started in a venv ---');
var started = {
  source: 'activated', name: 'mine', path: '/home/me/.venvs/mine', python: '3.13.0', dbt: '',
  others: ['env'], found: [{ name: 'env', path: '/home/me/proj/env', python: '3.11.9', dbt: 'dbt-core 1.8.7' }],
};
check('active until the terminal says otherwise', venvStatus(started, undefined).text, 'venv mine · py 3.13.0');
check('and coloured so', venvStatus(started, undefined).state, 'on');
check('its own terminal reports it', venvStatus(started, '/home/me/.venvs/mine').text, 'venv mine · py 3.13.0');
check('a deactivate in the terminal shows', venvStatus(started, '').text, 'venv (inactive) mine · py 3.13.0');
check('in its colour too', venvStatus(started, '').state, 'off');
check('the project\'s, once activated there', venvStatus(started, '/home/me/proj/env').text, 'venv env · py 3.11.9');

print('\n--- a venv outside the project ---');
var away = venvStatus(project, '/c/Users/me/.venvs/scratch');
check('named, and nothing more', away.text, 'venv scratch');
check('but active all the same', away.state, 'on');
check('nothing in it read', away.title.split('\n'), [
  '/c/Users/me/.venvs/scratch', 'activated in the terminal; not a venv dbt-edith read, so its versions are not shown',
  'also found: core-env, fusion-env',
]);

print('\n--- the project reached through a mapped drive ---');
// The server canonicalises the root, so it holds the share's path; Git Bash
// says the drive's, which is also what the activate script was written with.
var mapped = {
  source: 'project', name: 'env', path: '\\\\server\\home\\proj\\env', python: '3.12.4', activates: 'H:\\proj\\env',
  found: [
    { name: 'env', path: '\\\\server\\home\\proj\\env', python: '3.12.4', activates: 'H:\\proj\\env' },
    { name: 'other', path: '\\\\server\\home\\proj\\other', python: '3.11.9', activates: '/h/proj/other' },
  ],
};
check('the drive\'s spelling finds it', venvStatus(mapped, '/h/proj/env').text, 'venv env · py 3.12.4');
check('and it is not also listed as another', venvStatus(mapped, '/h/proj/env').title.split('\n').pop(), 'also found: other');
check('as the script wrote it in bash\'s own spelling', venvStatus(mapped, '/h/proj/other').text, 'venv other · py 3.11.9');
check('whatever the case and the trailing slash', venvMatch(mapped, '/H/PROJ/ENV/').path, '\\\\server\\home\\proj\\env');
check('no match is null', venvMatch(mapped, '/h/elsewhere/env'), null);

print('\n--- a Unix server ---');
var unix = { source: 'project', name: 'env', path: '/w/Proj/env', python: '3.12.4', found: [{ name: 'env', path: '/w/Proj/env', python: '3.12.4' }] };
check('the same folder in its own case', venvStatus(unix, '/w/Proj/env').text, 'venv env · py 3.12.4');
check('another case is another folder', venvStatus(unix, '/w/proj/env').text, 'venv env');

print('\n--- a click on an inactive venv types its line (0072) ---');
var lines = {
  source: 'project', name: 'core-env', path: 'C:\\p\\core-env', activation: "source 'C:/p/core-env/Scripts/activate'",
  found: [
    { name: 'bare', path: 'C:\\p\\bare' },
    { name: 'core-env', path: 'C:\\p\\core-env', activation: "source 'C:/p/core-env/Scripts/activate'" },
    { name: 'fusion-env', path: 'C:\\p\\fusion-env', activation: "source 'C:/p/fusion-env/Scripts/activate'" },
  ],
};
function names(st) { return st.offer.map(function (e) { return e.name; }); }
check('the one named first, then the others with a line', names(venvStatus(lines, undefined)), ['core-env', 'fusion-env']);
check('listed, since there are two', venvStatus(lines, undefined).line, '');
check('after a deactivate too', venvStatus(lines, '').offer.length, 2);
check('nothing while one is active', venvStatus(lines, '/c/p/fusion-env').offer, []);
check('nor while a venv nobody read is', venvStatus(lines, '/c/Users/me/.venvs/scratch').offer, []);
check('nothing without a line for the shell', venvStatus(project, undefined).offer, []);

var single = {
  source: 'project', name: 'env', path: '/w/proj/env', activation: "source '/w/proj/env/bin/activate'",
  found: [{ name: 'env', path: '/w/proj/env', activation: "source '/w/proj/env/bin/activate'" }],
};
check('the only one is typed at once', venvStatus(single, '').line, "source '/w/proj/env/bin/activate'");

// The one the bar names has no script for the shell, another has: a click
// must not type that other one's line as if it were the one named.
var scriptless = {
  source: 'project', name: 'core-env', path: '/w/proj/core-env',
  found: [
    { name: 'core-env', path: '/w/proj/core-env' },
    { name: 'fusion-env', path: '/w/proj/fusion-env', activation: "source '/w/proj/fusion-env/bin/activate'" },
  ],
};
check('another venv is never typed at once', venvStatus(scriptless, undefined).line, '');
check('it is listed, to be picked', names(venvStatus(scriptless, undefined)), ['fusion-env']);

var mine = {
  source: 'activated', name: 'mine', path: '/home/me/.venvs/mine', activation: "source '/home/me/.venvs/mine/bin/activate'",
  found: [{ name: 'env', path: '/home/me/proj/env', activation: "source '/home/me/proj/env/bin/activate'" }],
};
check('nothing while the one it started in is still active', venvStatus(mine, undefined).offer, []);
check('then that one first once deactivated', venvStatus(mine, '').offer.map(function (e) { return e.activation; }),
  ["source '/home/me/.venvs/mine/bin/activate'", "source '/home/me/proj/env/bin/activate'"]);

// Started in the project's own venv through a mapped drive: the server read it
// twice, once as VIRTUAL_ENV spells it and once as the project's folder.
var twice = {
  source: 'activated', name: 'env', path: 'H:\\proj\\env', activates: 'H:\\proj\\env', activation: "source 'H:/proj/env/Scripts/activate'",
  found: [{ name: 'env', path: '\\\\server\\home\\proj\\env', activates: 'H:\\proj\\env', activation: "source '//server/home/proj/env/Scripts/activate'" }],
};
check('the same venv is offered once', venvStatus(twice, '').offer.length, 1);
check('and typed at once, as the bar names it', venvStatus(twice, '').line, "source 'H:/proj/env/Scripts/activate'");
var cased = {
  source: 'activated', name: 'env', path: 'c:\\p\\env', activation: "source 'c:/p/env/Scripts/activate'",
  found: [{ name: 'env', path: 'C:\\P\\env', activation: "source 'C:/P/env/Scripts/activate'" }],
};
check('in another case on Windows too', venvStatus(cased, '').offer.length, 1);

print('\n--- no venv anywhere ---');
var none = { committed: ['shipped'] };
check('none at startup', venvStatus(none, undefined).text, 'no venv');
check('left uncoloured', venvStatus(none, undefined).state, 'none');
check('and offers nothing to type', venvStatus(none, '').offer, []);
check('none in the terminal either', venvStatus(none, '').title.split('\n'), [
  'none activated in the terminal, and none found in the project',
  'committed to the repository, so never run by dbt-edith: shipped',
]);
check('a committed venv the terminal activates is only named', venvStatus(none, '/c/p/shipped').text, 'venv shipped');
