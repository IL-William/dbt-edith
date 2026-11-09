// The settings panel's pure parts (0100, 0163): its sections and what each row
// of the rail says, the section it opens on, what the gear says of the profile,
// the environments a pick offers and what each .env file line says, what a job
// source's row says, the next look for a release, where a save of the job
// sources goes, and when a block is drawn again.
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
check('nothing known: no mark, no hint but Snowflake off', [row(none, 'project').warn, row(none, 'columns').hint,
  row(none, 'snowflake').hint, row(none, 'updates').hint], ['', '', 'off', '']);
var some = settingsSections({ profile: 'no profiles.yml at /home/u/.dbt/profiles.yml, where dbt looks for one',
  cllTool: 'collin', cll: 'Collin', snowflake: true, update: '1.77.0', sources: 2 });
check('the profile missing: a triangle on the project, saying why', row(some, 'project').warn,
  'no profiles.yml at /home/u/.dbt/profiles.yml, where dbt looks for one');
check('the column lineage on screen, in its tool\'s colour', [row(some, 'columns').hint, row(some, 'columns').tool],
  ['Collin', 'collin']);
check('none on screen: its glyph is the uncoloured one', row(none, 'columns').tool, '');
check('the theme: the name of the one on screen, nothing before it is known',
  [row(none, 'theme').hint, row(settingsSections({ theme: 'Light' }), 'theme').hint], ['', 'Light']);
check('and the section says it paints the canvas too', /the canvas included/.test(row(none, 'theme').title), true);
check('Snowflake on, and a newer release: its version, in the accent', [row(some, 'snowflake').hint, row(some, 'updates').hint, row(some, 'updates').news],
  ['on', '1.77.0', true]);
check('installed: the restart that is left', [settingsSections({ update: 'restart' })[4].hint, settingsSections({ update: 'restart' })[4].news], ['restart', true]);
check('no newer release: no hint and no accent', [row(none, 'updates').hint, row(none, 'updates').news], ['', false]);
check('the sources, once the page knows how many', [row(some, 'jobs').hint, row(settingsSections({ sources: 1 }), 'jobs').hint,
  row(settingsSections({ sources: 0 }), 'jobs').hint, row(none, 'jobs').hint, row(settingsSections({ sources: null }), 'jobs').hint],
  ['2 sources', '1 source', '0 sources', '', '']);
check('Help carries no hint', row(some, 'help').hint, undefined);
check('each line under a section\'s name ends with a full stop', settingsSections({}).every(function (s) { return /\.$/.test(s.title); }), true);
check('the tool sections carry a tool, the others none', settingsSections({}).map(function (s) { return 'tool' in s; }),
  [false, true, true, true, false, false, false]);

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
check('the manifest\'s choice, without a sandbox', [picks[2].label, picks[2].sub, picks[2].tone],
  ['manifest', 'as dbt parsed it', 'manifest']);
S.meta = { sandbox: 'dev_db.dbt_someone' };
var mine = envChoices();
check('on a sandbox manifest it is the developer\'s own compile, with the sandbox', [mine[2].label, mine[2].sub, mine[2].tone, mine[2].file],
  ['my compile', 'dev_db.dbt_someone, as dbt wrote it', 'manifest', '']);
check('and says what it shows', mine[2].title,
  'the Catalog\'s locations as dbt parsed them, the SQL and the Built column as this manifest\'s target wrote them, into dev_db.dbt_someone');
check('the .env files keep their names', [mine[0].label, mine[0].sub], ['DEV', '.env.dev']);
check('the pill\'s words come from the same place', [manifestChoice('').label, manifestChoice('dev_db.dbt_someone').now],
  ['manifest', 'Catalog locations are shown as dbt parsed them, the SQL as your own compile wrote it, into dev_db.dbt_someone.']);
S.meta = undefined;

