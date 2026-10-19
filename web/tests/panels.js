// What every panel over the editor and the dock shares (0091): one down at a
// time, what is under it inert while it is, the sidebar too under a wide one
// (0101), the focus given back to where it was before the first, and the
// hooks of a panel giving way run with the next one already down, so nothing
// under them counts as seen in between. Escape closes what a panel opened
// inside itself before the panel.
// Run from the repository root: jsc web/tests/panels.js
var src = read('web/app.js');

function check(label, got, want) {
  var g = JSON.stringify(got), w = JSON.stringify(want);
  print((g === w ? 'PASS  ' : 'FAIL  ') + label + (g === w ? '' : '\n        expected ' + w + '\n        got      ' + g));
}

// Just enough of the page: elements that hold a class, an attribute, the
// focus and whether they are inert.
var els = {};
function fakeEl(id, parent) {
  var cls = {};
  var el = {
    id: id, parent: parent || null, inert: false, attrs: {},
    classList: {
      add: function (c) { cls[c] = true; }, remove: function (c) { delete cls[c]; },
      contains: function (c) { return !!cls[c]; },
    },
    setAttribute: function (k, v) { this.attrs[k] = v; },
    focus: function () { document.activeElement = this; },
    blur: function () { document.activeElement = document.body; },
    contains: function (x) { for (var at = x; at; at = at.parent) if (at === this) return true; return false; },
  };
  els['#' + id] = el;
  return el;
}
var document = {
  body: null, activeElement: null,
};
document.body = fakeEl('body');
document.body.contains = function (x) { return !!x && x !== null; };
document.activeElement = document.body;
function $(sel) { return els[sel]; }

var S = { panel: '', panelReturn: null, reportsSection: 'jobs', dock: 'catalog' };
var log = [];
function closeHoverCard() { log.push('hovercard'); }
function closePopup() {
  log.push('popup');
  // A button in a popup goes with it.
  if (document.activeElement === popupButton) document.activeElement = document.body;
}
function paintReportsButton() { log.push('paint reports ' + S.panel); }
function fillCatalogJobs() { log.push('jobs, panel ' + JSON.stringify(S.panel)); }
function openReports(section) { log.push('reopen reports ' + section); }

['editor-pane', 'gutter-dock', 'dock', 'sidebar', 'gutter-side', 'reports', 'other', 'wide'].forEach(function (id) { fakeEl(id); });
var editorButton = fakeEl('editor-button', els['#editor-pane']);
var popupButton = fakeEl('popup-button');
var inReports = fakeEl('in-reports', els['#reports']);

// A `const` an eval declares stays inside it: the slice's are made `var`s.
eval(src.slice(src.indexOf('const PANELS = {'), src.indexOf('// ---------------------------------------------------------------- reports --'))
  .replace(/^const /gm, 'var '));

// A second panel, as each later one is: its own element and hooks.
PANELS.other = {
  el: '#other',
  paint: function () { log.push('paint other ' + S.panel); },
  closed: function () { log.push('other closed, panel ' + JSON.stringify(S.panel)); },
  save: function () { return { mine: 1 }; },
  reopen: function () {},
};
// A wide one, as the Custom selection's: over the sidebar too, and with
// something of its own that Escape closes first, as the selection's card.
var inside = false;
PANELS.wide = {
  el: '#wide', wide: true,
  paint: function () {}, closed: function () {}, save: function () { return {}; }, reopen: function () {},
  escape: function () { var was = inside; inside = false; return was; },
};
var covers = function () { return ['#editor-pane', '#gutter-dock', '#dock'].map(function (s) { return els[s].inert; }); };
var side = function () { return ['#sidebar', '#gutter-side'].map(function (s) { return els[s].inert; }); };

print('--- one panel ---');
editorButton.focus();
log = [];
openPanel('reports');
check('down: named, open, shown to a reader, focused', [S.panel, panelOpen('reports'), els['#reports'].classList.contains('open'),
  els['#reports'].attrs['aria-hidden'], document.activeElement === els['#reports']], ['reports', true, true, 'false', true]);
check('what it covers is inert, and the sidebar is not', [covers(), side()], [[true, true, true], [false, false]]);
check('the hover card and a popup close first, then its button is painted', log, ['hovercard', 'popup', 'paint reports reports']);
log = [];
openPanel('reports');
check('opened again: nothing happens', log, []);
inReports.focus();
log = [];
closePanel({ restore: true });
check('up: nothing down, what it covered usable, the focus back', [S.panel, covers(), document.activeElement === editorButton],
  ['', [false, false, false], true]);
