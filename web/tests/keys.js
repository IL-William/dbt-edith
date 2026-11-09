// Key names, how they read on each platform, and the shortcut list against the
// keys wireKeys really answers to.
// Run from the repository root: jsc web/tests/keys.js
var src = read('web/app.js');
eval(src.slice(src.indexOf('function keyCombo'), src.indexOf('function toggleShortcuts')));

function check(label, got, want) {
  var g = JSON.stringify(got), w = JSON.stringify(want);
  print((g === w ? 'PASS  ' : 'FAIL  ') + label + (g === w ? '' : '\n        expected ' + w + '\n        got      ' + g));
}

print('--- keyCombo ---');
check('Cmd on a Mac is Mod', keyCombo({ key: 's', code: 'KeyS', metaKey: true }, true), 'Mod+S');
check('Ctrl elsewhere is Mod', keyCombo({ key: 's', code: 'KeyS', ctrlKey: true }, false), 'Mod+S');
check('Option makes ∑ of W on a Mac, and it is still W', keyCombo({ key: '∑', code: 'KeyW', altKey: true }, true), 'Alt+W');
// Option with a letter is left as typed, whatever key it came from: on some
// layouts Option+B and Option+N write a letter, and binding them wrote it into
// the file instead of moving. Cmd is what recovers the physical key, which is
// why back and forward ask for it.
check('Option+B writing a letter is not Alt+B',
  keyCombo({ key: 'ß', code: 'KeyB', altKey: true }, true) === 'Alt+B', false);
check('Cmd+Option+P is P whatever Option typed',
  keyCombo({ key: 'ß', code: 'KeyP', metaKey: true, altKey: true }, true), 'Mod+Alt+P');
check('Cmd+Option+N is N whatever Option typed',
  keyCombo({ key: 'Dead', code: 'KeyN', metaKey: true, altKey: true }, true), 'Mod+Alt+N');
// A French Mac reaches neither Alt+W nor Alt+Shift+W, by both routes at once:
// the key marked W sits at KeyZ, and the key at KeyW is marked Z and writes a
// letter under Option, which the rule above leaves alone (0037).
check('the key marked W on a French keyboard is KeyZ, so it is not Alt+W',
  keyCombo({ key: '‹', code: 'KeyZ', altKey: true }, true), 'Alt+Z');
check('  nor is the key at KeyW, which writes a letter there',
  keyCombo({ key: 'Â', code: 'KeyW', altKey: true }, true) === 'Alt+W', false);
check('Cmd+Option+X is X, the close that every layout can type',
  keyCombo({ key: '≈', code: 'KeyX', metaKey: true, altKey: true }, true), 'Mod+Alt+X');
check('  and with Shift for closing every tab',
  keyCombo({ key: '˛', code: 'KeyX', metaKey: true, altKey: true, shiftKey: true }, true), 'Mod+Alt+Shift+X');
check('Cmd+Option+S is S whatever Option typed', keyCombo({ key: 'ß', code: 'KeyS', metaKey: true, altKey: true }, true), 'Mod+Alt+S');
check('Option alone typing a letter is someone writing',
  keyCombo({ key: 'Â', code: 'KeyW', altKey: true }, true) === 'Alt+W', false);
check('elsewhere the letter is the one on the key', keyCombo({ key: 'w', code: 'KeyZ', altKey: true }, false), 'Alt+W');
check('Shift is named for a letter', keyCombo({ key: 'G', code: 'KeyG', metaKey: true, shiftKey: true }, true), 'Mod+Shift+G');
check('  and for a named key', keyCombo({ key: 'Enter', code: 'Enter', shiftKey: true }, false), 'Shift+Enter');
check('  but not for a symbol, where it is in the character', keyCombo({ key: '?', code: 'Slash', shiftKey: true }, true), '?');
check('? typed with Shift on a French keyboard', keyCombo({ key: '?', code: 'KeyM', shiftKey: true }, true), '?');
check('a function key', keyCombo({ key: 'F1', code: 'F1' }, false), 'F1');
check('the backtick', keyCombo({ key: '`', code: 'Backquote', metaKey: true }, true), 'Mod+`');
check('the space bar has a name', keyCombo({ key: ' ', code: 'Space' }, false), 'Space');
check('a plain letter', keyCombo({ key: 'f', code: 'KeyF' }, false), 'F');

