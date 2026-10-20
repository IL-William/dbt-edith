// The settings panel's pure parts (0100): its sections and what each row of
// the rail says, the section it opens on, what the gear says of the profile,
// the environments a pick offers, where a save of the job sources goes, and
// when a block is drawn again.
// Run from the repository root: jsc web/tests/settings.js
var app = read('web/app.js');
eval(app.slice(app.indexOf('function settingsSections'), app.indexOf('function settingsNow')));
eval(app.slice(app.indexOf('function profileWarning'), app.indexOf('/* An icon drawn here rather than shipped')));
eval(app.slice(app.indexOf('function envChoices'), app.indexOf('/* One row of it, coloured by envTone. */')));

function check(label, got, want) {
  var g = JSON.stringify(got), w = JSON.stringify(want);
  print((g === w ? 'PASS  ' : 'FAIL  ') + label + (g === w ? '' : '\n        expected ' + w + '\n        got      ' + g));
}
function keys(sections) { return sections.map(function (s) { return s.key; }); }
function row(sections, key) { return sections.filter(function (s) { return s.key === key; })[0]; }

print('--- the sections ---');
check('every one, in the rail\'s order, whatever the page knows', keys(settingsSections({})),
  ['project', 'columns', 'snowflake', 'jobs', 'updates', 'theme', 'help']);
check('each says what it holds', settingsSections({}).every(function (s) { return s.label && s.title; }), true);
var none = settingsSections({});
check('nothing known: no dot, no hint but Snowflake off', [row(none, 'project').warn, row(none, 'columns').hint,
  row(none, 'snowflake').hint, row(none, 'updates').hint], ['', '', 'off', '']);
var some = settingsSections({ profile: 'no profiles.yml at /home/u/.dbt/profiles.yml, where dbt looks for one',
  cllTool: 'collin', cll: 'Collin', snowflake: true, update: 'newer' });
check('the profile missing: a dot on the project, saying why', row(some, 'project').warn,
  'no profiles.yml at /home/u/.dbt/profiles.yml, where dbt looks for one');
check('the column lineage on screen, in its tool\'s colour', [row(some, 'columns').hint, row(some, 'columns').tool],
  ['Collin', 'collin']);
check('none on screen: its dot is the uncoloured one', row(none, 'columns').tool, '');
check('the theme: nothing said while the look is off, on once it is',
  [row(none, 'theme').hint, row(settingsSections({ look: true }), 'theme').hint], ['', 'on']);
check('and the section says what waits for the look, the box style too', /box|style/.test(row(none, 'theme').title) && /once the look is on/.test(row(none, 'theme').title), true);
check('Snowflake on, and a newer release', [row(some, 'snowflake').hint, row(some, 'updates').hint], ['on', 'newer']);

print('\n--- the section it opens on ---');
check('the one asked for', settingsSection('updates', 'columns'), 'updates');
check('else the one last shown: the gear, or the tour putting it back', settingsSection(undefined, 'columns'), 'columns');
check('a click event is no section', settingsSection({ type: 'click' }, 'jobs'), 'jobs');
check('else the project', [settingsSection('', ''), settingsSection('later', '')], ['project', 'project']);

print('\n--- what the gear says of the profile ---');
check('nowhere to look for one: nothing to say', profileWarning(null), '');
check('found: nothing to say', profileWarning({ path: '/p/profiles.yml', found: true }), '');
check('missing: where dbt looks for it', profileWarning({ path: '/p/profiles.yml', found: false }),
  'no profiles.yml at /p/profiles.yml, where dbt looks for one');
check('refused, found or not: why it is not opened', profileWarning({ path: '/p/profiles.yml', found: true,
  refused: 'it links out of the project' }), 'profiles.yml not opened: it links out of the project');

print('\n--- the environments a pick offers ---');
function envTone(name) { return name.toLowerCase(); }
var S = { env: '.env.qa', envs: { files: [
  { file: '.env.dev', name: 'DEV', hidden: false },
  { file: '.env.qa', name: 'QA', hidden: false, target_mismatch: true, target: 'prod' },
  { file: '.env.example', name: 'EXAMPLE', hidden: true },
] } };
var picks = envChoices();
check('the files shown, then the manifest\'s own', picks.map(function (c) { return c.file; }), ['.env.dev', '.env.qa', '']);
check('the one in use is marked, the manifest not', picks.map(function (c) { return c.current; }), [false, true, false]);
check('a name its DBT_TARGET contradicts says so', [picks[1].label, picks[1].title],
  ['QA ⚠', 'DBT_TARGET in .env.qa is prod, which does not match the name']);
S.env = '';
check('none in use: the manifest is', envChoices().map(function (c) { return c.current; }), [false, false, true]);

print('\n--- where a save of the job sources goes ---');
eval(app.slice(app.indexOf('function jobsSaveRoute'), app.indexOf('// ----------------------------------------------------------------- panels --')));
check('on screen and read cleanly: it moves on', jobsSaveRoute({ kept: true, shown: true, clean: true }), 'read');
check('on screen, a source to correct: the form stays and says why', jobsSaveRoute({ kept: true, shown: true, clean: false }), 'paint');
check('behind another section, a source to correct: the form is said to, for when it is shown again',
  jobsSaveRoute({ kept: true, shown: false, clean: false }), 'paint');
