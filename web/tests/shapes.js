// One shape per meaning: a node's kind is a bar, a state a pictogram (check,
// triangle, cross, dashed ring, clock), an environment a lettered tag, a tool
// its glyph, and no round dot is left as a marker. The pictograms are masks
// over the colour that says which, so a rule that drops one brings the dot back.
// Run from the repository root: jsc web/tests/shapes.js
var css = read('web/app.css');
var app = read('web/app.js');
var lin = read('web/lineage.js');

function check(label, got, want) {
  var g = JSON.stringify(got), w = JSON.stringify(want);
  print((g === w ? 'PASS  ' : 'FAIL  ') + label + (g === w ? '' : '\n        expected ' + w + '\n        got      ' + g));
}

// The rules of a stylesheet, one by one as { sel, body }, a @media's inside
// them, comments out.
function rulesOf(text) {
  var plain = text.replace(/\/\*[\s\S]*?\*\//g, ''), found = [], at = 0;
  while (at < plain.length) {
    var open = plain.indexOf('{', at);
    if (open < 0) break;
    var depth = 1, end = open + 1;
    while (end < plain.length && depth) { depth += plain[end] === '{' ? 1 : plain[end] === '}' ? -1 : 0; end++; }
    var prelude = plain.slice(at, open).trim(), body = plain.slice(open + 1, end - 1);
    at = end;
    if (/^@(media|supports)/.test(prelude)) found = found.concat(rulesOf(body));
    else if (prelude[0] !== '@') found.push({ sel: prelude.replace(/\s+/g, ' '), body: body });
  }
  return found;
}
var rules = rulesOf(css);
var rootEnd = css.indexOf('\n}', css.indexOf(':root {'));
var root = css.slice(0, rootEnd);

// The rules whose selector, split at its commas, holds `sel` exactly.
function rulesFor(sel) {
  return rules.filter(function (r) { return r.sel.split(/\s*,\s*/).indexOf(sel) >= 0; });
}
function declares(sel, re) {
  return rulesFor(sel).some(function (r) { return re.test(r.body); });
}
// The --m-* a declaration block masks with: both the prefixed and the plain
// property must name the same one, else a browser of one kind loses the shape.
function shapeOf(body) {
  var pre = /(?:^|[\s;{])-webkit-mask(?:-image)?:\s*var\(--m-([\w-]+)\)/.exec(body);
  var plain = /(?:^|[\s;{])mask(?:-image)?:\s*var\(--m-([\w-]+)\)/.exec(body);
  return pre && plain && pre[1] === plain[1] ? pre[1] : '';
}
function masks(sel) { return rulesFor(sel).some(function (r) { return shapeOf(r.body) !== ''; }); }
function maskedAs(sel, shape) { return rulesFor(sel).some(function (r) { return shapeOf(r.body) === shape; }); }
check('shapeOf wants both properties, naming the same shape', [
  shapeOf('-webkit-mask: var(--m-ok) center; mask: var(--m-ok) center;'), shapeOf('mask: var(--m-ok) center;'),
  shapeOf('-webkit-mask-image: var(--m-ok); mask-image: var(--m-bad);'), shapeOf('-webkit-mask-image: var(--m-bad); mask-image: var(--m-bad);'),
], ['ok', '', '', 'bad']);

print('--- nothing is a round marker ---');
// A round shape stays where the shape is the control's, never a meaning.
var ROUND = {
  '.envmenu button.switch .knob': 'where it sits on its track is the state',
  '.keysbtn': 'a round button around a glyph',
  '.crumb.cfg': 'a round button around a glyph',
};
var round = rules.filter(function (r) { return /border-radius:\s*(50%|100%|\d{3,}px)/.test(r.body); });
check('every round rule of app.css is a control on the allowlist, each with its reason',
  round.filter(function (r) { return !ROUND[r.sel]; }).map(function (r) { return r.sel; }), []);
check('and each allowed one is still there to be round',
  Object.keys(ROUND).filter(function (sel) { return !round.some(function (r) { return r.sel === sel; }); }), []);
check('the export page\'s own CSS has no round corner', /border-radius:\s*50%/.test(app), false);
var circles = lin.split('\n').filter(function (l) { return /el\('circle'/.test(l); });
check('lineage.js draws one circle, the +N badge, a number', circles.length === 1 && /--more-bg/.test(circles[0]), true);

print('\n--- the shapes are defined once ---');
['ok', 'warn', 'bad', 'none', 'cancel', 'clock', 'cols', 'snow', 'newer', 'edit'].forEach(function (name) {
  var defs = css.match(new RegExp('--m-' + name + ':', 'g')) || [];
  var inRoot = root.match(new RegExp('--m-' + name + ': url\\("data:image/svg\\+xml,', 'g')) || [];
  check('--m-' + name + ' is one data:image/svg+xml, in the first :root', [defs.length, inRoot.length], [1, 1]);
});

print('\n--- every marker is drawn with a shape ---');
[
  '.freshbadge .dot', '.hovercard .freshdot', '.tooldot', '.qdot', '.jobdot', '.setwarn',
  '#settings-btn[data-warn=true]::after', '.cfgwarn-mark', '.brand.newer::after', '.venvpill::before',
  '.ftab.dirty .x::before', '.oe-row .state.editing::before',
].forEach(function (sel) {
  check(sel + ' masks a --m-* shape', masks(sel), true);
});
check('a kind\'s dot is a bar: 3px wide, corners of 2px',
  [declares('.dot', /width:\s*3px/), declares('.dot', /border-radius:\s*2px/)], [true, true]);
check('and so are its places in the card head and the Catalog title',
  [declares('.hc-head .dot', /width:\s*3px/), declares('.cat-title .dot', /width:\s*3px/)], [true, true]);
check('the legend gives a kind its bar too', declares('.legend i.kind', /width:\s*3px/), true);
check('the unsaved mark is the pencil, in the colour its button has: the tab\'s own, the row\'s amber',
  [maskedAs('.ftab.dirty .x::before', 'edit'), declares('.ftab.dirty .x::before', /background:\s*currentColor/), rulesFor('.ftab.dirty .x').some(function (r) { return /(^|[\s;])color:/.test(r.body); }),
    maskedAs('.oe-row .state.editing::before', 'edit'), declares('.oe-row .state.editing::before', /background:\s*currentColor/),
    declares('.oe-row.dirty .state', /color:\s*var\(--warn\)/), rulesFor('.oe-row .state.editing').length], [true, true, false, true, true, true, 0]);
check('and the pointer on it brings the cross back, the mask dropped', declares('.ftab.dirty .x:hover::before', /content:\s*"×"[\s\S]*mask:\s*none/), true);
check('the canvas\'s last-build mark is a path, drawn with strokes and one fill',
  [/\.bdot \{ fill: none; stroke-width: 2\.4;/.test(css), /\.bdot\.stale \{ fill: var\(--warn\); fill-rule: evenodd; stroke: none; \}/.test(css),
    /\.bdot\.none \{[^}]*stroke-dasharray/.test(css)], [true, true, true]);

print('\n--- every state the code produces has a shape ---');
// A tone with no mask rule of its own falls to the base rule's dashed ring,
// which is what none and unknown are.
// The freshness badge: ok, warn, bad, and the ring for none.
var fresh = app.match(/const tone = \{ ([^}]*) \}\[f\.state\] \|\| 'none';/);
var freshTones = fresh ? fresh[1].split(',').map(function (p) { return p.split(':')[1].trim().replace(/'/g, ''); }) : [];
check('the badge\'s states are read out of freshnessBadge', freshTones, ['ok', 'warn', 'bad', 'none']);
var SHAPE = { ok: 'ok', warn: 'warn', bad: 'bad' };
freshTones.forEach(function (t) {
  check('the badge and its card: ' + t, [
    t in SHAPE ? maskedAs('.freshbadge[data-tone="' + t + '"] .dot', SHAPE[t]) : maskedAs('.freshbadge .dot', 'none'),
    t in SHAPE ? maskedAs('.hovercard .freshdot[data-tone="' + t + '"]', SHAPE[t]) : maskedAs('.hovercard .freshdot', 'none'),
  ], [true, true]);
});
// A query: the ring is the base, the others have their own.
var qs = app.match(/function queryState\(row\) \{[\s\S]*?\n\}/)[0];
var queryStates = {};
(qs.match(/return [^;]*;/g) || []).forEach(function (r) { (r.match(/'[a-z]+'/g) || []).forEach(function (q) { queryStates[q.replace(/'/g, '')] = 1; }); });
var QSHAPE = { ok: 'ok', failed: 'bad', cancelled: 'cancel', running: 'clock', queued: 'clock' };
check('the history states are read out of queryState', Object.keys(queryStates).sort(), ['cancelled', 'failed', 'ok', 'queued', 'running', 'unknown']);
Object.keys(queryStates).forEach(function (q) {
  var sel = '[data-state=' + q + '] .qdot', own = rules.filter(function (r) { return r.sel.split(/\s*,\s*/).indexOf(sel) >= 0 && /mask-image/.test(r.body); });
  check('a query ' + q + (q in QSHAPE ? ' has its shape' : ' keeps the ring'), own.length ? shapeOf(own[0].body) : '', QSHAPE[q] || '');
});
check('a cancelled query is the slashed circle in the dim colour, a failed one the red cross',
  [declares('.qdot', /background:\s*var\(--fg-dim\)/), rulesFor('[data-state=cancelled] .qdot').some(function (r) { return /background:/.test(r.body); }),
    declares('[data-state=failed] .qdot', /background:\s*var\(--bad\)/)], [true, false, true]);
// A job's models: error, late, none, ok.
var jv = app.match(/function jobBuildVerdicts[\s\S]*?const rank = \{ ([^}]*) \};/);
var jobTones = jv ? jv[1].split(',').map(function (p) { return p.split(':')[0].trim(); }) : [];
check('the job cell\'s verdicts are read out of jobBuildVerdicts', jobTones, ['error', 'late', 'none', 'ok']);
var JSHAPE = { error: 'bad', late: 'warn', none: 'none' };
jobTones.forEach(function (t) {
  check('a job cell ' + t, t === 'ok' ? maskedAs('.jobdot', 'ok') : maskedAs('.jbcell[data-verdict=' + t + '] .jobdot', JSHAPE[t]), true);
});
// The venv: on, off, and none with the ring.
var venv = app.match(/function venvStatus\(v, reported\) \{[\s\S]*?\n\}/)[0];
var venvStates = [];
(venv.match(/state: [^,}]*/g) || []).forEach(function (m) { (m.match(/'[a-z]+'/g) || []).forEach(function (s) { s = s.replace(/'/g, ''); if (venvStates.indexOf(s) < 0) venvStates.push(s); }); });
check('the venv pill\'s states are read out of venvStatus', venvStates.sort(), ['none', 'off', 'on']);
check('on is the check, off the triangle, none the ring the pill starts from',
  [maskedAs('.venvpill[data-state=on]::before', 'ok'), maskedAs('.venvpill[data-state=off]::before', 'warn'),
    maskedAs('.venvpill::before', 'none'), rulesFor('.venvpill[data-state=none]::before').length], [true, true, true, 0]);
// A tool's last run: ok, warn, bad, over its glyph.
var runEnd = app.match(/function runEndTone\(c\) \{[\s\S]*?\n\}/)[0];
var runTones = [];
(runEnd.match(/'[a-z]*'/g) || []).forEach(function (t) { t = t.replace(/'/g, ''); if (runTones.indexOf(t) < 0) runTones.push(t); });
check('the tones of a run\'s end are read out of runEndTone', runTones.sort(), ['', 'bad', 'ok', 'warn']);
runTones.forEach(function (t) {
  if (t === '') {
    check('no outcome draws no mark: runEndWords has none for it, and the rail and the head draw the dot only on words',
      [/function runEndWords\(tone\) \{[\s\S]*?\|\| '';/.test(app),
        /const words = runEndWords\(sec\.tone\);\s*\n\s*if \(words\) \{\s*\n\s*const dot = jel\('span', 'tooldot'\)/.test(app),
        /const words = runEndWords\(tone\);\s*\n\s*if \(words\) \{\s*\n\s*const dot = jel\('span', 'tooldot'\)/.test(app)], [true, true, true]);
  } else {
    check('a tool\'s glyph takes ' + t + ' as a ' + SHAPE[t], maskedAs('.tooldot[data-tone=' + t + ']', SHAPE[t]), true);
  }
});
check('a tool is its glyph: collin\'s columns, Snowflake\'s flake, the jobs\' clock',
  [declares('[data-tool=collin]', /--m-tool:\s*var\(--m-cols\)/), declares('[data-tool=snowflake]', /--m-tool:\s*var\(--m-snow\)/),
    declares('[data-tool=jobs]', /--m-tool:\s*var\(--m-clock\)/)], [true, true, true]);
check('a tool with none of its own, Fusion or an other cache, has the columns glyph in its colour, not a blank or the ring',
  [declares('[data-tool]', /--m-tool:\s*var\(--m-cols\)/), rulesFor('[data-tool=fusion]').every(function (r) { return !/--m-tool/.test(r.body); }),
    declares('[data-tool=fusion]', /--tool:\s*var\(--tool-dbt\)/)], [true, true, true]);
check('the empty tool, none picked, is the ring wherever its glyph is drawn',
  ['.cllpick[data-tool=""] .tooldot', '.envmenu [data-tool=""] .tooldot', '.seticon[data-tool=""] .tooldot'].map(function (sel) {
    return rulesFor(sel).some(function (r) { return shapeOf(r.body) === 'none' && /background:\s*var\(--none\)/.test(r.body); });
  }), [true, true, true]);
// A build on the canvas: every verdict buildsWorst gives has a path.
var verdicts = [];
(app.match(/verdict: (?:'[a-z]+'|old \? 'stale' : 'ok')/g) || []).forEach(function (m) { (m.match(/'[a-z]+'/g) || []).forEach(function (v) { v = v.replace(/'/g, ''); if (verdicts.indexOf(v) < 0) verdicts.push(v); }); });
var marks = (lin.match(/const BUILD_MARK = \{([\s\S]*?)\n  \};/) || ['', ''])[1];
check('a build\'s verdicts are read out of the code', verdicts.sort(), ['error', 'none', 'ok', 'stale']);
check('each has a path in BUILD_MARK and a rule for its look', verdicts.map(function (v) {
  return new RegExp('\\n\\s+' + v + ': \'M').test(marks) && rulesFor('.bdot.' + v).length === 1;
}), [true, true, true, true]);

print('\n--- an environment is a tag, a role a square ---');
check('no envdot is left in app.js or app.css', [/envdot/.test(app), /envdot/.test(css)], [false, false]);
check('a tag reads its colour from --tone', declares('.envtag', /color:\s*var\(--tone\)/), true);
check('and the manifest\'s is outlined, not filled', declares('.envtag[data-tone=manifest]', /background:\s*transparent;[^}]*box-shadow:\s*inset 0 0 0 1px var\(--env-manifest\)/), true);
check('the column card\'s role marker is the square, not a kind\'s bar', /className: 'rolesq' \}\);\s*\n\s*tone\.style\.background = Lineage\.roleColor/.test(app), true);

print('\n--- the Halloween look leaves the markers alone ---');
check('no selector of the look names a marker class', rules.filter(function (r) {
  return /halloween|(^|[^\w-])[.#]?hw-/.test(r.sel) && /(\.|#)(envtag|freshbadge|qdot|jobdot|tooldot|bdot|setwarn|cfgwarn-mark|venvpill|rolesq)\b/.test(r.sel);
}).map(function (r) { return r.sel; }), []);
