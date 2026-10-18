// Custom selection's panel (0092): what the dock had is noted before a
// selection changes the mode, a column's lineage included, and given back as
// the panel rolls up, its card too; the card starts empty; a selection typed
// or asked for and not yet drawn is dropped with the panel.
// Run from the repository root: jsc web/tests/selpanel.js
var src = read('web/app.js');

function check(label, got, want) {
  var g = JSON.stringify(got), w = JSON.stringify(want);
  print((g === w ? 'PASS  ' : 'FAIL  ') + label + (g === w ? '' : '\n        expected ' + w + '\n        got      ' + g));
}

// Just enough of the page: elements in a tree, which the panel moves about.
var els = {};
function fakeEl(id, parent) {
  var cls = {};
  var el = {
    id: id, parentNode: null, childNodes: [], value: '', textContent: '', title: '',
    classList: {
      add: function (c) { cls[c] = true; }, remove: function (c) { delete cls[c]; },
      toggle: function (c, on) { if (on === undefined ? !cls[c] : on) cls[c] = true; else delete cls[c]; },
      contains: function (c) { return !!cls[c]; },
    },
    get nextSibling() {
      var at = this.parentNode ? this.parentNode.childNodes.indexOf(this) : -1;
      return at < 0 ? null : this.parentNode.childNodes[at + 1] || null;
    },
    appendChild: function (x) { return this.insertBefore(x, null); },
    insertBefore: function (x, next) {
      if (x.parentNode) x.parentNode.childNodes.splice(x.parentNode.childNodes.indexOf(x), 1);
      var at = next ? this.childNodes.indexOf(next) : -1;
      this.childNodes.splice(at < 0 ? this.childNodes.length : at, 0, x);
      x.parentNode = this;
      return x;
    },
    replaceChildren: function () {
      var self = this;
      this.childNodes.slice().forEach(function (c) { c.parentNode = null; });
      this.childNodes = [];
      [].slice.call(arguments).forEach(function (c) { self.appendChild(c); });
    },
    querySelector: function () { return hint; },
    setAttribute: function () {},
    focus: function () {},
  };
  if (id) els['#' + id] = el;
  if (parent) parent.appendChild(el);
  return el;
}
var document = { createElement: function () { return fakeEl(''); } };
function $(sel) { return els[sel]; }
var text = function (el) { return el.childNodes.map(function (c) { return c.textContent; }).join(''); };
var where = function (id) { return els['#' + id].parentNode.id; };

var dock = fakeEl('dock');
var dockLineage = fakeEl('dock-lineage', dock);
var empty = fakeEl('lineage-empty', dockLineage);
var hint = fakeEl('', empty);
['select-input', 'select-sum', 'lineage-status'].forEach(function (id) { fakeEl(id, dockLineage); });
var tools = fakeEl('lineage-tools', dock);
['with-tests', 'folder-tool', 'fit-btn', 'export-btn', 'image-btn'].forEach(function (id) { fakeEl(id, tools); });
var catalog = fakeEl('catalog', fakeEl('dock-catalog', dock));
var panel = fakeEl('select-panel');
['selpanel-canvas', 'selpanel-tools', 'selpanel-card'].forEach(function (id) { fakeEl(id, panel); });

var S, log, answers;
function reset(mode, extra) {
  S = { panel: '', graphMode: mode, colFocus: null, focus: null, node: null, dock: 'lineage',
    selectAsk: 0, colDraw: 0, selectRaise: null, selectHistory: [], selectSub: null };
  for (var k in extra || {}) S[k] = extra[k];
  log = [];
  answers = [];
  timers = {};
}
// The server's answer to each /api/select waits until the test gives it.
var api = {
  get: function (url) {
    log.push('ask ' + decodeURIComponent(url.split('q=')[1].split('&')[0]));
    return new Promise(function (resolve) { answers.push(function () { resolve({ nodes: [{}], edges: [], matched: 1, counts: { model: 1 } }); }); });
  },
};
var timers = {}, nextTimer = 0;
function setTimeout(f) { timers[++nextTimer] = f; return nextTimer; }
function clearTimeout(t) { delete timers[t]; }
function fire() { var ts = timers; timers = {}; for (var t in ts) ts[t](); }

var Lineage = {
  render: function () { log.push('drawn in ' + where('dock-lineage')); return null; },
  clear: function () { log.push('cleared'); }, fit: function () {},
};
function panelOpen(name) { return S.panel === name; }
function openPanel(name) { S.panel = name; }
// As closePanel does with PANELS.select: S.panel first, then its hook.
function closePanel() { if (S.panel !== 'select') return; S.panel = ''; selectPanelUp(); }
function focusColumn(id, column) { S.graphMode = 'column'; S.colFocus = { id: id, column: column }; log.push('column ' + id + '.' + column); }
function focusNode(id) { S.graphMode = 'model'; log.push('model ' + id); }
function closeHoverCard() {}
function paintMode() {}
function testsOn() { return false; }
function raiseFor() { return null; }
function paintSelectWarnings() {}
function selectWarnings(x) { return x; }
function selectCapNote() { return []; }
function selectNotes() { return []; }
function capOffer() { return null; }
function selectionText() { return ''; }
function selectSummary() { return ''; }
function selectEmptyText() { return ''; }
function canvasStatus() { return ''; }
function canvasOptions() { return {}; }
function paintLegend() {}
function paintTestsCount() {}
function rememberSelection() {}
var jel = function (tag, className, textContent) { var el = fakeEl(''); el.textContent = textContent; return el; };