print('\n--- keyLabel ---');
check('Mod on a Mac', keyLabel('Mod+K', true), '⌘K');
check('Mod elsewhere', keyLabel('Mod+K', false), 'Ctrl+K');
check('Apple\'s order for the glyphs', keyLabel('Mod+Alt+S', true), '⌥⌘S');
check('  three modifiers in Apple\'s order', keyLabel('Mod+Alt+Shift+X', true), '⌥⇧⌘X');
check('  Shift before Cmd', keyLabel('Mod+Shift+G', true), '⇧⌘G');
check('the written order elsewhere', keyLabel('Mod+Alt+S', false), 'Ctrl+Alt+S');
check('Enter is Return on a Mac', keyLabel('Shift+Enter', true), '⇧↩');
check('  and Enter elsewhere', keyLabel('Shift+Enter', false), 'Shift+Enter');
check('arrows', [keyLabel('ArrowUp', false), keyLabel('ArrowDown', true)], ['↑', '↓']);
check('Escape', [keyLabel('Escape', true), keyLabel('Escape', false)], ['Esc', 'Esc']);
check('a mouse action after a glyph', keyLabel('Alt+click', true), '⌥ click');
check('a mouse action elsewhere', keyLabel('Alt+click', false), 'Alt+click');
check('a bare symbol', [keyLabel('?', true), keyLabel('?', false)], ['?', '?']);
check('the terminal\'s line keys on a Mac', [keyLabel('Mod+ArrowLeft', true), keyLabel('Mod+Backspace', true), keyLabel('Alt+Delete', true)], ['⌘←', '⌘⌫', '⌥⌦']);
check('  and on Windows', [keyLabel('Mod+ArrowRight', false), keyLabel('Mod+Backspace', false), keyLabel('Home', false)], ['Ctrl+→', 'Ctrl+Backspace', 'Home']);

print('\n--- the list and the keys ---');
var sheet = shortcutSheet();
var listed = {};
sheet.forEach(function (g) { g.keys.forEach(function (row) { row[0].forEach(function (k) { listed[k] = true; }); }); });
// The keys wireKeys switches on, read from its source: a key bound there and
// missing from the list is a shortcut nobody can find out about.
var body = src.slice(src.indexOf('function wireKeys'), src.indexOf('// ------------------------------------------------------------------ boot --'));
var bound = [], re = /case '([^']+)'/g, m;
while ((m = re.exec(body))) bound.push(m[1]);
check('wireKeys still switches on named keys', bound.length >= 8, true);
bound.forEach(function (k) { check('bound and listed: ' + k, !!listed[k], true); });
// A combo keyCombo can never produce is a row that promises a dead key.
var order = { Mod: 0, Alt: 1, Shift: 2 };
Object.keys(listed).forEach(function (k) {
  var mods = k.split('+').slice(0, -1);
  var ok = mods.every(function (x, i) { return x in order && (i === 0 || order[mods[i - 1]] < order[x]); });
  check('modifiers in keyCombo\'s order: ' + k, ok, true);
});
check('every row says what it does', sheet.every(function (g) {
  return g.title && g.keys.length && g.keys.every(function (row) { return row[0].length && row[1]; });
}), true);
// The terminal's line keys differ on each platform, and the list says which
// apply where it is read (0073).
function termRows(os) {
  var t = shortcutSheet(os).filter(function (g) { return g.title === 'Terminal'; })[0];
  return t.keys.map(function (row) { return row[0].join(' '); });
}
check('a Mac lists Cmd+Left for the start of the command', termRows('mac').indexOf('Mod+ArrowLeft Mod+ArrowRight') >= 0, true);
check('Windows lists Home and End, for Git Bash', termRows('windows').indexOf('Home End') >= 0, true);
check('Windows has no row deleting the command back to its start, which is the Mac\'s', termRows('windows').indexOf('Mod+Backspace') < 0 && termRows('mac').indexOf('Mod+Backspace') >= 0, true);
check('elsewhere the shell reads xterm.js\'s, and nothing is added', termRows('other').length, 2);
['mac', 'windows'].forEach(function (os) {
  shortcutSheet(os).forEach(function (g) { g.keys.forEach(function (row) { row[0].forEach(function (k) {
    var mods = k.split('+').slice(0, -1);
    check('modifiers in keyCombo\'s order on ' + os + ': ' + k,
      mods.every(function (x, i) { return x in order && (i === 0 || order[mods[i - 1]] < order[x]); }), true);
  }); }); });
});

