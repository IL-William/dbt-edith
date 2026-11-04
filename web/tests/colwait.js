// A column asked for while collin runs (0042): the canvas is cleared and says
// what it waits for under a turning ring, the run's end draws the column asked
// for last and openColumn draws nothing itself, a failed run, or a draw that
// fails right after the wait, gives the model back and takes the ring down.
// Run from the repository root: jsc web/tests/colwait.js
var src = read('web/app.js');

function check(label, got, want) {
  var g = JSON.stringify(got), w = JSON.stringify(want);
  print((g === w ? 'PASS  ' : 'FAIL  ') + label + (g === w ? '' : '\n        expected ' + w + '\n        got      ' + g));
}

// Just enough of the page: the elements the wait touches, and an SVG made of
// plain nodes, found by their class.
function node(tag, parent) {
  var n = {
    tag: tag, attrs: {}, children: [], textContent: '', parent: null,
    setAttribute: function (k, v) { this.attrs[k] = v; },
    appendChild: function (c) { c.parent = this; this.children.push(c); return c; },
    remove: function () { var at = this.parent.children.indexOf(this); this.parent.children.splice(at, 1); this.parent = null; },
  };
  return n;
}
function byClass(root, cls) {
  var out = [];
  (function walk(n) {
    if ((n.attrs.class || '').split(' ').indexOf(cls) >= 0) out.push(n);
    n.children.forEach(walk);
  })(root);
  return out;
}
var document = { createElementNS: function (ns, tag) { return node(tag); } };
var els = {};
function fakeEl(sel) {
  var cls = { hidden: true };
  var el = {
    textContent: '', p: { textContent: '' }, children: [],
    classList: {
      add: function (c) { cls[c] = true; }, remove: function (c) { delete cls[c]; },
      toggle: function (c, on) { if (on) cls[c] = true; else delete cls[c]; },
      contains: function (c) { return !!cls[c]; },
    },
    querySelector: function (q) {
      if (q === 'p') return this.p;
      var found = this.children.filter(function (c) { return (c.attrs.class || '') === q.slice(1); });
      return found[0] || null;
    },
    insertBefore: function (c) { c.parent = this; this.children.unshift(c); return c; },
  };
  els[sel] = el;
  return el;
}
['#lineage-empty', '#lineage-status', '#lineage-legend'].forEach(fakeEl);
els['#up'] = { value: '1' }; els['#down'] = { value: '1' };
function $(sel) { return els[sel]; }
var hintShown = function () { return !$('#lineage-empty').classList.contains('hidden'); };
var ringOn = function () { return $('#lineage-empty').classList.contains('waiting'); };
var hintText = function () { return $('#lineage-empty').p.textContent; };
var drawing = function () { return $('#lineage-empty').children[0] || null; };
var centreName = function () { var t = drawing() && byClass(drawing(), 'wait-name')[0]; return t ? t.textContent : null; };

var S, log, asks, finishRun, stopAtMenu = null;
function reset() {
  S = { graphMode: 'model', colFocus: null, colSub: null, colDraw: 0, colWait: false, colCut: new Set(),
    colIndirect: false, colRaise: null, cllTool: 'collin', focus: 'model.shop.orders',
    collin: { found: true, stale: true, state: 'done' } };
  log = []; asks = [];
  $('#lineage-empty').children = [];
  $('#lineage-empty').classList.add('hidden');
  $('#lineage-empty').classList.remove('waiting');
  $('#lineage-status').textContent = '';
  $('#lineage-legend').textContent = 'legend';
}
var api = {
  get: function (url) {
    return new Promise(function (resolve, reject) { asks.push({ url: url, resolve: resolve, reject: reject }); });
  },
};
var Lineage = {
  clear: function () { log.push('cleared'); },
  render: function (sub) { log.push('drawn ' + sub.column); return null; },
};
function paintMode() {}
function paintColCap() {}
function paintLegend() {}
function paintTestsCount() {}
function closeStopAt() {}
function showDock(which) { log.push('dock ' + which); }
function toast(text) { log.push('toast ' + text); }
function canvasStatus(sub) { return 'status ' + sub.column; }
function canvasOptions() { return {}; }
function colCapOffer() { return null; }
function sidecarOn() { return false; }
function runSelection() {}
// As the real one begins: the mode, and the wait left.
function focusNode(id) { leaveColumnWait(); S.graphMode = 'model'; S.colFocus = null; log.push('model ' + id); }
// As the real one: the state is 'running' before it yields, its end either
// redraws what was asked for last (redrawLineage), or gives the wait up.
function regenerateCollin() {
  S.collin = { found: true, stale: true, state: 'running' };
  return new Promise(function (resolve) {
    finishRun = function (ok) {
      if (ok) { S.collin = { found: true, stale: false, state: 'done' }; rerender(); }
      else { S.collin = null; giveUpColumnWait(); }
      resolve(ok);
    };
  });
}

