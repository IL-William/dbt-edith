// The top bar's spaces (0140): the tab of the space in front is lit and marked
// aria-current, Code when no panel is down; a click on it does nothing, since
// a tab is a place and never a switch; Code from the settings opened over
// Reports does not bounce back to Reports, where Escape does; and Escape
// closes what a space opened inside itself before it goes back to Code.
// Run from the repository root: jsc web/tests/spaces.js
var src = read('web/app.js');

function check(label, got, want) {
  var g = JSON.stringify(got), w = JSON.stringify(want);
  print((g === w ? 'PASS  ' : 'FAIL  ') + label + (g === w ? '' : '\n        expected ' + w + '\n        got      ' + g));
}

// Just enough of the page: elements that hold a class, an attribute, the
// focus and whether they are inert.
var els = {};
function fakeEl(id, data) {
  var cls = {};
  var el = {
    id: id, inert: false, attrs: {}, dataset: data || {},
    classList: {
      add: function (c) { cls[c] = true; }, remove: function (c) { delete cls[c]; },
      toggle: function (c, on) { if (on === undefined ? !cls[c] : on) cls[c] = true; else delete cls[c]; },
      contains: function (c) { return !!cls[c]; },
    },
    setAttribute: function (k, v) { this.attrs[k] = String(v); },
    removeAttribute: function (k) { delete this.attrs[k]; },
    focus: function () { document.activeElement = this; },
    blur: function () { document.activeElement = document.body; },
    contains: function (x) { return x === this; },
  };
  if (id) els['#' + id] = el;
  return el;
}
var document = { body: null, activeElement: null };
document.body = fakeEl('body');
document.body.contains = function (x) { return !!x; };
document.activeElement = document.body;
function $(sel) { return els[sel]; }

var tabs = ['code', 'lineage', 'reports'].map(function (k) { return fakeEl('space-' + k, { space: k }); });
function $$(sel) { return sel === '#spaces .sp' ? tabs : []; }
var gear = fakeEl('settings-btn');
var dockSeg = fakeEl('');
els['#graph-mode [data-mode=select]'] = dockSeg;
['editor-pane', 'gutter-dock', 'dock', 'sidebar', 'gutter-side', 'reports', 'select-panel', 'settings'].forEach(function (id) { fakeEl(id); });
tabs[0].setAttribute('aria-current', 'page');

var S = { panel: '', panelReturn: null, settingsFrom: '', settingsSection: 'project', reportsSection: 'jobs' };
var log = [];
function closeHoverCard() {}
function closePopup() {}
function fillCatalogJobs() {}
function paintReportsButton() {}
function paintSettingsButton() {}
function selectPanelUp() {}
// What the status bar says of each space has its own test (status.js).
function updateStatus() {}
// What the selection's panel has open inside itself: Escape closes it first.
var card = false;
function closeSelCard() { var was = card; card = false; return was; }
// The ways in, as far as these checks go: what each does is its own test's.
function openSelection() { log.push('openSelection'); openPanel('select'); }
function openReports() { log.push('openReports'); openPanel('reports'); }
function settingsSection(section, last) { return section || last; }
function showSettingsSection() {}
function loadReportsIndex() { log.push('index'); return { then: function (f) { f(); } }; }
function showReportSection() {}

// A `const` an eval declares stays inside it: the slice's are made `var`s.
eval(src.slice(src.indexOf('const SPACE_OF = {'), src.indexOf('// ---------------------------------------------------------------- reports --'))
  .replace(/^const /gm, 'var '));
eval(src.slice(src.indexOf('function openSettings('), src.indexOf('/* A glyph per section')));

// Every call that would open or close a panel is counted, so a click that
// does nothing is told from one that closes a panel already closed.
var calls = [];
var realOpen = openPanel, realClose = closePanel;
openPanel = function (name) { calls.push('open ' + name); return realOpen(name); };
closePanel = function (o) { calls.push('close'); return realClose(o); };

