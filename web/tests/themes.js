// The four themes (0150): the variable blocks at the top of app.css, Light's
// contrast, Graphite's greys, which theme is on screen, the theme an export
// takes and the button in the top bar. Chrome shows the rest.
// Run from the repository root: jsc web/tests/themes.js
var src = read('web/app.js');
var window = {};
eval(src.slice(src.indexOf('const THEMES'), src.indexOf('function isNightHour')));
var css = read('web/app.css');
var html = read('web/index.html');

function check(label, got, want) {
  var g = JSON.stringify(got), w = JSON.stringify(want);
  print((g === w ? 'PASS  ' : 'FAIL  ') + label + (g === w ? '' : '\n        expected ' + w + '\n        got      ' + g));
}
function ok(label, cond) { print((cond ? 'PASS  ' : 'FAIL  ') + label); }

// The leading rules of the file whose selectors are only :root or [data-theme=...]
// (the variable blocks), each with its selector and its declarations.
var blocks = (function () {
  var out = [], at = 0;
  for (;;) {
    var rest = css.slice(at).replace(/^(?:\s|\/\*[\s\S]*?\*\/)*/, '');
    at = css.length - rest.length;
    var open = css.indexOf('{', at);
    var selector = css.slice(at, open).trim();
    if (!/^(?::root|\[data-theme="[a-z]+"\])(?:\s*,\s*(?::root|\[data-theme="[a-z]+"\]))*$/.test(selector)) break;
    var close = css.indexOf('\n}', open);
    var body = css.slice(open + 1, close);
    var decl = {}, names = [];
    // A line may carry several declarations (Light's twins): split them on `; `.
    body.replace(/\/\*[\s\S]*?\*\//g, '').split(';').forEach(function (d) {
      var colon = d.indexOf(':');
      if (colon < 0) return;
      var name = d.slice(0, colon).trim();
      if (!/^(--[\w-]+|color-scheme)$/.test(name)) return;
      decl[name] = d.slice(colon + 1).trim();
      names.push(name);
    });
    out.push({ selector: selector, decl: decl, names: names, end: close + 2 });
    at = close + 2;
  }
  return out;
}());
var night = blocks[0], derived = blocks[1], graphite = blocks[2], light = blocks[3], halloween = blocks[4];

print('--- the blocks ---');
check('app.css opens with Night, the rule the other three share, then Graphite, Light and Halloween, and nothing else before the rules',
  blocks.map(function (b) { return b.selector; }),
  [':root, [data-theme="night"]', '[data-theme="graphite"], [data-theme="light"], [data-theme="halloween"]',
    '[data-theme="graphite"]', '[data-theme="light"]', '[data-theme="halloween"]']);
check('the rules begin with the box model reset', css.slice(blocks[4].end).replace(/^(?:\s|\/\*[\s\S]*?\*\/)*/, '').indexOf('* { box-sizing'), 0);
ok('every name any block declares is declared in Night, so a theme has something to part from',
  [derived, graphite, light, halloween].every(function (b) { return b.names.every(function (n) { return n in night.decl; }); }));
check('Graphite and Halloween declare the same set',
  graphite.names.slice().sort(), halloween.names.slice().sort());
ok('Light declares that set and its own twins', graphite.names.every(function (n) { return n in light.decl; }) && light.names.length > graphite.names.length);
check('each says whether it is dark or light', [night.decl['color-scheme'], graphite.decl['color-scheme'], light.decl['color-scheme'], halloween.decl['color-scheme']],
  ['dark', 'dark', 'light', 'dark']);
check('the derived rule shares no name with a block, Night aside',
  derived.names.filter(function (n) { return [graphite, light, halloween].some(function (b) { return n in b.decl; }); }), []);
