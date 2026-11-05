// Every colour of the page is a variable: a theme is only a set of variables,
// so a literal in a rule or in the canvas's drawing is a colour no theme can
// reach. app.css names its colours in the blocks at its top (Night's, the rule
// the other themes share, and one block each for Graphite, Light and
// Halloween) and nowhere below them, and lineage.js names variables
// (`var(--k-view)`), never a value. The one exemption is the Halloween theme's
// section: the literals there are the art's drawing colours (stone, gold, wood,
// the rings, the silk), like index.html's gradients. A box style is a picture,
// not a set of colours a theme re-points (0106, 0150).
// Run from the repository root: jsc web/tests/colourvars.js
var css = read('web/app.css');
var lin = read('web/lineage.js');

function check(label, got, want) {
  var g = JSON.stringify(got), w = JSON.stringify(want);
  print((g === w ? 'PASS  ' : 'FAIL  ') + label + (g === w ? '' : '\n        expected ' + w + '\n        got      ' + g));
}

// A colour that is a value and not a variable: a hex of 3 to 8 digits, a
// colour function opened with a literal (`rgba(var(--accent-rgb), .13)` opens
// with a variable and stays), and any CSS named colour. Only in a declaration's
// value: never in a property or custom property name, a selector, an animation
// or font name, a url(...) or a string. `transparent`, `currentColor`,
// `inherit`, `initial`, `unset` and `none` are not colours of a theme.
var NAMED = ('aliceblue antiquewhite aqua aquamarine azure beige bisque black blanchedalmond blue blueviolet brown '
  + 'burlywood cadetblue chartreuse chocolate coral cornflowerblue cornsilk crimson cyan darkblue darkcyan darkgoldenrod '
  + 'darkgray darkgreen darkgrey darkkhaki darkmagenta darkolivegreen darkorange darkorchid darkred darksalmon '
  + 'darkseagreen darkslateblue darkslategray darkslategrey darkturquoise darkviolet deeppink deepskyblue dimgray '
  + 'dimgrey dodgerblue firebrick floralwhite forestgreen fuchsia gainsboro ghostwhite gold goldenrod gray green '
  + 'greenyellow grey honeydew hotpink indianred indigo ivory khaki lavender lavenderblush lawngreen lemonchiffon '
  + 'lightblue lightcoral lightcyan lightgoldenrodyellow lightgray lightgreen lightgrey lightpink lightsalmon '
  + 'lightseagreen lightskyblue lightslategray lightslategrey lightsteelblue lightyellow lime limegreen linen magenta '
  + 'maroon mediumaquamarine mediumblue mediumorchid mediumpurple mediumseagreen mediumslateblue mediumspringgreen '
  + 'mediumturquoise mediumvioletred midnightblue mintcream mistyrose moccasin navajowhite navy oldlace olive olivedrab '
  + 'orange orangered orchid palegoldenrod palegreen paleturquoise palevioletred papayawhip peachpuff peru pink plum '
  + 'powderblue purple rebeccapurple red rosybrown royalblue saddlebrown salmon sandybrown seagreen seashell sienna '
  + 'silver skyblue slateblue slategray slategrey snow springgreen steelblue tan teal thistle tomato turquoise violet '
  + 'wheat white whitesmoke yellow yellowgreen').split(' ');
var isNamed = {};
NAMED.forEach(function (n) { isNamed[n] = true; });
// Properties whose values are names of the author's, not colours.
var NOT_COLOUR = /^(?:animation|animation-name|font|font-family|transition|transition-property|will-change|content|grid-area|grid-template-areas|counter-reset|counter-increment|container-name|view-transition-name|anchor-name)$/;