print('\n--- what a .env file line says ---');
eval(app.slice(app.indexOf('function envLineSummary'), app.indexOf('/* The environments, once the .env files are read again')));
var refs = [{ var: 'A' }, { var: 'B' }];
function envFile(extra) { return Object.assign({ file: '.env.dev', name: 'DEV', hidden: false, target: 'dev', coverage: { defined: 2 },
  agreement: { checked: 4, equal: 4 } }, extra); }
check('all three known', envLineSummary(envFile({}), refs), 'DBT_TARGET=dev \u00b7 2/2 variables \u00b7 100% as parsed');
check('a file the config does not agree with, to a tenth', envLineSummary(envFile({ agreement: { checked: 3, equal: 2 } }), refs),
  'DBT_TARGET=dev \u00b7 2/2 variables \u00b7 66.7% as parsed');
check('no DBT_TARGET and nothing to compare: only the variables', envLineSummary(envFile({ target: '', agreement: { checked: 0, equal: 0 } }), refs),
  '2/2 variables');
check('nothing known: nothing', envLineSummary({ hidden: false, target: '', coverage: { defined: 0 }, agreement: { checked: 0, equal: 0 } }, []), '');
check('a hidden file says so, and nothing else', envLineSummary(envFile({ hidden: true }), refs), 'hidden');

print('\n--- what a job source\'s row says ---');
eval(app.slice(app.indexOf('function jobsPlural'), app.indexOf('function jobsReadClean')));
check('before a read: nothing', jobSourceSummary(undefined), { text: '', tone: '' });
check('Airflow: DAGs and tasks', jobSourceSummary({ kind: 'airflow', jobs: 1, steps: 3 }), { text: '1 DAG \u00b7 3 tasks', tone: '' });
check('dbt Cloud: jobs and steps', jobSourceSummary({ kind: 'dbt_cloud', jobs: 2, steps: 5 }), { text: '2 jobs \u00b7 5 steps', tone: '' });
check('a file refused, said after the counts', jobSourceSummary({ kind: 'airflow', jobs: 1, steps: 3, refused: [{ file: 'a.yml' }] }),
  { text: '1 DAG \u00b7 3 tasks \u00b7 1 file refused', tone: '' });
check('two refused', jobSourceSummary({ kind: 'airflow', jobs: 1, steps: 3, refused: [{}, {}] }).text, '1 DAG \u00b7 3 tasks \u00b7 2 files refused');
check('a walk stopped at its bound: said, as a warning', jobSourceSummary({ kind: 'airflow', jobs: 1, steps: 3, capped: true }),
  { text: '1 DAG \u00b7 3 tasks \u00b7 stopped at 5 000 files', tone: 'warn' });
check('refused and capped both', jobSourceSummary({ kind: 'dbt_cloud', jobs: 2, steps: 5, refused: [{}], capped: true }),
  { text: '2 jobs \u00b7 5 steps \u00b7 1 file refused \u00b7 stopped at 5 000 files', tone: 'warn' });
check('the error alone, as a warning', jobSourceSummary({ kind: 'airflow', jobs: 0, steps: 0, error: 'no such folder' }), { text: 'no such folder', tone: 'warn' });

print('\n--- the Column lineage cards\' tab stops ---');
eval(app.slice(app.indexOf('function cardTabStops'), app.indexOf('/* The tool cards of Column lineage as radios')));
function card(checked, left) { return { checked: checked, left: left }; }
check('the picked card is the one stop, the others are reached by the arrows', cardTabStops([card(false), card(true), card(false)]), [-1, 0, -1]);
check('none picked: the first card that can be stepped to', cardTabStops([card(false, true), card(false), card(false)]), [0, 0, -1]);
check('a card left out keeps its tab stop, the install one included', cardTabStops([card(false), card(true), card(false, true)]), [-1, 0, 0]);
check('none a step, none picked: every card is reached by Tab, the first included', cardTabStops([card(false, true), card(false, true)]), [0, 0]);
check('no cards: nothing', cardTabStops([]), []);