// A `const` or a `let` an eval declares stays inside it: the slice's are made `var`s.
eval(src.slice(src.indexOf('let selectTimer = null;'), src.indexOf('/* Session history, and no further.'))
  .replace(/^(const|let) /gm, 'var '));

var card = function (words) { var p = fakeEl(''); p.textContent = words; catalog.replaceChildren(p); };
var steps = Promise.resolve();
function step(f) { steps = steps.then(f); }

step(function () {
  print('--- a column\'s lineage in the dock ---');
  reset('column', { colFocus: { id: 'model.jaffle_shop.orders', column: 'customer_id' }, focus: 'model.jaffle_shop.orders',
    node: { id: 'model.jaffle_shop.orders' } });
  card('orders');
  $('#select-input').value = 'stg_orders+';
  var run = openSelection();
  check('down: the canvas, its tools and the Catalog in the panel',
    ['dock-lineage', 'with-tests', 'image-btn', 'catalog'].map(where), ['selpanel-canvas', 'selpanel-tools', 'selpanel-tools', 'selpanel-card']);
  check('the card says how to fill it, not the model the dock had', [S.node, text(catalog)], [null, 'Click a box to read its card here.']);
  answers.shift()();
  return run;
});
step(function () {
  check('the selection drawn in the panel', log, ['ask stg_orders+', 'drawn in selpanel-canvas']);
  card('stg_orders');
  S.node = { id: 'model.jaffle_shop.stg_orders' };
  log = [];
  closePanel();
  check('up: all of it back where it was, in order',
    [dock.childNodes[0] === dockLineage, tools.childNodes.map(function (c) { return c.id; }), where('catalog')],
    [true, ['with-tests', 'folder-tool', 'fit-btn', 'export-btn', 'image-btn'], 'dock-catalog']);
  check('the column\'s lineage drawn again', log, ['column model.jaffle_shop.orders.customer_id']);
  check('and its card back, the box clicked forgotten', [S.node.id, text(catalog)], ['model.jaffle_shop.orders', 'orders']);
});

step(function () {
  print('\n--- no model in the dock ---');
  reset('model');
  card('No node selected.');
  $('#select-input').value = 'stg_customers';
  var run = runSelection();
  answers.shift()();
  return run;
});
step(function () {
  card('stg_customers');
  S.node = { id: 'model.jaffle_shop.stg_customers' };
  log = [];
  closePanel();
  check('up: the canvas empty, the Catalog back on nothing', [S.graphMode, log, S.node, text(catalog)],
    ['model', ['cleared'], null, 'No node selected.']);
});

step(function () {
  print('\n--- a box double-clicked ---');
  reset('model', { focus: 'model.jaffle_shop.orders' });
  card('orders');
  var run = runSelection();
  answers.shift()();
  return run;
});
step(function () {
  card('customers');
  S.node = { id: 'model.jaffle_shop.customers' };
  // As focusNode does: the mode first, which rolls the panel up.
  S.graphMode = 'model';
  log = [];
  closePanel();
  check('up: nothing drawn over it, and the card stays on the box', [log, S.node.id, text(catalog)],
    [[], 'model.jaffle_shop.customers', 'customers']);
});

step(function () {
  print('\n--- typed, and Escape before it is asked for ---');
  reset('model', { focus: 'model.jaffle_shop.orders' });
  var run = runSelection();
  answers.shift()();
  return run;
});
step(function () {
  $('#select-input').value = 'customers';
  selectTimer = setTimeout(runSelection, 500);
  log = [];
  closePanel();
  fire();
  check('the keystroke\'s run is dropped, and the panel stays up', [S.panel, log], ['', ['model model.jaffle_shop.orders']]);
});

step(function () {
  print('\n--- asked for, and Escape before the answer ---');
  reset('model', { focus: 'model.jaffle_shop.orders' });
  $('#select-input').value = 'customers';
  var run = runSelection();
  closePanel();
  log = [];
  answers.shift()();
  return run;
});
step(function () {
  check('the answer is not drawn on the dock\'s canvas', log, []);
});

// jsc says nothing about a rejected promise and exits 0, so a step that throws
// has to say so itself, or the checks before it would vouch for the file.
steps.then(function () {}, function (e) { print('FAIL  a step threw ' + e); });