print('\n--- the find box ---');
var macLabel = function (c) { return keyLabel(c, true); }, pcLabel = function (c) { return keyLabel(c, false); };
var all = shortcutSheet('mac');
var kept = function (rows) { return rows.map(function (g) { return g.title + ':' + g.rows.length; }); };
check('a blank query keeps every group and row, marked nowhere',
  JSON.stringify(kept(filterShortcuts(all, '  ', macLabel, true))) === JSON.stringify(all.map(function (g) { return g.title + ':' + g.keys.length; }))
  && filterShortcuts(all, '', macLabel, true).every(function (g) { return g.rows.every(function (r) { return r.m === ''; }); }), true);
var undo = filterShortcuts(all, 'undo', macLabel, true);
check('a text hit is one row, marked with the case it was written in',
  JSON.stringify(undo.map(function (g) { return [g.title, g.rows.length]; })), '[["Editor",1]]');
check('  split before, at and after the hit', [undo[0].rows[0].a, undo[0].rows[0].m, undo[0].rows[0].b], ['', 'Undo', ' and redo']);
check('  its keys are the Mac labels', undo[0].rows[0].keys, ['⌘Z', '⇧⌘Z']);
var f3 = filterShortcuts(all, 'f3', macLabel, true);
check('a key hit keeps the row, unmarked', f3.length === 1 && f3[0].rows.length === 1 && f3[0].rows[0].m === '' && f3[0].rows[0].a === 'The same, as in VS Code', true);
var cmdG = filterShortcuts(all, '⌘g', macLabel, true);
check('a Mac label is found as it is drawn', cmdG.length === 1 && cmdG[0].rows[0].keys.join(' ') === '⌘G ⇧⌘G', true);
check('Ctrl is found on a PC label and not on a Mac one',
  [filterShortcuts(all, 'ctrl+k', pcLabel, false).length > 0, filterShortcuts(all, 'ctrl+k', macLabel, true).length], [true, 0]);
var term = filterShortcuts(all, 'terminal', macLabel, true);
check('a group title keeps the whole group',
  term.some(function (g) { return g.title === 'Terminal' && g.rows.length === all.filter(function (x) { return x.title === 'Terminal'; })[0].keys.length; }), true);
check('  and marks a row only where its text holds the title too',
  term.filter(function (g) { return g.title === 'Terminal'; })[0].rows.every(function (r) { return r.m === ''; }), true);
var rowTexts = function (groups) { return [].concat.apply([], groups.map(function (g) { return g.rows.map(function (r) { return r.a + r.m + r.b; }); })); };
check('keyCombo\'s own Mod is not searched: "mod" on a Mac keeps only what the text says',
  rowTexts(filterShortcuts(all, 'mod', macLabel, true)).indexOf('Save the file') < 0, true);