ok('the derived rule does derive: it holds more than sixty names', derived.names.length > 60);
check('Night names no var() and no color-mix: it is the literal block',
  night.names.filter(function (n) { return /var\(|color-mix/.test(night.decl[n]); }), []);
check('the derived rule is nothing but var() and color-mix',
  derived.names.filter(function (n) { return !/^(var\(--[\w-]+\)|color-mix\(in srgb, var\(--[\w-]+\) \d+%, var\(--(bg|fg)\)\))$/.test(derived.decl[n]); }), []);
check('a theme\'s own names are literals', [graphite, light, halloween].map(function (b) {
  return b.names.filter(function (n) { return /var\(|color-mix/.test(b.decl[n]); }).length;
}), [0, 0, 0]);

print('\n--- Light ---');
// WCAG relative luminance of #rrggbb.
function lum(hex) {
  var c = [1, 3, 5].map(function (i) {
    var v = parseInt(hex.slice(i, i + 2), 16) / 255;
    return v <= 0.03928 ? v / 12.92 : Math.pow((v + 0.055) / 1.055, 2.4);
  });
  return 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2];
}
function ratio(a, b) {
  var x = lum(a), y = lum(b);
  return (Math.max(x, y) + 0.05) / (Math.min(x, y) + 0.05);
}
check('the ratio is the WCAG one', [ratio('#000000', '#ffffff'), Math.round(ratio('#777777', '#ffffff') * 100) / 100], [21, 4.48]);
function channels2(v) { return [1, 3, 5].map(function (i) { return parseInt(v.slice(i, i + 2), 16); }); }
function lightValue(n) { return n in light.decl ? light.decl[n] : night.decl[n]; }
var MARKS = /^--(k-[\w-]+|t-(model|singular|column)|r-[\w-]+|ok|warn|bad|env-[\w-]+|tool-[\w-]+|f-[\w-]+)$/;
var marks = night.names.filter(function (n) { return MARKS.test(n) && /^#[0-9a-f]{6}$/i.test(night.decl[n]) && !(n in derived.decl); });
ok('the meaning colours Light has to take are many: ' + marks.length, marks.length >= 40);
check('every kind, test, role, state, environment and tool colour is declared by Light itself, not left at Night\'s',
  marks.filter(function (n) { return !(n in light.decl); }), []);
// The floor is the page and the sidebar: --bg-2 is the darker of the two, so a mark that holds there holds on both.
check('each is at least 3:1 on Light\'s sidebar (--bg-2), so on its page too', marks.filter(function (n) { return ratio(lightValue(n), light.decl['--bg-2']) < 3; })
  .map(function (n) { return n + ' ' + lightValue(n) + ' ' + ratio(lightValue(n), light.decl['--bg-2']).toFixed(2); }), []);
// --syn-var and --syn-comment are derived from the text colours, which Light's own check below covers.
var syn = night.names.filter(function (n) { return /^--syn-/.test(n) && !(n in derived.decl) && /^#[0-9a-f]{6}$/i.test(lightValue(n)); });
ok('the code colours are many: ' + syn.length, syn.length >= 10);
check('each is at least 4.5:1 on Light\'s page', syn.filter(function (n) { return ratio(lightValue(n), light.decl['--bg']) < 4.5; })
  .map(function (n) { return n + ' ' + lightValue(n) + ' ' + ratio(lightValue(n), light.decl['--bg']).toFixed(2); }), []);
check('its text is readable on its page and on its cards', ['--fg', '--fg-2', '--fg-dim', '--fg-mute'].map(function (n) {
  return ratio(light.decl[n], light.decl['--bg']) >= 4.5 && ratio(light.decl[n], light.decl['--bg-3']) >= 4.5;
}), [true, true, true, true]);
// The Compiled tab names an environment in its colour (0164): the picker's names, the note's tag and the marks in the SQL.
var ENV_NAMES = ['--env-dev', '--env-qa', '--env-prod', '--env-ci', '--env-stg', '--env-uat', '--env-other'];
check('Light declares every environment colour the picker can name, each at least 3:1 on its sidebar',
  ENV_NAMES.filter(function (n) { return !(n in light.decl) || ratio(light.decl[n], light.decl['--bg-2']) < 3; }), []);
// A name sits on its own wash, not on the bare sidebar: the picked radio's 16%, the note's tag on the note's 8% and its own 14% (0164).
(function () {
  function mixOver(c, base, p) { return [0, 1, 2].map(function (i) { return c[i] * p + base[i] * (1 - p); }); }
  function hex(c) { return '#' + c.map(function (v) { return ('0' + Math.round(v).toString(16)).slice(-2); }).join(''); }
  var base = channels2(light.decl['--bg-2']), bad = [];
  ENV_NAMES.forEach(function (n) {
    var c = channels2(light.decl[n]);
    var radio = hex(mixOver(c, base, .16)), note = mixOver(c, base, .08), tag = hex(mixOver(c, note, .14));
    [['the picked radio', radio], ['the note\'s tag', tag]].forEach(function (p) {
      if (ratio(light.decl[n], p[1]) < 3) bad.push(n + ' on ' + p[0] + ' ' + ratio(light.decl[n], p[1]).toFixed(2));
    });
  });
  check('and on the washes they sit on, the picked radio and the note\'s tag', bad, []);
}());
// The dotted underline of a swapped relation is the environment's full colour on the editor's page (--bg), over the 10% wash the same mark carries (0164).
(function () {
  function mixOver(c, base, p) { return [0, 1, 2].map(function (i) { return c[i] * p + base[i] * (1 - p); }); }
  function hex(c) { return '#' + c.map(function (v) { return ('0' + Math.round(v).toString(16)).slice(-2); }).join(''); }
  var bad = [];
  [['Night', night], ['Graphite', graphite], ['Light', light], ['Halloween', halloween]].forEach(function (t) {
    var pick = function (n) { return n in t[1].decl ? t[1].decl[n] : night.decl[n]; };
    ENV_NAMES.forEach(function (n) {
      var under = hex(mixOver(channels2(pick(n)), channels2(pick('--bg')), .10));
      if (ratio(pick(n), under) < 3) bad.push(t[0] + ' ' + n + ' ' + ratio(pick(n), under).toFixed(2));
    });
  });
  check('the underline of a swapped relation holds 3:1 over the editor\'s page and its wash, in every theme', bad, []);
  ok('and the rule draws it in the full colour, not a mix', /\.cm-env-swap \{[^}]*border-bottom: 1px dotted var\(--env\);/.test(css));
}());
check('--env-manifest-on and its line are gone with the chip that read them',
  [night, derived, graphite, light, halloween].map(function (b) { return '--env-manifest-on' in b.decl || '--env-manifest-on-line' in b.decl; }).concat([/--env-manifest-on/.test(css)]), [false, false, false, false, false, false]);
check('the swap colour the marks used to have is gone from every block and every rule',
  [night, derived, graphite, light, halloween].map(function (b) { return '--env-swap' in b.decl; }).concat([/--env-swap/.test(css)]), [false, false, false, false, false, false]);
// The two colours a file's icon takes beyond the kinds' (0151): dbt's YAML and Python's yellow.
check('every theme declares --dbt-yaml and --py, Light with its own twins',
  [night, graphite, light, halloween].map(function (b) { return ['--dbt-yaml', '--py'].filter(function (n) { return !(n in b.decl); }); }), [[], [], [], []]);
check('both are at least 3:1 on Light\'s sidebar, where the icons sit', ['--dbt-yaml', '--py'].filter(function (n) {
  return ratio(light.decl[n], light.decl['--bg-2']) < 3;
}), []);
// Git's letters are 10.5px text at the end of a tree row (0151): the sidebar, a selected row, and a folder's dimmed one all hold 4.5:1.
(function () {
  var bg2 = channels2(light.decl['--bg-2']);
  function hex(c) { return '#' + c.map(function (v) { return ('0' + v.toString(16)).slice(-2); }).join(''); }
  function mix(a, b, p) { return a.map(function (v, i) { return Math.round(v * p + b[i] * (1 - p)); }); }
  // A selected row is the wash (--select, an rgba) laid over the sidebar (0166).
  var wash = /rgba\((\d+), (\d+), (\d+), ([\d.]+)\)/.exec(light.decl['--select']);
  var onSel = mix([+wash[1], +wash[2], +wash[3]], bg2, parseFloat(wash[4]));
  var bad = [];
  ['--git-m', '--git-u'].forEach(function (n) {
    var c = channels2(light.decl[n]);
    [['sidebar', bg2], ['selected row', onSel]].forEach(function (p) { if (ratio(hex(c), hex(p[1])) < 4.5) bad.push(n + ' on ' + p[0]); });
    if (ratio(hex(mix(c, bg2, .85)), light.decl['--bg-2']) < 4.5) bad.push(n + ' dimmed on a folder');
  });
  check('Light\'s git letters are at least 4.5:1 on its sidebar, on a selected row and dimmed on a folder', bad, []);
  // The icons of a selected row are the kind colours (a file's, 0151): every kind colour, the YAML and the Python marks hold 3:1
  // there, in each theme on its own --select over its sidebar, which is where Open editors' and the tree's selected rows sit.
  // The macro glyph (--f-macro) sits on the selected row of the palette, whose box is --bg-3, so it is held on that wash.
  // Only the test colour --t-column is left out (no selected row draws a test; docs/state.md, the shell's limits).
  var kinds = night.names.filter(function (n) { return /^--k-/.test(n) && /^#[0-9a-f]{6}$/i.test(night.decl[n]) && !(n in derived.decl); }).concat(['--dbt-yaml', '--py']);
  ok('the names checked are every kind colour and the two file marks: ' + kinds.length, kinds.length === 13);
  check('Light\'s macro glyph is at least 3:1 on the selected row of the palette (--select over --bg-3)',
    ratio(lightValue('--f-macro'), hex(mix([+wash[1], +wash[2], +wash[3]], channels2(light.decl['--bg-3']), parseFloat(wash[4])))) < 3, false);
  [['Night', night], ['Graphite', graphite], ['Light', light], ['Halloween', halloween]].forEach(function (t) {
    var d = t[1].decl, bgT = channels2(d['--bg-2']);
    var w = /rgba\((\d+), (\d+), (\d+), ([\d.]+)\)/.exec(d['--select']);
    var sel = hex(mix([+w[1], +w[2], +w[3]], bgT, parseFloat(w[4])));
    check(t[0] + '\'s kind and file marks are at least 3:1 on a selected row (--select over --bg-2)', kinds.filter(function (n) {
      var v = n in d ? d[n] : (t[1] === light ? lightValue(n) : night.decl[n]);
      return ratio(v, sel) < 3;
    }), []);
  });
}());
// Reports is dimmed while it has nothing to show (0075, 0166): its glyph, --fg-mute at the tab's opacity over the bar, is a control and holds 3:1 in every theme.
(function () {
  var dim = /\.sp\.dim \{ opacity: ([\d.]+); \}/.exec(css);
  ok('the dimmed tab\'s opacity is declared once', !!dim);
  var o = dim ? parseFloat(dim[1]) : 1;
  function mixed(fg, bg) {
    var a = channels2(fg), b = channels2(bg);
    return '#' + a.map(function (v, i) { return ('0' + Math.round(v * o + b[i] * (1 - o)).toString(16)).slice(-2); }).join('');
  }
  var bad = [['Night', night], ['Graphite', graphite], ['Light', light], ['Halloween', halloween]].filter(function (t) {
    var fg = t[1].decl['--fg-mute'] || night.decl['--fg-mute'], bg = t[1].decl['--bg-2'] || night.decl['--bg-2'];
    return ratio(mixed(fg, bg), bg) < 3;
  }).map(function (t) { return t[0]; });
  check('the dimmed Reports glyph is at least 3:1 on the bar (--bg-2) in the four themes', bad, []);
}());
// A test's line on the canvas, the dotted one and the host's dashed stem (0160), is a graphic and holds 3:1 on the page in every theme.
(function () {
  var share = /color-mix\(in srgb, var\(--fg\) (\d+)%, var\(--bg\)\)/.exec(derived.decl['--edge-test'] || '');
  ok('the three themes derive --edge-test as a share of the text colour over the page', !!share);
  var p = share ? parseInt(share[1], 10) / 100 : 0;
  function edge(t) {
    if (t === night) return night.decl['--edge-test'];
    var fg = channels2(t.decl['--fg']), bg = channels2(t.decl['--bg']);
    return '#' + fg.map(function (v, i) { return ('0' + Math.round(v * p + bg[i] * (1 - p)).toString(16)).slice(-2); }).join('');
  }
  var bad = [['Night', night], ['Graphite', graphite], ['Light', light], ['Halloween', halloween]].filter(function (t) {
    return ratio(edge(t[1]), t[1].decl['--bg'] || night.decl['--bg']) < 3;
  }).map(function (t) { return t[0]; });
  check('--edge-test is at least 3:1 on the page (--bg) in the four themes', bad, []);
  ok('and the stem is drawn in it, in the page and in an exported file', /\.edge\.test\.stem \{ stroke: var\(--edge-test\);/.test(css)
    && src.indexOf("'.edge.test.stem { stroke: var(--edge-test);") > 0);
}());
// The warning colour as text (the pill's "manifest" and its age, the card's amber line): Light darkens it, the others keep Night's.
check('every theme but Light has Night\'s warning text, and Light\'s is at least 4.5:1 on its sidebar and on a card',
  [night.decl['--warn-text'] === night.decl['--warn'], '--warn-text' in graphite.decl, '--warn-text' in halloween.decl,
    ratio(light.decl['--warn-text'], light.decl['--bg-2']) >= 4.5, ratio(light.decl['--warn-text'], light.decl['--bg-3']) >= 4.5],
  [true, false, false, true, true]);
check('every theme but Light keeps Night\'s git letters, which are the warning and OK colours they have',
  [graphite, halloween].map(function (b) { return ['--git-m', '--git-u'].filter(function (n) { return n in b.decl; }).length; })
    .concat([night.decl['--git-m'] === night.decl['--warn'], night.decl['--git-u'] === night.decl['--ok'], graphite.decl['--warn'] === night.decl['--warn'], halloween.decl['--ok'] === night.decl['--ok']]),
  [0, 0, true, true, true, true]);
// The terminal is the page's surface and the theme's own sixteen colours (0161).
ok('no theme keeps a terminal background or scrollbar of its own: the terminal reads --bg and --scroll-thumb',
  [night, derived, graphite, light, halloween].every(function (b) { return !('--term-bg' in b.decl) && !('--term-thumb' in b.decl); })
  && /#term \{[^}]*background: var\(--bg\)/.test(css) && /xterm-viewport::-webkit-scrollbar-thumb \{[^}]*var\(--scroll-thumb\)/.test(css));
var ANSI = ['k', 'r', 'g', 'y', 'b', 'm', 'c', 'w'].reduce(function (list, c) { return list.concat('--ansi-' + c, '--ansi-' + c + '-hi'); }, []);
check('every theme declares all sixteen ANSI colours itself',
  [night, graphite, light, halloween].map(function (b) { return ANSI.filter(function (n) { return !(n in b.decl); }); }), [[], [], [], []]);
[['Night', night], ['Graphite', graphite], ['Light', light], ['Halloween', halloween]].forEach(function (t) {
  var b = t[1];
  function own(n) { return n in b.decl ? b.decl[n] : night.decl[n]; }
  check(t[0] + '\'s terminal text is at least 4.5:1 on its page', ratio(own('--fg'), own('--bg')) >= 4.5, true);
  // Black is drawn dim by design, a colour for backgrounds a program paints, so it is the one left out.
  check(t[0] + '\'s other fifteen ANSI colours are at least 3:1 on its page',
    ANSI.filter(function (n) { return n !== '--ansi-k' && ratio(b.decl[n], own('--bg')) < 3; })
      .map(function (n) { return n + ' ' + b.decl[n] + ' ' + ratio(b.decl[n], own('--bg')).toFixed(2); }), []);
});

print('\n--- Graphite ---');
function channels(v) {
  var hex = /^#([0-9a-f]{2})([0-9a-f]{2})([0-9a-f]{2})$/i.exec(v);
  if (hex) return hex.slice(1).map(function (h) { return parseInt(h, 16); });
  var rgba = /^rgba\((\d+),\s*(\d+),\s*(\d+)/.exec(v);
  if (rgba) return rgba.slice(1).map(Number);
  var list = /^(\d+),\s*(\d+),\s*(\d+)$/.exec(v);
  return list ? list.slice(1).map(Number) : null;
}
var GREY = /^--(bg(-\d)?|line(-[a-z]+)?|fg(-[a-z0-9]+)?|accent(-dim|-rgb)?|select|focus-bg|edge(-hot)?|hover|band(-line)?)$/;
var greys = graphite.names.filter(function (n) { return GREY.test(n); });
ok('the greys are many: ' + greys.length, greys.length >= 20);
check('each has equal channels: no tint', greys.filter(function (n) {
  var c = channels(graphite.decl[n]);
  return !c || c[0] !== c[1] || c[1] !== c[2];
}), []);
check('Halloween\'s page is warm, which is what Graphite is not', channels(halloween.decl['--bg'])[0] > channels(halloween.decl['--bg'])[2], true);

print('\n--- previews and swatches paint in any theme ---');
function rulesOf(sel) {
  var out = [], re = /^([^{}\n@][^{}\n]*)\{([^}]*)\}/gm, m;
  while ((m = re.exec(css))) if (m[1].indexOf(sel) >= 0) out.push(m[2]);
  return out;
}
var drawn = rulesOf('.tprev').concat(rulesOf('.tm-sw'));
ok('the card\'s preview and the menu\'s swatch have rules: ' + drawn.length, drawn.length >= 8);
var reads = drawn.join(' ').match(/var\(--[\w-]+\)/g) || [];
var wanted = reads.map(function (v) { return v.slice(4, -1); }).filter(function (v, i, a) { return a.indexOf(v) === i && v !== '--k'; });
check('they read only names Graphite declares, which every theme\'s section 1 block does too', wanted.filter(function (n) { return !(n in graphite.decl); }), []);

print('\n--- which theme is on screen ---');
check('the four, in the menu\'s order', [themeId('night'), themeId('graphite'), themeId('light'), themeId('halloween')], ['night', 'graphite', 'light', 'halloween']);
check('anything else is Night', [themeId('sepia'), themeId(undefined), themeId(null), themeId(''), themeId(3), themeId('Light')], ['night', 'night', 'night', 'night', 'night', 'night']);
check('Follow the system shows Night while the system is dark', shownTheme({ follow: true, theme: 'halloween' }, true), 'night');
check('and Light while it is light', shownTheme({ follow: true, theme: 'halloween' }, false), 'light');
check('Follow off shows the one picked', [shownTheme({ follow: false, theme: 'graphite' }, true), shownTheme({ follow: false, theme: 'graphite' }, false)], ['graphite', 'graphite']);
check('whatever was kept, a bad name is Night', shownTheme({ follow: false, theme: 'sepia' }, false), 'night');
check('an export takes the theme on screen', [exportTheme('night'), exportTheme('graphite'), exportTheme('light')], ['night', 'graphite', 'light']);
check('but Halloween\'s art never leaves the page: it takes Night', exportTheme('halloween'), 'night');
check('and a theme it does not know is Night', exportTheme('sepia'), 'night');
ok('the four are named for the menu and the cards, each with a line under its name',
  /id: 'night', name: 'Night', about: '[^']+'/.test(src) && /id: 'graphite', name: 'Graphite'/.test(src)
  && /id: 'light', name: 'Light'/.test(src) && /id: 'halloween', name: 'Halloween'/.test(src));

print('\n--- the pick ---');
ok('a pick turns Follow off and keeps the pick', /function pickTheme\(id\) \{ setLook\(\{ theme: id, follow: false \}\); \}/.test(src));
ok('Follow off keeps the theme on screen', /follow: false, theme: themeNow\(\)/.test(src));
ok('it tracks the system as it turns, only while Follow is on, closing an open menu whose tick is out of date',
  /DARK\.addEventListener\('change', \(\) => \{ if \(S\.look\.follow\) \{ closePopup\(\); applyLook\(\); repaintSettings\(\); \} \}\)/.test(src)
  && /matchMedia\('\(prefers-color-scheme: dark\)'\)/.test(src));
ok('applyLook sets data-theme on <html>', /root\.dataset\.theme = theme;/.test(src) && /const root = document\.documentElement;/.test(src));
ok('what is kept is the theme, whether to follow, the apparitions and the box style, and nothing of the old switch or sound',
  /const body = \{ theme: themeId\(S\.look\.theme\), follow: !!S\.look\.follow, apparitions: S\.look\.apparitions !== false, boxes: hwBoxes\(S\.look\.boxes\) \};/.test(src));
ok('the menu has one radio row per theme, keyed theme-<id>, with its swatch',
  /role', 'menu'/.test(src) && /'menuitemradio'/.test(src) && /dataset\.key = 'theme-' \+ t\.id/.test(src) && /sw\.dataset\.theme = t\.id/.test(src));
ok('and sits under its button, held inside the window from the right',
  /menu\.dataset\.align = 'right'/.test(src) && /el\.dataset\.align === 'right' \? at\.right - box\.width : at\.left/.test(src));
ok('the cards are radios, the arrows move and pick, the checked one is the tab stop',
  /setAttribute\('role', 'radiogroup'\)/.test(src) && /setAttribute\('role', 'radio'\)/.test(src) && /ArrowRight: at \+ 1/.test(src)
  && /card\.tabIndex = t\.id === shown \? 0 : -1/.test(src) && /Home: 0, End: cards\.length - 1/.test(src));
ok('the Halloween group is dimmed, its controls left usable', /box\.classList\.toggle\('faded', !on\)/.test(src) && /\.sgroup\.faded \{ opacity: \.55; \}/.test(css) && !/\.sgroup\.faded[^{]*\{[^}]*pointer-events/.test(css));

print('\n--- the top bar ---');
ok('the page starts in Night, so there is no flash for most', /<html lang="en" data-theme="night">/.test(html));
ok('the theme button sits between the shortcuts and GitHub',
  html.indexOf('id="keys-btn"') < html.indexOf('id="theme-btn"') && html.indexOf('id="theme-btn"') < html.indexOf('id="github-btn"'));
ok('and the pumpkin and the speaker are gone', !/look-btn|look-sound|hw-s-pump|hw-g-pump|hw-g-flame|hw-g-stem/.test(html + css + src));
ok('and with them every sound', !/AudioContext|createOscillator|hwSound|look-sound/.test(src));
ok('the accent tints the browser\'s own controls, and a lit edge reads its role\'s colour, --edge-hot where it has none',
  /html \{ accent-color: var\(--accent\); \}/.test(css) && /\.edge\.hi \{ stroke: var\(--edge-col, var\(--edge-hot\)\)/.test(css));

print('\n--- a column box\'s role tag reads on its box (0162) ---');
var lineage = read('web/lineage.js');
var tagRoles = /const ROLE_TEXT = new Set\(\[([^\]]*)\]\)/.exec(lineage)[1].match(/[a-z]+/g);
ok('nine roles wear a tag with a colour of their own: ' + tagRoles.length, tagRoles.length === 9);
check('each has a text twin declared in Night, and in each block, so a theme has something to part from',
  [night, graphite, light, halloween].map(function (b) { return tagRoles.filter(function (r) { return !('--r-' + r + '-text' in b.decl); }); }), [[], [], [], []]);
// The tag's word sits on the box's fill, on its focus fill, on find's current match, and on the dark fill Halloween puts behind it.
var BOX = { night: night, graphite: graphite, light: light, halloween: halloween };
// --hit-bg, find's current match, is a literal in Night and a mix of --warn into --bg in the others.
function hitBg(name) {
  var b = BOX[name], at = function (n) { return n in b.decl ? b.decl[n] : night.decl[n]; };
  var m = /^color-mix\(in srgb, var\(--warn\) (\d+)%, var\(--bg\)\)$/.exec(derived.decl['--hit-bg'] || '');
  if (name === 'night' || !m) return at('--hit-bg');
  var w = channels(at('--warn')), g = channels(at('--bg')), p = +m[1] / 100;
  return '#' + w.map(function (v, i) { return ('0' + Math.round(v * p + g[i] * (1 - p)).toString(16)).slice(-2); }).join('');
}
function boxPlaces(name) {
  var b = BOX[name], at = function (n) { return n in b.decl ? b.decl[n] : night.decl[n]; };
  return ['--bg-3', '--focus-bg'].concat(name === 'halloween' ? ['--bg-2'] : []).map(function (n) { return [n, at(n)]; })
    .concat([['--hit-bg', hitBg(name)]]);
}
Object.keys(BOX).forEach(function (name) {
  var b = BOX[name], at = function (n) { return n in b.decl ? b.decl[n] : night.decl[n]; };
  check(name + ': every tag word is at least 4.5:1 on the box, its focus fill, find\'s current match' + (name === 'halloween' ? ' and the tag\'s dark fill' : ''),
    tagRoles.map(function (r) {
      return boxPlaces(name).filter(function (p) { return ratio(at('--r-' + r + '-text'), p[1]) < 4.5; }).map(function (p) { return r + ' on ' + p[0]; });
    }).reduce(function (a, c) { return a.concat(c); }, []), []);
});
check('the word is the role\'s own colour where that already reads, and the colour taken toward the text where it does not: no twin is further than the role from the text',
  [night, graphite, light, halloween].map(function (b) {
    var fg = channels(b.decl['--fg'] || night.decl['--fg']);
    return tagRoles.filter(function (r) {
      var role = channels(b.decl['--r-' + r]), twin = channels(b.decl['--r-' + r + '-text']);
      var far = function (c) { return Math.abs(c[0] - fg[0]) + Math.abs(c[1] - fg[1]) + Math.abs(c[2] - fg[2]); };
      return far(twin) > far(role);
    });
  }), [[], [], [], []]);
check('transform is a step lighter in the three dark themes, 4.5:1 on the box, and Light\'s stays',
  [night, graphite, halloween].map(function (b) { return b.decl['--r-transform']; }).concat([light.decl['--r-transform']]), ['#e86b6b', '#e86b6b', '#e86b6b', '#c03a3a']);
ok('and the lightened one is 4.5:1 or more on each dark box fill', ['night', 'graphite', 'halloween'].every(function (n) {
  return ratio(BOX[n].decl['--r-transform'], BOX[n].decl['--bg-3']) >= 4.5;
}));

print('\n--- the editor\'s find hits ---');
// A hit's fill is --warn at a percentage over the editor's page (--bg); the percentage is read from the rule.
(function () {
  var hit = /\.CodeMirror span\.cm-findhit \{([^}]*)\}/.exec(css), cur = /\.CodeMirror span\.cm-findcur \{([^}]*)\}/.exec(css);
  ok('.CodeMirror span.cm-findhit (0,2,1, over .cm-reflink.dis) sets the editor\'s text colour with !important, so the syntax colours do not sit on the amber', !!hit && /(^|;)\s*color: var\(--code-fg\) !important;/.test(hit[1]));
  ok('.CodeMirror span.cm-findcur comes after .CodeMirror span.cm-findhit, so its !important colour wins on the current hit', !!hit && !!cur && css.indexOf(hit[0]) < css.indexOf(cur[0]));
  function pct(rule) { return +/background: color-mix\(in srgb, var\(--warn\) (\d+)%, transparent\)/.exec(rule)[1] / 100; }
  function blend(w, bg, p) { return '#' + channels2(w).map(function (v, i) { return ('0' + Math.round(v * p + channels2(bg)[i] * (1 - p)).toString(16)).slice(-2); }).join(''); }
  function resolve(b, n) {
    var v = n in b.decl ? b.decl[n] : (n in derived.decl ? derived.decl[n] : night.decl[n]);
    var m = /^var\((--[\w-]+)\)$/.exec(v);
    if (m) return resolve(b, m[1]);
    return /^#[0-9a-f]{3}$/i.test(v) ? '#' + v.slice(1).replace(/./g, '$&$&') : v;
  }
  [['Night', night], ['Graphite', graphite], ['Light', light]].forEach(function (t) {
    var fill = blend(resolve(t[1], '--warn'), resolve(t[1], '--bg'), pct(hit[1]));
    var r = ratio(resolve(t[1], '--code-fg'), fill);
    ok(t[0] + ': --code-fg is at least 4.5:1 on a find hit (' + r.toFixed(2) + ' on ' + fill + ')', r >= 4.5);
    // The current hit is selected and, in the main editor, on the cursor line: --cur-line, then --select, then the fill, over --bg.
    var under = resolve(t[1], '--bg');
    ['--cur-line', '--select'].forEach(function (n) {
      var m = /^rgba\((\d+), (\d+), (\d+), ([\d.]+)\)$/.exec(resolve(t[1], n));
      under = blend('#' + [m[1], m[2], m[3]].map(function (v) { return ('0' + (+v).toString(16)).slice(-2); }).join(''), under, +m[4]);
    });
    var now = blend(resolve(t[1], '--warn'), under, pct(cur[1]));
    var c = ratio(resolve(t[1], '--find-cur-fg'), now);
    ok(t[0] + ': --find-cur-fg is at least 4.5:1 on the current hit, selected on the cursor line (' + c.toFixed(2) + ' on ' + now + ')', c >= 4.5);
  });
}());
