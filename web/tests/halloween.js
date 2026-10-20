// The Halloween look (0106): after dark by the browser's clock, how long to wait
// for the next apparition, which way the witch faces, and that nothing of the
// look can reach an exported file. Chrome shows the rest.
// Run from the repository root: jsc web/tests/halloween.js
var src = read('web/app.js');
eval(src.slice(src.indexOf('function isNightHour'), src.indexOf('function placeWitch')));
var lin = read('web/lineage.js');
eval(lin.slice(lin.indexOf('const ART_MAX'), lin.indexOf('let skin = null')));
var ART_MAX = +/const ART_MAX = (\d+)/.exec(lin)[1];
var css = read('web/app.css');
var html = read('web/index.html');

function check(label, got, want) {
  var g = JSON.stringify(got), w = JSON.stringify(want);
  print((g === w ? 'PASS  ' : 'FAIL  ') + label + (g === w ? '' : '\n        expected ' + w + '\n        got      ' + g));
}
function ok(label, cond) { print((cond ? 'PASS  ' : 'FAIL  ') + label); }

print('--- night ---');
check('8pm is night', isNightHour(20), true);
check('midnight is night', isNightHour(0), true);
check('6am is still night', isNightHour(6), true);
check('7am is day', isNightHour(7), false);
check('noon is day', isNightHour(12), false);
check('7pm is day', isNightHour(19), false);

print('\n--- when the next apparition comes ---');
check('the ghost\'s shortest wait is 4 minutes', nextApparitionDelay(4, 9, 0), 4 * 60 * 1000);
check('and its longest, just under 9', nextApparitionDelay(4, 9, 0.999999), 9 * 60 * 1000);
check('halfway through its range', nextApparitionDelay(4, 9, 0.5), 6.5 * 60 * 1000);
check('the eyes\' range', nextApparitionDelay(15, 20, 0.2), 16 * 60 * 1000);
check('a draw outside 0..1 stays in the range (low)', nextApparitionDelay(4, 9, -3), 4 * 60 * 1000);
check('and high', nextApparitionDelay(4, 9, 7), 9 * 60 * 1000);

print('\n--- which way the witch faces ---');
check('towards a tab on her right', witchFacing(10, 120), 1);
check('towards a tab on her left', witchFacing(120, 10), -1);
check('not moved: she keeps her way', witchFacing(40, 40), 0);
check('placed for the first time: no way yet', witchFacing(undefined, 40), 0);

print('\n--- a switch drawn again ---');
check('seen on the other side, the look on: it walks', switchWalks(false, true, true), true);
check('and back', switchWalks(true, false, true), true);
check('the same side: it stands', switchWalks(true, true, true), false);
check('never seen: it stands', switchWalks(undefined, true, true), false);
check('the look off: it stands', switchWalks(false, true, false), false);

