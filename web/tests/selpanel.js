// Custom selection's panel (0092): what the dock had is noted before a
// selection changes the mode, a column's lineage included, and given back as
// the panel rolls up, its card too; the card starts closed, opens on a click,
// at once or once a double-click is past when it would move the box, and
// stays with its box while the box is drawn, asks for its jobs once shown,
// gives the focus inside it to the panel and its edge stops at its bounds
// (0101); a selection typed or asked for and not yet drawn is dropped with
// the panel.
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
    setAttribute: function (k, v) { this[k] = v; },
    focus: function () { document.activeElement = this; },
    contains: function (x) { for (var at = x; at; at = at.parentNode) if (at === this) return true; return false; },
  };
  if (id) els['#' + id] = el;
  if (parent) parent.appendChild(el);
  return el;
}
var document = { createElement: function () { return fakeEl(''); }, activeElement: null };
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
var cardEl = $('#selpanel-card');
// The Catalog's jobs box, which the card asks to fill once it is seen.
var jobsBox = fakeEl('');
els['#catalog .cat-jobs'] = jobsBox;
function fillCatalogJobs(box) { if (box) log.push('jobs asked'); }
// The two buttons that open the panel.
fakeEl('select-btn');
els['#graph-mode [data-mode=select]'] = fakeEl('');
var cardShown = function () { return !cardEl.classList.contains('hidden'); };

var S, log, answers;
function reset(mode, extra) {
  S = { panel: '', graphMode: mode, colFocus: null, focus: null, node: null, dock: 'lineage',
    selectAsk: 0, cardAsk: 0, colDraw: 0, selectRaise: null, selectHistory: [], selectSub: null };
  for (var k in extra || {}) S[k] = extra[k];
  log = [];
  answers = [];
  details = {};
  timers = {};
}
// The server's answer to each /api/select waits until the test gives it,
// with the nodes it is given, one with no id otherwise.
var api = {
  get: function (url) {
    log.push('ask ' + decodeURIComponent(url.split('q=')[1].split('&')[0]));
    return new Promise(function (resolve) {
      answers.push(function (ids) {
        var nodes = (ids || [undefined]).map(function (id) { return { id: id }; });
        resolve({ nodes: nodes, edges: [], matched: nodes.length, counts: { model: nodes.length } });
      });
    });
  },
};
// Each box's /api/node answer waits until the test gives it, by its id.
var details = {};
function nodeDetail(q) { return new Promise(function (resolve) { details[q.id] = function () { resolve({ id: q.id }); }; }); }
function renderCatalog(n) { S.node = n; card(n.id); }
var timers = {}, nextTimer = 0;
function setTimeout(f) { timers[++nextTimer] = f; return nextTimer; }
function clearTimeout(t) { delete timers[t]; }
function fire() { var ts = timers; timers = {}; for (var t in ts) ts[t](); }

