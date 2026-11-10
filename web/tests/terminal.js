// What a key in the terminal does to the clipboard: copied and pasted on
// Windows the way Windows Terminal does it, left alone on a Mac. And what the
// line keys send the shell: on a Mac the bytes VS Code sends for them, and to
// Git Bash on Windows the keys readline binds by itself.
// Run from the repository root: jsc web/tests/terminal.js
var src = read('web/app.js');
eval(src.slice(src.indexOf('function termClipboard'), src.indexOf('function initTerm')));
// sendToTerminal runs up to the Catalog's section marker, where nothing else of the terminal's is.
var sendSrc = src.slice(src.indexOf('function sendToTerminal'), src.indexOf('// ---------------------------------------------------------------- catalog --'));

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

print('\n--- the terminal takes the theme\'s colours (0161) ---');
var vars = {
  '--bg': '#fbfbfa', '--fg': '#383a42', '--accent': '#1f6fd1', '--select': 'rgba(31, 111, 209, .12)',
  '--ansi-k': 'k', '--ansi-r': 'r', '--ansi-g': 'g', '--ansi-y': 'y', '--ansi-b': 'b', '--ansi-m': 'm', '--ansi-c': 'c', '--ansi-w': 'w',
  '--ansi-k-hi': 'K', '--ansi-r-hi': 'R', '--ansi-g-hi': 'G', '--ansi-y-hi': 'Y', '--ansi-b-hi': 'B', '--ansi-m-hi': 'M', '--ansi-c-hi': 'C', '--ansi-w-hi': 'W',
};
var asked = [];
var theme = termTheme(function (n) { asked.push(n); return vars[n]; });
check('the page and the text are the theme\'s, the cursor its accent', [theme.background, theme.foreground, theme.cursor, theme.cursorAccent], ['#fbfbfa', '#383a42', '#1f6fd1', '#fbfbfa']);
check('the selection is the theme\'s translucent one, in focus or not', [theme.selectionBackground, theme.selectionInactiveBackground], ['rgba(31, 111, 209, .12)', 'rgba(31, 111, 209, .12)']);
check('the eight colours', [theme.black, theme.red, theme.green, theme.yellow, theme.blue, theme.magenta, theme.cyan, theme.white], ['k', 'r', 'g', 'y', 'b', 'm', 'c', 'w']);
check('and the eight brights', [theme.brightBlack, theme.brightRed, theme.brightGreen, theme.brightYellow, theme.brightBlue, theme.brightMagenta, theme.brightCyan, theme.brightWhite], ['K', 'R', 'G', 'Y', 'B', 'M', 'C', 'W']);
check('it reads variables the themes declare and nothing else', asked.filter(function (n) { return !(n in vars); }), []);
check('a value is never invented when a variable is empty', termTheme(function () { return ''; }).brightWhite, '');
check('xterm.js is made with it, and may register decorations, and repaints when a theme is picked',
  [/theme: termTheme\(cssVar\)/.test(src), /allowProposedApi: true/.test(src), /if \(S\.term\) S\.term\.options\.theme = termTheme\(cssVar\);/.test(src)], [true, true, true]);
check('the text is 12.5px, on rows of 19px where the font\'s cell is 15px: xterm.js multiplies the two', /fontSize: 12\.5, lineHeight: 19 \/ 15/.test(src), true);

print('\n--- the chip beside a command typed for you ---');
check('typed, with Enter left to the person', typedChipText('the Jobs tab', false), 'typed from the Jobs tab · Enter is yours');
check('run, when the app pressed Enter', typedChipText('the manifest\'s card', true), 'run from the manifest\'s card');
check('no page named, still honest', [typedChipText('', false), typedChipText(undefined, true)], ['typed for you · Enter is yours', 'run for you']);
check('the shell echoed it when the line grew by it', echoed('(dbt) jaffle_shop % ', '(dbt) jaffle_shop % dbt parse', 'dbt parse'), true);
check('not before', echoed('(dbt) jaffle_shop % ', '(dbt) jaffle_shop % ', 'dbt parse'), false);
check('not half of it', echoed('% ', '% dbt pa', 'dbt parse'), false);
check('a command typed twice is seen twice', [echoed('% dbt parse', '% dbt parse', 'dbt parse'), echoed('% dbt parse', '% dbt parsedbt parse', 'dbt parse')], [false, true]);
check('a key or a paste takes the chip away', ['a', '\r', '\x1b[A', '\x1bOA', '\x1b[200~select 1\x1b[201~', '\x7f'].map(typedByPerson), [true, true, true, true, true, true]);
check('xterm.js\'s own answers to the shell do not', ['\x1b[?1;2c', '\x1b[>0;276;0c', '\x1b[12;34R', '\x1b[0n', '\x1b[I', '\x1b[O', '\x1bP>|xterm.js(5.5.0)\x1b\\', '\x1b]11;rgb:0f0f/1313/1818\x07'].map(typedByPerson),
  [false, false, false, false, false, false, false, false]);