print('\n--- what the look touches ---');
var themed = css.split('\n').filter(function (l) { return /halloween/.test(l); });
ok('the theme\'s rules are in app.css', themed.length > 10);
ok('every rule of the look hangs from body.halloween, or from its own hw- class',
  css.slice(css.indexOf('halloween look --')).split('\n').filter(function (l) {
    return /^[^\s@}\/*].*\{/.test(l) && !/^(body\.halloween|:where\(body\.halloween\)|\.hw-|\.hwg-|\.lookbtn|#look-btn|#hw-layer)/.test(l);
  }).length === 0);
ok('the pumpkin and the speaker are in the top bar, the speaker out of sight until the look is on',
  /id="look-btn"[^>]*aria-pressed="false"/.test(html) && /id="look-sound"[^>]*class="[^"]*hidden/.test(html));
ok('the brand button is still one button with its name inside',
  /<button id="brand" class="brand" type="button" aria-haspopup="menu">[\s\S]*<b>Edith<\/b><\/button>/.test(html));
ok('the witch is in the tab row, and takes no click', /id="hw-witch"/.test(html)
  && /\.hw-witch svg \{[^}]*\}/.test(css) && /body\.halloween \.hw-witch \{[^}]*pointer-events: none/.test(css));
ok('the apparitions\' layer takes no click', /body\.halloween #hw-layer \{[^}]*pointer-events: none/.test(css));
var boxCss = css.slice(css.indexOf('/* Lineage boxes, in one of three styles'), css.indexOf('/* Spider silk'));
['tombstone', 'coffin', 'grimoire'].forEach(function (st) {
  var fill = { tombstone: 'hw-stone', coffin: 'hw-wood', grimoire: 'hw-leather' }[st];
  ok(st + ': its own class of the body, a fill of one gradient that index.html defines',
    boxCss.indexOf('body.halloween.hw-boxes-' + st + ' #graph .nd:not(.rider) rect.box { fill: url(#' + fill + ')') >= 0
    && html.indexOf('<linearGradient id="' + fill + '"') >= 0);
  ok(st + ': a hover that moves the rect and the kindbar only, never the group that carries the box\'s place',
    new RegExp('body\\.halloween\\.hw-boxes-' + st + ' \\.nd:not\\(\\.rider\\):hover rect\\.box,\\s*body\\.halloween\\.hw-boxes-' + st
      + ' \\.nd:not\\(\\.rider\\):hover \\.kindbar \\{[^}]*animation').test(boxCss)
    && !/\.nd:hover \{[^}]*(transform|animation)/.test(boxCss));
});
ok('no filter on a box, and nothing but fills, strokes and clips', !/filter:/.test(boxCss));
ok('reduced motion stops the hover', /prefers-reduced-motion: reduce\) \{[\s\S]*hover rect\.box[^}]*animation: none/.test(css));
ok('the defs sit in the page, not in the canvas lineage.js draws', html.indexOf('id="hw-stone"') < html.indexOf('<header id="topbar">') && lin.indexOf('hw-stone') < 0);

print('\n--- the box style ---');
check('each of the three is itself', [hwBoxes('tombstone'), hwBoxes('coffin'), hwBoxes('grimoire')], ['tombstone', 'coffin', 'grimoire']);
check('anything else is the tombstone', [hwBoxes(undefined), hwBoxes(null), hwBoxes(''), hwBoxes('mummy'), hwBoxes(3)], ['tombstone', 'tombstone', 'tombstone', 'tombstone', 'tombstone']);
ok('the body carries the class of the one picked', /hw-boxes-' \+ b, on && b === hwBoxes\(S\.look\.boxes\)/.test(src));
ok('and it is saved with the rest', /boxes: hwBoxes\(S\.look\.boxes\) \};/.test(src));
ok('the Theme section offers the three, and a click draws no lineage again',
  /\['tombstone', 'Tombstone'[\s\S]*\['coffin', 'Coffin'[\s\S]*\['grimoire', 'Grimoire'/.test(src) && /setLook\(\{ boxes: value \}\)/.test(src)
  && !/function setLook[\s\S]*?\n\}\n/.exec(src)[0].match(/rerender|Lineage\./));

print('\n--- the art (0107) ---');
check('art is wanted under a skin and a count', [artWanted(3, 'coffin', ART_MAX), artWanted(ART_MAX, 'tombstone', ART_MAX)], [true, true]);
check('not past the count, without a skin, or with nothing drawn', [artWanted(ART_MAX + 1, 'coffin', ART_MAX), artWanted(5, null, ART_MAX), artWanted(0, 'coffin', ART_MAX)], [false, false, false]);
check('the count is the model lineage\'s cap, where the pan and the drawing were measured', ART_MAX, 400);
check('a name has less room under a skin than the usual 27 and 24, a column less than a model',
  ['tombstone', 'coffin', 'grimoire'].map(function (k) { var m = artBudget(k, false), c = artBudget(k, true); return m.name < 27 && c.name < 24 && c.name <= m.name && c.sub <= m.sub; }),
  [true, true, true]);
check('an unknown skin takes the tombstone\'s room', artBudget('mummy', false), artBudget('tombstone', false));
['tombstone', 'coffin', 'grimoire'].forEach(function (k) {
  [[200, 48], [180, 40]].forEach(function (d) {
    var g = artGeometry(k, d[0], d[1]);
    ok(k + ' ' + d.join('x') + ': a shape, a trim, and parts that stay in the box',
      /^M/.test(g.shape) && /^M/.test(g.trim) && g.parts.length >= 2
      && g.parts.every(function (p) { return p.x >= -0.01 && p.y >= -0.01 && p.x + p.w <= d[0] + 0.01 && p.y + p.h <= d[1] + 0.01 && /^hw-/.test(p.id); }));
  });
  var m = artGeometry(k, 200, 48), c = artGeometry(k, 180, 40);
  ok(k + ': what sits at the right follows the width, what sits at the left does not',
    m.parts.some(function (p) { return Math.abs(p.x + p.w - 200) < 11 && p.x > 100 - 1; })
    && c.parts.some(function (p) { return Math.abs(p.x + p.w - 180) < 11 && p.x > 70; })
    && m.parts[0].x <= 3.01 && c.parts[0].x <= 3.01);
});
ok('every part a skin lays is a symbol of index.html',
  (function () {
    var ids = ['tombstone', 'coffin', 'grimoire'].reduce(function (a, k) { return a.concat(artGeometry(k, 200, 48).parts.map(function (p) { return p.id; })); }, []);
    return ids.every(function (id) { return html.indexOf('<symbol id="' + id + '"') >= 0; });
  })());
ok('the art is laid after each render, and in the one place that keeps the rects the hit area',
  /paintArt\(\);\s*if \(!opts\.keepView\) fit\(\); else apply\(\);/.test(lin) && /g\.hw-on \.nd:not\(\.rider\) rect\.box \{\s*fill: transparent; stroke: none; clip-path: none;/.test(css));
ok('every ring is a stroke of the silhouette, and the art takes no click',
  /\.nd\.sel \.hw-shape \{ stroke: #a78bfa/.test(css) && /\.nd\.focus \.hw-shape/.test(css) && /\.nd\.hit:not\(\.hitcur\) \.hw-shape/.test(css) && /\.nd\.off \.hw-shape \{ stroke-dasharray/.test(css)
  && /body\.halloween \.hw-art \{ pointer-events: none; \}/.test(css) && /'pointer-events': 'none'/.test(lin));
ok('the kind colour comes in on the group, inline, never by a rule', /style: `color:\$\{nodeColor\(n\)\}`/.test(lin) && !/\.hw-art \{[^}]*color:/.test(css));
ok('switching the look off takes the web off a selected box, which paintWeb() cannot find once the skin is null',
  /if \(!next && root\) root\.querySelectorAll\('\.hw-web'\)\.forEach\(\(w\) => w\.remove\(\)\);/.test(lin));
ok('the audio unlock keeps listening through a key that is no gesture',
  /navigator\.userActivation && !navigator\.userActivation\.isActive\) return;\s*document\.removeEventListener\('pointerdown', unlock, true\)/.test(src));
ok('the hover card picks its anchor when it is placed',
  /getBoundingClientRect: \(\) => \(g\.querySelector\('\.hw-web'\)/.test(src));
ok('the app asks for it from applyLook, and for none when the look is off', /Lineage\.setSkin\(on \? hwBoxes\(S\.look\.boxes\) : null\)/.test(src));

print('\n--- the dewdrops and the web (0108) ---');
check('a plain strand gets dewdrops', [beadWanted('edge'), beadWanted('edge hi')], [true, true]);
check('not a dashed one, whose dashes mean something', [beadWanted('edge back'), beadWanted('edge test'), beadWanted('edge indirect'), beadWanted('edge test back'), beadWanted('edge indirect back')], [false, false, false, false, false]);
check('a role keeps them', beadWanted('edge'), true);
check('a plain box under a skin grows the web, a hanging test and no skin do not', [webWanted('coffin', false), webWanted('coffin', true), webWanted(null, false)], [true, false, false]);
ok('the beads sit in a layer of their own between the edges and the boxes, so no edge is wrapped',
  /root\.insertBefore\(layer, root\.children\[2\] \|\| null\)/.test(lin) && /edgeLayer\.appendChild\(path\)/.test(lin));
ok('select() lights the beads of the edge it lights, and a legend pick fades them with it',
  /querySelectorAll\('\.hw-bead'\)\.forEach\(\(b\) => b\.classList\.toggle\('hi'/.test(lin) && /beadEls\.forEach\(\(b, i\) => b\.classList\.toggle\('kept'/.test(lin)
  && /\.hw-bead\.hi \{ stroke: #ffb36b/.test(css) && /g\.picking \.hw-bead:not\(\.kept\)/.test(css));
ok('beads take no click and carry no dash meaning of their own', /body\.halloween \.hw-bead \{[^}]*pointer-events: none/.test(css) && !/\.hw-bead[^{]*\{[^}]*stroke-dasharray: [35]/.test(css));
ok('the web is a template in the page, cloned on the selected box, pointer-events none, still for the same box again',
  html.indexOf('id="hw-web-src"') >= 0 && /tpl\.cloneNode\(true\)/.test(lin) && /web\.setAttribute\('pointer-events', 'none'\)/.test(lin) && /webFor === id/.test(lin));
ok('it draws by stroke-dashoffset and is static under reduced motion',
  /@keyframes hw-draw \{ from \{[^}]*stroke-dashoffset: 1;[^}]*\} to \{[^}]*stroke-dashoffset: 0/.test(css) && /prefers-reduced-motion: reduce\) \{[\s\S]*\.hw-web \.spider \{ animation: none/.test(css));

print('\n--- what the timers and the saves do ---');
ok('the next apparition is set before the one due is drawn, which is guarded',
  /scheduleApparition\(kind\);\s*if \(document\.hidden\) return;\s*try \{ show\(\); \}/.test(src));
ok('the timers run only while the look and its apparitions are both wanted',
  /const want = on && S\.look\.apparitions !== false;/.test(src));
ok('a sound context is only made with the look and its sound on',
  /function hwAudio\(\) \{\s*if \(!S\.look\.halloween \|\| !S\.look\.sound\) return null;/.test(src));
ok('the saves go out one after the other', /S\.hw\.saving = S\.hw\.saving\.then\(/.test(src));
ok('the Theme section draws the look, the apparitions and the sound as switches',
  /\['halloween', 'Halloween look'[\s\S]*\['apparitions', 'Random apparitions'[\s\S]*\['sound', 'Sound'/.test(src)
  && /case 'theme': return paintSetTheme\(box\);/.test(src));
ok('a redrawn switch walks from where it was, in the box on screen and not in the probe',
  /function walkSwitch\(toggle, key, now, real\) \{\s*if \(!real\) return;/.test(src));
ok('and never touches aria-checked to do it', !/function walkSwitch[\s\S]*?\n\}\n/.exec(src)[0].match(/setAttribute/));