// A `const` an eval declares stays inside it: the slice's are made `var`s.
function lift(from, to) { return src.slice(src.indexOf(from), src.indexOf(to)).replace(/^const /gm, 'var '); }
(0, eval)(lift('function collinRunning', "/* What a request for a column's canvas is"));
(0, eval)(lift('function colAskKey', '/* The button beside a column canvas cut at the cap'));
(0, eval)(lift('const emptyHint', '/* Session history, and no further.'));
(0, eval)(lift('/* A run that failed leaves nothing', "/* Paints what collin's state shows in"));
(0, eval)(lift('const rerender', '/* How the canvas draws what it is sent'));
(0, eval)(lift('async function openColumn', '  if (!sidecarOn()) return focusColumn') + '  if (!sidecarOn()) return focusColumn(n.id, column);\n}');

// The server's answer to the request asks[i], for the column it names.
function answer(i) {
  var column = decodeURIComponent(asks[i].url.split('column=')[1].split('&')[0]);
  asks[i].resolve({ column: column, nodes: [], edges: [], edge_kinds: [] });
}
// A few turns of the microtask queue, for what an answer sets going.
function settle() { var p = Promise.resolve(); for (var i = 0; i < 8; i++) p = p.then(function () {}); return p; }
var steps = Promise.resolve();
function step(f) { steps = steps.then(f); }
var ORDERS = { id: 'model.shop.orders' };

step(function () {
  print('--- the wait ---');
  reset();
  S.collin.state = 'running';
  var done = focusColumn(ORDERS.id, 'amount');
  check('nothing is asked of the server', asks.length, 0);
  check('the canvas is cleared, the legend emptied', [log, $('#lineage-legend').textContent], [['cleared', 'dock lineage'], '']);
  check('the hint names the column under a ring', [hintShown(), ringOn(), hintText()],
    [true, true, "collin is running over the compiled SQL: amount's lineage is drawn once it ends"]);
  check('the drawing is there, with the column in its centre box', [!!drawing(), drawing() && drawing().tag, centreName(),
    drawing() && byClass(drawing(), 'wait-box').length, drawing() && byClass(drawing(), 'wait-grow').length], [true, 'svg', 'amount', 4, 4]);
  check('the status line does not say it twice', $('#lineage-status').textContent, '');
  check('the column is the focus, and the wait is on', [S.colFocus.column, S.colWait], ['amount', true]);

  S.cllTool = 'snowflake';
  asks = []; log = [];
  focusColumn(ORDERS.id, 'amount');
  check('another tool does not wait', asks.length, 1);
  return done;
});

step(function () {
  print('\n--- a column clicked on a stale cache ---');
  reset();
  var click = openColumn(ORDERS, 'amount');
  check('the wait is up at once, with no request yet', [S.colWait, asks.length, ringOn()], [true, 0, true]);
  finishRun(true);
  return click;
});
step(function () {
  check('the run\'s end asks for the column, openColumn did not', [asks.length, asks[0].url.indexOf('column=amount') > 0], [1, true]);
  answer(0);
  return settle();
});
step(function () {
  check('and draws it once, taking the ring down', [log.filter(function (l) { return /^drawn/.test(l); }), hintShown(), S.colWait],
    [['drawn amount'], false, false]);
});

