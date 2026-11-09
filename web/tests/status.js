// The shell's two bars (0166): what the status bar says on its left for each space
// in front, its counts, the selection it names, and the order of the controls in
// the top bar and the status bar.
// Run from the repository root: jsc web/tests/status.js
var src = read('web/app.js');
var html = read('web/index.html');

function check(label, got, want) {
  var g = JSON.stringify(got), w = JSON.stringify(want);
  print((g === w ? 'PASS  ' : 'FAIL  ') + label + (g === w ? '' : '\n        expected ' + w + '\n        got      ' + g));
}

eval(src.slice(src.indexOf('function statusWhere('), src.indexOf('/* The name of the section in front')));
eval(src.slice(src.indexOf('function boxSelector('), src.indexOf("/* The menu's filter.")));

print('--- Code: the file, its cursor and its state ---');
var at = { line: 11, ch: 3 };
check('no file open', statusWhere({ space: 'code', file: null }), { lead: '', here: 'no file', mono: false, notes: [] });
check('a file: its path in mono, then the cursor',
  statusWhere({ space: 'code', file: { kind: 'file', path: 'models/customers.sql' }, at: at }),
  { lead: '', here: 'models/customers.sql', mono: true, notes: ['12:4'] });
check('modified and truncated follow the cursor',
  statusWhere({ space: 'code', file: { kind: 'file', path: 'a.sql', dirty: true, truncated: true }, at: { line: 0, ch: 0 } }).notes,
  ['1:1', 'modified', 'truncated']);
check('a diff is read only and has no cursor',
  statusWhere({ space: 'code', file: { kind: 'diff', path: 'models/a.sql' }, at: at }),
  { lead: '', here: 'models/a.sql', mono: true, notes: ['diff against HEAD', 'read only'] });
check('a pair of collin reports',
  statusWhere({ space: 'code', file: { kind: 'compare', pair: { a: 'x.json', b: 'y.json' } }, at: at }),
  { lead: '', here: 'target/collin/x.json against target/collin/y.json', mono: true, notes: ['read only'] });
check('the profile: outside the project, its secrets counted',
  statusWhere({ space: 'code', file: { kind: 'profile', path: '~/.dbt/profiles.yml', dirty: true, hidden: 1 }, at: at }).notes,
  ['12:4', 'modified', 'outside the project', '1 secret kept on the server']);
check('and in the plural',
  statusWhere({ space: 'code', file: { kind: 'profile', path: 'p.yml', hidden: 3 }, at: at }).notes.slice(-1),
  ['3 secrets kept on the server']);
check('a profile with nothing hidden says no secrets',
  statusWhere({ space: 'code', file: { kind: 'profile', path: 'p.yml' }, at: at }).notes, ['12:4', 'outside the project']);

print('\n--- the other spaces: the space, then where in it ---');
check('Lineage with a selection: the line in mono',
  statusWhere({ space: 'lineage', file: { kind: 'file', path: 'a.sql' }, selection: '1+customers+1' }),
  { lead: 'Lineage', here: '1+customers+1', mono: true, notes: [] });
check('Lineage with an empty box: the space alone', statusWhere({ space: 'lineage', selection: '' }).here, '');
check('Reports: the section in front',
  statusWhere({ space: 'reports', section: 'Job coverage' }), { lead: 'Reports', here: 'Job coverage', mono: false, notes: [] });
check('Settings: the section in front',
  statusWhere({ space: 'settings', section: 'Column lineage' }), { lead: 'Settings', here: 'Column lineage', mono: false, notes: [] });
check('a file open under another space is not named', statusWhere({ space: 'reports', file: { kind: 'file', path: 'a.sql' }, at: at }).here, '');

print('\n--- the counts ---');
check('kinds in order, plurals, joined', statusCounts({ test: 23, model: 5, seed: 3 }), '5 models · 3 seeds · 23 tests');
check('one is singular, none is left out', statusCounts({ model: 1, source: 0, snapshot: 2 }), '1 model · 2 snapshots');
check('no counts, no words', [statusCounts({}), statusCounts(undefined)], ['', '']);

print('\n--- the selection the lineage names ---');
check('a line is shown as typed', selectionHere('  stg_orders+  '), 'stg_orders+');
check('a --selector is shown as its name', selectionHere('--selector nightly'), 'nightly');
check('a --selector among other words is shown as typed', selectionHere('+orders --selector nightly'), '+orders --selector nightly');
check('nothing is nothing', selectionHere(''), '');

print('\n--- the bars as the board orders them ---');
function order(from, to, ids) {
  var block = html.slice(html.indexOf(from), html.indexOf(to));
  var at = ids.map(function (id) { return block.indexOf('id="' + id + '"'); });
  return at.every(function (x, i) { return x >= 0 && (i === 0 || x > at[i - 1]); });
}
check('top bar: brand, project, spaces, search, manifest pill, ?, theme, GitHub, settings',
  order('<header id="topbar">', '</header>', ['brand', 'project', 'spaces', 'palette-btn', 'manifest-pill', 'keys-btn', 'theme-btn', 'github-btn', 'settings-btn']), true);