// How far each box runs past the canvas once the card is open: none unless
// the test says so.
var overruns = {};
var Lineage = {
  render: function () { log.push('drawn in ' + where('dock-lineage')); return null; },
  clear: function () { log.push('cleared'); }, fit: function () {},
  overrun: function (id) { return cardShown() ? overruns[id] || 0 : 0; },
  reveal: function (id) { log.push('revealed ' + id); },
  select: function (id) { log.push('lit ' + id); },
  placeHeads: function () {},
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
  check('the card closed and empty, not on the model the dock had', [S.node, text(catalog), cardShown()], [null, '', false]);
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

step(function () {
  print('\n--- the card ---');
  reset('model', { focus: 'model.jaffle_shop.orders' });
  $('#select-input').value = 'stg_orders+';
  var run = runSelection();
  answers.shift()(['model.jaffle_shop.stg_orders', 'model.jaffle_shop.orders']);
  return run;
});
step(function () {
  log = [];
  openSelCard('model.jaffle_shop.stg_orders');
  check('a click on a box clear of it opens it at once, nothing moved', [cardShown(), log], [true, []]);
  S.node = { id: 'model.jaffle_shop.stg_orders' };
  var run = runSelection();
  answers.shift()(['model.jaffle_shop.stg_orders', 'model.jaffle_shop.orders']);
  return run;
});
step(function () {
  check('drawn again with its box: it stays, the box lit again', [cardShown(), log.slice(-1)], [true, ['lit model.jaffle_shop.stg_orders']]);
  var run = runSelection();
  answers.shift()(['model.jaffle_shop.orders']);
  return run;
});
step(function () {
  check('drawn without it: closed, the box forgotten and no box lit', [cardShown(), S.node, log.slice(-1)], [false, null, ['lit null']]);
  log = [];
  overruns['model.jaffle_shop.orders'] = 40;
  openSelCard('model.jaffle_shop.orders');
  check('a box the card would cover: no card yet, nothing moved', [cardShown(), log], [false, []]);
  fire();
  check('a double-click past, it opens and the box comes back into sight', [cardShown(), log], [true, ['revealed model.jaffle_shop.orders', 'jobs asked']]);
  closeSelCard();
  openSelCard('model.jaffle_shop.orders');
  S.panel = '';
  fire();
  check('the panel rolled up first: no card', cardShown(), false);
  S.panel = 'select';
  openSelCard('model.jaffle_shop.orders');
  closeSelCard();
  fire();
  check('closed first: no card either', cardShown(), false);

  log = [];
  openSelCard('model.jaffle_shop.orders');
  fire();
  check('a card that waited asks for its jobs once shown', log, ['revealed model.jaffle_shop.orders', 'jobs asked']);
  log = [];
  closeSelCard();
  openSelCard('model.jaffle_shop.orders');
  selCardStill = false;
  fire();
  check('panned while it waited: shown where the canvas is left, not moved along', log, ['lit null', 'jobs asked']);

  var filter = fakeEl('', catalog);
  filter.focus();
  closeSelCard();
  check('closed with the focus inside: the panel has it, and the card is emptied',
    [document.activeElement === panel, catalog.childNodes.length, S.node], [true, 0, null]);

  check('its edge stops at 280px, and where the canvas keeps 320px',
    [selCardWidth(700, 0, 1000), selCardWidth(990, 0, 1000), selCardWidth(1020, 0, 1000), selCardWidth(-50, 0, 1000)],
    [300, 280, 280, 680]);

  paintSelectButtons();
  check('the buttons lit and expanded while the panel is down',
    [$('#select-btn').classList.contains('on'), $('#select-btn')['aria-expanded'], els['#graph-mode [data-mode=select]']['aria-expanded']],
    [true, 'true', 'true']);
  toggleSelection();
  paintSelectButtons();
  check('the top bar\'s button again: the panel rolls up, the buttons unlit',
    [S.panel, $('#select-btn').classList.contains('on'), $('#select-btn')['aria-expanded']], ['', false, 'false']);
});

var asked = [];
step(function () {
  print('\n--- the card\'s answer ---');
  reset('model', { focus: 'model.jaffle_shop.orders', node: { id: 'model.jaffle_shop.orders' } });
  card('orders');
  $('#select-input').value = 'stg_orders+';
  var run = runSelection();
  answers.shift()(['model.jaffle_shop.stg_orders', 'model.jaffle_shop.orders', 'model.jaffle_shop.customers']);
  return run;
});
step(function () {
  asked = [catalogFor('model.jaffle_shop.stg_orders'), catalogFor('model.jaffle_shop.customers')];
  details['model.jaffle_shop.customers']();
  details['model.jaffle_shop.stg_orders']();
  return Promise.all(asked);
});
step(function () {
  check('two clicks answered out of order: the card on the second box', [S.node.id, text(catalog)],
    ['model.jaffle_shop.customers', 'model.jaffle_shop.customers']);
  openSelCard('model.jaffle_shop.stg_orders');
  asked = [catalogFor('model.jaffle_shop.stg_orders')];
  closeSelCard();
  details['model.jaffle_shop.stg_orders']();
  return Promise.all(asked);
});
step(function () {
  check('closed before its answer: the answer dropped, the card still empty', [S.node, text(catalog)], [null, '']);
  openSelCard('model.jaffle_shop.orders');
  asked = [catalogFor('model.jaffle_shop.orders')];
  details['model.jaffle_shop.orders']();
  return Promise.all(asked);
});
step(function () {
  check('a card waiting out a double-click: its answer in, still hidden', [cardShown(), S.node.id], [false, 'model.jaffle_shop.orders']);
  var run = runSelection();
  answers.shift()(['model.jaffle_shop.stg_orders', 'model.jaffle_shop.orders']);
  return run;
});
step(function () {
  fire();
  check('drawn again while it waited: never opened, and the box forgotten', [cardShown(), S.node, text(catalog)], [false, null, '']);
  asked = [catalogFor('model.jaffle_shop.stg_orders')];
  log = [];
  closePanel();
  details['model.jaffle_shop.stg_orders']();
  return Promise.all(asked);
});
step(function () {
  check('rolled up before its answer: the dock\'s card stays', [S.node.id, text(catalog)], ['model.jaffle_shop.orders', 'orders']);
});

// jsc says nothing about a rejected promise and exits 0, so a step that throws
// has to say so itself, or the checks before it would vouch for the file.
steps.then(function () {}, function (e) { print('FAIL  a step threw ' + e); });