['the Jobs tab', 'the selection', 'the Compiled tab', 'the Run tab', 'the manifest\\\'s card', 'the settings', 'the column lineage menu', 'Edith\\\'s menu'].forEach(function (name) {
  check('a page that types a command names itself: ' + name.replace(/\\/g, ''), src.indexOf("'" + name + "'") >= 0, true);
});
check('Enter follows the echo of a command the page runs, on a line nobody touched', enterAfterEcho(true, 4, 4, true), true);
check('not after a key the person typed in between', enterAfterEcho(true, 4, 5, true), false);
check('not into a shell that was restarted', enterAfterEcho(true, 4, 4, false), false);
check('and never for a command that only waits', enterAfterEcho(false, 4, 4, true), false);

print('\n--- where the chip fits ---');
// 100 columns over 800px is 8px a cell; the chip is 120px wide and starts 12px right of the cursor.
check('beside a short line', chipFits(10, 100, 800, 120), true);
check('up to the edge exactly', chipFits(83, 100, 800, 124), true);
check('a line that fills the width has no room', chipFits(90, 100, 800, 120), false);
check('a cursor waiting to wrap, at the last column, has none', chipFits(100, 100, 800, 120), false);
check('nor one at the last column of the row', chipFits(99, 100, 800, 8), false);
check('a chip that fits nowhere', chipFits(0, 100, 800, 800), false);

print('\n--- a command typed for you, and the person\'s keys meanwhile (0161, 0073) ---');
// The shell's echo, the timers, the socket and the page are stood in for.
var S, sent, toasts, chips, echoed_ = true, timers;
function fresh(over) {
  S = { termKeys: 0, termEnters: 0, termPrompt: true, typedChip: null, ws: null, term: null };
  sent = []; toasts = []; chips = []; timers = [];
  var ws = { readyState: 1, send: function (m) { sent.push(JSON.parse(m).d); } };
  S.ws = ws;
  for (var k in over || {}) S[k] = over[k];
  return ws;
}
var closePanel = function () {}, showDock = function () {};
var toast = function (t) { toasts.push(t); };
var showTypedChip = function (label) { chips.push(label); return true; };
var dropTypedChip = function () {};
var termLineToCursor = function () { return ''; };
var done_;
// The echo is heard when the test says so, so a key can be typed before it.
var awaitEcho = function (first, before, cmd, done) { done_ = done; };
var setTimeout = function (f) { f(); };
eval(sendSrc);
function send(run, typed, from) {
  S.term = { buffer: { active: { baseY: 0, cursorY: 0 } }, focus: function () {} };
  sendToTerminal('dbt ls', run, typed, from);
}

fresh(); send(true, null, 'the Jobs tab');
check('the command is sent at once, and Enter waits for its echo', sent, ['dbt ls']);
check('  and bash is still at its prompt until Enter is pressed', S.termPrompt, true);
done_(true);
check('an untouched line gets its Enter', sent, ['dbt ls', '\r']);
check('  after which bash has handed the line on', S.termPrompt, false);
check('  and the chip says it was run', [chips, toasts], [['run from the Jobs tab'], []]);

fresh(); send(true, null, 'the Jobs tab');
termInput('x');
done_(true);
check('a key typed before the echo: no Enter is sent', sent, ['dbt ls', 'x']);
check('  and bash is still at its prompt, so its line keys stay on', S.termPrompt, true);
check('  and the line is left to the person, with the chip', chips, ['typed from the Jobs tab \u00b7 Enter is yours']);

fresh(); send(false, null, 'the Jobs tab');
done_(true);
check('a command that only waits sends no Enter and leaves bash at its prompt', [sent, S.termPrompt], [['dbt ls'], true]);

fresh(); send(false, null, 'the Jobs tab');
termInput('\r');
done_(true);
check('the person pressed Enter themselves while the page waited: no chip', chips, []);
check('  and no toast saying to press Enter', toasts, []);
check('  and the Enter was counted once', S.termEnters, 1);

fresh(); send(true, null, 'the Jobs tab');
termInput('\r');
done_(true);
check('  and none of the page\'s own Enter either', [sent, chips, toasts], [['dbt ls', '\r'], [], []]);

fresh(); send(false, null, 'the Jobs tab');
termInput('\x1b[200~select 1\nfrom orders\x1b[201~');
done_(true);
check('a bracketed paste holding a newline is no Enter: the chip is drawn', [S.termEnters, S.termKeys, chips.length], [0, 1, 1]);
check('  and bash is still at its prompt', S.termPrompt, true);

fresh(); send(false, null, 'the Jobs tab');
termInput('\x1b[?1;2c');
termInput('\x1b[I');
done_(true);
check('xterm.js\'s own answers are no key and no Enter', [S.termKeys, S.termEnters, chips.length], [0, 0, 1]);

fresh(); send(false, null, 'the Jobs tab');
done_(false);
check('no echo seen: the toast says Enter is theirs', toasts, ['command ready in the terminal, press Enter to run it']);

fresh(); send(true, null, 'the Jobs tab');
done_(false);
check('no echo seen, and run: Enter is pressed and the toast says so', [sent, toasts], [['dbt ls', '\r'], ['running dbt ls in the terminal']]);

fresh(); S.ws.readyState = 3;
send(false, null, 'the Jobs tab');
check('a socket that never opens is told, and nothing is sent', [sent, toasts], [[], ['terminal is not connected']]);

print('\n--- a line with no room for the chip says it in the toast ---');
fresh(); send(false, null, 'the Jobs tab');
showTypedChip = function (label) { chips.push(label); return false; };
done_(true);
check('chip not made: the toast', toasts, ['command ready in the terminal, press Enter to run it']);
