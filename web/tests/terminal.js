// What a key in the terminal does to the clipboard: copied and pasted on
// Windows the way Windows Terminal does it, left alone on a Mac.
// Run from the repository root: jsc web/tests/terminal.js
var src = read('web/app.js');
eval(src.slice(src.indexOf('function termClipboard'), src.indexOf('function initTerm')));

function check(label, got, want) {
  var g = JSON.stringify(got), w = JSON.stringify(want);
  print((g === w ? 'PASS  ' : 'FAIL  ') + label + (g === w ? '' : '\n        expected ' + w + '\n        got      ' + g));
}
function down(key, mods) {
  var e = { type: 'keydown', key: key, ctrlKey: false, altKey: false, metaKey: false, shiftKey: false };
  (mods || []).forEach(function (m) { e[m + 'Key'] = true; });
  return e;
}

print('--- outside a Mac ---');
check('Ctrl+C with a selection copies', termClipboard(down('c', ['ctrl']), false, true), 'copy');
check('Ctrl+C with nothing selected interrupts', termClipboard(down('c', ['ctrl']), false, false), '');
check('Ctrl+V pastes rather than sending ^V', termClipboard(down('v', ['ctrl']), false, false), 'paste');
check('Ctrl+Shift+C copies, rather than opening the element picker',
  termClipboard(down('C', ['ctrl', 'shift']), false, false), 'copy');
check('Ctrl+Shift+V pastes', termClipboard(down('V', ['ctrl', 'shift']), false, false), 'paste');
check('Caps Lock does not change Ctrl+C', termClipboard(down('C', ['ctrl']), false, true), 'copy');
check('AltGr is Ctrl with Alt, and types a character',
  termClipboard(down('c', ['ctrl', 'alt']), false, true), '');
check('the release of the key does nothing twice',
  termClipboard({ type: 'keyup', key: 'v', ctrlKey: true }, false, false), '');
check('Ctrl+D stays the shell\'s', termClipboard(down('d', ['ctrl']), false, true), '');
check('a letter alone is typing', termClipboard(down('v'), false, false), '');

print('\n--- on a Mac ---');
check('Ctrl+C interrupts even with a selection', termClipboard(down('c', ['ctrl']), true, true), '');
check('Ctrl+V is the shell\'s', termClipboard(down('v', ['ctrl']), true, false), '');
check('Cmd+C is the browser\'s already', termClipboard(down('c', ['meta']), true, true), '');