check('  nor is its +', rowTexts(filterShortcuts(all, '+', macLabel, true)).indexOf('Save the file') < 0, true);
check('Cmd and Command find the Mac\'s Mod rows', [rowTexts(filterShortcuts(all, 'cmd', macLabel, true)).indexOf('Save the file') >= 0,
  rowTexts(filterShortcuts(all, 'command', macLabel, true)).indexOf('Save the file') >= 0], [true, true]);
check('Option finds the Mac\'s Alt rows', rowTexts(filterShortcuts(all, 'option', macLabel, true)).indexOf('Close the tab') >= 0, true);
check('Control is not Cmd', rowTexts(filterShortcuts(all, 'control', macLabel, true)).indexOf('Save the file') < 0, true);
check('on a PC Cmd and Option find nothing, and Alt finds Alt+W',
  [filterShortcuts(all, 'cmd', pcLabel, false).length, filterShortcuts(all, 'option', pcLabel, false).length,
    rowTexts(filterShortcuts(all, 'alt', pcLabel, false)).indexOf('Close the tab') >= 0], [0, 0, true]);
var findG = filterShortcuts(all, 'find', macLabel, true).filter(function (g) { return g.title === 'Find'; })[0];
check('a title hit keeps the Find group whole', findG && findG.rows.length, 7);
check('  marking "find" where the text says it',
  findG.rows.filter(function (r) { return r.m; }).map(function (r) { return r.a + '[' + r.m + ']' + r.b; }).pop(),
  'Close the [find] bar, or empty the column filter');
check('  and not "In the Catalog: filter..."', findG.rows.filter(function (r) { return r.a.indexOf('In the Catalog') === 0; })[0].m, '');
check('Enter and Return find the Enter rows on a Mac', [rowTexts(filterShortcuts(all, 'enter', macLabel, true)).indexOf('Draw it now') >= 0,
  rowTexts(filterShortcuts(all, 'return', macLabel, true)).indexOf('Draw it now') >= 0], [true, true]);
check('  and Return is a Mac word only', rowTexts(filterShortcuts(all, 'return', pcLabel, false)).indexOf('Draw it now') < 0, true);
check('Arrow finds the arrow rows on a Mac and on a PC', [rowTexts(filterShortcuts(all, 'arrow', macLabel, true)).indexOf('Earlier expressions') >= 0,
  rowTexts(filterShortcuts(all, 'arrow', pcLabel, false)).indexOf('Earlier expressions') >= 0], [true, true]);
check('Backspace finds the row that deletes the command', rowTexts(filterShortcuts(all, 'backspace', macLabel, true)).indexOf('Delete the command back to its start, all of it in zsh') >= 0, true);
check('Alt is also a Mac word, for the keyboards that print it', rowTexts(filterShortcuts(all, 'alt', macLabel, true)).indexOf('Close the tab') >= 0, true);
check('a spelled name is matched from the start of a word: "man" and "and" keep no Cmd row by Command',
  [rowTexts(filterShortcuts(all, 'man', macLabel, true)).indexOf('Save the file') < 0, rowTexts(filterShortcuts(all, 'and', macLabel, true)).indexOf('Save the file') < 0], [true, true]);
check('nothing matching leaves no group', filterShortcuts(all, 'zzqx', macLabel, true), []);
check('the query is trimmed', filterShortcuts(all, ' undo ', macLabel, true)[0].rows[0].m, 'Undo');

print('\n--- isTyping ---');
check('a text box', isTyping({ tagName: 'INPUT', type: 'text' }), true);
check('a search box', isTyping({ tagName: 'INPUT', type: 'search' }), true);
check('the editor and the terminal are textareas', isTyping({ tagName: 'TEXTAREA' }), true);
check('a checkbox is not', isTyping({ tagName: 'INPUT', type: 'checkbox' }), false);
check('a button is not', isTyping({ tagName: 'BUTTON' }), false);
check('editable text is', isTyping({ tagName: 'DIV', isContentEditable: true }), true);
check('nothing focused is not', isTyping(null), false);