check('its button painted, then its hook, with nothing down', log, ['paint reports ', 'jobs, panel ""']);
check('closed again: nothing happens', (log = [], closePanel(), log), []);

print('\n--- a popup had the focus ---');
popupButton.focus();
openPanel('reports');
closePanel({ restore: true });
check('the focus is not sent to the popup\'s button, gone with it', document.activeElement === popupButton, false);

print('\n--- one panel takes another\'s place ---');
editorButton.focus();
openPanel('reports');
log = [];
openPanel('other');
check('the second is down, the first is not', [S.panel, els['#reports'].classList.contains('open'), els['#other'].classList.contains('open')],
  ['other', false, true]);
check('the first one\'s hooks run with the second already down', log,
  ['hovercard', 'popup', 'paint reports other', 'jobs, panel "other"', 'paint other other']);
check('what they cover stayed inert', covers(), [true, true, true]);
closePanel({ restore: true });
check('Escape gives back the focus from before the first', document.activeElement === editorButton, true);

print('\n--- a wide panel ---');
openPanel('wide');
check('down: the sidebar and its gutter inert as well', [covers(), side()], [[true, true, true], [true, true]]);
openPanel('reports');
check('the reports in its place: the sidebar usable again, the rest not', [covers(), side()], [[true, true, true], [false, false]]);
openPanel('wide');
check('and wide again: the sidebar inert again', side(), [true, true]);
closePanel();
check('up: nothing inert', [covers(), side()], [[false, false, false], [false, false]]);

print('\n--- what the tour keeps ---');
S.reportsSection = 'collin';
check('the section on screen', PANELS.reports.save(), { section: 'collin' });
log = [];
PANELS.reports.reopen({ section: 'collin' });
check('put back on it', log, ['reopen reports collin']);
check('every panel has every hook', Object.keys(PANELS).filter(function (k) {
  return ['el', 'paint', 'closed', 'save', 'reopen'].some(function (h) { return !PANELS[k][h]; });
}), []);

print('\n--- Escape on the page, with a panel down ---');
// A key goes through the window's capture listeners, then the document's, then
// the window's own, and one stopped goes no further. The page's keys are
// wireKeys' first two listeners; the rest of it wires buttons.
var keyListeners = { wc: [], dc: [], wb: [] };
function listen(where) {
  return {
    add: function (type, f, capture) { if (type === 'keydown') keyListeners[where + (capture ? 'c' : 'b')].push(f); },
    remove: function (type, f, capture) {
      var l = keyListeners[where + (capture ? 'c' : 'b')] || [];
      if (l.indexOf(f) >= 0) l.splice(l.indexOf(f), 1);
    },
  };
}
var window = { innerWidth: 1200, innerHeight: 800, addEventListener: listen('w').add, removeEventListener: listen('w').remove };
document.addEventListener = listen('d').add;
document.removeEventListener = listen('d').remove;
function press(key) {
  var stopped = false, prevented = false;
  var e = {
    key: key, code: '', target: document.body, metaKey: false, ctrlKey: false, altKey: false, shiftKey: false,
    preventDefault: function () { prevented = true; }, stopPropagation: function () { stopped = true; },
  };
  Object.defineProperty(e, 'defaultPrevented', { get: function () { return prevented; } });
  ['wc', 'dc', 'wb'].forEach(function (phase) {
    if (!stopped) keyListeners[phase].slice().forEach(function (f) { f(e); });
  });
}
var IS_MAC = true;
function clearTimeout() {}
var crumbMenu = null, envMenu = null, selectorMenu = null, stopAtMenu = null;
fakeEl('palette').classList.add('hidden');
function escapesGraphFind() { return false; }
eval(src.slice(src.indexOf('function keyCombo'), src.indexOf('function toggleShortcuts')));
eval(src.slice(src.indexOf('let popup = null;'), src.indexOf('/* `anchor`\'s menu opens under the pointer'))
  .replace(/^(const|let) /gm, 'var '));