print('\n--- the rail\'s count of sources ---');
check('it comes from the Reports index, which reads no job file, and nothing asks the jobs for it',
  [/sources: S\.reports \? \(S\.reports\.jobs \? S\.reports\.jobs\.sources : 0\) : null/.test(app), /askSourceCount|jobSourceCount|jobSourceAsked/.test(app)], [true, false]);

print('\n--- the next look for a release ---');
eval(app.slice(app.indexOf('function whenLooked'), app.indexOf('/* What the menu says of the looks for a newer dbt-edith')));
eval(app.slice(app.indexOf('function nextLook'), app.indexOf('/* The Updates section: this build')));
var noon = Math.floor(new Date(2026, 10, 8, 12, 0, 0).getTime() / 1000);
check('one due tomorrow', nextLook({ next: noon + 86400 }, noon), 'The next look at GitHub is tomorrow at 12:00.');
check('one due now', nextLook({ next: noon - 5 }, noon), 'The next look at GitHub is due now.');
check('none known, the check off, or one on its way: nothing', [nextLook({}, noon), nextLook({ off: true, next: noon + 60 }, noon),
  nextLook({ checking: true, next: noon + 60 }, noon), nextLook(null, noon)], ['', '', '', '']);

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

print('\n--- the status bar\'s env pill (0142) ---');
// The click is wired in the page's startup, which no harness runs: hold the line itself.
var pillClick = app.split('\n').filter(function (l) { return l.indexOf("$('#status-env').addEventListener('click'") >= 0; });
check('one handler on the env pill', pillClick.length, 1);
check('it opens the environment menu, not the settings', [/openEnvMenu\(/.test(pillClick[0]), /openSettings\(/.test(pillClick[0])], [true, false]);

print('\n--- the source rows and the warnings of the settings (0163) ---');
var css = read('web/app.css');
var rule = function (sel) { var m = css.match(new RegExp('^' + sel.replace(/[.*+?^${}()|[\]\\=]/g, '\\$&') + ' \\{([^}]*)\\}', 'm')); return m ? m[1] : ''; };
check('a source row bounds its summary so the name keeps room, and the summary wraps a path',
  [/fit-content\(50%\)/.test(rule('.srcline')), /overflow-wrap: anywhere/.test(rule('.srcsum'))], [true, true]);
check('the warning text of a row, a path note and an urgent action is the readable --warn-text',
  [rule('.srcsum.warn'), rule('.sp2.pathwarn'), rule('.radios button.subaction[data-urgent=true]')].map(function (r) { return /var\(--warn-text\)/.test(r); }), [true, true, true]);
check('the settings\' primary button takes the on-accent colour', /color: var\(--on-accent\)/.test(rule('.setinner .btn.primary')), true);
check('a source row\'s button says which source it edits', /edit\.setAttribute\('aria-label', `Edit \$\{called\(f\)\}`\)/.test(app), true);

print('\n--- the rail\'s count follows the sources\' own read (0163) ---');
check('an index not yet loaded says nothing, so nothing is read', jobsIndexBehind(undefined, 2), false);
check('the same count: nothing to read', jobsIndexBehind({ jobs: { sources: 2 } }, 2), false);
check('the file changed outside the form: the index is behind', [jobsIndexBehind({ jobs: { sources: 0 } }, 2), jobsIndexBehind({ jobs: { sources: 3 } }, 2)], [true, true]);
check('an index with no jobs entry counts none', [jobsIndexBehind({}, 0), jobsIndexBehind({}, 1)], [false, true]);
check('the section reads the index again after its own read, when it is behind',
  /forgetCatalogJobs\(\);\n[^\n]*\n\s*if \(jobsIndexBehind\(S\.reports, summary\.sources\.length\)\) loadReportsIndex\(\);/.test(app), true);
check('Edith\'s menu no longer asks whether it sits in a panel', /const from = 'Edith\\'s menu';/.test(app) && !/classList\.contains\('inpanel'\)/.test(app), true);