check('behind another section and read cleanly: a toast, and nothing moves under the reader',
  jobsSaveRoute({ kept: true, shown: false, clean: true }), 'gone');
check('rolled up with the panel while it read: a toast', [jobsSaveRoute({ kept: false, shown: false, clean: true }),
  jobsSaveRoute({ kept: false, shown: false, clean: false })], ['gone', 'gone']);

print('\n--- a clean read ---');
var did = [];
function toast(text) { did.push('toast'); }
function jobsSaveToast() { return ['Saved and read.', 'ok']; }
function closePanel(o) { did.push('roll up' + (o && o.restore ? ', focus back' : '')); }
function loadJobs() { did.push('Jobs tab read'); }
function loadReportsIndex() { did.push('index read'); }
function openReports(section) { did.push('Reports on ' + section); }
var node = null;
function jobsNode() { return node; }
function cleanRead(from, dock, model) {
  did = [];
  S.settingsKept = { jobs: 'the form' };
  S.settingsFrom = from;
  S.dock = dock;
  node = model;
  jobSourcesRead({});
  return did;
}
check('the Jobs tab under it, with a model: the panel rolls up onto it, read again',
  cleanRead('', 'jobs', 'model.a'), ['toast', 'roll up, focus back', 'Jobs tab read', 'index read']);
check('the form is done with', S.settingsKept, {});
check('the Jobs tab with no model: the coverage, and the tab read again for when it is seen',
  cleanRead('', 'jobs', null), ['toast', 'Jobs tab read', 'Reports on jobs']);
check('from Reports\' Sources…, over the Jobs tab: back to the coverage, the tab read again',
  cleanRead('reports', 'jobs', 'model.a'), ['toast', 'Jobs tab read', 'Reports on jobs']);
check('another tab in the dock: the coverage alone', cleanRead('', 'catalog', 'model.a'), ['toast', 'Reports on jobs']);

print('\n--- a block drawn again only when it changes ---');
// Just enough of an element: attributes, a dataset, children, the markup they
// make, a shallow clone and the focus.
var document = { activeElement: null };
function fake(tag) {
  return {
    tag: tag, attrs: {}, dataset: {}, kids: [],
    get innerHTML() { return this.kids.map(function (k) { return k.outerHTML; }).join(''); },
    get outerHTML() {
      var a = this.attrs, d = this.dataset;
      return '<' + this.tag + Object.keys(a).map(function (k) { return ' ' + k + '="' + a[k] + '"'; }).join('')
        + Object.keys(d).map(function (k) { return ' data-' + k + '="' + d[k] + '"'; }).join('') + '>' + this.innerHTML + '</' + this.tag + '>';
    },
    cloneNode: function () {
      var c = fake(this.tag);
      c.attrs = Object.assign({}, this.attrs);
      c.dataset = Object.assign({}, this.dataset);
      return c;
    },
    replaceChildren: function () { this.kids = Array.prototype.slice.call(arguments); },
    contains: function (x) { return x === this || this.kids.some(function (k) { return k.contains(x); }); },
    querySelectorAll: function () {
      var out = [];
      (function walk(e) { e.kids.forEach(function (k) { if (k.dataset.key) out.push(k); walk(k); }); })(this);
      return out;
    },
    focus: function () { document.activeElement = this; },
  };
}
eval(app.slice(app.indexOf('function redrawIfChanged'), app.indexOf('/* The rail and every live block')));
var shown = { tip: 'fetch on click', rows: ['a', 'b'] };
var draws = 0;
function paintRows(box) {
  draws++;
  box.attrs.title = shown.tip;
  box.replaceChildren.apply(box, shown.rows.map(function (r) {
    var b = fake('button');
    b.dataset.key = r;
    b.attrs.label = r;
    return b;
  }));
}
var block = fake('div');
redrawIfChanged(block, paintRows);
check('drawn the first time', block.kids.map(function (k) { return k.dataset.key; }), ['a', 'b']);
var first = block.kids[0];
redrawIfChanged(block, paintRows);
check('the same again: what is there stays, the same nodes', block.kids[0] === first, true);
shown.tip = 'starts with the next column click';
redrawIfChanged(block, paintRows);
check('only its own tooltip changed: drawn again', [block.attrs.title, block.kids[0] === first], ['starts with the next column click', false]);
block.kids[1].focus();
shown.rows = ['a', 'b', 'c'];
redrawIfChanged(block, paintRows);
check('drawn again with the focus in it: back on the control with the same key',
  [document.activeElement === block.kids[1], document.activeElement.dataset.key], [true, 'b']);
var elsewhere = fake('input');
elsewhere.focus();
shown.rows = ['a', 'c'];
redrawIfChanged(block, paintRows);
check('the focus elsewhere: left there', document.activeElement === elsewhere, true);