check('the search sits between two grows', /<div class="grow"><\/div>\s*(<!--[\s\S]*?-->\s*)?<button id="palette-btn"[\s\S]*?<\/button>\s*<div class="grow"><\/div>/.test(html), true);
check('the counts are gone from the top bar', html.slice(html.indexOf('<header id="topbar">'), html.indexOf('</header>')).indexOf('id="counts"'), -1);
check('status bar: where, counts, venv, env, branch, shell, manifest, build, update',
  order('<footer id="statusbar">', '</footer>', ['status-where', 'status-counts', 'status-venv', 'status-env', 'status-branch', 'status-shell', 'status-manifest', 'status-build', 'status-update']), true);
check('the right group are four squares; the settings one names itself by label, not by an unfolding word',
  [/id="settings-btn" class="keysbtn"/.test(html), /aria-label="Settings"/.test(html), html.indexOf('rlabel')], [true, true, -1]);

print('\n--- the top bar folds search first, then the name, then the tabs ---');
var fold = src.slice(src.indexOf('function foldTopbar()'), src.indexOf('function wireTopbar()'));
check('nosearch is tried before folded', fold.indexOf("add('nosearch')") > 0 && fold.indexOf("add('folded')") > fold.indexOf("add('nosearch')"), true);
check('no step of the old bar is left', /nolabel|nocounts|rlabel/.test(fold), false);

print('\n--- the editor\'s chrome (0166) ---');
var css = read('web/app.css');
check('Open editors starts collapsed behind a button that says so',
  [/<div id="open-editors" class="collapsed">/.test(html), /<button class="oe-toggle" id="oe-toggle" type="button" aria-expanded="false" aria-controls="oe-list">/.test(html)], [true, true]);
check('its toggle flips aria-expanded with the list', /\$\('#oe-toggle'\)\.addEventListener\('click'[\s\S]*?setAttribute\('aria-expanded', String\(open\)\)/.test(src), true);
check('the tab bar draws its line as a shadow, and no tab is pulled over it',
  [/\.tabbar \{[^}]*box-shadow: inset 0 -1px 0 var\(--line\)/.test(css), /\.ftab[^{]*\{[^}]*margin-bottom/.test(css)], [true, false]);
check('a tab\'s × is hidden unless it is in front, dirty, under the pointer or focused',
  /\.ftab:not\(\.active\):not\(\.dirty\):not\(:hover\):not\(:focus-within\) \.x \{ visibility: hidden; \}/.test(css), true);
check('the gutter rules are the main editor\'s alone', /(^|\n)\.CodeMirror-gutters \{[^}]*border-right: 1px/.test(css)
  && /#editor-host \.CodeMirror-gutters \{ border-right: 0; \}/.test(css) && /#editor-host \.CodeMirror-linenumber \{ min-width: 46px; padding: 0 18px 0 0;/.test(css), true);

// The cursor's line: one lit at a time, with its document, repainted when a tab's document is swapped in.
var S = { cm: null };
eval(src.slice(src.indexOf('let curLine = null;'), src.indexOf('/* The folder a config key in dbt_project.yml')));
function fakeDoc(cursorLine) {
  var d = { classes: [], cursor: cursorLine, handles: [] };
  d.getCursor = function () { return { line: d.cursor, ch: 0 }; };
  d.getLineHandle = function (n) { return d.handles[n] || (d.handles[n] = { n: n }); };
  d.addLineClass = function (h, w, c) { d.classes.push(w + ':' + c + ':' + h.n); };
  d.removeLineClass = function (h, w, c) { d.classes = d.classes.filter(function (x) { return x !== w + ':' + c + ':' + h.n; }); };
  return d;
}
var docA = fakeDoc(2), docB = fakeDoc(0);
S.cm = { getDoc: function () { return S.cm.doc; }, doc: docA };
paintCurLine();
check('the cursor\'s line is lit, page and gutter', docA.classes, ['background:cm-curline:2', 'gutter:cm-curline-gutter:2']);
paintCurLine();
check('the same line again lights nothing twice', docA.classes.length, 2);
docA.cursor = 5; paintCurLine();
check('a move clears the old line and lights the new one', docA.classes, ['background:cm-curline:5', 'gutter:cm-curline-gutter:5']);
S.cm.doc = docB; paintCurLine();
check('a document swapped in has its line lit, and the one left behind is cleared', [docA.classes, docB.classes],
  [[], ['background:cm-curline:0', 'gutter:cm-curline-gutter:0']]);
