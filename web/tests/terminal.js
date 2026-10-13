// What a key in the terminal does to the clipboard: copied and pasted on
// Windows the way Windows Terminal does it, left alone on a Mac. And what the
// line keys send the shell: on a Mac the bytes VS Code sends for them, and to
// Git Bash on Windows the keys readline binds by itself.
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

// Where a key lands: at a prompt bash reported, or one nothing reported.
function at(os, shell, more) {
  var a = { os: os, shell: shell, prompt: true, screen: 'normal' };
  for (var k in more || {}) a[k] = more[k];
  return a;
}
var MAC = at('mac', 'zsh', { prompt: undefined }), BASH = at('windows', 'bash');

print('\n--- the line keys of a Mac, as VS Code sends them ---');
check('Cmd+Left goes to the start of the line', termLineKey(down('ArrowLeft', ['meta']), MAC), '\x01');
check('Cmd+Right goes to its end', termLineKey(down('ArrowRight', ['meta']), MAC), '\x05');
check('Cmd+Backspace deletes back to its start', termLineKey(down('Backspace', ['meta']), MAC), '\x15');
check('Option+Delete deletes the next word', termLineKey(down('Delete', ['alt']), MAC), '\x1bd');
check('whatever the shell', termLineKey(down('ArrowLeft', ['meta']), at('mac', 'bash')), '\x01');
check('Option+Left is left to xterm.js, which sends ESC b', termLineKey(down('ArrowLeft', ['alt']), MAC), '');
check('Option+Backspace is left to xterm.js', termLineKey(down('Backspace', ['alt']), MAC), '');
check('Home is left to xterm.js', termLineKey(down('Home'), at('mac', 'bash')), '');
check('Cmd+Shift+Left is not Cmd+Left', termLineKey(down('ArrowLeft', ['meta', 'shift']), MAC), '');
check('Cmd+Option+Left is not Cmd+Left', termLineKey(down('ArrowLeft', ['meta', 'alt']), MAC), '');
check('Ctrl+Left is the system\'s, which moves between desktops', termLineKey(down('ArrowLeft', ['ctrl']), MAC), '');
check('Cmd+C is still the browser\'s', termLineKey(down('c', ['meta']), MAC), '');
check('the release of the key sends nothing',
  termLineKey({ type: 'keyup', key: 'ArrowLeft', metaKey: true }, MAC), '');
var composing = down('ArrowLeft', ['meta']); composing.isComposing = true;
check('nothing while an input method composes', termLineKey(composing, MAC), '');

print('\n--- the line keys of Windows, as readline binds them in bash ---');
check('Ctrl+Left goes back a word', termLineKey(down('ArrowLeft', ['ctrl']), BASH), '\x1bb');
check('Ctrl+Right goes forward one', termLineKey(down('ArrowRight', ['ctrl']), BASH), '\x1bf');
check('Home goes to the start of the line', termLineKey(down('Home'), BASH), '\x01');
check('End goes to its end', termLineKey(down('End'), BASH), '\x05');
check('Ctrl+Backspace deletes the word before', termLineKey(down('Backspace', ['ctrl']), BASH), '\x17');
check('Ctrl+Delete deletes the word after', termLineKey(down('Delete', ['ctrl']), BASH), '\x1bd');
check('PowerShell reads the keys themselves', termLineKey(down('ArrowLeft', ['ctrl']), at('windows', 'powershell')), '');
check('so does Home there', termLineKey(down('Home'), at('windows', 'powershell')), '');
check('before the server has said which shell', termLineKey(down('Home'), at('windows', '')), '');
check('bash on Linux reads xterm.js\'s sequences already', termLineKey(down('Home'), at('other', 'bash')), '');
check('AltGr is Ctrl with Alt, and types a character',
  termLineKey(down('ArrowLeft', ['ctrl', 'alt']), BASH), '');
check('Ctrl+Shift+Left is left to xterm.js', termLineKey(down('ArrowLeft', ['ctrl', 'shift']), BASH), '');
check('a plain arrow is xterm.js\'s', termLineKey(down('ArrowLeft'), BASH), '');
check('Backspace alone deletes a character', termLineKey(down('Backspace'), BASH), '');
check('Ctrl+C is the clipboard\'s or an interrupt', termLineKey(down('c', ['ctrl']), BASH), '');
check('the release of the key sends nothing',
  termLineKey({ type: 'keyup', key: 'Home' }, BASH), '');

print('\n--- a program that reads the keys itself ---');
check('less or vim on the alternate screen gets xterm.js\'s Home',
  termLineKey(down('Home'), at('windows', 'bash', { screen: 'alternate' })), '');
check('  and a Mac\'s Cmd+Right sends it nothing',
  termLineKey(down('ArrowRight', ['meta']), at('mac', 'zsh', { prompt: undefined, screen: 'alternate' })), '');
check('a command bash started, less -X or PowerShell typed at its prompt, gets xterm.js\'s',
  termLineKey(down('Home'), at('windows', 'bash', { prompt: false })), '');
check('  on a Mac too, under a bash that reports its prompt',
  termLineKey(down('ArrowLeft', ['meta']), at('mac', 'bash', { prompt: false })), '');
check('under oh-my-zsh, whose prompt asks for cursor keys, a Mac still sends Cmd+Right',
  termLineKey(down('ArrowRight', ['meta']), at('mac', 'zsh', { prompt: undefined, appCursor: true })), '\x05');
check('on Windows, a pager asking for cursor keys under a bash that never reported gets xterm.js\'s',
  termLineKey(down('Home'), at('windows', 'bash', { prompt: undefined, appCursor: true })), '');
check('at a prompt bash reported, cursor keys asked by its inputrc change nothing',
  termLineKey(down('Home'), at('windows', 'bash', { appCursor: true })), '\x01');
check('a bash that never reported its prompt still gets them',
  termLineKey(down('Home'), at('windows', 'bash', { prompt: undefined })), '\x01');

print('\n--- whether bash is still at its prompt ---');
check('typing keeps it there', promptAfter(true, 'dbt ls'), true);
check('Enter hands the line on', promptAfter(true, '\r'), false);
check('  so does a line typed with its Enter', promptAfter(true, 'dbt parse\r'), false);
check('  and Ctrl+J', promptAfter(true, '\n'), false);
check('a bracketed paste keeps its newlines in the line',
  promptAfter(true, '\x1b[200~select 1\nfrom orders\x1b[201~'), true);
check('a command running stays so until the next prompt', promptAfter(false, 'y'), false);
check('a prompt never reported stays unknown', promptAfter(undefined, '\r'), undefined);