step(function () {
  print('\n--- another column clicked during the run ---');
  reset();
  var first = openColumn(ORDERS, 'amount');
  openColumn(ORDERS, 'customer_id');
  check('the second waits too, naming itself', [S.colFocus.column, hintText().indexOf('customer_id') > 0, asks.length],
    ['customer_id', true, 0]);
  finishRun(true);
  return first;
});
step(function () {
  check('only the column clicked last is asked for', asks.map(function (a) { return a.url.split('column=')[1].split('&')[0]; }), ['customer_id']);
  answer(0);
  return settle();
});
step(function () {
  check('and drawn', [log.filter(function (l) { return /^drawn/.test(l); }), S.colFocus.column], [['drawn customer_id'], 'customer_id']);
});

step(function () {
  print('\n--- the run fails ---');
  reset();
  var click = openColumn(ORDERS, 'amount');
  finishRun(false);
  return click;
});
step(function () {
  check('the model is drawn again and the ring is down', [log.filter(function (l) { return /^model/.test(l); }), hintShown(), S.colWait],
    [['model model.shop.orders'], false, false]);
});

step(function () {
  print('\n--- the draw after the wait fails ---');
  reset();
  var click = openColumn(ORDERS, 'ghost');
  finishRun(true);
  return click;
});
step(function () {
  asks[0].reject(new Error('unknown column'));
  return settle();
});
step(function () {
  check('the ring and the hint are down, the model back', [hintShown(), ringOn(), $('#lineage-status').textContent, S.graphMode, S.colWait],
    [false, false, '', 'model', false]);
  check('the error is said, and the model drawn', log.filter(function (l) { return /^(toast|model)/.test(l); }),
    ['toast column lineage: unknown column', 'model model.shop.orders']);
});
step(function () {
  print('\n--- the drawing goes with the wait ---');
  reset();
  S.collin.state = 'running';
  focusColumn(ORDERS.id, 'amount');
  S.colWait = true;
  leaveColumnWait();
  check('leaving the wait takes it away', [drawing(), hintShown()], [null, false]);
  focusColumn(ORDERS.id, 'amount');
  emptyHint('Select a model to see its lineage.');
  check('a hint that is not a wait carries none', [drawing(), ringOn(), hintText()], [null, false, 'Select a model to see its lineage.']);
  focusColumn(ORDERS.id, 'amount');
  focusColumn(ORDERS.id, 'customer_id');
  check('a second wait replaces the drawing', [$('#lineage-empty').children.length, centreName()], [1, 'customer_id']);
});

step(function () {
  print('\n--- where the boxes go ---');
  var short = waitArmLayout('amount');
  check('a short name keeps the minimum', [short.label, short.cw, short.cx0, short.cx1, short.ghostR, short.width, short.height],
    ['amount', 58, 111, 169, 214, 280, 100]);
  var mid = waitArmLayout('a_rather_long_column');
  check('a longer one widens the box and moves the right side', [mid.cw > 58, mid.cx1 - mid.cx0 === mid.cw, mid.ghostR - mid.cx1, mid.width - mid.ghostR],
    [true, true, 45, 66]);
  var long = waitArmLayout('a_column_name_much_longer_than_the_box_can_hold');
  check('a long one is capped and ends in an ellipsis', [long.cw <= 150, long.label.length <= 25, long.label.slice(-1)],
    [true, true, '\u2026']);
  check('the name that just fits is not cut', waitArmLayout('abcdefghijklmnopqrstuvwxy').label, 'abcdefghijklmnopqrstuvwxy');
  check('the right side follows the centre box', [long.ghostR, long.width], [long.cx1 + 45, long.ghostR + 58 + 8]);
});

// A step that throws would otherwise end the file silently, and check.sh would
// count the PASS lines before it.
steps.then(function () { print('\ndone'); }, function (e) { print('FAIL  a step threw ' + e); });