function valueLiterals(value) {
  var found = [], m;
  value = value.replace(/url\([^)]*\)/g, '').replace(/(['"])(?:\\.|(?!\1).)*\1/g, '');
  var hex = /#[0-9a-fA-F]{3,8}(?![\w-])/g;
  while ((m = hex.exec(value))) found.push(m[0]);
  var fn = /(^|[^\w-])(?:rgba?|hsla?|hwb|lab|lch|oklab|oklch|color)\(\s*(?!var\(|from\b)/g;
  while ((m = fn.exec(value))) found.push(value.slice(m.index + m[1].length, m.index + m[1].length + 12));
  value.split(/[\s,()\/]+/).forEach(function (word) { if (isNamed[word.toLowerCase()]) found.push(word); });
  return found;
}
function declarationLiterals(prop, value) {
  return NOT_COLOUR.test(prop.trim().toLowerCase()) ? [] : valueLiterals(value);
}
// CSS text: every `prop: value` between braces.
function literals(text) {
  var found = [];
  (text.replace(/\/\*[\s\S]*?\*\//g, '').match(/\{[^{}]*\}/g) || []).forEach(function (body) {
    body.slice(1, -1).replace(/url\([^)]*\)/g, 'url()').split(';').forEach(function (d) {
      var at = d.indexOf(':');
      if (at > 0) found = found.concat(declarationLiterals(d.slice(0, at), d.slice(at + 1)));
    });
  });
  return found;
}
// JavaScript text: what is in a string, as a whole or as `fill:x;stroke:y`.
function jsLiterals(code) {
  var found = [], m, str = /(['"`])((?:\\.|(?!\1)[^\\])*)\1/g;
  while ((m = str.exec(code))) {
    var body = m[2];
    if (isNamed[body.trim().toLowerCase()]) found.push(body.trim());
    else if (body.indexOf(':') < 0) found = found.concat(valueLiterals(body).filter(function (v) { return !isNamed[v.toLowerCase()]; }));
    else found = found.concat(literals('{' + body + '}'));
  }
  return found;
}

// What the matcher itself calls a literal, so a pass means something.
check('it catches a hex of every length', literals('a { color: #abc; b: #abcd; c: #aabbcc; d: #aabbccdd; }').length, 4);
check('and rgb, rgba and hsl opened with a number', literals('a { b: rgb(1, 2, 3); c: rgba(0,0,0,.5); d: hsl(1 2% 3%); e: rgba( 4, 5, 6, 1); }').length, 4);
check('and the newer functions', literals('a { fill: oklch(70% .1 200); b: hwb(1 2% 3%); c: lab(50% 1 1); d: lch(1 2 3); e: oklab(1 2 3); f: color(srgb 1 0 0); }').length, 6);
check('and the names white and black as a value', literals('a { color: white; b: 1px solid black; }'), ['white', 'black']);
check('and every other named colour, whatever its case', literals('a { color: red; border-color: Gray; b: 1px solid rebeccapurple; c: color-mix(in srgb, tomato 40%, transparent); }'),
  ['red', 'Gray', 'rebeccapurple', 'tomato']);
check('but not a channel variable, white-space, an id, a mask or a variable', literals(
  'a { b: rgba(var(--accent-rgb), .13); white-space: nowrap; c: url(#add); d: var(--white); e: color-mix(in srgb, var(--x) 40%, transparent); }'), []);
check('nor a keyword that is not a colour, a name of an animation or a font, or a property', literals(
  'a { color: transparent; b: currentColor; c: inherit; d: initial; e: unset; f: none; animation: red-pulse 1s; animation-name: red; '
  + 'font-family: Tan, "Red Hat Text", gray-sans; g: var(--red); --tone: var(--env-dev); red: 1; content: "red"; h: url(data:image/png;base64,red) }'), []);
check('nor the ids of a selector, which sit outside a declaration', literals(declarations('#add, #red, #face { margin: 0; }')), []);

// What sits between braces, which is where a colour can be: a selector holds
// ids that spell a hex.
function declarations(text) {
  return (text.replace(/\/\*[\s\S]*?\*\//g, '').match(/\{[^{}]*\}/g) || []).join('\n');
}

// The rules of a stylesheet's tail, one by one, with the Halloween theme's left
// out: a selector that names `halloween` or `hw-`, and the steps of a
// `@keyframes hw-*`. Everything else is searched, so a rule appended after the
// look's section is held to the same rule as the rest of the file.
function outsideTheLook(text) {
  var plain = text.replace(/\/\*[\s\S]*?\*\//g, ''), found = [], at = 0;
  while (at < plain.length) {
    var open = plain.indexOf('{', at);
    if (open < 0) break;
    var depth = 1, end = open + 1;
    while (end < plain.length && depth) { depth += plain[end] === '{' ? 1 : plain[end] === '}' ? -1 : 0; end++; }
    var prelude = plain.slice(at, open).trim(), body = plain.slice(open + 1, end - 1);
    at = end;
    if (/^@keyframes\s+hw-/.test(prelude) || (prelude[0] !== '@' && /halloween|(^|[^\w-])[.#]?hw-/.test(prelude))) continue;
    // A @media or @supports holds rules; a rule and a @keyframes hold declarations.
    found = found.concat(/^@(media|supports)/.test(prelude) ? outsideTheLook(body) : literals('{' + body + '}'));
  }
  return found;
}
check('the theme\'s rules are left out: its selectors and its keyframes', outsideTheLook(
  'html[data-theme="halloween"] .x { color: #abc; } .hw-art { fill: white; } @keyframes hw-glow { 0% { color: #fff; } }'), []);
check('a rule appended after them is not', outsideTheLook(
  'html[data-theme="halloween"] .x { color: #abc; } .appended-later { color: #abcdef; } .branch-hw-note { fill: #123; } @keyframes pulse { 0% { color: white; } }'),
  ['#abcdef', '#123', 'white']);
check('nor one inside a @media', outsideTheLook('@media (min-width: 1px) { .y { background: rgba(0, 0, 0, .5); } }').length, 1);

print('\n--- app.css outside the theme blocks ---');
// The variable blocks are the leading rules whose selectors are only :root or
// [data-theme=...]: where they end is where the rules begin.
function variableBlocksEnd(text) {
  var at = 0, end = 0, blocks = 0;
  for (;;) {
    var rest = text.slice(at).replace(/^(?:\s|\/\*[\s\S]*?\*\/)*/, '');
    at = text.length - rest.length;
    var open = text.indexOf('{', at);
    if (open < 0) break;
    var prelude = text.slice(at, open).trim();
    if (!/^(?::root|\[data-theme="[a-z]+"\])(?:\s*,\s*(?::root|\[data-theme="[a-z]+"\]))*$/.test(prelude)) break;
    var close = text.indexOf('}', open);
    end = close + 1; at = end; blocks++;
  }
  return { end: end, blocks: blocks };
}
var vb = variableBlocksEnd(css);
check('the file opens with five variable blocks: Night, the derived rule, Graphite, Light, Halloween', vb.blocks, 5);
// The Halloween theme's art has its own colours: its rules, from its header
// on, are left out, and nothing else is. Before the header there is no Halloween
// rule, and after it a rule that is not one is checked too.
var mark = css.indexOf('halloween look --');
check('the Halloween section has its header, which is where the exemption starts', mark > 0, true);
var head = css.slice(vb.end, mark);
check('nothing between the blocks and it is a Halloween rule', /halloween/.test(head.replace(/\/\*[\s\S]*?\*\//g, '')), false);
var tail = outsideTheLook(css.slice(mark));
check('and none of what follows it that is not the theme has a literal: ' + tail.length, tail.slice(0, 8), []);
var rules = declarations(head);
var left = literals(rules);
check('no colour literal (hex, a colour function, a named colour) is left in a rule: ' + left.length, left.slice(0, 8), []);
check('the translucent copies of the accent still read its channels', /rgba\(var\(--accent-rgb\), \.\d+\)/.test(rules), true);
var night = css.slice(0, css.indexOf('\n}', 0));
check('every colour of Night is there to be re-pointed', (night.match(/--[\w-]+:\s*(#|rgba?\()/g) || []).length > 150, true);

print('\n--- lineage.js ---');
// Comments say what a colour is and may quote a value; the drawing may not.
var code = lin.replace(/\/\*[\s\S]*?\*\//g, '').replace(/(^|[^:'"`\\])\/\/.*$/gm, '$1');
var drawn = jsLiterals(code);
check('no colour value is left in the drawing: ' + drawn.length, drawn.slice(0, 8), []);
check('its constants name variables', /view: 'var\(--k-view\)'/.test(lin) && /EDGE_PLAIN = 'var\(--edge\)'/.test(lin)
  && /TEST = \{ model: 'var\(--t-model\)'/.test(lin) && /const CUSTOM = 'var\(--k-custom\)'/.test(lin), true);
// An SVG presentation attribute takes a colour, not var(): `{ fill: x }` as a
// key of el() sets an attribute, where `style: 'fill:x'` sets a property.
function attributeColours(text) { return text.match(/[{,]\s*(?:fill|stroke|'stop-color'):/g) || []; }
check('an el() key of fill, stroke or stop-color is found, quoted or not',
  attributeColours("el('rect', { fill: nodeColor(n) }); el('c', { r: 2, stroke: '#fff' }); el('c', { style: 'fill:var(--x)' })").length, 2);
check('and none is in the drawing: they go in the style', attributeColours(code), []);
// The same through setAttribute: a literal colour as the second argument fails,
// a var() string or a variable does not.
function attributeLiterals(text) {
  var found = [], m, call = /setAttribute\(\s*['"](?:fill|stroke|stop-color)['"]\s*,\s*((['"`])(?:\\.|(?!\2)[^\\])*\2|[^,)]*)/g;
  while ((m = call.exec(text))) found = found.concat(jsLiterals(m[1]));
  return found;
}
check('a setAttribute of fill, stroke or stop-color with a colour in it is found',
  attributeLiterals("a.setAttribute('fill', '#fff'); b.setAttribute('stroke', 'red'); c.setAttribute('stop-color', `rgb(1, 2, 3)`); d.setAttribute('fill', 'white')").length, 4);
check('but not a var() string, a variable, none, or another attribute',
  attributeLiterals("a.setAttribute('fill', 'var(--x)'); b.setAttribute('stroke', colour); c.setAttribute('fill', 'none'); d.setAttribute('x', '#fff')"), []);
check('and none in the drawing sets one', attributeLiterals(code), []);