eval(src.slice(src.indexOf('function closePopup(anchor)'), src.indexOf('/* A snowflake drawn here')));
eval(src.slice(src.indexOf('function tourKey(e)'), src.indexOf('function tourStep(by)')));
var wire = src.slice(src.indexOf('function wireKeys'), src.indexOf('  // What Cmd+F searches when nothing has the focus'));
eval(wire + '}');
wireKeys();
document.body.appendChild = function () {};
var menuAnchor = fakeEl('menu-anchor');
function fakeMenu() {
  var m = fakeEl('menu');
  m.style = {};
  m.getBoundingClientRect = function () { return { top: 0, bottom: 20, left: 0, height: 20, width: 50 }; };
  m.querySelectorAll = function () { return []; };
  m.addEventListener = function () {};
  m.remove = function () {};
  return m;
}
menuAnchor.getBoundingClientRect = function () { return { top: 0, bottom: 10, left: 0, height: 10, width: 10 }; };

openPanel('reports');
press('Escape');
check('nothing else in front: the panel rolls up', S.panel, '');

openPanel('reports');
S.tour = { ending: false };
log = [];
function endTour() { log.push('end tour'); }
window.addEventListener('keydown', tourKey, true);
press('Escape');
check('the tour on: Escape ends the tour, and the panel it put down stays for its put-back', [log, S.panel], [['end tour'], 'reports']);
window.removeEventListener('keydown', tourKey, true);
S.tour = null;

openPopup(menuAnchor, fakeMenu(), null, true);
press('Escape');
check('a menu the pointer opened: Escape closes it, and only it', [popup, S.panel], [null, 'reports']);
press('Escape');
check('a second Escape then rolls the panel up', S.panel, '');

openPanel('reports');
openPopup(menuAnchor, fakeMenu(), null, false);
press('Escape');
check('a menu a click opened: Escape closes it, and only it', [popup, S.panel], [null, 'reports']);
closePanel();

print('\n--- the settings keep a form for the tour ---');
S.settingsSection = 'jobs';
S.settingsKept = { jobs: 'the form, half filled' };
S.settingsFrom = 'reports';
var settingsSaved = PANELS.settings.save();
S.panel = 'other';
PANELS.settings.closed({});
check('rolled up, a form left half filled goes with it', S.settingsKept, {});
var settingsOpened = [];
function openSettings(section) { settingsOpened.push(section); S.settingsFrom = ''; }
PANELS.settings.reopen(settingsSaved);
check('the tour puts it back as it was left, on its section, over what it was over',
  [S.settingsKept, settingsOpened, S.settingsFrom], [{ jobs: 'the form, half filled' }, ['jobs'], 'reports']);

print('\n--- the settings down over Reports give it back ---');
fakeEl('settings');
function paintSettingsButton() {}
// Answered at once, so what comes after the index is checked here.
function loadReportsIndex() { log.push('index'); return { then: function (f) { f(); } }; }
function showReportSection(key) { log.push('section ' + key + ', panel ' + S.panel); }
S.reportsSection = 'jobs';
openPanel('reports');
S.settingsFrom = 'reports';
openPanel('settings');
log = [];
press('Escape');
check('Escape: Reports down again at once, its section drawn afresh once the index is read',
  [S.panel, log.filter(function (l) { return l === 'index' || l.indexOf('section') === 0; })], ['reports', ['index', 'section jobs, panel reports']]);
check('  and what is under it inert all along', covers(), [true, true, true]);
S.settingsFrom = 'reports';
openPanel('settings');
closePanel();
check('rolled up by a file opened or the tour: nothing comes back', S.panel, '');
S.settingsFrom = 'reports';
openPanel('settings');
openPanel('other');
check('giving way to another panel: that one stays', S.panel, 'other');
closePanel();
S.settingsFrom = '';
openPanel('settings');
press('Escape');
check('down over nothing: nothing comes back', S.panel, '');
S.panel = '';
S.settingsFrom = 'reports';
log = [];
loadReportsIndex = function () { log.push('index'); return { then: function (f) { S.panel = 'other'; f(); } }; };
PANELS.settings.closed({ restore: true });
check('another panel down by the time the index is read: its section is not drawn over it',
  log.filter(function (l) { return l.indexOf('section') === 0; }), []);
S.panel = '';

openPanel('wide');
inside = true;
press('Escape');
check('what the panel opened inside itself closes first, and only it', [inside, S.panel], [false, 'wide']);
press('Escape');
check('then Escape rolls the panel up', S.panel, '');
// The panel's own listener, which the focus inside it reaches, calls the same.
inside = true;
openPanel('wide');
escapePanel();
check('the same from the panel\'s own Escape', [inside, S.panel], [false, 'wide']);
escapePanel();
check('and the second one rolls it up', S.panel, '');
