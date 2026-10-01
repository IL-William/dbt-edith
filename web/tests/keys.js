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

print('\n--- isTyping ---');
check('a text box', isTyping({ tagName: 'INPUT', type: 'text' }), true);
check('a search box', isTyping({ tagName: 'INPUT', type: 'search' }), true);
check('the editor and the terminal are textareas', isTyping({ tagName: 'TEXTAREA' }), true);
check('a checkbox is not', isTyping({ tagName: 'INPUT', type: 'checkbox' }), false);
check('a button is not', isTyping({ tagName: 'BUTTON' }), false);
check('editable text is', isTyping({ tagName: 'DIV', isContentEditable: true }), true);
check('nothing focused is not', isTyping(null), false);