var lit = function () { return tabs.filter(function (t) { return t.attrs['aria-current'] === 'page'; }).map(function (t) { return t.dataset.space; }); };
// A click on a tab, as wireKeys' neighbour wires it.
var click = function (b) { goSpace(b.dataset ? b.dataset.space : b); };

print('--- the lit tab ---');
paintSpaces();
check('no panel down: Code is lit, the gear and the dock\'s button are not', [lit(), gear.classList.contains('on'), gear.attrs['aria-expanded'], dockSeg.attrs['aria-expanded']],
  [['code'], false, 'false', 'false']);
click(tabs[1]);
check('Lineage: its panel down, its tab the only one lit, the dock\'s button expanded', [S.panel, lit(), dockSeg.attrs['aria-expanded']], ['select', ['lineage'], 'true']);
click(tabs[2]);
check('Reports takes its place: its tab lit alone, the dock\'s button not expanded', [S.panel, lit(), dockSeg.attrs['aria-expanded']], ['reports', ['reports'], 'false']);
goSpace('settings');
check('the gear: the settings down, no tab lit, the gear lit and expanded', [S.panel, lit(), gear.classList.contains('on'), gear.attrs['aria-expanded']],
  ['settings', [], true, 'true']);
click(tabs[0]);
check('Code: nothing down, Code lit, the gear not', [S.panel, lit(), gear.classList.contains('on'), gear.attrs['aria-expanded']], ['', ['code'], false, 'false']);
check('what the panels covered is usable again', ['#editor-pane', '#gutter-dock', '#dock', '#sidebar', '#gutter-side'].map(function (s) { return els[s].inert; }),
  [false, false, false, false, false]);
check('the settings are wide, as the other spaces are: the sidebar was inert under them',
  (function () { openPanel('settings'); var r = els['#sidebar'].inert; closePanel(); return r; })(), true);

print('\n--- a tab is a place, not a switch ---');
log = [];
calls = [];
S.settingsFrom = 'kept';
click(tabs[0]);
check('Code in front: a click on Code calls nothing and forgets nothing', [S.panel, lit(), log, calls, S.settingsFrom], ['', ['code'], [], [], 'kept']);
S.settingsFrom = '';
click(tabs[1]);
log = [];
calls = [];
click(tabs[1]);
check('Lineage in front: a click on Lineage does not close it nor draw it again', [S.panel, lit(), log, calls], ['select', ['lineage'], [], []]);
goSpace('settings');
log = [];
calls = [];
S.settingsFrom = 'kept';
goSpace('settings');
check('the gear with the settings down: nothing is called, nothing remembered again', [S.panel, log, calls, S.settingsFrom], ['settings', [], [], 'kept']);
S.settingsFrom = '';
goSpace('code');

print('\n--- Code from the settings opened over Reports ---');
goSpace('reports');
goSpace('settings');
check('Settings over Reports remember where they came from', S.settingsFrom, 'reports');
log = [];
goSpace('code');
check('Code is Code: nothing comes back, Reports not reopened', [S.panel, lit(), log], ['', ['code'], []]);
check('and what was remembered is cleared', S.settingsFrom, '');

goSpace('reports');
goSpace('settings');
log = [];
escapePanel();
check('Escape from the same settings gives Reports back', [S.panel, lit()], ['reports', ['reports']]);
goSpace('settings');
log = [];
goSpace('reports');
check('the Reports tab from the settings: Reports, opened once', [S.panel, log], ['reports', ['openReports']]);
goSpace('code');

print('\n--- Escape inside a space ---');
goSpace('lineage');
card = true;
escapePanel();
check('its card open: Escape closes the card and the space stays', [S.panel, card, lit()], ['select', false, ['lineage']]);
escapePanel();
check('then Escape goes back to Code', [S.panel, lit()], ['', ['code']]);
