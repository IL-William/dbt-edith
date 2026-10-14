/* dbt-edith UI shell: sidebar, editor tabs, lineage, node detail, terminal. */
(() => {
const $ = (s) => document.querySelector(s);
const $$ = (s) => [...document.querySelectorAll(s)];

const S = {
  meta: null,
  open: new Map(),   // path -> {doc, dirty, stamp, disk}
  order: [],
  active: null,
  focus: null,       // unique_id currently in the lineage view
  unsynced: '',      // file whose tab came up with no node in the manifest, for a reload to look again
  stampsAsked: false, // a look at the open files' stamps is on its way, so another waits its turn
  stampsAgain: false, // one was asked for meanwhile, and runs once that look is done
  cm: null,
  term: null, ws: null, fit: null,
  venv: null,                 // /api/meta's venv: the one dbt-edith was started in, and each found
  termVenv: undefined,        // the terminal's VIRTUAL_ENV at its last prompt, '' for none, undefined unheard (0069)
  shellName: '',              // /api/meta's name for the terminal's shell, bash or zsh, '' until it answers (0073)
  termPrompt: undefined,      // true at bash's prompt, false once Enter hands its line on, undefined unheard (0073)
  venvAsked: undefined,       // the last folder the terminal named that sent the page to read the venvs again
  preview: null,              // path of the reusable preview tab, VS Code style
  back: [], fwd: [],          // where following a link came from, a browser's two stacks
  rows: new Map(),            // tree rows currently rendered, by path
  gitMod: new Set(), gitUnt: new Set(),
  gitModDirs: new Set(), gitUntDirs: new Set(),
  gitPrefixes: [], gitKey: '',
  git: null,                  // full payload of /api/git
  artifactCm: { compiled: null, run: null },  // read-only editors for the two files under target/
  envs: null,                 // payload of /api/envs
  env: '',                    // this tab's chosen .env file, '' for the manifest as parsed
  node: null,                 // catalog: node currently displayed
  catTab: 'preview',          // catalog: preview | columns
  graphMode: 'model',         // lineage canvas: model | column | select
  colFocus: null,             // {id, column} when the canvas shows columns
  selectSub: null,            // last /api/select payload, for the two copy buttons
  selectAsk: 0,               // bumped per run, so only the latest answer is drawn
  selectHistory: [],          // expressions typed this session, newest first
  selectHistAt: -1,           // where the arrow keys are in that history, -1 being the empty line
  colHighlight: '',           // column row to mark in the Columns table
  colSort: 'az',              // catalog: az | tests | key
  colReveal: '',              // the node whose Columns draw scrolls to colHighlight's row
  colFilter: '',              // catalog: what the Columns filter holds, kept from one node to the next
  dock: 'lineage',            // the dock tab on screen
  lastPane: '',               // the pane last clicked or focused, for a Cmd+F with nothing focused
  keysReturn: null,           // where focus goes back to when the shortcut list closes
  sidecar: null,              // payload of /api/sidecar: whether Snowflake is picked, and its script
  sidecarPoll: 0,             // the timer asking again while the script starts
  collin: null,               // the collin part of /api/meta: installed, stale, and its last run
  collinPoll: 0,              // the timer asking again while a run started by a pick goes on
  update: null,               // payload of /api/update: the newest dbt-edith known, and how to install it
  updatePoll: 0,              // the timer asking again, every few seconds while a look is on its way
  updateMenu: null,           // Edith's menu, drawn again while open when an answer comes in
  build: '',                  // the build stamp, under the version in Edith's menu
  cllSources: [],             // column-lineage caches /api/meta offers, newest first
  cllActive: '',              // file name of the one the graph holds, '' for none
  cllTool: '',                // fusion | collin | snowflake | '' for another producer or none
  features: null,             // { snowflake, snowflake_set }: which feature families are on
  profiles: null,             // { path, found }: where dbt looks for its profile, null with nowhere to look
  colAsk: 0,                  // bumped on every column click, so only the latest answer is drawn
  colAnswered: false,         // Snowflake has answered once on this page, so no sign-in tab is expected
  history: null,              // the query history on screen: rows, more, session, targets, busy, error
  historyRole: null,          // whose queries it shows: null for the target's role, '*' for all, or a role
  historyAsk: 0,              // bumped per history request, so only the latest answer is drawn
  jobsRun: 0,                 // bumped per Jobs request, so only the latest answer is drawn
  jobsKeys: null,             // the Airflow keys and the shape the server proposes (0063)
  sfSession: null,            // { org, account, user, role } of the script's connection, once asked
  cloneFrom: null,            // where this tab last copied from: a .env file, '' for built, null before any
  cloneMode: 'clone',         // clone | ctas, as last chosen in this tab
  nodeCache: new Map(),       // /api/node payloads, by query: hover asks far more often than click
  nodeGen: 0,                 // bumped when the manifest or a .env file changes under the cache
  vars: null,                 // payload of /api/vars, for the editor's var() marks
  outline: null,              // { path, nodes } scanned for the breadcrumb's symbol half
  crumbLine: -1,              // the line that half was last drawn for
  version: '',                // this build's version, named in an exported graph
  keyless: false,             // the server refused this tab for want of its launch key (0052)
  collinSeen: undefined,      // collin's run counter when its report was last shown, for the news
  reports: null,              // /api/reports: which reports have something to show, and when written
  reportsAt: undefined,       // the newest report file the freshness poll last named
  reportsOpen: false,         // the panel is rolled down over the editor and the dock (0064)
  reportsSection: '',         // the section on screen in it, or last on screen this session
  reportsShown: null,         // the index entry the section on screen was read at
  reportsReturn: null,        // where focus goes back to when the panel is closed from the keyboard
  reportsAsk: 0,              // bumped per section read, so only the latest answer is drawn
  reportsDrawn: '',           // the section whose answer is in the panel, '' while one is read
  reportsPlace: {},           // per section, its open folds and its scroll, for when it is shown again
  runSeen: undefined,         // the last run's file time when its section was last shown, for the news
  jobsSeen: undefined,        // how many models no scheduled job ran when the coverage was last shown
  jobsReveal: null,           // { id, key, index } the step a Catalog chip asked the Jobs tab to show, { id, maybe } for the unresolved
  jobsFor: null,              // the node a Catalog chip opened the Jobs tab on, when not the focus, while the Catalog shows it
  catJobs: new Map(),         // per node, the jobs answer its Catalog section drew, until the jobs may have changed
  catJobsGen: 0,              // counts those changes, so an answer asked for before one is dropped
  tour: null,                 // the guided tour on screen: its steps, the one shown, and the page as it was (0074)
  tourOfferHeld: false,       // the offer waits for a manifest with a model, and is asked for again when one lands (0078)
};

// ------------------------------------------------------------------ util --
/* A route that can fail in several ways answers with JSON, and its fields end
   up on the Error: the message stays readable either way. */
function apiError(text, fallback) {
  try {
    const body = JSON.parse(text);
    if (body && body.error) return Object.assign(new Error(body.error), body);
  } catch { /* a plain sentence, which is the usual case */ }
  return new Error(text || fallback);
}

const api = {
  async get(path) {
    const r = await fetch(path);
    if (!r.ok) throw apiError(await refusal(r), r.statusText);
    return r.json();
  },
  async send(path, method, body) {
    const r = await fetch(path, { method, headers: { 'content-type': 'application/json' }, body: JSON.stringify(body) });
    if (!r.ok) throw apiError(await refusal(r), r.statusText);
    return r.json();
  },
};

/* The body of a refused request. A 401 is the server saying this tab never got
   its launch key: every later request will fail the same way, so it is said
   once, in a bar that stays, rather than in a toast per request (0052). */
async function refusal(r) {
  const text = await r.text();
  if (r.status === 401) keyMissing(text);
  return text;
}

function keyMissing(text) {
  if (S.keyless) return;
  S.keyless = true;
  const bar = document.createElement('div');
  bar.className = 'keyless';
  bar.setAttribute('role', 'alert');
  bar.textContent = (text || 'no key for this server in this tab')
    + '. The terminal where dbt-edith runs shows it after "open". A link from an earlier launch no longer works.';
  document.body.prepend(bar);
}

/* One /api/node payload per node, shared by the catalog, the ref() marks and the
   hover card. The server rediscovers the .env files and walks the graph twice on
   every request, which a click could afford and a hover cannot. The promise is
   what is stored, not the value: a hover and the click that follows it 30ms
   later are then one request rather than two.

   The entry expires, because the server reloads the graph by itself when dbt
   rewrites the artifacts (watch_artifacts, every three seconds) and never tells
   the browser. Before this cache existed every click refetched, so a `dbt build`
   in the terminal showed up on the next click; an entry that outlived that poll
   would take that back. Three seconds is the server's own interval, so the cache
   is never staler than the thing it is caching. */
const NODE_CACHE_MAX = 200;
const NODE_CACHE_TTL = 3000;

function nodeKey(q) {
  return q.id ? 'id:' + q.id : 'file:' + q.file;
}

function nodeDetail(q) {
  const key = nodeKey(q);
  const hit = S.nodeCache.get(key);
  if (hit && hit.gen === S.nodeGen && Date.now() - hit.at < NODE_CACHE_TTL) return hit.p;
  const path = q.id ? '/api/node?id=' + encodeURIComponent(q.id) : '/api/node?file=' + encodeURIComponent(q.file);
  // A failed request must not be remembered as the answer.
  const p = api.get(path).catch((e) => { S.nodeCache.delete(key); throw e; });
  S.nodeCache.set(key, { gen: S.nodeGen, at: Date.now(), p });
  if (S.nodeCache.size > NODE_CACHE_MAX) S.nodeCache.delete(S.nodeCache.keys().next().value);
  return p;
}

/* The project's vars, plus any env var asked for by name. Keyed by the chosen
   environment as well, because the same var resolves differently under each. */
function fetchVars(names) {
  const key = (S.env || '') + '|' + (names || '');
  if (!S.vars) S.vars = new Map();
  const hit = S.vars.get(key);
  if (hit) return hit;
  let url = '/api/vars?env=' + encodeURIComponent(S.env || '');
  if (names) url += '&names=' + encodeURIComponent(names);
  const p = api.get(url).catch((e) => { S.vars.delete(key); throw e; });
  S.vars.set(key, p);
  return p;
}

/* The manifest was reloaded, or a file the payload derives from was saved. */
function dropNodeCache() {
  S.nodeGen++;
  S.nodeCache.clear();
  S.vars = null;
}

function toast(msg, kind = '') {
  const t = document.createElement('div');
  t.className = 'toast ' + kind;
  t.textContent = msg;
  $('#toast').appendChild(t);
  setTimeout(() => t.remove(), 3200);
}

/* Same colour rule as the lineage canvas, so a model looks the same everywhere. */
const dot = (n) => {
  const d = document.createElement('span');
  d.className = 'dot';
  d.style.background = Lineage.nodeColor(typeof n === 'string' ? { kind: n } : n);
  d.title = Lineage.matLabel(typeof n === 'string' ? { kind: n } : n);
  return d;
};

/* Whether CodeMirror took this for a Mac. Asked of CodeMirror rather than of
   the user agent, so a key the shortcut list names as ⌘ is one CodeMirror
   binds to Cmd, and never a Ctrl it is waiting for instead. */
const IS_MAC = CodeMirror.keyMap.default === CodeMirror.keyMap.macDefault;
/* 'mac', 'windows' or 'other', for the keys that differ between the three:
   the terminal's line keys and the rows of the shortcut list naming them. */
const KEYS_OS = IS_MAC ? 'mac' : /^Win/.test(navigator.platform || '') ? 'windows' : 'other';

// ---------------------------------------------------------------- editor --
/* Jinja in a model, one token at a time, from an opening delimiter to its close.
   Each piece is styled by what it is, so a call reads apart from a variable, a
   keyword or a string. `jinja-dbt` is what dbt itself provides, taken from its list
   of Jinja functions and context variables: `ref`, `this` and `adapter` look alike,
   `dbt_utils.star` does not. */
const JINJA_KEYWORDS = new Set(('and as block break call continue do docs elif else endblock endcall ' +
  'enddocs endfilter endfor endif endmacro endmaterialization endraw endset endsnapshot endtest ' +
  'endwith extends filter for from if import in include is macro materialization not or raw ' +
  'recursive set snapshot test with without').split(' '));
const JINJA_ATOMS = new Set(['true', 'false', 'none', 'True', 'False', 'None']);
const DBT_GLOBALS = new Set(('adapter api builtins config dbt_version debug env_var exceptions ' +
  'execute flags fromjson fromyaml graph invocation_id is_incremental load_result local_md5 log ' +
  'model modules print project_name ref return run_query run_started_at selected_resources set ' +
  'set_strict source statement target this thread_id tojson toyaml var zip zip_strict').split(' '));

function jinjaStart() {
  return { close: null, quote: null, depth: 0, tag: false, dot: false, pipe: false };
}

function jinjaToken(stream, state) {
  if (!state.close) {
    // dbtSqlMode only calls in here outside a block when a delimiter opens.
    const open = stream.match(/^\{([{%#])[-+]?/);
    Object.assign(state, jinjaStart(), { close: { '{': '}}', '%': '%}', '#': '#}' }[open[1]] });
    state.tag = open[1] === '%';
    return open[1] === '#' ? 'jinja-comment' : 'jinja-delim';
  }
  if (state.close === '#}') {
    if (stream.match(/^.*?#\}/)) state.close = null;
    else stream.skipToEnd();
    return 'jinja-comment';
  }
  if (state.quote || stream.match(/^['"]/, false)) {
    if (!state.quote) state.quote = stream.next();
    while (!stream.eol()) {
      const ch = stream.next();
      if (ch === '\\') stream.next();
      else if (ch === state.quote) { state.quote = null; break; }
    }
    state.tag = state.dot = state.pipe = false;
    return 'jinja-string';
  }
  const end = stream.match(/^[-+]?(?:\}\}|%\})/, false);
  if (end && end[0].endsWith(state.close)) {
    stream.match(end[0]);
    state.close = null;
    return 'jinja-delim';
  }
  if (stream.eatSpace()) return null;
  if (stream.match(/^\d+(?:\.\d+)?/)) {
    state.tag = state.dot = state.pipe = false;
    return 'jinja-number';
  }
  const word = stream.match(/^[A-Za-z_]\w*/);
  if (word) {
    const w = word[0];
    const { tag, dot, pipe } = state;
    state.tag = state.dot = state.pipe = false;
    // The first word of a {% %} block is its tag, whatever else it could be.
    if (tag) return 'jinja-keyword';
    // A named argument before a keyword: dbt_utils.star(from=...) is common.
    if (state.depth > 0 && stream.match(/^\s*=(?!=)/, false)) return 'jinja-param';
    if (JINJA_KEYWORDS.has(w) && !stream.match('(', false)) return 'jinja-keyword';
    const called = stream.match(/^\s*\(/, false);
    // After a dot it is an attribute or a method, never a dbt global.
    if (dot) return called ? 'jinja-fn' : 'jinja-var';
    if (JINJA_ATOMS.has(w)) return 'jinja-atom';
    if (DBT_GLOBALS.has(w)) return 'jinja-dbt';
    return called || pipe ? 'jinja-fn' : 'jinja-var';
  }
  const ch = stream.next();
  if ('([{'.includes(ch)) state.depth++;
  else if (')]}'.includes(ch) && state.depth > 0) state.depth--;
  state.tag = false;
  state.dot = ch === '.';
  state.pipe = ch === '|';
  return 'jinja-punct';
}

/* SQL with Jinja in it. The SQL mode is never shown the Jinja: while it tokenizes,
   the line is cut short at the next delimiter, as CodeMirror's multiplex addon
   does. Shown the whole line, it took the apostrophe in {# don't #} for the start
   of a string and coloured the rest of the file as one. */
function dbtSqlMode(sql) {
  return {
    startState: () => ({ sql: CodeMirror.startState(sql), jinja: jinjaStart() }),
    copyState: (s) => ({ sql: CodeMirror.copyState(sql, s.sql), jinja: { ...s.jinja } }),
    token(stream, state) {
      if (state.jinja.close || stream.match(/^\{[{%#]/, false)) return jinjaToken(stream, state.jinja);
      const at = stream.string.slice(stream.pos).search(/\{[{%#]/);
      if (at < 0) return sql.token(stream, state.sql);
      const line = stream.string;
      stream.string = line.slice(0, stream.pos + at);
      try {
        return sql.token(stream, state.sql);
      } finally {
        stream.string = line;
      }
    },
    indent: sql.indent && ((state, textAfter, line) => sql.indent(state.sql, textAfter, line)),
    innerMode: (state) => ({ state: state.sql, mode: sql }),
  };
}

CodeMirror.defineMode('dbt-sql', (cfg) => dbtSqlMode(CodeMirror.getMode(cfg, 'text/x-sql')));

/* ref('x') / ref('pkg','x') / source('src','table') -> clickable marks.
   Marks live on the Doc, so they survive tab switches and are scanned once. */
const REF_RE = /\b(ref|source)\s*\(\s*(['"])([^'"\n]+)\2\s*(?:,\s*(['"])([^'"\n]+)\4\s*)?\)/g;
const TOKEN_RE = /[A-Za-z0-9_]+/g;

/* Blanks out {# ... #} blocks, keeping both offsets and line breaks intact: dbt
   never evaluates what is inside them, so neither should the link scanner.
   Only where Jinja's lexer opens one, in the text between blocks: a `{#` inside
   a string of a {{ }} or {% %} block, or in a raw block, is text, and masking
   from there cut the string in half, so every quote after it was read the
   wrong way round to the end of the file. */
function maskJinjaComments(src) {
  const opener = /\{([{%#])/g;
  let out = '', at = 0, m;
  while ((m = opener.exec(src)) !== null) {
    if (m[1] === '#') {
      const close = src.indexOf('#}', opener.lastIndex);
      if (close < 0) break;
      out += src.slice(at, m.index) + src.slice(m.index, close + 2).replace(/[^\n]/g, ' ');
      at = opener.lastIndex = close + 2;
    } else if (m[1] === '%' && /^[-+]?\s*raw\s*[-+]?%\}/.test(src.slice(opener.lastIndex, opener.lastIndex + 16))) {
      const endraw = /\{%[-+]?\s*endraw\s*[-+]?%\}/g;
      endraw.lastIndex = opener.lastIndex;
      opener.lastIndex = endraw.exec(src) ? endraw.lastIndex : src.length;
    } else {
      opener.lastIndex = Math.min(src.length, jinjaBlockEnd(src, opener.lastIndex, m[1] === '%' ? '%}' : '}}') + 2);
    }
  }
  return out + src.slice(at);
}

/* Where a block opened before `from` closes: the first `close` outside a
   string with every bracket shut, since Jinja's lexer reads the `}}` ending
   `{'a': {'b': 1}}` as two braces of the dict. A block still being typed, whose
   brackets never balance, ends at its first `close`, or at the end of the text. */
function jinjaBlockEnd(text, from, close) {
  let quote = '', depth = 0, first = -1;
  for (let i = from; i < text.length - 1; i++) {
    const c = text[i];
    if (quote) {
      if (c === '\\') i++;
      else if (c === quote) quote = '';
      continue;
    }
    if (c === '"' || c === "'") { quote = c; continue; }
    if (c === close[0] && text[i + 1] === '}') {
      if (depth <= 0) return i;
      if (first < 0) first = i;
    }
    if ('([{'.includes(c)) depth++;
    else if (')]}'.includes(c)) depth--;
  }
  return first >= 0 ? first : text.length;
}

/* Explicit ref() / source() calls, with the exact range of each name. */
function scanCalls(text) {
  const found = [];
  let m;
  REF_RE.lastIndex = 0;
  while ((m = REF_RE.exec(text)) !== null) {
    const isSource = m[1] === 'source';
    const q1 = m[2] + m[3] + m[2];
    const o1 = m[0].indexOf(q1);
    const a1 = [m.index + o1 + 1, m.index + o1 + 1 + m[3].length];
    let a2 = null;
    if (m[5] !== undefined) {
      const q2 = m[4] + m[5] + m[4];
      const o2 = m[0].indexOf(q2, o1 + q1.length);
      a2 = [m.index + o2 + 1, m.index + o2 + 1 + m[5].length];
    }
    if (isSource && !a2) continue;
    const name = isSource ? `${m[3]}.${m[5]}` : (m[5] !== undefined ? m[5] : m[3]);
    found.push({ name, ranges: isSource ? [a1, a2] : [a2 || a1] });
  }
  return found;
}

/* var('x') / var('x', default) / env_var('X') / env_var('X', 'd') -> hover marks.
   The dbt-sql mode gives var, env_var, ref and source the same `jinja-dbt`
   token, so the token type cannot tell them apart. The scanner can. */
const VAR_RE = /\b(var|env_var)\s*\(\s*(['"])([^'"\n]+)\2\s*(?:,([^)\n]*))?\)/g;

function scanVars(text) {
  const found = [];
  let m;
  VAR_RE.lastIndex = 0;
  while ((m = VAR_RE.exec(text)) !== null) {
    // \b matches after a dot too, so dbt_utils.var('x') would otherwise count.
    if (text[m.index - 1] === '.') continue;
    // The whole call, not just the name inside the quotes the way scanCalls
    // marks a ref(). A ref's name is the thing you click, so a tight target is
    // right there; a variable has nothing to click, and `var('x')` reads as one
    // word, so anything less than the whole of it is a target you have to aim at.
    found.push({
      kind: m[1],
      name: m[3],
      fallback: m[4] === undefined ? '' : m[4].trim(),
      ranges: [[m.index, m.index + m[0].length]],
    });
  }
  return found;
}

/* The line under a variable's value, saying where the value came from. Pure, so
   the wording can be checked without a DOM. `env` is the chosen file's name, or
   '' when this tab is showing the manifest as dbt parsed it. */
function varNote(row, env) {
  const names = (row.vars || []).join(', ');
  const where = env || 'any environment file';
  if (row.redacted) {
    return row.status === 'env' || row.status === 'placeholder'
      ? `set in ${where}, hidden because the name reads as a credential`
      : 'hidden because the name reads as a credential';
  }
  switch (row.status) {
    case 'env':
      return row.default_used ? 'the default written in the call, not from a file' : `read from ${where}`;
    case 'missing':
      return env ? `${names} is not set in ${env}` : `${names} needs an environment, and none is selected`;
    case 'placeholder':
      return `${names} is a placeholder in ${where}, not a real value`;
    case 'unevaluated':
      return 'not evaluated: a secret, or Jinja this does not read';
    default:
      return '';
  }
}

/* automate_dv declares parents through a YAML metadata block rather than ref(),
   in more shapes than are worth chasing with one regex per shape:

       source_model: "stg_x"           source_model = ["a", "b"]
       source_model: stg_x             satellites:
       as_of_date_table: as_of_dates     sat_x__y:

   So the vocabulary comes from the manifest (the node's real parents) and only
   those names are looked for in the text. A name is linked when it is quoted,
   is a YAML key, or is a YAML value, which keeps short model names such as
   "order" or "status" from matching every column that happens to share the name.
*/
function scanVocabulary(text, vocab) {
  const found = [];
  if (!vocab.size) return found;
  let m;
  TOKEN_RE.lastIndex = 0;
  while ((m = TOKEN_RE.exec(text)) !== null) {
    const name = m[0];
    if (!vocab.has(name)) continue;
    const s = m.index, e = s + name.length;
    const prev = text[s - 1], next = text[e];
    const quoted = (prev === '"' && next === '"') || (prev === "'" && next === "'");
    const isKey = /^\s*:/.test(text.slice(e, e + 8));
    const isValue = prev !== '.' && /[:=]\s*$/.test(text.slice(Math.max(0, s - 32), s));
    if (quoted || isKey || isValue) found.push({ name, ranges: [[s, e]] });
  }
  return found;
}

/* The names a properties file declares nodes under, each to be linked to its
   node: a table under `sources:`, named `source.table` the way the graph names
   it, and an entry of one of these top-level lists. Only a `name:` on an item
   of such a list counts, so a column named like a model is never taken for
   one, and the `models:` of dbt_project.yml, a mapping of configs, declares
   nothing. Entries under `macros:` come back as well, for the macro links. */
const DECLARING_LISTS = new Set(['models', 'seeds', 'snapshots', 'analyses', 'exposures']);

function yamlDeclared(text) {
  const nodes = yamlOutline(text);
  const lines = text.split('\n');
  const starts = [];
  let offset = 0;
  for (const l of lines) { starts.push(offset); offset += l.length + 1; }
  // The key an item hangs from, when that key is a top-level list.
  const listOf = (item) => {
    const list = item.parent >= 0 ? nodes[item.parent] : null;
    return list && list.parent < 0 && list.kind === 'seq' ? list.label : '';
  };
  const nameUnder = (item) => {
    for (let j = item + 1; j < nodes.length && nodes[j].line <= nodes[item].endLine; j++) {
      if (nodes[j].parent !== item || nodes[j].label !== 'name') continue;
      const v = yamlNameAt(lines[nodes[j].line], nodes[j].col);
      return v ? v.value : '';
    }
    return '';
  };
  const sourceNames = new Map();
  const found = [];
  for (const n of nodes) {
    if (n.label !== 'name' || n.parent < 0) continue;
    const v = yamlNameAt(lines[n.line], n.col);
    if (!v) continue;
    const ranges = [[starts[n.line] + v.from, starts[n.line] + v.to]];
    const item = nodes[n.parent];
    const list = listOf(item);
    if (DECLARING_LISTS.has(list)) {
      found.push({ kind: 'node', name: v.value, ranges });
      continue;
    }
    if (list === 'macros') {
      found.push({ kind: 'macro', name: v.value, ranges });
      continue;
    }
    // A table's item hangs from `tables`, which sits on an item of `sources`.
    const tables = item.parent >= 0 ? nodes[item.parent] : null;
    if (!tables || tables.label !== 'tables' || tables.kind !== 'seq' || tables.parent < 0) continue;
    if (listOf(nodes[tables.parent]) !== 'sources') continue;
    if (!sourceNames.has(tables.parent)) sourceNames.set(tables.parent, nameUnder(tables.parent));
    const source = sourceNames.get(tables.parent);
    if (source) found.push({ kind: 'source', name: `${source}.${v.value}`, ranges });
  }
  return found;
}

/* The value of a `name:` key written at `col`, and where it sits in the line.
   Null for anything but a plain or quoted scalar: Jinja, a flow collection, an
   anchor or a block scalar never names a node the way it is written. */
function yamlNameAt(line, col) {
  const colon = line.indexOf(':', col);
  if (colon < 0) return null;
  let from = colon + 1;
  while (line[from] === ' ' || line[from] === '\t') from++;
  let to;
  const q = line[from];
  if (q === '"' || q === "'") {
    to = line.indexOf(q, from + 1);
    if (to < 0) return null;
    from++;
  } else {
    if (q === undefined || '[{|>&*!#'.includes(q)) return null;
    const comment = line.slice(from).search(/[ \t]#/);
    to = comment < 0 ? line.length : from + comment;
    while (to > from && /\s/.test(line[to - 1])) to--;
  }
  const value = line.slice(from, to);
  if (!value || value.includes('{{') || value.includes('{%')) return null;
  return { value, from, to };
}

/* The blocks of dbt_project.yml whose keys are folders. `vars`, `sources`,
   `flags`, `dispatch`, `exposures`, `metrics`, `semantic-models` and
   `saved-queries` nest the same way and are left alone: their keys are names,
   never paths. Both spellings of the test block are here, since dbt-core 1.8
   renamed it and projects carry either. */
const PATH_BLOCKS = new Set(['models', 'seeds', 'snapshots', 'analyses', 'macros', 'tests', 'data_tests']);

/* dbt's own config names. They decide nothing about what a key is: dbt reads a
   key holding a mapping as a folder unless it starts with `+`, whatever its
   name (0039). What the list does is name the mistake that rule punishes, a
   `docs:` or `meta:` written without its `+`, which dbt then looks for as a
   folder and applies to nothing, so the card of such a key can say why.

   Taken from what dbt writes: every key of `config` on a model, a seed and a
   test node of an 18 825 node manifest, plus the snapshot and seed configs its
   documentation lists, which that project has none of. A config missing from
   it costs only that sentence in a card. */
const DBT_CONFIGS = new Set([
  'access', 'alias', 'as_columnstore', 'batch_size', 'begin', 'check_cols', 'classifiers',
  'cluster_by', 'column_types', 'compute', 'concurrent_batches', 'contract', 'database',
  'dbt_valid_to_current', 'delimiter', 'docs', 'enabled', 'error_if', 'event_time', 'fail_calc',
  'file_format', 'freshness', 'full_refresh', 'grants', 'group', 'hard_deletes',
  'incremental_predicates', 'incremental_strategy', 'indexes', 'invalidate_hard_deletes',
  'latest_version_pointer', 'limit', 'lookback', 'materialized', 'meta', 'on_configuration_change',
  'on_error', 'on_schema_change', 'packages', 'persist_docs', 'post-hook', 'post_hook',
  'pre-hook', 'pre_hook', 'primary_key', 'query_tag', 'quote_columns', 'quoting', 'refresh_mode',
  'schema', 'severity', 'snapshot_meta_column_names', 'sql_header', 'static_analysis',
  'store_failures', 'store_failures_as', 'strategy', 'tags', 'target_database', 'target_lag',
  'target_schema', 'transient', 'unique_key', 'updated_at', 'warn_if', 'where',
]);

/* The key written at `col`, and where its own text sits in the line. One layer
   of quotes is left out, so a click on a quote does nothing, the way
   yamlNameAt treats a value. */
function yamlKeyRange(line, col) {
  const written = line.slice(col);
  const kv = yamlKey(written);
  if (!kv || !kv.key) return null;
  const q = written[0];
  const quoted = (q === '"' || q === "'") && written[1 + kv.key.length] === q;
  const from = col + (quoted ? 1 : 0);
  return [from, from + kv.key.length];
}

/* Every key in dbt_project.yml that names a folder, as the chain of keys the
   server places on disk (0036). One entry per key, so `models:` links to its
   root and each level below links to its own folder.

   dbt's rule decides, in dbt-core and in Fusion alike: a key is a path segment
   when it holds a mapping and does not start with `+`, and a config otherwise,
   whatever its name (0039). So a folder named `contract` or `schema` is a
   folder, and a `docs:` written without its `+` is one too, which dbt looks for
   and applies to nothing. A key under a `+config`, or under a sequence item,
   is out: nothing below either is a path.

   Each entry also carries where its key sits and the configs set directly
   under it, which is what the breadcrumb's list of the keys reaching a file
   shows (0038). */
function projectPathKeys(text) {
  const nodes = yamlOutline(text);
  const lines = text.split('\n');
  const starts = [];
  let offset = 0;
  for (const l of lines) { starts.push(offset); offset += l.length + 1; }
  const kids = nodes.map(() => []);
  nodes.forEach((n, j) => { if (n.parent >= 0) kids[n.parent].push(j); });

  const out = [];
  for (let i = 0; i < nodes.length; i++) {
    if (nodes[i].kind !== 'map') continue;
    const chain = [];
    let at = i;
    let ok = true;
    while (nodes[at].parent >= 0) {
      const parent = nodes[nodes[at].parent];
      // A sequence item is not a mapping key, whatever it holds.
      if (parent.kind === 'seq') { ok = false; break; }
      chain.unshift(nodes[at].label);
      at = nodes[at].parent;
    }
    if (!ok) continue;
    const block = nodes[at].label;
    if (!PATH_BLOCKS.has(block)) continue;
    if (chain.some((seg) => seg.startsWith('+'))) continue;
    const range = yamlKeyRange(lines[nodes[i].line], nodes[i].col);
    if (!range) continue;
    const base = starts[nodes[i].line];
    // The same rule the other way round: what a level sets is a `+`, or a value
    // that is no mapping, whatever the name.
    const sets = kids[i]
      .filter((j) => nodes[j].label.startsWith('+') || nodes[j].kind !== 'map')
      .map((j) => ({ name: nodes[j].label, value: yamlLineValue(lines[nodes[j].line], nodes[j].col) }));
    out.push({
      block, chain, ranges: [[base + range[0], base + range[1]]],
      line: nodes[i].line, col: range[0], sets,
    });
  }
  return out;
}

/* The value a key holds on its own line, as written, less a trailing comment
   and one layer of quotes. Empty when the value sits on the lines below, as a
   mapping, a list or a `|` block, since one line of a menu cannot hold it. A
   quote only opens where YAML could start a quoted scalar, so the apostrophe
   in `don't` hides no comment. */
function yamlLineValue(line, col) {
  const kv = yamlKey(line.slice(col));
  if (!kv) return '';
  let v = kv.value;
  let quote = '';
  for (let i = 0; i < v.length; i++) {
    const c = v[i];
    if (quote) { if (c === quote) quote = ''; continue; }
    if ((c === '"' || c === "'") && (i === 0 || '[{, '.includes(v[i - 1]))) { quote = c; continue; }
    if (c === '#' && (i === 0 || v[i - 1] === ' ' || v[i - 1] === '\t')) { v = v.slice(0, i).trimEnd(); break; }
  }
  if (v[0] === '|' || v[0] === '>') return '';
  return unquote(v);
}

/* The project's `name:`, which is the first level of every fqn it owns. */
function projectNameIn(text) {
  const lines = text.split('\n');
  const top = yamlOutline(text).find((n) => n.parent < 0 && n.label === 'name');
  return (top && yamlLineValue(lines[top.line], top.col)) || null;
}

/* The blocks whose keys can reach a file, and the roots dbt reads them under
   when a package's own dbt_project.yml is not read, as 0036 places them. Macros
   are not here: a macro is no node, and a file holds several. */
const REACHED_BLOCKS = ['models', 'seeds', 'snapshots', 'analyses'];
const PACKAGE_ROOTS = { models: ['models'], seeds: ['seeds'], snapshots: ['snapshots'], analyses: ['analyses'], tests: ['tests'] };

/* A file's fqn as dbt builds it: the project, the folders under the resource
   path it sits in, and its name without the extension. Read off the path, so
   it holds with no manifest at all, and it matched dbt's own for every model,
   seed and singular test of the 18 825 node project (0038).

   `roots` are the server's, one list per block. The longest root holding the
   file decides its kind, since one path may sit inside another. The test
   block's roots include the model roots, because a key there may name a
   model folder for the generic tests hanging on it (0036); those are left out
   here, so a model is never reached through `data_tests:`, which configures
   its tests rather than the model. A file under a package directory takes
   the package's name and the conventional roots. Null for a file that is no
   resource. */
function fileFqn(file, roots, name) {
  let parts = file.split('/');
  let head = name;
  let byBlock = roots || {};
  if ((parts[0] === 'dbt_packages' || parts[0] === 'dbt_modules') && parts.length > 2) {
    head = parts[1];
    parts = parts.slice(2);
    byBlock = PACKAGE_ROOTS;
  }
  if (!head) return null;
  const rel = parts.join('/');
  const models = byBlock.models || [];
  const candidates = [];
  for (const b of REACHED_BLOCKS) for (const root of byBlock[b] || []) candidates.push({ blocks: [b], root });
  for (const root of byBlock.tests || []) {
    if (!models.includes(root)) candidates.push({ blocks: ['tests', 'data_tests'], root });
  }
  let best = null;
  for (const c of candidates) {
    if (rel.startsWith(c.root + '/') && (!best || c.root.length > best.root.length)) best = c;
  }
  if (!best) return null;
  const segs = rel.slice(best.root.length + 1).split('/');
  const leaf = segs.pop();
  const dot = leaf.lastIndexOf('.');
  segs.push(dot > 0 ? leaf.slice(0, dot) : leaf);
  return { blocks: best.blocks, fqn: [head, ...segs] };
}

/* The keys of dbt_project.yml that reach a file, broadest first. dbt's rule:
   a key applies to every node whose fqn its chain begins, so a model three
   folders below the only key written is reached by that key and by those
   above it, and the deepest one is not necessarily its own folder's. The key
   of the block itself applies to the whole kind, packages included. Where the
   config is written, never what wins: a `config()` in the file or in its
   properties YAML comes first, and tags, meta and hooks add up. */
function configLevels(keys, at) {
  if (!at) return [];
  return keys
    .filter((k) => at.blocks.includes(k.block) && k.chain.length <= at.fqn.length
      && k.chain.every((seg, i) => seg === at.fqn[i]))
    .sort((a, b) => a.chain.length - b.chain.length);
}

/* What the card of a key placed nowhere says. A key named after a config is
   almost always that config without its `+`, which dbt reads as a folder and
   so never applies: worth saying in those words, since nothing else will until
   dbt warns at parse time, and Fusion refuses the keys under it outright. */
function missingKeyNote(t) {
  const roots = (t.roots || []).join(', ');
  if (!roots) return `${t.block} has no readable path in this file, so there is nothing to look under.`;
  const last = t.chain[t.chain.length - 1];
  if (last && DBT_CONFIGS.has(last)) {
    return `Without its +, dbt reads ${last}: as a folder, finds none under ${roots}, and applies `
      + `nothing under it. Written +${last}:, it is the config.`;
  }
  return `No such folder under ${roots}. dbt reports a config path that matches nothing as a warning when it parses the project.`;
}

/* Calls inside Jinja, as the dotted name before a `(`: `hub`, `automate_dv.hub`.
   Only inside a block, since outside one `coalesce(` is SQL, and a project
   macro named like a SQL function would otherwise link every use of it.
   Whether a name is a macro, and which one, is the server's answer: the same
   bare `hub` is the project's own or automate_dv's depending on what the
   project defines (0028). A name after a dot is a method, after a pipe a
   filter, and after `macro` the definition itself; none of those is a call.
   The range is the macro's own name, without its package: the SQL mode draws
   `automate_dv`, the dot and `hub` as three tokens, and a mark across them
   would light up one piece at a time under the pointer. */
const JINJA_DEFINERS = new Set(['macro', 'test', 'materialization']);

/* `yaml` for a properties or project file, where Jinja inside a double-quoted
   scalar writes its own quotes as `\"`: YAML hands Jinja a plain `"`, so that
   is what the scan reads, one space and one quote keeping every offset. */
function scanMacroCalls(text, yaml = false) {
  if (yaml) text = text.replace(/\\"/g, ' "');
  const found = new Map();
  const opener = /\{([{%])[-+]?/g;
  let m;
  while ((m = opener.exec(text)) !== null) {
    const tag = m[1] === '%';
    const end = jinjaBlockEnd(text, opener.lastIndex, tag ? '%}' : '}}');
    let i = opener.lastIndex;
    let first = tag;           // the first word of a {% %} block is its tag
    let defines = false;       // and after `macro`, the next name is being defined
    let quote = '';
    while (i < end) {
      const c = text[i];
      if (quote) {
        if (c === '\\') i++;
        else if (c === quote) quote = '';
        i++;
        continue;
      }
      if (c === '"' || c === "'") { quote = c; i++; continue; }
      if (!/[A-Za-z_]/.test(c)) { i++; continue; }
      const from = i;
      while (i < end && /[\w.]/.test(text[i])) i++;
      const chain = text.slice(from, i);
      if (first) {
        first = false;
        defines = JINJA_DEFINERS.has(chain);
        // Nothing in a raw block is Jinja, so the scan resumes after it.
        if (chain === 'raw') {
          const close = /\{%[-+]?\s*endraw\s*[-+]?%\}/g;
          close.lastIndex = end;
          const r = close.exec(text);
          opener.lastIndex = r ? close.lastIndex : text.length;
          break;
        }
        continue;
      }
      if (defines) { defines = false; continue; }
      let j = i;
      while (text[j] === ' ' || text[j] === '\t') j++;
      if (text[j] !== '(' || text[from - 1] === '.') continue;
      let k = from - 1;
      while (k >= 0 && /\s/.test(text[k])) k--;
      if (text[k] === '|') continue;
      const parts = chain.split('.');
      if (parts.length > 2 || parts.some((p) => !p)) continue;
      if (!found.has(chain)) found.set(chain, []);
      found.get(chain).push([from + chain.lastIndexOf('.') + 1, from + chain.length]);
    }
    if (opener.lastIndex < end) opener.lastIndex = end;
  }
  return [...found].map(([call, ranges]) => ({ call, ranges }));
}

/* Where `{% macro name(` sits, as a line and a column, so a link lands on the
   definition rather than at the top of a file that may hold twenty. Comments
   are masked first: a commented-out copy is not the macro dbt reads. */
function macroDefLine(text, name) {
  const esc = name.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const m = new RegExp('(\\{%[-+]?\\s*macro\\s+)' + esc + '\\s*\\(').exec(maskJinjaComments(text));
  if (!m) return null;
  const at = m.index + m[1].length;
  const before = text.slice(0, at);
  const line = before.split('\n').length - 1;
  return { line, ch: at - (before.lastIndexOf('\n') + 1) };
}

/* Where a data test is declared in a properties file (0041). The manifest
   gives a test no line, so it is looked for over the buffer: under the entry
   of `t.host`, the node whose YAML declares it, in the tests of its column,
   else in the entry's own, an item naming its generic, bare or through its
   package. Two items of one generic on one column differ by arguments no
   payload carries, so one whose `name:` is dbt's name for the test wins, and
   the first otherwise. Short of the item, the nearest thing found: the list,
   the column, the entry; nothing when the entry is not there. `end` is the
   item's last line, its arguments included, and the line itself otherwise. */
function testLine(text, t) {
  if (!t || !t.host || !t.test_name) return null;
  const nodes = yamlOutline(text), lines = text.split('\n');
  const kids = new Map();
  nodes.forEach((n, i) => {
    if (!kids.has(n.parent)) kids.set(n.parent, []);
    kids.get(n.parent).push(i);
  });
  const under = (i) => kids.get(i) || [];
  const key = (i, label) => (i == null ? null : under(i).find((j) => nodes[j].label === label));
  const value = (j) => yamlLineValue(lines[nodes[j].line], nodes[j].col);
  // dbt matches a column by name whatever its case, and a node by its name.
  const named = (list, name, fold) => (list == null ? null : under(list).find((i) => {
    const j = key(i, 'name');
    if (j == null) return false;
    const v = value(j);
    return fold ? v.toLowerCase() === name.toLowerCase() : v === name;
  }));
  const entry = (list, name) => {
    const i = named(list, name, false);
    return i != null ? i : named(list, name, true);
  };
  const tops = under(-1);
  let host = null;
  for (const top of tops) {
    if (host == null && DECLARING_LISTS.has(nodes[top].label)) host = entry(top, t.host);
  }
  // A source is `source.table` in the graph, a table under its source here.
  const dot = t.host.indexOf('.');
  if (host == null && dot > 0) {
    for (const top of tops) {
      if (host != null || nodes[top].label !== 'sources') continue;
      const src = entry(top, t.host.slice(0, dot));
      if (src != null) host = entry(key(src, 'tables'), t.host.slice(dot + 1));
    }
  }
  if (host == null) return null;

  const testsOf = (i) => {
    const d = key(i, 'data_tests');
    return d != null ? d : key(i, 'tests');
  };
  const col = t.column ? named(key(host, 'columns'), t.column, true) : null;
  const g = t.test_name;
  const same = (k) => {
    const v = unquote(String(k).replace(/\s+#.*$/, '').trim());
    return v === g || v.endsWith('.' + g);
  };
  // `- not_null`, `- relationships:` and its arguments, or an item naming
  // its generic under `test_name:`.
  const isTest = (i) => {
    const first = under(i)[0];
    if (first == null) return same(nodes[i].label);
    if (nodes[first].line === nodes[i].line && same(nodes[first].label)) return true;
    const tn = key(i, 'test_name');
    return tn != null && same(value(tn));
  };
  const called = (i) => {
    for (let j = i + 1; j < nodes.length && nodes[j].line <= nodes[i].endLine; j++) {
      if (nodes[j].label === 'name' && value(j) === t.name) return true;
    }
    return false;
  };
  const at = (i, whole) => ({ line: nodes[i].line, ch: nodes[i].col, end: whole ? nodes[i].endLine : nodes[i].line });
  // A generic on the whole model may name a column in its arguments, which
  // gives it a column in the manifest though it sits in the entry's tests.
  for (const list of [testsOf(col), testsOf(host)]) {
    if (list == null) continue;
    const found = under(list).filter(isTest);
    if (!found.length) continue;
    const mine = found.find(called);
    return at(mine != null ? mine : found[0], true);
  }
  return at([testsOf(col), col, testsOf(host), host].find((i) => i != null));
}

/* Whether a file defines a macro, a generic test or a materialization: the
   .sql files that carry no node of their own however fresh the manifest. Read
   from the buffer rather than asked of the manifest, so a macro written since
   the last parse counts as one too. */
function definesMacros(text) {
  return new RegExp('\\{%[-+]?\\s*(' + [...JINJA_DEFINERS].join('|') + ')\\s').test(maskJinjaComments(text));
}

async function markRefs(doc, path) {
  doc.getAllMarks().forEach((mk) => { if (mk.refTarget) mk.clear(); });
  const text = maskJinjaComments(doc.getValue());
  const calls = scanCalls(text);
  // In a properties file, the names that declare its nodes link to them too.
  const declared = path && modeFor(path) === 'text/x-yaml' && text.length <= OUTLINE_MAX
    ? yamlDeclared(text).filter((d) => d.kind !== 'macro')
    : [];

  // The manifest is the authority on what this file actually depends on.
  let parents = [];
  if (path) {
    try { parents = (await nodeDetail({ file: path })).parents; }
    catch { /* not a dbt node: ref() calls are still linked below */ }
  }
  const known = {};
  parents.forEach((p) => { known[p.name] = p; });

  const unknown = [...new Set([...calls, ...declared].map((c) => c.name))].filter((n) => !known[n]);
  const resolved = {};
  if (unknown.length) {
    try {
      const rows = await api.send('/api/resolve', 'POST', { names: unknown });
      rows.forEach((r) => { resolved[r.name] = r; });
    } catch { /* leave them unresolved */ }
  }
  // A declared name is linked where it is declared and nowhere else: in the
  // vocabulary below, a table would also link every column sharing its name.
  for (const c of calls) if (resolved[c.name]) known[c.name] = resolved[c.name];

  // A source parent is named "src.table", which never appears as one token.
  // Custom macros such as replicate('crm', 'X') hide the source() call, so
  // the table half is added to the vocabulary when no model already claims it.
  for (const [name, target] of Object.entries({ ...known })) {
    const dot = name.indexOf('.');
    if (dot < 0) continue;
    const table = name.slice(dot + 1);
    if (!known[table]) known[table] = target;
  }
  const vocab = new Set(Object.keys(known).filter((n) => !n.includes('.')));
  // Declared first, so a name that is both keeps the click that stays put.
  const hits = [...declared, ...calls, ...scanVocabulary(text, vocab)];
  const isCall = new Set(calls), isDeclared = new Set(declared);

  const seen = new Set();
  for (const hit of hits) {
    // A declared name asked for itself, and `known` may hold a source table
    // of the same name: a model called after its source links to the model.
    const target = isDeclared.has(hit)
      ? resolved[hit.name] || known[hit.name]
      : known[hit.name] || resolved[hit.name];
    for (const [from, to] of hit.ranges) {
      if (seen.has(from)) continue;
      seen.add(from);
      if (!target && !isCall.has(hit) && !isDeclared.has(hit)) continue;
      const title = target
        ? `${target.disabled ? 'DISABLED ' : ''}${target.kind}${target.materialized ? ' · ' + target.materialized : ''}\n${target.file}`
        : `${hit.name} is in no manifest node (stale manifest or a typo)`;
      // No `attributes: {title}`: the native tooltip it drew is now the hover
      // card's job, and the two would overlap. The line it used to show is kept
      // on the mark so the card can say exactly the same thing.
      const mark = doc.markText(doc.posFromIndex(from), doc.posFromIndex(to), {
        className: 'cm-reflink' + (target ? (target.disabled ? ' dis' : '') : ' missing'),
      });
      mark.refTarget = target || { name: hit.name };
      mark.refTitle = title;
      if (isDeclared.has(hit)) mark.declared = true;
    }
  }
}

/* Marks every var() and env_var() call so the hover card has something to
   attach to. Synchronous on purpose: the values arrive when a card opens, not
   on every keystroke. markRefs only ever clears its own refTarget marks and
   this only ever clears its own, and ref|source and var|env_var are disjoint,
   so the two never contend for a range. */
function markVars(doc) {
  doc.getAllMarks().forEach((mk) => { if (mk.varTarget) mk.clear(); });
  const text = maskJinjaComments(doc.getValue());
  for (const hit of scanVars(text)) {
    for (const [from, to] of hit.ranges) {
      const mark = doc.markText(doc.posFromIndex(from), doc.posFromIndex(to), { className: 'cm-varlink' });
      mark.varTarget = hit;
    }
  }
}

/* Links every macro call in the buffer, and in a properties file every entry
   under `macros:`, to the file that defines the macro. The names come from the
   buffer and the answer from the manifest (0028): only a call the server can
   place in the project comes back, so dbt's own macros, Jinja's builtins and a
   typo all stay text. The marks are replaced only once the answer is in, and
   not at all when a later scan has started or the buffer has moved on, since
   ranges read from one text would land on the wrong words of another. */
async function markMacros(doc, path) {
  const scan = doc.macroScan = (doc.macroScan || 0) + 1;
  const raw = doc.getValue();
  const text = maskJinjaComments(raw);
  const yaml = !!path && modeFor(path) === 'text/x-yaml';
  const hits = scanMacroCalls(text, yaml);
  if (yaml && text.length <= OUTLINE_MAX) {
    for (const d of yamlDeclared(text)) if (d.kind === 'macro') hits.push({ call: d.name, ranges: d.ranges });
  }
  let links = [];
  if (hits.length) {
    try {
      links = await api.send('/api/macros/resolve', 'POST', { file: path || '', calls: hits.map((h) => h.call) });
    } catch { return; }
  }
  if (doc.macroScan !== scan || doc.getValue() !== raw) return;
  doc.getAllMarks().forEach((mk) => { if (mk.macroTarget) mk.clear(); });
  const byCall = new Map(links.map((l) => [l.call, l]));
  for (const hit of hits) {
    const target = byCall.get(hit.call);
    if (!target) continue;
    for (const [from, to] of hit.ranges) {
      const mark = doc.markText(doc.posFromIndex(from), doc.posFromIndex(to), { className: 'cm-macrolink' });
      mark.macroTarget = target;
    }
  }
}

/* Links every folder key in dbt_project.yml to the folder it configures. The
   chains come from the buffer and the answer from the server, the only side
   that sees the disk and reads `model-paths` (0036). A chain it cannot place
   is marked all the same, dashed: dbt says a config reaching nothing only once,
   at parse time, and the card says it where the key is. Guarded like
   markMacros, since ranges read from one text would land on the words of
   another. */
async function markProjectDirs(doc, path) {
  if (path !== 'dbt_project.yml') return;                 // a package's own is its own
  const scan = doc.dirScan = (doc.dirScan || 0) + 1;
  const raw = doc.getValue();
  if (raw.length > OUTLINE_MAX) return;
  const keys = projectPathKeys(raw);
  let answer = { placed: [], roots: {} };
  if (keys.length) {
    try { answer = await api.send('/api/project/resolve', 'POST', { keys: keys.map((k) => ({ block: k.block, chain: k.chain })) }); }
    catch { return; }
  }
  if (doc.dirScan !== scan || doc.getValue() !== raw) return;
  doc.getAllMarks().forEach((mk) => { if (mk.dirTarget) mk.clear(); });
  const placed = new Map((answer.placed || []).map((p) => [dirKey(p), p]));
  for (const key of keys) {
    const target = placed.get(dirKey(key)) || { block: key.block, chain: key.chain, roots: dirRoots(answer.roots, key.block) };
    for (const [from, to] of key.ranges) {
      const mark = doc.markText(doc.posFromIndex(from), doc.posFromIndex(to), {
        className: 'cm-dirlink' + (target.path ? '' : ' missing'),
      });
      mark.dirTarget = target;
    }
  }
}

/* The block and the chain together, which is how a mark finds its answer: a
   pair, not a joined string, so a key holding a slash cannot collide. */
const dirKey = (k) => JSON.stringify([k.block, k.chain]);

/* Where the server looked, which is what a key that found nothing can say. */
const dirRoots = (roots, block) => (roots || {})[block] || [];

function wireRefClicks(cm) {
  cm.getWrapperElement().addEventListener('mousedown', (e) => {
    if (e.button !== 0 || e.altKey) return;              // alt-click still places the cursor
    const cl = e.target.classList;
    if (!cl || !(cl.contains('cm-reflink') || cl.contains('cm-macrolink') || cl.contains('cm-dirlink'))) return;
    const pos = cm.coordsChar({ left: e.clientX, top: e.clientY }, 'window');
    const marks = cm.findMarksAt(pos);
    const macro = cl.contains('cm-macrolink') && marks.find((mk) => mk.macroTarget);
    if (macro) {
      e.preventDefault();
      openMacro(macro.macroTarget);
      return;
    }
    const folder = cl.contains('cm-dirlink') && marks.find((mk) => mk.dirTarget);
    if (folder) {
      e.preventDefault();
      openConfigPath(folder.dirTarget);
      return;
    }
    const mark = marks.find((mk) => mk.refTarget);
    if (!mark) return;
    e.preventDefault();
    const t = mark.refTarget;
    if (!t.id) return toast(`${t.name} is not in the manifest`, 'err');
    pushJump();
    if (mark.declared) return focusDeclared(t.id);
    openFile(t.file, { focusLineage: false, preview: true });
    focusNode(t.id);
    revealInTree(t.file);
  });
}

/* A name clicked in the file that declares it. The editor stays where it is,
   since the definition is the line that was clicked, and the lineage moves.
   The terminal is the one dock that shows nothing of a node, so from there
   the lineage is brought forward; the others all follow the focus. */
function focusDeclared(id) {
  focusNode(id);
  if (!$('#dock-terminal').classList.contains('hidden')) showDock('lineage');
}

/* The file a macro is defined in, at its `{% macro %}` line. The lineage stays
   on the model being read: a macro is not a node. */
async function openMacro(t) {
  pushJump();
  await openFile(t.file, { focusLineage: false, preview: true });
  const f = S.open.get(t.file);
  if (S.active !== t.file || !f || !f.doc) return;          // it could not be opened
  revealInTree(t.file);
  const at = macroDefLine(f.doc.getValue(), t.name);
  if (at) gotoPos(at.line, at.ch);
}

/* A generic test opened where it is declared (0041): its properties file, at
   the item `testLine` finds under its host, even when that file is the one
   shown. `pin` keeps the tab, as a double click does; `focus` false leaves the
   keyboard on the canvas a click came from. A second click on the test the
   cursor already sits on, the first half of a double click among them, is no
   new place to come back to, so it records none. */
async function openTest(t, { pin = false, focus = true } = {}) {
  if (!t || !t.file) return;
  const spot = (f) => {
    if (!f || !f.doc) return null;
    const text = f.doc.getValue();
    return text.length <= OUTLINE_MAX ? testLine(text, t) : null;
  };
  const there = S.active === t.file && S.cm ? spot(S.open.get(t.file)) : null;
  const cur = there && S.cm.getCursor();
  if (!(cur && cur.line === there.line && cur.ch === there.ch)) pushJump({ pin });
  await openFile(t.file, { focusLineage: false, preview: !pin });
  revealInTree(t.file);
  const f = S.open.get(t.file);
  if (S.active !== t.file || !f || !f.doc) return;           // it could not be opened
  const at = spot(f);
  if (!at) return;
  gotoPos(at.line, at.ch, focus);
  // In the upper third rather than at an edge, its arguments below it.
  const third = S.cm.getScrollInfo().clientHeight / 3;
  S.cm.scrollIntoView({ from: { line: at.line, ch: 0 }, to: { line: at.line, ch: 0 } }, third);
  flashLines(at.line, at.end);
}

/* Lines lit for a moment, then faded, so the eye finds where a jump landed
   among a few hundred alike. Only one at a time: a second jump clears the
   first rather than leaving two lit. */
let flashed = null;
function flashLines(from, to) {
  if (!S.cm) return;
  const doc = S.cm.getDoc();
  if (flashed) {
    clearTimeout(flashed.timer);
    flashed.handles.forEach((h) => flashed.doc.removeLineClass(h, 'background', 'cm-flash'));
  }
  const handles = [];
  for (let l = from; l <= Math.min(to, from + 40); l++) handles.push(doc.addLineClass(l, 'background', 'cm-flash'));
  const timer = setTimeout(() => {
    handles.forEach((h) => doc.removeLineClass(h, 'background', 'cm-flash'));
    flashed = null;
  }, 1800);
  flashed = { doc, handles, timer };
}

/* The folder a config key in dbt_project.yml configures, opened in the tree
   rather than in a tab: there is nothing to read in a folder. A chain the
   server could not place does nothing here, since the card is what explains
   it. The lineage stays where it is, as it does for a macro: a folder is not a
   node. */
async function openConfigPath(t) {
  if (!t.path) return;
  showSide('files');
  if (!t.dir) {
    await openFile(t.path, { focusLineage: false, preview: true });
    revealInTree(t.path);
    return;
  }
  if (!await revealInTree(t.path, { expand: true })) toast(`${t.path} is not in the tree`, 'err');
}

/* Hovering a mark opens the card. The classList gate comes first because
   coordsChar snaps to the nearest character even far past the end of a line, so
   without it every move over the empty area to the right of a line would
   resolve onto whatever mark that line ends with. */
function wireHovers(cm) {
  const wrap = cm.getWrapperElement();
  wrap.addEventListener('mousemove', (e) => {
    const cl = e.target.classList;
    if (!cl) return hoverLeave();
    const isVar = cl.contains('cm-varlink');
    const isMacro = cl.contains('cm-macrolink');
    const isDir = cl.contains('cm-dirlink');
    if (!isVar && !isMacro && !isDir && !cl.contains('cm-reflink')) return hoverLeave();
    const pos = cm.coordsChar({ left: e.clientX, top: e.clientY }, 'window');
    const at = () => cm.charCoords(pos, 'window');
    if (isDir) {
      const dm = cm.findMarksAt(pos).find((mk) => mk.dirTarget);
      if (!dm) return hoverLeave();
      return hoverEnter('dir:' + dirKey(dm.dirTarget), at, (el) => fillDirCard(el, dm.dirTarget));
    }
    if (isMacro) {
      const mm = cm.findMarksAt(pos).find((mk) => mk.macroTarget);
      if (!mm) return hoverLeave();
      return hoverEnter('macro:' + mm.macroTarget.id, at, (el) => fillMacroCard(el, mm.macroTarget));
    }
    if (isVar) {
      const vm = cm.findMarksAt(pos).find((mk) => mk.varTarget);
      if (!vm) return hoverLeave();
      return hoverEnter(`${vm.varTarget.kind}:${vm.varTarget.name}`, at, (el) => fillVarCard(el, vm.varTarget));
    }
    const mark = cm.findMarksAt(pos).find((mk) => mk.refTarget);
    if (!mark) return hoverLeave();
    const t = mark.refTarget;
    if (!t.id) {
      return hoverEnter('miss:' + t.name, at, (el) => {
        hoverCardBody(el, { title: t.name });
        el.append(Object.assign(document.createElement('div'), { className: 'hc-desc muted', textContent: mark.refTitle }));
      });
    }
    hoverEnter('node:' + t.id, at, (el) => fillNodeCard(el, t.id, t));
  });
  wrap.addEventListener('mouseleave', hoverLeave);
}

function modeFor(path) {
  const p = path.toLowerCase();
  if (p.endsWith('.sql')) return 'dbt-sql';
  if (p.endsWith('.yml') || p.endsWith('.yaml')) return 'text/x-yaml';
  if (p.endsWith('.md')) return 'text/x-markdown';
  return null;
}

function initEditor() {
  S.cm = CodeMirror($('#editor-host'), {
    lineNumbers: true, lineWrapping: false, indentUnit: 4, tabSize: 4,
    styleActiveLine: false, value: '',
  });
  S.cm.on('change', (cm, ch) => {
    // The file itself changing on disk is no edit of the user's (0067).
    if (!S.active || ch.origin === 'disk') return;
    const f = S.open.get(S.active);
    if (!f || ownsPane(f)) return;
    const pinned = S.preview === S.active;
    if (pinned) S.preview = null;       // editing a preview tab pins it
    if (f && !f.dirty) { f.dirty = true; renderTabs(); }
    else if (pinned) renderTabs();
  });
  S.cm.on('cursorActivity', updateStatus);
  S.cm.on('cursorActivity', crumbCursor);
  let rescan = null;
  S.cm.on('change', (cm, ch) => {
    // A reload scans its own tab: under a diff tab, S.cm still holds the last
    // file's Doc, and S.active would name the diff (0067).
    if (ch.origin === 'disk') return;
    clearTimeout(rescan);
    // The path as well as the Doc, taken now: a tab switched within the delay
    // would otherwise scan this file as that one, and a .yml would lose the
    // links only a properties file gets.
    const doc = S.cm.getDoc(), path = S.active;
    rescan = setTimeout(() => {
      markLinks(doc, path);
      refreshOutline();
      renderCrumbs();
    }, 500);
  });
  wireRefClicks(S.cm);
  wireHovers(S.cm);
}

// ------------------------------------------------------------------ find --
/* Find inside one editor: the file being edited, the Compiled or Run SQL, a
   diff. The browser's own find cannot do this, because CodeMirror keeps only
   the lines on screen in the page, so a match two screens down is not there to
   be found. The search runs over the document CodeMirror holds instead, and the
   highlight is an overlay, which CodeMirror paints a line at a time as it draws
   it. Both go through matchInLine, so the count and the highlight cannot
   disagree about what a match is. */

function escapeRegExp(s) {
  return s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

/* The query as a global RegExp, or why it cannot be one. A plain query is
   escaped, so `.` and `(` mean themselves. */
function findPattern(query, opts) {
  if (!query) return { re: null, error: '' };
  const flags = opts.caseSensitive ? 'g' : 'gi';
  if (!opts.regex) return { re: new RegExp(escapeRegExp(query), flags), error: '' };
  try {
    return { re: new RegExp(query, flags), error: '' };
  } catch (e) {
    return { re: null, error: e.message };
  }
}

/* A letter, a digit or an underscore, in any script: `donn` is not a whole
   word inside `données`. */
function isWordChar(ch) {
  return !!ch && /[\p{L}\p{N}_]/u.test(ch);
}

/* VS Code's rule for a whole word: each end of the match meets the edge of the
   line or something that is not part of a word, or is not part of one itself,
   so `.id` is a whole word in `o.id` and `id` is not one in `order_id`. */
function wholeWord(text, from, to) {
  const left = from === 0 || !isWordChar(text[from - 1]) || !isWordChar(text[from]);
  const right = to === text.length || !isWordChar(text[to]) || !isWordChar(text[to - 1]);
  return left && right;
}

/* The first match in one line at or after `from`, as {from, to}, or null. An
   empty match is stepped over: `^` or `x*` matches between every two
   characters, and a find that stopped there would never move. */
function matchInLine(text, re, from, whole) {
  for (let at = from; at <= text.length;) {
    re.lastIndex = at;
    const m = re.exec(text);
    if (!m) return null;
    const end = m.index + m[0].length;
    if (end > m.index && (!whole || wholeWord(text, m.index, end))) return { from: m.index, to: end };
    at = m.index + 1;
  }
  return null;
}

/* Every match in document order, as {line, from, to}, and whether there were
   more than `cap`. Line by line, as the overlay paints, so `^` and `$` are the
   ends of a line and no match runs across two. Past the cap the count says so
   rather than going on: nobody steps through ten thousand matches, and each
   one is an object. */
function findMatches(lines, re, whole, cap) {
  const hits = [];
  if (!re) return { hits, capped: false };
  for (let line = 0; line < lines.length; line++) {
    const text = lines[line];
    for (let m = matchInLine(text, re, 0, whole); m; m = matchInLine(text, re, m.to, whole)) {
      if (hits.length === cap) return { hits, capped: true };
      hits.push({ line, from: m.from, to: m.to });
    }
  }
  return { hits, capped: false };
}

/* The match a step lands on, as {line, from, to}, or null when there is none.
   Forward, the first one starting at or after `pos`; backward, the last one
   starting before it; both wrap around the end. Read from the text rather
   than from the counted matches, so a step past the cap still finds the next
   one. Each line is read from its start, as the count and the overlay read it,
   so a step never lands on a match neither of them has: `aa` in `aaa` is the
   first two letters, even from a cursor after the first. */
function nextMatch(lines, re, whole, pos, dir) {
  const n = lines.length;
  if (!re || !n) return null;
  const at = Math.min(pos.line, n - 1);
  // k runs once round the document and back onto the starting line, where
  // only the part the first pass left out still counts.
  for (let k = 0; k <= n; k++) {
    const line = dir > 0 ? (at + k) % n : (at - k + n) % n;
    let pick = null;
    for (let m = matchInLine(lines[line], re, 0, whole); m; m = matchInLine(lines[line], re, m.to, whole)) {
      const after = m.from >= pos.ch;
      if (k === 0 && after !== dir > 0) continue;
      if (k === n && after === dir > 0) continue;
      pick = m;
      if (dir > 0) break;
    }
    if (pick) return { line, from: pick.from, to: pick.to };
  }
  return null;
}

/* Whether a selection is exactly a match, however far past the cap. */
function isMatch(lines, re, whole, from, to) {
  if (!re || from.line !== to.line || from.line >= lines.length) return false;
  const text = lines[from.line];
  for (let m = matchInLine(text, re, 0, whole); m && m.from <= from.ch; m = matchInLine(text, re, m.to, whole)) {
    if (m.from === from.ch && m.to === to.ch) return true;
  }
  return false;
}

/* Which counted match a selection is, or -1: the ordinal in "3 of 17". Past
   the cap there is none to give. */
function hitAt(hits, from, to) {
  let lo = 0, hi = hits.length;
  while (lo < hi) {
    const mid = (lo + hi) >> 1;
    const h = hits[mid];
    if (h.line < from.line || (h.line === from.line && h.from < from.ch)) lo = mid + 1;
    else hi = mid;
  }
  const h = hits[lo];
  return h && h.line === from.line && h.from === from.ch && to.line === h.line && to.ch === h.to ? lo : -1;
}

/* What the bar says beside the box. `at` is the counted match the selection
   is on, or -1; `onMatch` says the selection is a match all the same, one past
   the cap, which has no number to give. */
function findLabel(query, found, at, error, onMatch) {
  if (!query) return '';
  if (error) return 'invalid pattern';
  const n = found.hits.length;
  if (!n) return 'No results';
  const total = n + (found.capped ? '+' : '');
  if (at >= 0) return (at + 1) + ' of ' + total;
  if (onMatch) return '? of ' + total;
  return total + (n === 1 && !found.capped ? ' result' : ' results');
}

/* Every match painted, as an overlay mode. CodeMirror hands it one line at a
   time from its start, so each call either paints the match it stands on or
   skips to the next one. */
function findOverlay(re, whole) {
  return {
    token(stream) {
      const m = matchInLine(stream.string, re, stream.pos, whole);
      if (!m) { stream.skipToEnd(); return null; }
      if (m.from > stream.pos) { stream.pos = m.from; return null; }
      stream.pos = m.to;
      return 'findhit';
    },
  };
}

/* What the last bar used, which is what the next one opens with, the way VS
   Code carries a search from one editor to the next: Cmd+F in the Compiled tab
   after a search in the model looks for the same thing. Each open bar keeps
   its own copy, so typing in one never changes what another highlights. */
const findPrefs = { query: '', caseSensitive: false, wholeWord: false, regex: false };
const FIND_CAP = 10000;

/* One editor's bar, built the first time Cmd+F reaches that editor. It lives
   inside CodeMirror's own wrapper, where CodeMirror's dialogs go, so it moves
   with the editor: the Compiled one is detached and put back between nodes. */
function findBar(cm) {
  if (cm.state.findBar) return cm.state.findBar;
  const st = {
    cm, open: false, q: { ...findPrefs }, key: '', re: null, error: '', lines: [],
    found: { hits: [], capped: false }, at: -1, cur: null, mark: null, overlay: null,
    origin: { line: 0, ch: 0 }, moving: false, timer: 0,
    keys: { Esc: () => closeFind(cm) },
  };
  const bar = document.createElement('div');
  bar.className = 'findbar hidden';
  bar.setAttribute('role', 'search');
  const input = document.createElement('input');
  input.className = 'find-q';
  input.placeholder = 'Find';
  input.spellcheck = false;
  input.autocomplete = 'off';
  input.setAttribute('aria-label', 'Find in this text');
  const option = (name, label, title) => {
    const b = document.createElement('button');
    b.type = 'button';
    b.className = 'find-opt find-' + name;
    b.textContent = label;
    b.title = title;
    b.addEventListener('click', () => {
      st.q[name] = !st.q[name];
      findPrefs[name] = st.q[name];
      refreshFind(st, true);
      input.focus();
    });
    return [name, b];
  };
  st.opts = [
    option('caseSensitive', 'Aa', 'Match case'),
    option('wholeWord', 'ab', 'Match whole word'),
    option('regex', '.*', 'Use a regular expression'),
  ];
  const count = document.createElement('span');
  count.className = 'find-count';
  count.setAttribute('aria-live', 'polite');
  const button = (label, title, run) => {
    const b = document.createElement('button');
    b.type = 'button';
    b.className = 'icon';
    b.textContent = label;
    b.title = title;
    b.addEventListener('click', run);
    return b;
  };
  bar.append(input, ...st.opts.map(([, b]) => b), count,
    button('↑', `Previous match (${keyLabel('Shift+Enter', IS_MAC)})`, () => stepFind(cm, -1)),
    button('↓', `Next match (${keyLabel('Enter', IS_MAC)})`, () => stepFind(cm, 1)),
    button('×', `Close (${keyLabel('Escape', IS_MAC)})`, () => closeFind(cm)));
  Object.assign(st, { bar, input, count });

  input.addEventListener('input', () => {
    st.q.query = findPrefs.query = input.value;
    refreshFind(st, true);
  });
  bar.addEventListener('keydown', (e) => {
    const k = keyCombo(e, IS_MAC);
    // Enter steps from the box only: on one of the buttons it presses it.
    const dir = { 'Mod+G': 1, F3: 1, 'Mod+Shift+G': -1, 'Shift+F3': -1 }[k]
      || (e.target === input && { Enter: 1, 'Shift+Enter': -1 }[k]);
    if (dir) stepFind(cm, dir);
    else if (k === 'Escape') closeFind(cm);
    else if (k === 'Mod+F') { input.focus(); input.select(); }
    else return;
    e.preventDefault();
  });
  // An edit moves matches; waiting for a pause keeps a burst of typing to one
  // count. A document swapped in, another tab's, is counted at once.
  cm.on('changes', () => {
    if (!st.open) return;
    clearTimeout(st.timer);
    st.timer = setTimeout(() => refreshFind(st, false), 150);
  });
  cm.on('swapDoc', (_, old) => {
    if (!st.open || old === cm.getDoc()) return;
    st.origin = cm.getCursor('from');
    refreshFind(st, false);
  });
  // Typing searches from where the cursor was put, never from where the last
  // keystroke's match happened to land: after `x` has jumped far ahead, a `y`
  // typed in its place still starts from the same line.
  cm.on('cursorActivity', () => {
    if (st.open && !st.moving) st.origin = cm.getCursor('from');
  });
  cm.getWrapperElement().appendChild(bar);
  cm.state.findBar = st;
  return st;
}

/* Counts again and repaints. `jump` is for typing and the option buttons: it
   moves to the first match from where the search started. */
function refreshFind(st, jump) {
  const { cm, q } = st;
  clearTimeout(st.timer);
  st.timer = 0;
  const key = [q.query, q.caseSensitive, q.wholeWord, q.regex].join('\u0000');
  if (key !== st.key) {
    // Only a new pattern repaints: the overlay reads the text as it draws it,
    // so an edit needs no new one.
    st.key = key;
    const pat = findPattern(q.query, q);
    st.re = pat.re;
    st.error = pat.error;
    if (st.overlay) cm.removeOverlay(st.overlay);
    st.overlay = st.re ? findOverlay(st.re, q.wholeWord) : null;
    if (st.overlay) cm.addOverlay(st.overlay);
  }
  st.lines = [];
  cm.getDoc().eachLine((l) => { st.lines.push(l.text); });
  st.found = findMatches(st.lines, st.re, q.wholeWord, FIND_CAP);
  const m = jump && nextMatch(st.lines, st.re, q.wholeWord, st.origin, 1);
  if (m) return gotoMatch(st, m, false);
  const from = cm.getCursor('from'), to = cm.getCursor('to');
  settleFind(st, isMatch(st.lines, st.re, q.wholeWord, from, to) ? { from, to } : null);
}

/* A step moves where the next one, and the next query, start from. */
function gotoMatch(st, m, step) {
  const { cm } = st;
  const from = { line: m.line, ch: m.from }, to = { line: m.line, ch: m.to };
  st.moving = true;
  cm.setSelection(from, to, { scroll: false });
  // A third of the height clear on either side, so the match never lands under
  // the bar or on the bottom line, where it would still have to be looked for.
  cm.scrollIntoView({ from, to }, Math.round(cm.getScrollInfo().clientHeight / 3));
  st.moving = false;
  if (step) st.origin = from;
  settleFind(st, { from, to });
}

/* `cur` is the match the selection is on, or null. It is painted brighter than
   the rest, because the selection alone is a faint blue once the focus is in
   the box. */
function settleFind(st, cur) {
  const { q } = st;
  st.cur = cur;
  st.at = cur ? hitAt(st.found.hits, cur.from, cur.to) : -1;
  if (st.mark) st.mark.clear();
  st.mark = cur ? st.cm.markText(cur.from, cur.to, { className: 'cm-findcur' }) : null;
  st.input.classList.toggle('bad', !!st.error);
  st.input.title = st.error;
  st.count.textContent = findLabel(q.query, st.found, st.at, st.error, !!cur);
  st.count.classList.toggle('none', !!q.query && (!!st.error || !st.found.hits.length));
  for (const [name, b] of st.opts) b.setAttribute('aria-pressed', String(!!q[name]));
}

function showFind(st) {
  if (!st.open) {
    st.open = true;
    st.origin = st.cm.getCursor('from');
    st.bar.classList.remove('hidden');
    st.cm.addKeyMap(st.keys);
    // Room above the first line, as VS Code makes, or a match there would sit
    // under the bar with nowhere to scroll it to.
    st.cm.getWrapperElement().classList.add('find-open');
    st.cm.refresh();
  }
  st.input.value = st.q.query;
  refreshFind(st, false);
}

/* Cmd+F. A selection on one line becomes the query, as in VS Code, unless it
   is the match already selected: that would swap a pattern for the one string
   it happened to match. */
function openFind(cm) {
  const st = findBar(cm);
  const onMatch = st.open && !!st.cur;
  if (!st.open) st.q = { ...findPrefs };
  const sel = cm.getSelection();
  if (sel && !sel.includes('\n') && sel.length <= 200 && !onMatch) {
    st.q.query = findPrefs.query = st.q.regex ? escapeRegExp(sel) : sel;
  }
  showFind(st);
  st.input.focus();
  st.input.select();
}

/* Enter and Shift+Enter in the bar, Cmd+G and F3 in the text. With the bar
   closed it opens on the last query first, and leaves the focus in the text. */
function stepFind(cm, dir) {
  const st = findBar(cm);
  if (!st.open) {
    st.q = { ...findPrefs };
    showFind(st);
  } else if (st.timer) {
    refreshFind(st, false);                  // an edit is still waiting to be counted
  }
  const m = nextMatch(st.lines, st.re, st.q.wholeWord, cm.getCursor(dir > 0 ? 'to' : 'from'), dir);
  if (m) gotoMatch(st, m, true);
}

/* The selection stays on the last match, so Escape leaves the cursor where the
   search brought it, ready to type. */
function closeFind(cm) {
  const st = cm.state.findBar;
  if (!st || !st.open) return;
  st.open = false;
  clearTimeout(st.timer);
  st.timer = 0;
  st.bar.classList.add('hidden');
  cm.getWrapperElement().classList.remove('find-open');
  cm.refresh();
  cm.removeKeyMap(st.keys);
  if (st.overlay) cm.removeOverlay(st.overlay);
  st.overlay = null;
  st.key = '';
  st.lines = [];
  if (st.mark) st.mark.clear();
  st.mark = st.cur = null;
  cm.focus();
}

/* CodeMirror's default keymaps already bind Cmd+F, Cmd+G and Shift+Cmd+G (Ctrl
   elsewhere) to these three names and leave them for the embedder to define.
   Defining them is what reaches every editor here, the diff panes included,
   and only the one with the focus answers. */
CodeMirror.commands.find = (cm) => openFind(cm);
CodeMirror.commands.findNext = (cm) => stepFind(cm, 1);
CodeMirror.commands.findPrev = (cm) => stepFind(cm, -1);
// The same two under the keys Windows and VS Code give them.
CodeMirror.keyMap.default.F3 = 'findNext';
CodeMirror.keyMap.default['Shift-F3'] = 'findPrev';

/* The editor a Cmd+F from outside every editor means: the file on screen, or
   the working-tree side of a diff. */
function activeEditor() {
  const f = S.active && S.open.get(S.active);
  if (!f) return null;
  if (f.kind === 'diff') return f.mv ? f.mv.editor() : null;
  return ownsPane(f) ? null : S.cm;
}

/* Where an element sits, by pane. Everything in the dock belongs to the tab on
   screen, its tab bar included, so Cmd+F after a click on Compiled searches the
   SQL under it. '' is the page itself, with nothing focused. */
function paneOf(el) {
  if (!el || !el.closest || el === document.body) return '';
  if (el.closest('#editor-pane')) return 'editor';
  if (el.closest('#sidebar')) return 'sidebar';
  if (el.closest('#dock')) return S.dock;
  return 'other';
}

/* Cmd+F wherever CodeMirror has not answered it already. False where there is
   nothing of ours to search, and the browser's own find opens instead: over the
   lineage it finds a box by its name. */
function findInPane(pane) {
  // The reports cover every pane: the browser's own find searches them.
  if (S.reportsOpen) return false;
  if (pane === 'catalog') return focusColumnFilter();
  if (pane === 'compiled' || pane === 'run') {
    const cm = S.artifactCm[pane];
    if (!cm || !document.body.contains(cm.getWrapperElement())) return false;
    openFind(cm);
    return true;
  }
  if (pane !== 'editor' && pane !== 'sidebar' && pane !== '') return false;
  const cm = activeEditor();
  if (!cm) return false;
  openFind(cm);
  return true;
}

const base = (path) => path.split('/').pop();
const fileName = (path) => path.split(/[\\/]/).pop();
const dirOf = (path) => {
  const d = path.slice(0, path.lastIndexOf('/'));
  return d.length > 36 ? '\u2026' + d.slice(-35) : d;
};

/* `preview` opens the file in the single reusable tab instead of stacking a new
   one, the way a single click does in VS Code. Editing it, or opening it again
   with preview off, pins it. */
async function openFile(path, { focusLineage = true, preview = false } = {}) {
  closeHoverCard();
  if (S.open.has(path)) {
    if (!preview && S.preview === path) S.preview = null;
    return activate(path, focusLineage);
  }
  let body;
  try { body = await api.get('/api/file?path=' + encodeURIComponent(path)); }
  catch (e) { return toast(`cannot open ${path}: ${e.message}`, 'err'); }

  const doc = CodeMirror.Doc(body.content, modeFor(path));
  let slot = S.order.length;
  const stale = S.preview && S.open.has(S.preview) && !S.open.get(S.preview).dirty;
  if (preview && stale) {
    slot = S.order.indexOf(S.preview);
    S.open.delete(S.preview);
    S.order.splice(slot, 1);
  }
  S.open.set(path, { doc, dirty: false, truncated: body.truncated, stamp: body.stamp, disk: '' });
  S.order.splice(slot, 0, path);
  if (preview) S.preview = path;
  markLinks(doc, path);
  activate(path, focusLineage);
}

/* Every kind of link a file's text carries, scanned again whenever the text
   changes: on opening, after typing, and after a reload from disk. */
function markLinks(doc, path) {
  markRefs(doc, path);
  markVars(doc);
  markMacros(doc, path);
  markProjectDirs(doc, path);
}

/* A diff tab holds no CodeMirror Doc: it owns a MergeView of HEAD against the
   working tree. Read-only on purpose, the file itself is one click away. */
async function openDiff(path) {
  const key = 'diff:' + path;
  if (!S.open.has(key)) {
    let view;
    try { view = await api.get('/api/git/diff?path=' + encodeURIComponent(path)); }
    catch (e) { return toast('diff: ' + e.message, 'err'); }
    S.open.set(key, { kind: 'diff', path, view, dirty: false, host: null, mv: null });
    S.order.push(key);
  }
  activate(key);
}

function mountDiff(key) {
  const f = S.open.get(key);
  const host = $('#diff-host');
  [...host.children].forEach((c) => c.classList.add('hidden'));
  if (!f.host) {
    f.host = document.createElement('div');
    f.host.className = 'diffpane';
    host.appendChild(f.host);

    const head = document.createElement('div');
    head.className = 'diff-head';

    const title = document.createElement('div');
    title.className = 'diff-title';
    title.append(fileIcon(f.path));
    const nm = document.createElement('b');
    nm.textContent = base(f.path);
    const state = document.createElement('span');
    state.className = 'diff-state';
    state.textContent = f.view.before_missing ? '(new file)'
      : f.view.after_missing ? '(deleted)' : '(working tree)';
    title.append(nm, state);
    const lock = document.createElement('span');
    lock.className = 'diff-lock';
    lock.textContent = 'read only';
    lock.title = 'Open the file itself to edit it';
    title.append(lock, Object.assign(document.createElement('div'), { className: 'grow' }));
    const open = document.createElement('button');
    open.className = 'btn sm';
    open.textContent = 'Open the file';
    open.addEventListener('click', () => openFile(f.path, { preview: true }));
    const close = document.createElement('button');
    close.className = 'icon';
    close.textContent = '×';
    close.title = 'Close this diff';
    close.addEventListener('click', () => closeFile(key));
    title.append(open, close);

    const crumb = document.createElement('div');
    crumb.className = 'diff-crumb';
    f.path.split('/').forEach((seg, i, all) => {
      if (i) crumb.append(Object.assign(document.createElement('span'), { textContent: '›', className: 'sep' }));
      const b = document.createElement('span');
      b.textContent = seg;
      if (i === all.length - 1) b.className = 'leaf';
      crumb.append(b);
    });
    if (f.view.truncated) {
      crumb.append(Object.assign(document.createElement('span'),
        { textContent: '  ·  truncated', className: 'sep' }));
    }
    head.append(title, crumb);
    f.host.appendChild(head);

    if (f.view.binary) {
      f.host.append(Object.assign(document.createElement('div'), {
        className: 'compiled-empty', textContent: 'Binary file, nothing to show side by side.' }));
      return;
    }
    const pane = document.createElement('div');
    pane.className = 'mergepane';
    f.host.appendChild(pane);
    f.mv = CodeMirror.MergeView(pane, {
      value: f.view.after,
      origLeft: f.view.before,
      lineNumbers: true,
      // Forwarded to both panes by the addon, so each gets a +/- column.
      gutters: ['CodeMirror-linenumbers', 'diff-mark'],
      mode: modeFor(f.path),
      highlightDifferences: true,
      connect: 'align',
      collapseIdentical: 12,
      revertButtons: false,
      readOnly: true,
    });
    decorateDiff(f.mv, pane);
  }
  f.host.classList.remove('hidden');
  setTimeout(() => {
    if (!f.mv) return;
    f.mv.editor().refresh();
    if (f.mv.leftOriginal()) f.mv.leftOriginal().refresh();
  }, 0);
}

/* The merge addon paints the changed backgrounds; these are the parts that make
   a diff readable at a glance: a +/- per line, and a ruler showing where the
   changes sit in a file too long to scroll through. */
/* Position of one chunk on the overview ruler, clamped so a mark can never
   escape the bar on a short file. */
function rulerMark(chunk, total) {
  const lines = Math.max(1, total);
  const span = Math.max(1, chunk.editTo - chunk.editFrom);
  const top = Math.min(99, Math.max(0, (100 * chunk.editFrom) / lines));
  return {
    kind: chunk.editTo > chunk.editFrom ? 'add' : 'del',
    top,
    height: Math.max(0.5, Math.min(100 - top, (100 * span) / lines)),
  };
}

function decorateDiff(mv, pane) {
  // leftChunks() computes the diff first, unlike reading mv.left.chunks raw.
  const chunks = (mv.leftChunks && mv.leftChunks()) || [];
  if (!chunks.length) return;
  const mark = (cm, line, sign) => {
    const el = document.createElement('div');
    el.className = 'diff-sign ' + (sign === '+' ? 'add' : 'del');
    el.textContent = sign;
    cm.setGutterMarker(line, 'diff-mark', el);
  };
  const leftCm = mv.leftOriginal(), rightCm = mv.editor();
  for (const c of chunks) {
    for (let l = c.origFrom; l < c.origTo && l < leftCm.lineCount(); l++) mark(leftCm, l, '\u2212');
    for (let l = c.editFrom; l < c.editTo && l < rightCm.lineCount(); l++) mark(rightCm, l, '+');
  }

  const ruler = document.createElement('div');
  ruler.className = 'diff-ruler';
  const total = Math.max(1, rightCm.lineCount());
  for (const c of chunks) {
    const g = rulerMark(c, total);
    const m = document.createElement('div');
    m.className = g.kind;
    m.style.top = g.top + '%';
    m.style.height = g.height + '%';
    m.title = g.kind === 'add' ? `lines ${c.editFrom + 1}-${c.editTo}` : `removed at line ${c.editFrom + 1}`;
    m.addEventListener('click', () => rightCm.scrollIntoView({ line: c.editFrom, ch: 0 }, 120));
    ruler.appendChild(m);
  }
  pane.appendChild(ruler);
}

/* A tab that owns a pane of its own in #diff-host rather than a CodeMirror
   Doc: a diff, or two collin versions compared. Neither has a cursor or a file
   to save. */
function ownsPane(f) {
  return !!f && (f.kind === 'diff' || collinTab(f));
}

/* A comparison of collin versions: a tab no file stands behind. collin's
   report is a section of the reports instead (0064). */
function collinTab(f) {
  return !!f && f.kind === 'compare';
}

/* Showing a tab is asking for the editor, so the reports roll up: a file
   opened from the sidebar, the palette or a link lands here. `keepReports` is
   for a tab shown because another was closed, from the sidebar while the
   reports stay down. */
function activate(path, focusLineage = true, keepReports = false) {
  if (!keepReports) closeReports();
  S.active = path;
  const f = S.open.get(path);
  $('#editor-empty').classList.add('hidden');
  if (ownsPane(f)) {
    $('#editor-host').style.display = 'none';
    $('#diff-host').classList.remove('hidden');
    if (f.kind === 'diff') mountDiff(path);
    else mountCollinTab(path);
  } else {
    $('#diff-host').classList.add('hidden');
    $('#editor-host').style.display = '';
    S.cm.swapDoc(f.doc);
    S.cm.refresh();
  }
  renderTabs();
  updateStatus();
  refreshOutline();
  renderCrumbs();
  const inProject = projectPath(path);
  markTreeSelection(inProject);
  if (focusLineage && inProject) syncNode(inProject);
  // A tab clicked after a pull in the terminal is the moment it is looked at,
  // sooner than the next poll. A file that could not be read is tried again,
  // as when Excel lets go of a seed.
  if (f && !f.dirty && f.disk === 'changed') f.diskSeen = '';
  refreshOpenFiles();
}

/* The project path a tab points at, which is not its key: a diff is keyed
   `diff:<path>`, and a profile is not in the project at all, so no tree row and
   no node answer to it. */
function projectPath(key) {
  const f = S.open.get(key);
  if (!f) return '';
  return f.kind === 'profile' || collinTab(f) ? '' : f.kind === 'diff' ? f.path : key;
}

/* Takes one tab down without asking anything and without drawing: a diff owns
   a MergeView element outside the editor host, which has to go with it. The
   three close commands all come through here, so none of them can forget it. */
function dropTab(key) {
  const f = S.open.get(key);
  if (!f) return;
  if (f.host) f.host.remove();
  S.open.delete(key);
  S.order = S.order.filter((p) => p !== key);
  if (S.preview === key) S.preview = null;
}

/* Where the editor lands once tabs have gone, and the one redraw a close does,
   however many tabs it took. */
function settleTabs() {
  if (!S.active || !S.open.has(S.active)) {
    S.active = S.order[S.order.length - 1] || null;
    if (S.active) activate(S.active, true, true);
    else {
      $('#editor-host').style.display = 'none';
      $('#diff-host').classList.add('hidden');
      $('#editor-empty').classList.remove('hidden');
      updateStatus();
      renderCrumbs();
    }
  }
  renderTabs();
}

/* Whether closing the tab would lose text: unsaved edits, or a file deleted
   on disk whose tab holds the last copy of it. */
function holdsWork(f) {
  return !!f && (f.dirty || f.disk === 'gone');
}

function closeFile(path) {
  const f = S.open.get(path);
  if (holdsWork(f) && !confirm(f.dirty ? `${path} has unsaved changes. Close anyway?`
    : `${path} was deleted on disk, and this tab holds the last copy of it. Close anyway?`)) return;
  dropTab(path);
  settleTabs();
}

/* One confirmation for the lot, naming the files that would lose work, rather
   than one dialog per tab. */
function closeMany(keys, what) {
  if (!keys.length) return;
  const dirty = keys.filter((p) => holdsWork(S.open.get(p)));
  if (dirty.length && !confirm(
    `${dirty.length} file${dirty.length > 1 ? 's have' : ' has'} unsaved changes.\n\n`
    + `${dirty.join('\n')}\n\n${what} The files themselves are not touched.`)) return;
  for (const key of keys) dropTab(key);
  settleTabs();
}

/* True once written, false when it failed, and null when the user chose to
   leave the disk as it is: Save all counts that apart, and Mark resolved does
   not stage a file that was not written. */
async function saveFile(path) {
  const f = S.open.get(path);
  if (ownsPane(f)) return true;
  // A second save while the first is out waits for it, then saves what was
  // typed since, if anything. Sent at once, it would carry the stamp the first
  // is replacing, and be refused as a change on disk.
  if (f && f.saving) { await f.saving; return saveFile(path); }
  // A struck-through tab is saved too: it writes the deleted file back.
  if (!f || (!f.dirty && f.disk !== 'gone')) return true;
  if (f.kind === 'profile') return saveProfile(path, f);
  // The poll leaves a tab alone while it saves: the stamp it would compare is
  // about to change.
  f.saving = writeTab(path, f);
  try { return await f.saving; } finally { f.saving = null; }
}

async function writeTab(path, f) {
  const content = f.doc.getValue();
  // What is typed while the request is out stays unsaved, and the tab edited:
  // a reload would otherwise take a tab marked clean for one with nothing to
  // lose.
  const gen = f.doc.changeGeneration(true);
  const put = (stamp) => api.send('/api/file', 'PUT', { path, content, stamp });
  try {
    let saved;
    try { saved = await put(f.stamp); }
    catch (e) {
      // The disk moved since the tab read it. Saving would put back, unseen,
      // whatever a pull just brought, so it asks, and on yes sends the stamp
      // it showed, empty for a file that was gone: a second change meanwhile
      // is refused too (0067).
      if (!e.changed) throw e;
      if (!confirm(diskMovedQuestion(path, e.stamp))) return null;
      saved = await put(e.stamp === null ? '' : e.stamp);
    }
    f.dirty = !f.doc.isClean(gen);
    f.stamp = saved.stamp;
    f.disk = '';
    f.diskSeen = '';
    if (feedsPayloads(path)) dropNodeCache();
    return true;
  } catch (e) {
    toast(`save failed for ${base(path)}: ${e.message}`, 'err');
    return false;
  }
}

/* Resolved locations and var values are derived from these files, so a cached
   payload would keep showing what they used to say once they change. */
function feedsPayloads(path) {
  return base(path).startsWith('.env') || base(path) === 'dbt_project.yml';
}

function diskMovedQuestion(path, stamp) {
  const keep = 'Cancel leaves the file as it is, and your edits in the tab, unsaved.';
  return stamp === null
    ? `${path} was deleted on disk since it was opened here.\n\nOK writes it back with your version. ${keep}`
    : `${path} changed on disk since it was opened here, by a pull, a checkout or another program.\n\n`
      + `OK writes your version over it. ${keep}`;
}

/* The one stretch of `next` that differs from `cur`, as the offsets of a
   single replace: what both begin and end with stays where it is, and with it
   a cursor, the scroll and the marks of the lines nobody changed. */
function changedSpan(cur, next) {
  const most = Math.min(cur.length, next.length);
  let head = 0;
  while (head < most && cur.charCodeAt(head) === next.charCodeAt(head)) head++;
  let tail = 0;
  while (tail < most - head && cur.charCodeAt(cur.length - 1 - tail) === next.charCodeAt(next.length - 1 - tail)) tail++;
  return { from: head, to: cur.length - tail, text: next.slice(head, next.length - tail) };
}

/* Follows the files open in tabs when something else rewrites them: a pull,
   a checkout, dbt or another editor. A tab with no unsaved edit takes the new
   text in place. One with edits keeps them, and says the disk moved, which its
   save then asks about rather than writing over it (0067). */
async function refreshOpenFiles() {
  // One look at a time, reloads included, or two could read one file and
  // land out of order. One asked meanwhile, after a pull from the panel
  // say, runs once this one is done rather than being dropped.
  if (S.stampsAsked) { S.stampsAgain = true; return; }
  S.stampsAsked = true;
  try {
    do { S.stampsAgain = false; await followDisk(); } while (S.stampsAgain);
  } finally { S.stampsAsked = false; }
}

async function followDisk() {
  const paths = S.order.filter((p) => S.open.get(p).stamp !== undefined);
  if (!paths.length) return;
  let stamps;
  try { stamps = await api.send('/api/file/stamps', 'POST', { paths }); }
  catch { return; }
  let repaint = false;
  for (const path of paths) {
    const f = S.open.get(path);
    // Closed or being saved while the answer was on its way.
    if (!f || f.saving || !(path in stamps)) continue;
    const now = stamps[path];
    if (now === f.stamp) {
      if (f.disk) { f.disk = ''; repaint = true; }
      continue;
    }
    // A deleted file leaves its tab as it was, text and all, since it may
    // be all that is left of it.
    if (now === null) {
      if (f.disk !== 'gone') { f.disk = 'gone'; repaint = true; }
      continue;
    }
    // Already said for this version: a tab with edits keeps them, and a file
    // that could not be read is not read again every poll.
    if (f.diskSeen === now) continue;
    if (f.dirty) {
      f.disk = 'changed';
      f.diskSeen = now;
      repaint = true;
      toast(`${base(path)} changed on disk: your unsaved edits are kept, and saving asks first`, 'warn');
      continue;
    }
    if (await reloadFromDisk(path, f, now)) repaint = true;
  }
  if (repaint) renderTabs();
}

/* True when the tab needs repainting. `now` is the stamp the poll saw, kept
   when the file cannot be read, so it is not read again until it changes. */
async function reloadFromDisk(path, f, now) {
  const seen = f.stamp;
  let body;
  try { body = await api.get('/api/file?path=' + encodeURIComponent(path)); }
  catch (e) {
    if (S.open.get(path) !== f || f.stamp !== seen) return false;
    // Turned binary, or held by another program on Windows: the tab keeps
    // its text and says the disk moved on without it.
    f.disk = 'changed';
    f.diskSeen = now;
    toast(`${base(path)} changed on disk and cannot be read again here: ${e.message}`, 'warn');
    return true;
  }
  // Edited, saved or closed during the read: the user's text wins, and a save
  // made and done meanwhile has moved the stamp.
  if (S.open.get(path) !== f || f.dirty || f.saving || f.stamp !== seen) return false;
  const next = editorText(body.content);
  const cur = f.doc.getValue();
  if (next !== cur) {
    const span = changedSpan(cur, next);
    f.doc.replaceRange(span.text, f.doc.posFromIndex(span.from), f.doc.posFromIndex(span.to), 'disk');
    // Undo would take the pull back out, and the next save, carrying the
    // pull's stamp, would write the old text over it without asking.
    f.doc.clearHistory();
    markLinks(f.doc, path);
    if (path === S.active) { refreshOutline(); renderCrumbs(); }
    if (feedsPayloads(path)) dropNodeCache();
  }
  f.stamp = body.stamp;
  f.truncated = body.truncated;
  f.disk = '';
  f.diskSeen = '';
  return true;
}

async function save() {
  if (!S.active) return;
  const ok = await saveFile(S.active);
  renderTabs();
  // The status bar carries the modified marker too, and it was keeping it.
  updateStatus();
  refreshGit();
  if (ok) toast('saved ' + base(S.active), 'ok');
}

async function saveAll() {
  const dirty = S.order.filter((p) => S.open.get(p).dirty);
  if (!dirty.length) return toast('nothing to save');
  let done = 0, kept = 0;
  for (const path of dirty) {
    const ok = await saveFile(path);
    if (ok) done++;
    else if (ok === null) kept++;
  }
  renderTabs();
  updateStatus();
  refreshGit();
  // A file left as the disk has it was the user's answer, not a failure.
  toast(`saved ${done} of ${dirty.length} file${dirty.length > 1 ? 's' : ''}`
    + (kept ? `, ${kept} left unsaved as you asked` : ''), done + kept === dirty.length ? 'ok' : 'err');
}

function closeAll() {
  closeMany([...S.order], 'Close every tab anyway?');
}

/* Which tabs a close command takes, in the order they would go. Pure, so the
   edges are checked rather than argued about: the kept tab, the last tab, a
   bar with one tab in it. */
function closeTargets(what, keep, order, open) {
  if (what === 'saved') return order.filter((p) => !(open.get(p) || {}).dirty);
  // A tab closed while its menu was open: indexOf would be -1 and `right`
  // would then slice from 0 and take the whole bar.
  const at = order.indexOf(keep);
  if (at < 0) return [];
  if (what === 'others') return order.filter((p) => p !== keep);
  if (what === 'right') return order.slice(at + 1);
  return [keep];
}

function renderTabs() {
  const bar = $('#tabbar');
  bar.textContent = '';
  for (const path of S.order) {
    const f = S.open.get(path);
    const t = document.createElement('div');
    t.dataset.key = path;
    t.className = 'ftab' + (path === S.active ? ' active' : '') + (f.dirty ? ' dirty' : '') + (f.disk ? ' disk-' + f.disk : '');
    const name = document.createElement('span');
    name.textContent = f.kind === 'diff' ? base(f.path) + '  ↔' : f.kind === 'profile' || collinTab(f) ? fileName(f.path) : base(path);
    name.title = f.kind === 'diff' ? f.path + '  (HEAD against the working tree)'
      : f.kind === 'profile' ? f.path + '  (the dbt profile, outside the project)'
      : f.kind === 'compare' ? 'What two collin versions read, from the runs kept in target/collin/'
      : path + diskNote(f);
    const x = document.createElement('span');
    x.className = 'x';
    if (!f.dirty) x.textContent = '×';
    x.addEventListener('click', (e) => { e.stopPropagation(); closeFile(path); });
    t.append(name, x);
    if (path === S.preview) t.classList.add('preview');
    t.addEventListener('click', () => activate(path));
    t.addEventListener('dblclick', () => { if (S.preview === path) { S.preview = null; renderTabs(); } });
    t.addEventListener('mousedown', (e) => { if (e.button === 1) { e.preventDefault(); closeFile(path); } });
    bar.appendChild(t);
    // The bar scrolls, so a tab reached by Cmd+K or from the sidebar list can
    // sit off screen. `nearest` so the page itself never moves, and no focus()
    // here: the cursor belongs to the editor.
    if (path === S.active) t.scrollIntoView({ block: 'nearest', inline: 'nearest' });
  }
  renderOpenEditors();
  paintTree();
}

function diskNote(f) {
  return f.disk === 'gone' ? '  (deleted on disk)'
    : f.disk === 'changed' ? '  (changed on disk: saving asks before writing over it)' : '';
}

const SAVE_SVG = '<svg viewBox="0 0 16 16" width="12" height="12" fill="none" stroke="currentColor" ' +
  'stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><path d="M8 2v7M5 6.2l3 3 3-3M3 12.5h10"/></svg>';

function renderOpenEditors() {
  const list = $('#oe-list');
  list.textContent = '';
  const dirty = S.order.filter((p) => S.open.get(p).dirty).length;
  $('#oe-count').textContent = String(S.order.length);
  $('#save-all-btn').disabled = dirty === 0;
  $('#close-all-btn').disabled = S.order.length === 0;
  $('#reveal-btn').disabled = !S.active || !projectPath(S.active);

  for (const path of S.order) {
    const f = S.open.get(path);
    const row = document.createElement('div');
    row.dataset.key = path;
    row.className = 'oe-row'
      + (path === S.active ? ' active' : '')
      + (f.dirty ? ' dirty' : '')
      + (path === S.preview ? ' preview' : '');

    const state = document.createElement('span');
    state.className = 'state';
    const paint = (hover) => { state.textContent = f.dirty && !hover ? '\u25cf' : '\u00d7'; };
    paint(false);
    state.title = f.dirty ? 'Close, losing unsaved changes' : 'Close';
    state.addEventListener('mouseenter', () => paint(true));
    state.addEventListener('mouseleave', () => paint(false));
    state.addEventListener('click', (e) => { e.stopPropagation(); closeFile(path); });

    const nm = document.createElement('span');
    nm.className = 'nm';
    // The comparison is keyed `collin:compare`, which is no path to show.
    const compare = collinTab(f);
    nm.textContent = compare ? f.path : base(path);
    const dir = document.createElement('span');
    dir.className = 'dir';
    dir.textContent = compare ? 'target/collin' : dirOf(path);

    const sv = document.createElement('button');
    sv.className = 'icon save';
    sv.title = 'Save this file';
    sv.innerHTML = SAVE_SVG;
    sv.addEventListener('click', async (e) => {
      e.stopPropagation();
      if (await saveFile(path)) { renderTabs(); toast('saved ' + base(path), 'ok'); }
    });

    row.append(state, nm, dir, sv);
    row.title = compare ? 'target/collin/' : path;
    row.addEventListener('click', () => activate(path));
    row.addEventListener('mousedown', (e) => { if (e.button === 1) { e.preventDefault(); closeFile(path); } });
    list.appendChild(row);
  }
}

/* The commands a tab's own menu offers, each with the tabs it would take.
   Right-click is the first context menu in the app; every other menu here is
   click-opened, and this one is the one people arrive expecting. */
function openTabMenu(anchor, key) {
  if (closePopup(anchor)) return;
  const menu = document.createElement('div');
  menu.className = 'envmenu';
  menu.setAttribute('role', 'menu');
  menu.append(Object.assign(document.createElement('div'),
    { className: 'menuhead', textContent: base(projectPath(key) || key) }));

  for (const [what, label, phrase] of [
    ['close', 'Close', 'Close it anyway?'],
    ['others', 'Close others', 'Close the other tabs anyway?'],
    ['right', 'Close to the right', 'Close them anyway?'],
    ['saved', 'Close saved', 'Close the saved tabs anyway?'],
  ]) {
    const keys = closeTargets(what, key, S.order, S.open);
    const b = document.createElement('button');
    b.type = 'button';
    b.setAttribute('role', 'menuitem');
    b.append(Object.assign(document.createElement('span'), { className: 'lbl', textContent: label }));
    if (keys.length > 1) {
      b.append(Object.assign(document.createElement('span'),
        { className: 'sub', textContent: String(keys.length) }));
    }
    // openPopup's roving focus skips these, so a command that would take
    // nothing is neither clickable nor reachable by arrow.
    if (!keys.length) b.setAttribute('aria-disabled', 'true');
    else b.addEventListener('click', () => { closePopup(); closeMany(keys, phrase); });
    menu.append(b);
  }
  openPopup(anchor, menu);
}

/* Following a ref(), a macro or a search hit moves the cursor somewhere nobody
   typed, and until now there was no way back. Two stacks, the way a browser
   keeps them: going somewhere new empties the forward one. */
const JUMPS = 50;

function here() {
  const f = S.active && S.open.get(S.active);
  if (!f) return null;
  // A diff and the profile have no cursor worth keeping, only the tab.
  const at = ownsPane(f) || !S.cm ? null : S.cm.getCursor();
  // And the lineage's depth, which going back puts back (jump).
  const depth = [$('#up').value, $('#down').value];
  return at ? { key: S.active, line: at.line, ch: at.ch, depth } : { key: S.active, depth };
}

/* Called before a jump, never after: afterwards the place left is gone. `pin`
   is off only for a move that is browsing rather than following a link, one
   click on a lineage box, which must leave the tabs as one click in the tree
   does. */
function pushJump({ pin = true } = {}) {
  const at = here();
  if (!at) return;
  S.back.push(at);
  if (S.back.length > JUMPS) S.back.shift();
  S.fwd = [];
  if (pin && pinLeftPreview(S)) renderTabs();
  renderCrumbs();
}

/* The first entry still worth going to, with the rest of the stack. A tab
   closed since is skipped rather than purged on close, because closing a file
   and opening it again is the ordinary thing to do. */
function nextJump(stack, open) {
  const rest = stack.slice();
  while (rest.length) {
    const at = rest.pop();
    if (open.has(at.key)) return { at, rest };
  }
  return { at: null, rest };
}

/* The tab a link is followed from has to outlive the jump, or Back has nowhere
   to land. A preview tab would not: the link opens its target as a preview,
   which takes the one preview slot, and nextJump skips a closed tab. So the tab
   left is pinned first, as VS Code pins an editor a code navigation starts from
   (`workbench.editor.enablePreviewFromCodeNavigation`, off by default). A
   chain of links leaves a pinned tab per step; the target alone stays a
   preview, until a link is followed from it in turn. */
function pinLeftPreview(state) {
  if (!state.preview || state.preview !== state.active) return false;
  state.preview = null;
  return true;
}

function jump(from, to) {
  const { at, rest } = nextJump(S[from], S.open);
  S[from] = rest;
  if (!at) return renderCrumbs();
  const back = here();
  if (back) S[to].push(back);
  if (S.active !== at.key) {
    // The lineage follows the tab as it does a click on it, but a return is
    // not a model picked anew: it comes back at the depth it was left with.
    // The boxes take it at once, so a second jump before this one has drawn
    // records it, and a canvas the tab does not move is drawn again at it.
    activate(at.key, false);
    const deeper = !!at.depth && ($('#up').value !== at.depth[0] || $('#down').value !== at.depth[1]);
    if (at.depth) [$('#up').value, $('#down').value] = at.depth;
    const inProject = projectPath(at.key);
    const moved = inProject ? syncNode(inProject, { keepDepth: true }) : Promise.resolve(false);
    moved.then((did) => { if (!did && deeper) rerender(); });
  }
  if (at.line !== undefined) gotoPos(at.line, at.ch);
  renderCrumbs();
}

const jumpBack = () => jump('back', 'fwd');
const jumpForward = () => jump('fwd', 'back');

function updateStatus() {
  const s = $('#status-file');
  if (!S.active) { s.textContent = 'no file'; return; }
  const f = S.open.get(S.active);
  if (f.kind === 'diff') {
    s.textContent = `${f.path}  ·  diff against HEAD  ·  read only`;
    return;
  }
  if (f.kind === 'compare') {
    s.textContent = `target/collin/${f.pair.a} against target/collin/${f.pair.b}  ·  read only`;
    return;
  }
  if (f.kind === 'profile') {
    const at = S.cm.getCursor();
    const hidden = f.hidden ? `  ·  ${f.hidden} secret${f.hidden === 1 ? '' : 's'} kept on the server` : '';
    s.textContent = `${f.path}  ·  ${at.line + 1}:${at.ch + 1}${f.dirty ? '  ·  modified' : ''}  ·  outside the project${hidden}`;
    return;
  }
  const c = S.cm.getCursor();
  s.textContent = `${S.active}  ·  ${c.line + 1}:${c.ch + 1}${f.dirty ? '  ·  modified' : ''}${f.truncated ? '  ·  truncated' : ''}`;
}

// ------------------------------------------------------------ breadcrumbs --
/* The bar under the tabs: where the file sits in the project, then where the
   cursor sits inside the file. Both halves navigate, the way VS Code's
   breadcrumb does. The path half asks /api/dir; the symbol half is scanned in
   the browser, from the document CodeMirror already holds, so an unsaved edit
   is reflected without a round trip. */

/* One entry per path segment. `dir` is the folder that segment's menu lists,
   which is its parent, so the first segment lists the project root.

   Split on either separator and rejoin with `/`. The graph normalises what the
   manifest gave it, so this is belt and braces rather than the fix, but a bar
   that silently becomes one long segment is not a failure anyone would read as
   a path problem. */
function pathCrumbs(path) {
  if (!path) return [];
  const parts = path.split(/[\\/]/).filter((p) => p !== '');
  return parts.map((label, i) => ({
    label,
    path: parts.slice(0, i + 1).join('/'),
    dir: parts.slice(0, i).join('/'),
  }));
}

/* Leading spaces, or -1 when the indentation contains a tab. YAML forbids a tab
   there, and misreading the nesting is worse than dropping the line. The same
   refusal src/project.rs makes for dbt_project.yml. */
function yamlIndent(line) {
  let n = 0;
  while (line[n] === ' ') n++;
  return line[n] === '\t' ? -1 : n;
}

/* Splits `key: value` at the first `:` outside quotes. YAML only starts a
   mapping when the colon is followed by a space or ends the line, so `a:b` stays
   a scalar and `url: http://x` splits once, at the right colon. */
function yamlKey(s) {
  let quote = '';
  for (let i = 0; i < s.length; i++) {
    const c = s[i];
    if (quote) { if (c === quote) quote = ''; continue; }
    if (c === '"' || c === "'") { quote = c; continue; }
    if (c !== ':') continue;
    const next = s[i + 1];
    if (next !== undefined && next !== ' ' && next !== '\t') continue;
    return { key: unquote(s.slice(0, i).trim()), value: s.slice(i + 1).trim() };
  }
  return null;
}

/* One layer of matching quotes, so a quoted key or list entry reads as itself. */
function unquote(s) {
  const q = s[0];
  return (q === '"' || q === "'") && s.length > 1 && s[s.length - 1] === q ? s.slice(1, -1) : s;
}

/* A file's text as CodeMirror hands it back: \n alone, whatever the file has.
   What is read from the disk goes through this before it is scanned or set
   beside a tab's text. In a CRLF file a `key:` ending its line is followed by
   a \r, which yamlKey takes for no key at all, so every folder written that
   way vanished and its configs piled onto the level above. */
function editorText(text) {
  return text.replace(/\r\n?/g, '\n');
}

/* Every node owns the lines up to the next node that is not below it. Filled in
   one pass so a cursor line resolves by containment rather than by guessing. */
function closeRanges(nodes, lastLine) {
  const open = [];
  for (let j = 0; j < nodes.length; j++) {
    while (open.length && nodes[open[open.length - 1]].depth >= nodes[j].depth) {
      const t = open.pop();
      nodes[t].endLine = Math.max(nodes[t].line, nodes[j].line - 1);
    }
    open.push(j);
  }
  for (const t of open) nodes[t].endLine = Math.max(nodes[t].line, lastLine);
  return nodes;
}

/* A flat outline of a YAML document: one node per mapping key and per sequence
   item, in line order, each pointing at its parent. Enough for a breadcrumb and
   no more. Flow collections, anchors and multi-document files are deliberately
   not modelled: a dbt properties file uses none of them, and a crumb that is
   sometimes wrong is worse than a crumb that is absent. */
function yamlOutline(text) {
  const lines = text.split('\n');
  const nodes = [];
  // The root frame is never popped, so every line has somewhere to attach.
  const stack = [{ indent: -1, node: -1, item: false, count: 0, leaf: false }];
  let block = -1;              // indent of the key owning a `|` or `>` body

  const add = (line, col, label, parent) => {
    nodes.push({ line, col, label, kind: 'scalar', depth: stack.length - 1, parent, endLine: line });
    return nodes.length - 1;
  };

  for (let i = 0; i < lines.length; i++) {
    const raw = lines[i];
    const trimmed = raw.trim();
    const ind = yamlIndent(raw);
    if (block >= 0) {
      // A description block is full of dashes and colons that are not structure.
      if (trimmed === '' || (ind > block && ind >= 0)) continue;
      block = -1;
    }
    if (trimmed === '' || trimmed[0] === '#' || ind < 0) continue;

    let col = ind;
    let rest = raw.slice(ind);
    let item = -1;               // the item opened on this line, if any
    // `- - a` opens two levels on one line, so the dashes are taken in a loop.
    while (rest === '-' || rest.startsWith('- ')) {
      while (stack.length > 1) {
        const top = stack[stack.length - 1];
        // A key frame at the same indent is kept: YAML lets a sequence sit at
        // its key's own column, and that is how dbt files are usually written.
        if (top.indent > col || top.leaf || (top.indent === col && top.item)) stack.pop();
        else break;
      }
      const parent = stack[stack.length - 1];
      const idx = add(i, col, String(parent.count++), parent.node);
      item = idx;
      if (parent.node >= 0) nodes[parent.node].kind = 'seq';
      stack.push({ indent: col, node: idx, item: true, count: 0, leaf: false });
      let k = 1;
      while (rest[k] === ' ') k++;
      col += k;
      rest = rest.slice(k);
      if (rest === '') break;
    }
    // `- # note` is an item with nothing on its line yet, not a string item
    // holding the comment: its keys follow on the lines below.
    if (rest === '' || rest === '-' || rest[0] === '#') continue;

    const kv = yamlKey(rest);
    if (!kv) {
      // A list of plain strings, which dbt files are full of, reads by value
      // rather than by index: `satellites > sat_web__order`, not `> 4`.
      if (item >= 0 && rest !== '') {
        nodes[item].label = unquote(rest);
        stack[stack.length - 1].leaf = true;
      }
      continue;
    }
    while (stack.length > 1) {
      const top = stack[stack.length - 1];
      if (top.indent >= col || top.leaf) stack.pop();
      else break;
    }
    const parent = stack[stack.length - 1];
    // A value that is only a comment leaves the key free to adopt children.
    const value = kv.value[0] === '#' ? '' : kv.value;
    const leaf = value !== '';
    const idx = add(i, col, kv.key, parent.node);
    if (parent.node >= 0) nodes[parent.node].kind = 'map';
    stack.push({ indent: col, node: idx, item: false, count: 0, leaf });
    if (leaf && (value[0] === '|' || value[0] === '>')) block = col;
  }
  return closeRanges(nodes, lines.length - 1);
}

/* ATX headings, and none inside a fenced block: that is what a reader navigates
   a markdown file by. */
function mdOutline(text) {
  const lines = text.split('\n');
  const nodes = [];
  let fence = '';
  for (let i = 0; i < lines.length; i++) {
    const t = lines[i].trim();
    if (fence) { if (t.startsWith(fence)) fence = ''; continue; }
    if (t.startsWith('```')) { fence = '```'; continue; }
    if (t.startsWith('~~~')) { fence = '~~~'; continue; }
    if (t[0] !== '#') continue;
    let level = 0;
    while (t[level] === '#') level++;
    if (level > 6 || (t[level] !== undefined && t[level] !== ' ')) continue;
    const label = t.slice(level).trim();
    if (!label) continue;
    let parent = -1;
    for (let j = nodes.length - 1; j >= 0; j--) {
      if (nodes[j].depth < level - 1) { parent = j; break; }
    }
    nodes.push({
      line: i, col: lines[i].indexOf('#'), label,
      kind: 'heading', depth: level - 1, parent, endLine: i,
    });
  }
  return closeRanges(nodes, lines.length - 1);
}

/* SQL is absent on purpose. A CTE name can only be found honestly by masking
   strings and comments first, and a bar that occasionally names a case arm as a
   model section is worse than a bar with nothing after the file name. */
function documentOutline(path, text) {
  const mode = modeFor(path);
  if (mode === 'text/x-yaml') return yamlOutline(text);
  if (mode === 'text/x-markdown') return mdOutline(text);
  return [];
}

/* The nodes containing a line, outermost first. */
function outlineChainAt(nodes, line) {
  const out = [];
  for (let i = 0; i < nodes.length; i++) {
    if (nodes[i].line > line) break;
    if (nodes[i].endLine >= line) out.push(i);
  }
  return out;
}

function outlineSiblings(nodes, i) {
  const out = [];
  for (let j = 0; j < nodes.length; j++) if (nodes[j].parent === nodes[i].parent) out.push(j);
  return out;
}

/* The glyph stands for the kind of the node's value, which is what VS Code
   shows: a mapping, a sequence, or a plain scalar. */
function crumbIcon(kind) {
  if (kind === 'map') return '{ }';
  if (kind === 'seq') return '[ ]';
  if (kind === 'heading') return '#';
  return 'abc';
}

function renderCrumbs() {
  const bar = $('#crumbs');
  const f = S.active ? S.open.get(S.active) : null;
  /* Hidden for a diff, which holds two documents and no cursor, and for the
     profile, which lives outside the project where /api/dir cannot list. */
  bar.textContent = '';
  if (!f) {
    bar.classList.add('hidden');
    return;
  }
  bar.classList.remove('hidden');
  bar.append(navButton('back'), navButton('fwd'));
  /* A diff holds two documents and no cursor, and the profile lives outside
     the project where /api/dir cannot list, so neither gets a path. The
     chevrons stay: they are how you get back out of one. */
  if (ownsPane(f) || f.kind === 'profile') return;

  const segs = pathCrumbs(S.active);
  segs.forEach((seg, i) => {
    if (i) bar.appendChild(crumbSep());
    bar.appendChild(crumbButton(seg.label, '', (btn) =>
      openCrumbMenu(btn, { kind: 'path', dir: seg.dir, current: seg.path })));
  });

  const nodes = S.outline && S.outline.path === S.active ? S.outline.nodes : [];
  for (const idx of outlineChainAt(nodes, S.crumbLine)) {
    const n = nodes[idx];
    bar.appendChild(crumbSep());
    bar.appendChild(crumbButton(n.label, n.kind, (btn) =>
      openCrumbMenu(btn, { kind: 'symbol', index: idx })));
  }
  if (CONFIGURABLE.test(S.active)) {
    bar.append(Object.assign(document.createElement('span'), { className: 'crumb-gap' }), configCrumb(S.active));
  }
  // A deep path scrolls: the end is the part that says where you are.
  bar.scrollLeft = bar.scrollWidth;
}

/* Where following a link came from, and where it went. Greyed at the end of
   its stack rather than hidden, so the pair does not shift the path sideways
   as you move. */
function navButton(which) {
  const b = document.createElement('button');
  b.className = 'crumb nav';
  b.type = 'button';
  b.textContent = which === 'back' ? '\u2039' : '\u203a';
  const live = nextJump(S[which], S.open).at;
  b.disabled = !live;
  b.title = which === 'back'
    ? `Back${live ? ' to ' + base(projectPath(live.key) || live.key) : ''} (${keyLabel('Mod+Alt+P', IS_MAC)})`
    : `Forward${live ? ' to ' + base(projectPath(live.key) || live.key) : ''} (${keyLabel('Mod+Alt+N', IS_MAC)})`;
  b.setAttribute('aria-label', b.title);
  b.addEventListener('click', which === 'back' ? jumpBack : jumpForward);
  return b;
}

/* A file dbt builds a node from, by its extension. Whether it sits under a
   resource path is only known once the server names the roots, which is not
   worth a request on every render: the menu says so instead. */
const CONFIGURABLE = /\.(sql|py|csv)$/i;

/* The way from a file to the keys of dbt_project.yml that reach it (0038),
   at the far end of the bar, apart from the path it is not a segment of. A
   round button in the violet of dbt's own YAML, like the profile's. */
function configCrumb(file) {
  const b = crumbButton('', '', (btn) => openCrumbMenu(btn, { kind: 'config', file }));
  b.classList.add('cfg');
  b.textContent = '';
  b.append(strokeIcon('0 0 20 16', 'cfgicon', [
    'M8.5 1.5h5l3.5 3.5v9.5h-8.5zM13.5 1.5V5H17M11 8.5h3.5M11 11.5h3.5',
    'M1.5 8h5.5M4.5 5.5 7 8l-2.5 2.5',
  ]));
  b.title = 'dbt_project.yml\nWhere it configures this file';
  b.setAttribute('aria-label', 'Where dbt_project.yml configures this file');
  return b;
}

/* The menu of configCrumb: each key reaching the file, broadest first and
   indented as the file nests them, with what it sets beside it. The note
   heading it is the point of the design: this is where dbt_project.yml
   configures the file, which is not the config the file ends up with. */
async function configMenu(file) {
  const open = S.open.get('dbt_project.yml');
  let raw;
  let roots;
  try {
    raw = open && open.doc ? open.doc.getValue() : (await api.get('/api/file?path=dbt_project.yml')).content;
    // No keys, so the server only names the roots and stats nothing.
    roots = (await api.send('/api/project/resolve', 'POST', { keys: [] })).roots;
  } catch (e) {
    toast(e.message, 'err');
    return null;
  }
  return configMenuFor(raw, file, roots);
}

/* configMenu once dbt_project.yml and the roots are read, apart so a test
   reaches it. The text comes from the tab when one is open and from the disk
   otherwise, in the endings the file was saved with, so it is read as the tab
   would hold it either way: a level's line is CodeMirror's line. */
function configMenuFor(raw, file, roots) {
  const project = { label: 'dbt_project.yml', pick: () => openProjectAt(0, 0) };
  if (raw.length > OUTLINE_MAX) return { notes: ['dbt_project.yml is too large to scan.'], rows: [project] };
  const text = editorText(raw);
  const at = fileFqn(file, roots, projectNameIn(text));
  if (!at) {
    return {
      notes: ['Not a model, seed, snapshot, analysis or test under the paths dbt_project.yml sets.'],
      rows: [project],
    };
  }
  const levels = configLevels(projectPathKeys(text), at);
  if (!levels.length) return { notes: ['No key of dbt_project.yml reaches this file.'], rows: [project] };
  return {
    notes: ['Where dbt_project.yml configures this file. A config() in it, or in its properties YAML, wins.'],
    rows: levels.map((k, i) => {
      const sets = k.sets.map((s) => (s.value ? `${s.name}: ${s.value}` : s.name)).join(' · ');
      return {
        label: k.chain.length ? k.chain[k.chain.length - 1] : k.block,
        kind: 'map',
        depth: k.chain.length,
        sets,
        title: [k.block, ...k.chain].join(' › ') + (sets ? `\n${sets}` : ''),
        current: i === levels.length - 1,
        pick: () => openProjectAt(k.line, k.col),
      };
    }),
  };
}

/* dbt_project.yml at a key, as a link is followed: the chevrons lead back. */
async function openProjectAt(line, ch) {
  closeCrumbMenu();
  pushJump();
  await openFile('dbt_project.yml', { focusLineage: false, preview: true });
  const f = S.open.get('dbt_project.yml');
  if (S.active !== 'dbt_project.yml' || !f || !f.doc) return;  // it could not be opened
  revealInTree('dbt_project.yml');
  gotoPos(line, ch);
}

function crumbSep() {
  return Object.assign(document.createElement('span'), { className: 'crumb-sep', textContent: '›' });
}

function crumbButton(label, kind, open) {
  const b = document.createElement('button');
  b.className = 'crumb';
  b.type = 'button';
  if (kind) {
    const ic = document.createElement('span');
    ic.className = 'sicon';
    ic.dataset.kind = kind;
    ic.textContent = crumbIcon(kind);
    b.appendChild(ic);
  }
  b.append(Object.assign(document.createElement('span'), { className: 'lbl', textContent: label }));
  b.addEventListener('click', () => {
    if (crumbMenu && crumbMenu.anchor === b) return closeCrumbMenu();
    open(b);
  });
  return b;
}

// A document larger than this is not worth scanning on every keystroke pause.
const OUTLINE_MAX = 2 * 1024 * 1024;

function refreshOutline() {
  S.outline = null;
  S.crumbLine = -1;
  const f = S.active ? S.open.get(S.active) : null;
  if (!f || f.kind === 'diff' || f.kind === 'profile' || !f.doc) return;
  const text = f.doc.getValue();
  if (text.length > OUTLINE_MAX) return;
  S.outline = { path: S.active, nodes: documentOutline(S.active, text) };
  S.crumbLine = S.cm ? S.cm.getCursor().line : 0;
}

/* Only a change of line can change the chain, and the cursor moves far more
   often than that. */
function crumbCursor() {
  if (!S.outline || S.outline.path !== S.active || !S.cm) return;
  const line = S.cm.getCursor().line;
  if (line === S.crumbLine) return;
  S.crumbLine = line;
  renderCrumbs();
}

/* Puts the cursor somewhere in the active document and shows it. Shared by the
   search results and the symbol crumbs, which want the same three steps. */
// `focus` false moves the cursor and the view and leaves the keyboard where
// it was, for a click on the canvas that shows a line without writing there.
function gotoPos(line, ch = 0, focus = true) {
  if (!S.cm) return;
  const pos = { line: Math.max(0, line), ch: Math.max(0, ch) };
  S.cm.setCursor(pos);
  S.cm.scrollIntoView({ from: pos, to: pos }, 120);
  if (focus) S.cm.focus();
}

let crumbMenu = null;                        // { el, anchor, off }
let crumbAsk = 0;                            // the latest menu asked for

function closeCrumbMenu({ refocus = false } = {}) {
  if (!crumbMenu) return;
  const { el, anchor, off } = crumbMenu;
  crumbMenu = null;
  off();
  el.remove();
  anchor.classList.remove('open');
  if (refocus) anchor.focus();
}

/* The menu a crumb opens. A path crumb lists the folder it sits in, so picking
   a sibling is one click; a folder inside it reopens the menu one level down,
   which is how VS Code lets you walk the tree without leaving the bar. */
async function openCrumbMenu(anchor, spec) {
  closeCrumbMenu();
  // A card that opened by accident must not sit over a menu opened on purpose.
  closeHoverCard();
  // The config menu waits on two requests: a second click, or a bar redrawn
  // under it, must not leave a menu floating nowhere.
  const ask = ++crumbAsk;

  let rows = [];
  let notes = [];
  if (spec.kind === 'config') {
    const menu = await configMenu(spec.file);
    if (!menu) return;
    ({ rows, notes } = menu);
  } else if (spec.kind === 'path') {
    let entries;
    try { entries = await api.get('/api/dir?path=' + encodeURIComponent(spec.dir)); }
    catch (e) { return toast(e.message, 'err'); }
    rows = entries.map((entry) => ({
      label: entry.name,
      dir: entry.dir,
      current: entry.path === spec.current,
      pick: () => {
        if (entry.dir) return openCrumbMenu(anchor, { kind: 'path', dir: entry.path, current: '' });
        closeCrumbMenu();
        pushJump();
        openFile(entry.path, { preview: true });
        revealInTree(entry.path);
      },
    }));
  } else {
    const nodes = S.outline ? S.outline.nodes : [];
    if (!nodes[spec.index]) return;
    rows = outlineSiblings(nodes, spec.index).map((j) => ({
      label: nodes[j].label,
      kind: nodes[j].kind,
      current: j === spec.index,
      pick: () => { closeCrumbMenu(); gotoPos(nodes[j].line, nodes[j].col); },
    }));
  }
  if (!rows.length || ask !== crumbAsk || !anchor.isConnected) return;

  const el = document.createElement('div');
  el.className = spec.kind === 'config' ? 'crumbmenu config' : 'crumbmenu';
  for (const text of notes) el.appendChild(Object.assign(document.createElement('div'), { className: 'note', textContent: text }));
  for (const row of rows) {
    const b = document.createElement('button');
    b.type = 'button';
    b.className = row.current ? 'on' : '';
    if (row.depth) b.style.paddingLeft = `${8 + 12 * row.depth}px`;
    if (row.title) b.title = row.title;
    const ic = document.createElement('span');
    if (row.kind) {
      ic.className = 'sicon';
      ic.dataset.kind = row.kind;
      ic.textContent = crumbIcon(row.kind);
      b.appendChild(ic);
    } else if (row.dir) {
      ic.className = 'caret';
      ic.innerHTML = CHEVRON;
      b.appendChild(ic);
    } else {
      b.appendChild(fileIcon(row.label));
    }
    b.append(Object.assign(document.createElement('span'), { className: 'lbl', textContent: row.label }));
    if (row.sets) b.append(Object.assign(document.createElement('span'), { className: 'sets', textContent: row.sets }));
    b.addEventListener('click', row.pick);
    el.appendChild(b);
  }
  document.body.appendChild(el);
  anchor.classList.add('open');

  const box = el.getBoundingClientRect();
  const p = placeFloating(anchor.getBoundingClientRect(), { width: box.width, height: box.height },
    { width: window.innerWidth, height: window.innerHeight }, 2);
  el.style.top = `${p.top}px`;
  el.style.left = `${p.left}px`;

  const buttons = [...el.querySelectorAll('button')];
  const onKey = (e) => {
    if (e.key === 'Tab') return closeCrumbMenu();
    const i = buttons.indexOf(document.activeElement);
    const step = { ArrowDown: 1, ArrowUp: -1 }[e.key];
    if (e.key !== 'Escape' && !step) return;
    // Handled here only: the editor and the global shortcuts never see it.
    e.preventDefault();
    e.stopPropagation();
    if (e.key === 'Escape') closeCrumbMenu({ refocus: true });
    else buttons[(i + step + buttons.length) % buttons.length].focus();
  };
  const onDown = (e) => { if (!el.contains(e.target) && !anchor.contains(e.target)) closeCrumbMenu(); };
  // Scrolling the crumb bar away leaves the menu floating in the wrong place.
  const onScroll = (e) => {
    const t = e.target;
    if (t === document || (t instanceof Node && t.contains(anchor))) closeCrumbMenu();
  };
  const onResize = () => closeCrumbMenu();
  document.addEventListener('keydown', onKey, true);
  document.addEventListener('mousedown', onDown, true);
  document.addEventListener('scroll', onScroll, true);
  window.addEventListener('resize', onResize);
  crumbMenu = {
    el, anchor,
    off: () => {
      document.removeEventListener('keydown', onKey, true);
      document.removeEventListener('mousedown', onDown, true);
      document.removeEventListener('scroll', onScroll, true);
      window.removeEventListener('resize', onResize);
    },
  };
  (buttons.find((b) => b.classList.contains('on')) || buttons[0]).focus();
}

// ----------------------------------------------------------------- icons --
const CHEVRON = '<svg viewBox="0 0 16 16" width="11" height="11" fill="none" stroke="currentColor" ' +
  'stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"><path d="M6 3.5L10.5 8 6 12.5"/></svg>';

/* A plain database glyph for .sql files. Drawn for this project: it is not
   derived from any vendor logo. */
const SQL_ICON = '<svg viewBox="0 0 16 16" width="13" height="13" fill="none" stroke="currentColor" stroke-width="1.3"><ellipse cx="8" cy="3.6" rx="5" ry="1.9"/><path d="M3 3.6v8.8c0 1.05 2.24 1.9 5 1.9s5-.85 5-1.9V3.6"/><path d="M3 8c0 1.05 2.24 1.9 5 1.9S13 9.05 13 8"/></svg>';

const DOC = '<svg viewBox="0 0 16 16" width="13" height="13" fill="none" stroke="currentColor" stroke-width="1.2" ' +
  'stroke-linecap="round" stroke-linejoin="round"><path d="M9 1.8H4.2a1 1 0 00-1 1v10.4a1 1 0 001 1h7.6a1 1 0 001-1V5.6z"/>' +
  '<path d="M9 1.8v3.8h3.8"/></svg>';
const BRACES = '<svg viewBox="0 0 16 16" width="13" height="13" fill="none" stroke="currentColor" stroke-width="1.3" ' +
  'stroke-linecap="round" stroke-linejoin="round"><path d="M6.4 2.5C4.8 2.5 5 5 5 6.2S3.6 8 3.6 8 5 8.6 5 9.8 4.8 13.5 6.4 13.5"/>' +
  '<path d="M9.6 2.5c1.6 0 1.4 2.5 1.4 3.7S12.4 8 12.4 8 11 8.6 11 9.8s.2 3.7-1.4 3.7"/></svg>';
const TABLE = '<svg viewBox="0 0 16 16" width="13" height="13" fill="none" stroke="currentColor" stroke-width="1.2" ' +
  'stroke-linejoin="round"><rect x="2.3" y="3.3" width="11.4" height="9.4" rx="1.2"/><path d="M2.3 6.6h11.4M6.4 6.6v6.1"/></svg>';

function fileIcon(name) {
  const ext = name.slice(name.lastIndexOf('.') + 1).toLowerCase();
  const span = document.createElement('span');
  if (ext === 'sql') { span.className = 'ficon sql'; span.innerHTML = SQL_ICON; }
  else if (ext === 'yml' || ext === 'yaml') { span.className = 'ficon yml'; span.innerHTML = DOC; }
  else if (ext === 'md') { span.className = 'ficon md'; span.innerHTML = DOC; }
  else if (ext === 'json') { span.className = 'ficon json'; span.innerHTML = BRACES; }
  else if (ext === 'csv') { span.className = 'ficon csv'; span.innerHTML = TABLE; }
  else { span.className = 'ficon other'; span.innerHTML = DOC; }
  return span;
}

// ------------------------------------------------------------------ tree --
async function loadDir(container, path, depth) {
  let entries;
  try { entries = await api.get('/api/dir?path=' + encodeURIComponent(path)); }
  catch (e) { return toast(e.message, 'err'); }
  for (const entry of entries) {
    const row = document.createElement('div');
    row.className = 'row ' + (entry.dir ? 'folder' : 'file');
    row.style.paddingLeft = 6 + depth * 12 + 'px';
    row.dataset.path = entry.path;
    const caret = document.createElement('span');
    caret.className = 'caret';
    if (entry.dir) caret.innerHTML = CHEVRON;
    const nm = document.createElement('span');
    nm.className = 'nm';
    nm.textContent = entry.name;
    row.append(caret);
    if (!entry.dir) row.append(fileIcon(entry.name));
    row.append(nm);
    container.appendChild(row);
    S.rows.set(entry.path, { el: row, dir: entry.dir });

    if (entry.dir) {
      const kids = document.createElement('div');
      kids.className = 'kids hidden';
      container.appendChild(kids);
      row.expand = async () => {
        if (!row.loading) row.loading = loadDir(kids, entry.path, depth + 1);
        await row.loading;
        kids.classList.remove('hidden');
        row.classList.add('open');
        return kids;
      };
      row.addEventListener('click', async () => {
        if (!kids.classList.contains('hidden')) {
          kids.classList.add('hidden');
          row.classList.remove('open');
          return;
        }
        await row.expand();
      });
    } else {
      row.addEventListener('click', () => openFile(entry.path, { preview: true }));
      row.addEventListener('dblclick', () => openFile(entry.path));
    }
  }
  paintTree();
}

// ------------------------------------------------------------ git status --
function ancestorsOf(paths) {
  const out = new Set();
  for (const p of paths) {
    const parts = p.replace(/\/$/, '').split('/');
    for (let i = 1; i < parts.length; i++) out.add(parts.slice(0, i).join('/'));
  }
  return out;
}

function applyGit(info) {
  const files = (list) => new Set(list.filter((p) => !p.endsWith('/')));
  // git collapses an untracked directory into a single entry ending in "/".
  S.gitPrefixes = [...info.modified, ...info.untracked].filter((p) => p.endsWith('/'));
  S.gitMod = files(info.modified);
  S.gitUnt = files(info.untracked);
  S.gitModDirs = ancestorsOf(info.modified);
  S.gitUntDirs = ancestorsOf(info.untracked);
  for (const pre of S.gitPrefixes) S.gitUntDirs.add(pre.replace(/\/$/, ''));
  $('#status-branch').textContent = info.repo ? 'git ' + (info.branch || 'detached') : '';
  paintTree();
}

const underGitPrefix = (path) => S.gitPrefixes.some((pre) => (path + '/').startsWith(pre));

function paintTree() {
  const dirtyFiles = new Set([...S.open].filter(([, f]) => f.dirty).map(([p]) => p));
  const dirtyDirs = ancestorsOf(dirtyFiles);
  for (const [path, row] of S.rows) {
    if (!row.el.isConnected) { S.rows.delete(path); continue; }
    let state = '';
    if (row.dir) {
      if (dirtyDirs.has(path)) state = 'unsaved';
      else if (S.gitModDirs.has(path)) state = 'modified';
      else if (S.gitUntDirs.has(path) || underGitPrefix(path)) state = 'untracked';
    } else {
      if (dirtyFiles.has(path)) state = 'unsaved';
      else if (S.gitMod.has(path)) state = 'modified';
      else if (S.gitUnt.has(path) || underGitPrefix(path)) state = 'untracked';
    }
    row.el.classList.toggle('st-unsaved', state === 'unsaved');
    row.el.classList.toggle('st-modified', state === 'modified');
    row.el.classList.toggle('st-untracked', state === 'untracked');
    row.el.title = state === 'unsaved' ? path + '  (unsaved changes)'
      : state === 'modified' ? path + '  (modified, not committed)'
      : state === 'untracked' ? path + '  (untracked)'
      : path;
  }
}

async function refreshGit() {
  try {
    const info = await api.get('/api/git');
    const key = JSON.stringify(info);
    if (key === S.gitKey) return;
    S.gitKey = key;
    S.git = info;
    applyGit(info);
    renderGit(info);
  } catch { /* git missing or not a repo */ }
}

/* Repainted only when the answer changed, like the git poll above: this runs
   every few seconds and the badge is usually saying the same thing. */
async function refreshFreshness() {
  try {
    const f = await api.get('/api/freshness');
    // A poll that read run_results.json while dbt was writing it got the time
    // without the counts; if dbt finished within the same second, the stamp
    // did not move, so the index is asked again until it has them.
    const run = S.reports && S.reports.run;
    if (run && typeof run.error !== 'number') loadReportsIndex();
    const key = JSON.stringify(f);
    if (key === S.freshKey) return;
    S.freshKey = key;
    S.fresh = f;
    paintFreshness(f);
    // A run that writes a report moves this stamp, and the index is read again
    // only then. The first answer is only noted: boot has just read it.
    if (S.reportsAt !== undefined && f.reports_at !== S.reportsAt) loadReportsIndex();
    S.reportsAt = f.reports_at;
    // The watcher reloads the manifest when dbt rewrites it, and this poll is
    // the only way the page hears of it. Newer only: the answer is cached for
    // a moment, so it can trail a reload the button has just made. The canvas
    // is not redrawn beyond following the tab in front, because dbt run
    // rewrites the manifest too, and a graph reset under the pointer at every
    // run would be worse than one a click behind.
    if (S.meta && f.manifest_at > (S.meta.manifest_mtime || 0)) {
      applyInfo(await api.get('/api/meta'));
      followActive();
    }
  } catch { /* the badge is the one thing that must never interrupt anything */ }
}

function paintFreshness(f) {
  const el = $('#fresh-badge');
  const b = freshnessBadge(f);
  el.classList.remove('hidden');
  el.dataset.tone = b.tone;
  el.querySelector('.lbl').textContent = b.label;
  el.querySelector('.drift').textContent = b.drift;
  // No `title`: the hover card says all of this, and a native tooltip on top
  // of it would arrive a second later and say it again.
  el.setAttribute('aria-label', b.head + '. ' + b.desc);
}

function markTreeSelection(path) {
  $$('#tree .row').forEach((r) => r.classList.toggle('sel', r.dataset.path === path));
}

/* Reveals a path in the tree, expanding folders as needed. */
async function revealInTree(path, { expand = false } = {}) {
  const parts = path.split('/');
  let container = $('#tree');
  // `expand` walks one level further, so the folder clicked opens rather than
  // sitting selected and shut.
  for (let i = 0; i < parts.length - (expand ? 0 : 1); i++) {
    const sub = parts.slice(0, i + 1).join('/');
    const row = [...container.children].find((c) => c.dataset && c.dataset.path === sub);
    if (!row || !row.expand) return false;
    container = await row.expand();
  }
  markTreeSelection(path);
  const row = $$('#tree .row').find((r) => r.dataset.path === path);
  if (row) row.scrollIntoView({ block: 'nearest' });
  return !!row;
}

/* Everything the tree knows about what is open is in the DOM, from loadDir: a
   folder is open when its `.kids` is not hidden. So a global collapse is those
   two classes and nothing to keep in step. The children stay in the DOM, held
   by row.loading, so reopening a folder asks /api/dir nothing a second time. */
function collapseTree() {
  $$('#tree .kids').forEach((k) => k.classList.add('hidden'));
  $$('#tree .row.open').forEach((r) => r.classList.remove('open'));
}

/* The companion of the collapse: after flattening the tree, the file being
   edited is the one folder worth reopening. */
function revealActive() {
  const path = S.active ? projectPath(S.active) : '';
  if (!path) return toast('no file to reveal');
  revealInTree(path);
}

// ------------------------------------------------------------ model list --
let modelTimer = null;
async function refreshModels() {
  const q = $('#model-filter').value;
  const list = $('#model-list');
  const hits = await api.get(`/api/search?q=${encodeURIComponent(q)}&kind=model,source,seed,snapshot&limit=300`);
  list.textContent = '';
  for (const h of hits) {
    const row = document.createElement('div');
    row.className = 'row';
    const nm = document.createElement('span');
    nm.className = 'nm';
    nm.textContent = h.name;
    const sub = document.createElement('span');
    sub.className = 'sub';
    sub.textContent = h.disabled ? 'disabled' : h.kind === 'source' ? 'source' : (h.materialized || h.kind);
    if (h.disabled) row.classList.add('off');
    row.append(dot(h), nm, sub);
    row.title = h.file;
    // A click here asks for the lineage and the Catalog, which the reports
    // cover; focusNode cannot close them itself, since a run's end calls it.
    row.addEventListener('click', () => { closeReports(); focusNode(h.id, { open: false }); });
    row.addEventListener('dblclick', () => { closeReports(); focusNode(h.id, { open: true }); });
    list.appendChild(row);
  }
  if (!hits.length) list.innerHTML = '<p class="muted pad">No match.</p>';
}


// --------------------------------------------------------------- git panel --
/* A modal that resolves to the id of the button pressed, or null if dismissed.
   Used for every action that cannot simply be undone. */
function modal({ title, body, actions, wide = false, onOpen = null }) {
  return new Promise((resolve) => {
    const box = $('#modal');
    box.querySelector('.modal-box').classList.toggle('wide', wide);
    $('#modal-title').textContent = title;
    const host = $('#modal-body');
    host.textContent = '';
    host.append(body);
    const bar = $('#modal-actions');
    bar.textContent = '';
    const esc = (e) => { if (e.key === 'Escape') close(null); };
    let done = false;
    // Every way out removes the Escape listener. It used to be removed only on
    // Escape, so each button close left one behind.
    const close = (value) => {
      if (done) return;
      done = true;
      document.removeEventListener('keydown', esc);
      box.classList.add('hidden');
      resolve(value);
    };
    for (const a of actions) {
      const b = document.createElement('button');
      b.className = 'btn ' + (a.style || '');
      b.textContent = a.label;
      b.addEventListener('click', () => close(a.id));
      bar.appendChild(b);
    }
    box.classList.remove('hidden');
    box.onclick = (e) => { if (e.target === box) close(null); };
    document.addEventListener('keydown', esc);
    bar.lastChild && bar.lastChild.focus();
    // For a body that closes the modal on an answer of its own. This close
    // is this modal's alone: by the time an answer lands, the box may hold
    // another one, whose last button a click on the box would confirm.
    if (onOpen) onOpen(close);
  });
}

const para = (text) => { const p = document.createElement('p'); p.textContent = text; p.style.margin = '0 0 8px'; return p; };
const list = (items) => {
  const ul = document.createElement('ul');
  items.slice(0, 40).forEach((i) => { const li = document.createElement('li'); li.textContent = i; ul.appendChild(li); });
  if (items.length > 40) ul.appendChild(Object.assign(document.createElement('li'), { textContent: `… ${items.length - 40} more` }));
  return ul;
};
const pre = (text) => { const p = document.createElement('pre'); p.textContent = text.trim(); return p; };

/* Shows whatever git actually said. Pre-commit hooks fail here, so the output
   matters more than a tidy message. */
async function gitPost(path, body) {
  let res;
  try {
    res = await api.send('/api/git/' + path, 'POST', body || {});
  } catch (e) {
    toast('git: ' + e.message, 'err');
    return null;
  }
  await refreshGit();
  // A pull, a checkout or an abort rewrites files that may be open.
  refreshOpenFiles();
  return res;
}

async function showGitFailure(title, res) {
  const body = document.createElement('div');
  body.append(para('git exited with code ' + res.code + '.'));
  if (res.stderr.trim()) body.append(pre(res.stderr));
  if (res.stdout.trim()) body.append(pre(res.stdout));
  await modal({ title, body, actions: [{ id: 'ok', label: 'Close' }] });
}

function renderGit(info) {
  const host = $('#git-body');
  if (!host) return;
  $('#branch-name').textContent = info.repo ? (info.branch || 'detached HEAD') : 'no repository';
  const badge = $('#git-badge');
  const changes = info.staged.length + info.unstaged.length + info.untracked.length + info.conflicted.length;
  badge.textContent = String(changes);
  badge.classList.toggle('hidden', changes === 0);
  for (const [id, n] of [['#ahead-n', info.ahead], ['#behind-n', info.behind]]) {
    const el = $(id);
    el.textContent = String(n);
    el.classList.toggle('hidden', !n);
  }

  host.textContent = '';
  if (!info.repo) {
    host.append(Object.assign(document.createElement('div'), { className: 'git-empty', textContent: 'Not a git repository.' }));
    return;
  }

  if (info.merging) {
    const banner = document.createElement('div');
    banner.className = 'conflict-banner';
    banner.append(document.createTextNode(
      info.conflicted.length
        ? `Merge in progress, ${info.conflicted.length} file(s) still conflicted. Edit them, then mark each resolved.`
        : 'Merge in progress, all conflicts resolved. Commit to finish it.'));
    const abort = document.createElement('button');
    abort.className = 'btn sm danger';
    abort.style.marginTop = '7px';
    abort.textContent = 'Abort merge';
    abort.addEventListener('click', abortMerge);
    banner.append(document.createElement('br'), abort);
    host.append(banner);
  }

  const section = (label, count, action) => {
    const h = document.createElement('div');
    h.className = 'git-sec';
    h.append(document.createTextNode(label));
    const n = document.createElement('span');
    n.className = 'n';
    n.textContent = count;
    h.append(n);
    if (action) {
      const a = document.createElement('span');
      a.className = 'act';
      a.textContent = action.label;
      a.addEventListener('click', action.run);
      h.append(a);
    }
    host.append(h);
  };

  const row = (path, code, button) => {
    const r = document.createElement('div');
    r.className = 'git-row';
    const c = document.createElement('span');
    c.className = 'code st-' + (code.trim()[0] || 'Q');
    c.textContent = code.trim() || '?';
    const nm = document.createElement('span');
    nm.className = 'nm';
    nm.textContent = base(path);
    const dir = document.createElement('span');
    dir.className = 'dir';
    dir.textContent = dirOf(path);
    r.append(c, nm, dir);
    if (button) {
      const b = document.createElement('button');
      b.className = 'icon go';
      b.title = button.title;
      b.textContent = button.glyph;
      b.addEventListener('click', (e) => { e.stopPropagation(); button.run(); });
      r.append(b);
    }
    r.title = path + '  (click to see the changes)';
    r.addEventListener('click', () => openDiff(path));
    host.append(r);
  };

  if (info.conflicted.length) {
    section('Conflicts', info.conflicted.length);
    info.conflicted.forEach((p) => row(p, 'U', {
      glyph: '✓', title: 'Mark resolved (git add)',
      run: () => markResolved(p),
    }));
  }
  if (info.staged.length) {
    section('Staged', info.staged.length, { label: 'unstage all', run: () => gitPost('unstage', { paths: info.staged.map((f) => f.path) }) });
    info.staged.forEach((f) => row(f.path, f.code[0], {
      glyph: '−', title: 'Unstage', run: () => gitPost('unstage', { paths: [f.path] }),
    }));
  }
  const pending = [...info.unstaged.map((f) => ({ path: f.path, code: f.code[1] })),
                   ...info.untracked.map((p) => ({ path: p, code: 'U' }))];
  if (pending.length) {
    section('Changes', pending.length, { label: 'stage all', run: () => gitPost('stage', { paths: pending.map((f) => f.path) }) });
    pending.forEach((f) => row(f.path, f.code, {
      glyph: '+', title: 'Stage', run: () => gitPost('stage', { paths: [f.path] }),
    }));
  }
  if (!changes) {
    host.append(Object.assign(document.createElement('div'), { className: 'git-empty', textContent: 'Working tree clean.' }));
  }
}

async function markResolved(path) {
  const doc = S.open.get(path);
  if (doc && doc.dirty && !confirm(`${base(path)} has unsaved changes. Save it before marking resolved?`)) return;
  // Not written, failed or declined, the file still holds what it held: maybe
  // its markers, which staging would commit.
  if (doc && doc.dirty && !(await saveFile(path))) return;
  const res = await gitPost('stage', { paths: [path] });
  if (res && !res.ok) showGitFailure('Could not mark resolved', res);
  else toast('marked resolved: ' + base(path), 'ok');
  renderTabs();
}

async function abortMerge() {
  const body = document.createElement('div');
  body.append(para('This runs git merge --abort. The merge is undone and the working tree returns to where it was before it started.'));
  if (await modal({ title: 'Abort the merge?', body,
                    actions: [{ id: null, label: 'Cancel' }, { id: 'go', label: 'Abort merge', style: 'danger' }] }) !== 'go') return;
  const res = await gitPost('merge-abort');
  if (res && !res.ok) showGitFailure('Abort failed', res);
  else toast('merge aborted', 'ok');
}

async function doCommit() {
  const message = $('#commit-msg').value.trim();
  if (!message) return toast('write a commit message first');
  const info = S.git;
  if (info && info.conflicted.length) return toast('resolve the conflicts first', 'err');
  if (info && !info.staged.length && !info.merging) return toast('nothing staged: stage some files first', 'err');

  const btn = $('#commit-btn');
  btn.disabled = true;
  btn.textContent = 'Fetching, then committing…';   // hooks can take a while
  const res = await gitPost('commit', { message });
  btn.disabled = false;
  btn.textContent = 'Commit';
  if (!res) return;
  if (!res.ok) return showGitFailure('Commit failed', res);
  $('#commit-msg').value = '';
  toast('committed', 'ok');
  if (res.fetch_note) toast(res.fetch_note, 'err');
}

async function doPush() {
  let commits = [];
  try { commits = await api.get('/api/git/outgoing'); } catch { /* no upstream yet */ }
  const info = S.git || {};
  const body = document.createElement('div');
  body.append(para(`Pushing ${info.branch} to origin${info.upstream ? '' : ' (and setting it as upstream)'}.`));
  body.append(commits.length ? list(commits) : para('No upstream yet, so this is the first push of this branch.'));
  if (await modal({ title: 'Push to origin?', body,
                    actions: [{ id: null, label: 'Cancel' }, { id: 'go', label: 'Push', style: 'primary' }] }) !== 'go') return;
  const res = await gitPost('push');
  if (res && !res.ok) showGitFailure('Push failed', res);
  else toast('pushed', 'ok');
}

async function doPull() {
  const res = await gitPost('pull');
  if (!res) return;
  if (!res.ok) {
    if (/diverge|not possible to fast-forward/i.test(res.stderr)) {
      const body = document.createElement('div');
      body.append(para('Your branch and origin have diverged, so a fast-forward is impossible.'));
      body.append(para('dbt-edith will not pick a merge or a rebase for you: run the one you want in the terminal.'));
      body.append(pre(res.stderr));
      return modal({ title: 'Cannot fast-forward', body, actions: [{ id: 'ok', label: 'Close' }] });
    }
    return showGitFailure('Pull failed', res);
  }
  toast(res.stdout.includes('Already up to date') ? 'already up to date' : 'pulled', 'ok');
}

// ------------------------------------------------------------ branch picker --
let branchHits = [], branchIndex = 0;

async function openBranches() {
  const box = $('#branches');
  box.classList.remove('hidden');
  const input = $('#branch-filter');
  input.value = '';
  input.focus();
  try { branchHits = await api.get('/api/git/branches'); }
  catch (e) { return toast('git: ' + e.message, 'err'); }
  paintBranches('');
}

function paintBranches(query) {
  const q = query.trim().toLowerCase();
  const rows = branchHits.filter((b) => !q || b.name.toLowerCase().includes(q)).slice(0, 80);
  branchIndex = 0;
  const host = $('#branch-results');
  host.textContent = '';
  rows.forEach((b, i) => {
    const r = document.createElement('div');
    r.className = 'pres' + (i === 0 ? ' on' : '');
    const p1 = document.createElement('span');
    p1.className = 'p1';
    p1.textContent = b.name;
    if (b.current) p1.style.color = 'var(--accent)';
    const p2 = document.createElement('span');
    p2.className = 'p2';
    p2.textContent = (b.current ? 'current · ' : '') + b.when;
    r.append(p1, p2);
    r.addEventListener('click', () => chooseBranch(b));
    host.appendChild(r);
  });
  host.dataset.rows = rows.length;
  branchHits.visible = rows;
}

function moveBranch(delta) {
  const rows = $$('#branch-results .pres');
  if (!rows.length) return;
  rows[branchIndex].classList.remove('on');
  branchIndex = (branchIndex + delta + rows.length) % rows.length;
  rows[branchIndex].classList.add('on');
  rows[branchIndex].scrollIntoView({ block: 'nearest' });
}

async function chooseBranch(b) {
  $('#branches').classList.add('hidden');
  if (b.current) return;
  const res = await gitPost('checkout', { branch: b.name, stash: false });
  if (!res) return;
  if (res.ok) return toast('switched to ' + b.name, 'ok');

  // git refused. Never force: show what is in the way and let the user choose.
  const blocking = res.blocking || [];
  const body = document.createElement('div');
  body.append(para(`git will not switch to ${b.name} because local changes would be overwritten.`));
  if (blocking.length) body.append(list(blocking));
  else body.append(pre(res.stderr));
  body.append(para('Stashing puts those changes aside, tagged with the branch you are leaving, and git stash pop brings them back. Nothing is discarded.'));
  const choice = await modal({
    title: 'Switch blocked',
    body,
    actions: [{ id: null, label: 'Cancel' }, { id: 'stash', label: 'Stash and switch', style: 'primary' }],
  });
  if (choice !== 'stash') return;
  const second = await gitPost('checkout', { branch: b.name, stash: true });
  if (second && !second.ok) showGitFailure('Switch still failed', second);
  else toast('stashed, then switched to ' + b.name, 'ok');
}

// ------------------------------------------------------------- conflicts ----
/* Decorates the <<<<<<< ======= >>>>>>> blocks git leaves behind, and offers a
   one-click choice per block. */
/* Locates the conflict blocks in a text. A block only counts once all three
   markers have been seen in order, so half-edited files are simply ignored
   rather than half-resolved. */
function findConflicts(text) {
  const blocks = [];
  let open = null;
  text.split('\n').forEach((line, i) => {
    if (line.startsWith('<<<<<<<')) open = { start: i, mid: -1 };
    else if (line.startsWith('=======') && open && open.mid < 0) open.mid = i;
    else if (line.startsWith('>>>>>>>') && open && open.mid >= 0) { blocks.push({ start: open.start, mid: open.mid, end: i }); open = null; }
  });
  return blocks;
}

function decorateConflicts(cm) {
  (cm.state.conflictWidgets || []).forEach((w) => w.clear());
  cm.state.conflictWidgets = [];
  cm.eachLine((line) => {
    cm.removeLineClass(line, 'background', 'cm-conflict-head');
    cm.removeLineClass(line, 'background', 'cm-conflict-mid');
  });

  const blocks = findConflicts(cm.getValue());
  if (!blocks.length) return 0;

  for (const b of blocks) {
    cm.addLineClass(b.start, 'background', 'cm-conflict-head');
    cm.addLineClass(b.mid, 'background', 'cm-conflict-mid');
    cm.addLineClass(b.end, 'background', 'cm-conflict-head');
    const bar = document.createElement('div');
    bar.className = 'conflict-bar';
    for (const [label, side] of [['Keep ours', 'ours'], ['Keep theirs', 'theirs'], ['Keep both', 'both']]) {
      const btn = document.createElement('button');
      btn.textContent = label;
      btn.addEventListener('click', () => resolveBlock(cm, b, side));
      bar.appendChild(btn);
    }
    cm.state.conflictWidgets.push(cm.addLineWidget(b.start, bar, { above: true }));
  }
  return blocks.length;
}

function resolveBlock(cm, b, side) {
  const ours = cm.getRange({ line: b.start + 1, ch: 0 }, { line: b.mid, ch: 0 });
  const theirs = cm.getRange({ line: b.mid + 1, ch: 0 }, { line: b.end, ch: 0 });
  const keep = side === 'ours' ? ours : side === 'theirs' ? theirs : ours + theirs;
  cm.replaceRange(keep, { line: b.start, ch: 0 }, { line: b.end + 1, ch: 0 });
  decorateConflicts(cm);
}

// ---------------------------------------------------------------- lineage --
/* The depth is how far a walk on the canvas has looked around: a box
   double-clicked keeps it, as a +N badge raises it. A model reached any other
   way, from the palette, a tab, a link or the Catalog's lists, is a fresh
   start, and leaving the depth of the last one drew a graph nobody asked for.
   Back and Forward are returns, and bring back the depth they left (jump). */
async function focusNode(id, { open = false, keepDepth = false } = {}) {
  closeHoverCard();
  if (id !== S.focus && !keepDepth) {
    for (const el of [$('#up'), $('#down')]) el.value = el.defaultValue;
  }
  S.focus = id;
  S.jobsFor = null;
  S.unsynced = '';
  S.graphMode = 'model';
  S.colFocus = null;
  paintMode();
  const up = +$('#up').value, down = +$('#down').value;
  const tests = testsOn() ? 1 : 0;
  try {
    const [sub, detail] = await Promise.all([
      api.get(`/api/lineage?id=${encodeURIComponent(id)}&up=${up}&down=${down}&tests=${tests}`),
      nodeDetail({ id }),
    ]);
    $('#lineage-empty').classList.add('hidden');
    const drawn = Lineage.render(sub, canvasOptions());
    $('#lineage-status').textContent = canvasStatus(sub, drawn);
    paintLegend(sub);
    paintTestsCount(sub);
    renderCatalog(detail);
    for (const kind of ['compiled', 'run']) {
      if (!$('#dock-' + kind).classList.contains('hidden')) loadArtifact(kind, id);
    }
    if (S.dock === 'jobs') loadJobs();
    if (open && detail.file) openFile(detail.file, { focusLineage: false });
  } catch (e) { toast('lineage: ' + e.message, 'err'); }
}

/* Column nodes carry a composite id, "<unique_id>::<column>". Model nodes carry
   a bare unique_id, and a dbt unique_id never contains "::". */
function splitColId(id) {
  const at = id.indexOf('::');
  return at < 0 ? { node: id, column: '' } : { node: id.slice(0, at), column: id.slice(at + 2) };
}

/* Column-level lineage. Sibling of focusNode: same canvas, same pan/zoom, the
   nodes are columns instead of models. */
async function focusColumn(id, column) {
  S.graphMode = 'column';
  S.colFocus = { id, column };
  paintMode();
  const up = +$('#up').value, down = +$('#down').value;
  try {
    const sub = await api.get(
      `/api/collineage?id=${encodeURIComponent(id)}&column=${encodeURIComponent(column)}&up=${up}&down=${down}`);
    $('#lineage-empty').classList.add('hidden');
    const drawn = Lineage.render(sub, canvasOptions());
    $('#lineage-status').textContent = canvasStatus(sub, drawn);
    paintLegend(sub);
    paintTestsCount(sub);
    showDock('lineage');
  } catch (e) {
    toast('column lineage: ' + e.message, 'err');
    S.graphMode = 'model';
    S.colFocus = null;
    paintMode();
  }
}

/* Every control that used to re-run focusNode has to respect the current mode. */
const rerender = () => S.graphMode === 'select'
  ? runSelection()
  : (S.graphMode === 'column' && S.colFocus)
    ? focusColumn(S.colFocus.id, S.colFocus.column)
    : (S.focus ? focusNode(S.focus) : undefined);

/* How the canvas draws what it is sent, in every mode. The by folder toggle
   hands the layout this project's package, which is how it tells the
   project's own folders from an installed package's (0029). */
function canvasOptions() {
  return {
    folders: foldersOn() ? ((S.meta && S.meta.project) || '') : null,
    adapter: (S.meta && S.meta.adapter) || '',
  };
}

// ------------------------------------------------------------- selection --
/* A dbt selector expression, resolved by the server against the manifest and
   drawn as a set rather than as a neighbourhood of one node. dbt is never run
   for it (0024): the terminal button is how you check that against dbt itself. */
let selectTimer = null;

async function runSelection() {
  closeHoverCard();
  S.graphMode = 'select';
  S.colFocus = null;
  paintMode();
  const expr = $('#select-input').value.trim();
  const ask = ++S.selectAsk;
  if (!expr) {
    S.selectSub = null;
    Lineage.clear();
    emptyHint('Type a dbt selector, for example my_model+');
    $('#select-sum').textContent = '';
    $('#lineage-status').textContent = '';
    paintSelectWarnings([]);
    return;
  }
  const tests = testsOn() ? 1 : 0;
  try {
    const sub = await api.get(`/api/select?q=${encodeURIComponent(expr)}&tests=${tests}`);
    if (ask !== S.selectAsk) return;          // a later keystroke already asked
    S.selectSub = sub;
    // A pasted command is written back as what was actually resolved, so the
    // box and the picture never say two different things.
    if (sub.stripped) $('#select-input').value = selectionText(sub);
    rememberSelection($('#select-input').value.trim());
    // Before drawing: the warnings make the bar taller, and the folder names
    // are placed below it.
    paintSelectWarnings(selectWarnings(selectNotes(sub).concat(sub.warnings || [])));
    let drawn = null;
    if (sub.nodes.length) {
      $('#lineage-empty').classList.add('hidden');
      drawn = Lineage.render(sub, canvasOptions());
      paintLegend(sub);
      paintTestsCount(sub);
    } else {
      Lineage.clear();
      emptyHint(selectEmptyText(sub));
    }
    $('#select-sum').textContent = selectSummary(sub);
    $('#select-sum').title = sub.description || '';
    $('#lineage-status').textContent = canvasStatus(sub, drawn);
  } catch (e) {
    if (ask !== S.selectAsk) return;
    S.selectSub = null;
    $('#select-sum').textContent = '';
    $('#select-sum').title = '';
    paintSelectWarnings([e.message], true);
  }
}

const emptyHint = (text) => {
  $('#lineage-empty').classList.remove('hidden');
  $('#lineage-empty').querySelector('p').textContent = text;
};

/* Session history, and no further. Persisting it would need a settings route
   of its own (0011), which retyping a selector has not yet been worth. */
function rememberSelection(expr) {
  if (!expr) return;
  S.selectHistory = [expr].concat(S.selectHistory.filter((e) => e !== expr)).slice(0, 20);
  S.selectHistAt = -1;
}

/* A +N badge names its own box in the expression instead of bumping a depth
   box, because in this mode the text is what the picture is made of. */
function expandSelection(dir, node) {
  // A named selector is dbt's set as the project defined it: a term appended
  // to it would be refused, since --selector stands alone.
  if (S.selectSub && S.selectSub.selector) {
    toast('a named selector is the project\'s own set: type an expression to grow it', 'err');
    return;
  }
  const box = $('#select-input');
  box.value = expandTerm(box.value, node.name, dir);
  runSelection();
}

function paintSelectWarnings(lines, bad) {
  const host = $('#select-warn');
  host.textContent = '';
  host.classList.toggle('hidden', !lines.length);
  host.classList.toggle('bad', !!bad);
  for (const line of lines) {
    const row = document.createElement('div');
    row.textContent = line;
    row.title = line;
    host.appendChild(row);
  }
}

/* One place for the clipboard, because a Copy that silently does nothing is
   worse than no button: the browser refuses it often enough to need the toast. */
async function copyText(text, label) {
  try {
    await navigator.clipboard.writeText(text);
    toast('copied ' + (label || text), 'ok');
  } catch { toast('clipboard unavailable', 'err'); }
}

/* What the selection is made of, by kind, biggest first. Counted by the server
   over everything that matched, so a capped canvas still reports the whole set. */
function selectKindCounts(counts) {
  return Object.entries(counts || {}).sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0]));
}

function selectSummary(sub) {
  if (!sub.matched) return 'nothing matched';
  return selectKindCounts(sub.counts).map(([kind, n]) => `${n} ${kind}${n > 1 ? 's' : ''}`).join('  ·  ');
}

/* The canvas half of the answer, which is not the same number: what was drawn,
   against what matched, so a capped picture says so instead of looking whole. */
function selectStatus(sub) {
  if (!sub.matched) return '';
  // A model drawn only for a selected test's sake is no part of the answer,
  // and a test the tests box keeps off the canvas still is (0032).
  const context = sub.nodes.filter((n) => n && n.context).length;
  const drawn = sub.nodes.length - context;
  const hidden = sub.hidden_tests || 0;
  const head = sub.truncated ? `${drawn} of ${sub.matched} drawn` : `${drawn} node${drawn === 1 ? '' : 's'}`;
  const edges = `${sub.edges.length} edge${sub.edges.length === 1 ? '' : 's'}`;
  const tests = hidden && `${hidden} test${hidden === 1 ? '' : 's'} hidden`;
  // Context alone on the canvas, a selector of tests with the tests box off,
  // leads with what is there: `0 nodes` above a canvas of boxes reads as a bug.
  if (!drawn && context && !sub.truncated) return [`${context} for context`, edges, tests].filter(Boolean).join(' · ');
  const tail = [context && `${context} for context`, tests].filter(Boolean).map((s) => ` · ${s}`).join('');
  return `${head} · ${edges}${tail}`;
}

/* The line under the canvas, whichever mode drew it. One function rather than
   a string at each caller, so an exported graph repeats the screen's words.
   `drawn` is what Lineage.render() said the folder bands added, if any. */
function canvasStatus(sub, drawn) {
  if (sub.mode === 'select') return selectStatus(sub) + foldersStatus(drawn);
  const tail = `${sub.edges.length} edges${sub.truncated ? ' · truncated' : ''}`;
  return (sub.mode === 'column'
    ? `${sub.focus_column} · ${sub.nodes.length} columns · ${tail}`
    : `${sub.nodes.length} nodes · ${tail}`) + foldersStatus(drawn);
}

/* How many folders the canvas is drawn in, and how many edges run from a
   later folder to an earlier one, which the canvas dashes: without the count,
   a dashed edge in model mode would look like a loop dbt does not allow. */
function foldersStatus(drawn) {
  if (!drawn) return '';
  const n = drawn.folders, back = drawn.against, folded = drawn.folded || 0;
  // Folded tests are in the answer and off the canvas, under a chip that says
  // how many (0035): the line counts them too, so nothing reads as missing.
  return (n ? ` · ${n} folder${n === 1 ? '' : 's'}`
    + (back ? ` · ${back} edge${back === 1 ? '' : 's'} against their order` : '') : '')
    + (folded ? ` · ${folded} test${folded === 1 ? '' : 's'} folded` : '');
}

/* Four at most: a selector with twenty bad terms has one mistake in it, not
   twenty, and a wall of amber hides the graph it is about. */
function selectWarnings(list) {
  const seen = [...new Set(list || [])];
  return seen.length <= 4 ? seen : seen.slice(0, 4).concat(`and ${seen.length - 4} more`);
}

/* The command that should print the same list, for checking this build against
   dbt itself. No escaping: the server strips quotes while parsing, so what
   comes back can never contain one. */
function lsCommand(select, exclude) {
  const quote = (s) => '"' + String(s).replace(/"/g, '') + '"';
  const tail = exclude ? ` --exclude ${quote(exclude)}` : '';
  return `dbt ls --select ${quote(select)}${tail} --output name`;
}

/* A named selector's command has nothing else to carry: dbt ignores --select
   and --exclude beside --selector, and so does the box. */
function selectorCommand(name) {
  return `dbt ls --selector "${String(name).replace(/"/g, '')}" --output name`;
}

/* What was resolved, in the box's own words. */
function selectionText(sub) {
  if (sub.selector) return `--selector ${sub.selector}`;
  return (sub.select || '') + (sub.exclude ? ` --exclude ${sub.exclude}` : '');
}

/* The name in a `--selector NAME` in the box, so the dbt ls button can still
   type the command for a selector this build refused: dbt settles those too. */
function boxSelector(text) {
  const m = /(?:^|\s)--selector(?:=|\s+)["']?([^\s"']+)/.exec(String(text || ''));
  return m ? m[1] : '';
}

/* The menu's filter. A name that matches comes before a description that
   does, and each half keeps the list's order, which is by name. */
function filterSelectors(list, q) {
  const needle = String(q || '').trim().toLowerCase();
  if (!needle) return (list || []).slice();
  const byName = [];
  const byText = [];
  for (const s of list || []) {
    if (s.name.toLowerCase().includes(needle)) byName.push(s);
    else if ((s.description || '').toLowerCase().includes(needle)) byText.push(s);
  }
  return byName.concat(byText);
}

/* Said before any warning, so the cap on warnings can never drop it: it
   changes what every test in the answer means (0032). The server sends it
   only for a selector the lost setting could change. */
function selectNotes(sub) {
  return sub && sub.lost_indirect
    ? ['This manifest lost the indirect_selection selectors.yml sets, and this selector\'s tests depend on it: '
      + 'they follow dbt\'s default here. Re-parse with a recent dbt Fusion to fix it; dbt ls settles any doubt.']
    : [];
}

/* The menu's footer, about the manifest rather than any one selector. */
function selectorsNote(body) {
  return body && body.lost_indirect
    ? ['This manifest lost the indirect_selection selectors.yml sets. '
      + 'A selector whose tests depend on it says so when drawn.']
    : [];
}

/* Why the canvas is empty. A selector that keeps only tests, with the tests
   box off, has not matched nothing, and saying so would send someone off to
   debug a selector that works. */
function selectEmptyText(sub) {
  const hidden = (sub && sub.hidden_tests) || 0;
  if (hidden && hidden === sub.matched) {
    return `This selector keeps only tests (${hidden}): open the tests eye to see them.`;
  }
  return 'Nothing matched.';
}

/* The word on the tests eye, with the count of tests sent when there are some. */
function testsLabel(n) {
  return n ? `tests · ${n}` : 'tests';
}

/* The tests toggle's words. Data tests can outnumber the models several times
   over, and every one is a box: the tooltip says so before a click draws them. */
function testsTitle(on) {
  return on
    ? 'Data tests shown: each one is a box on the canvas, and there can be hundreds, '
      + 'which slows the canvas and buries the models. Click to hide them.'
    : 'Data tests hidden. Click to draw them on the canvas: there can be hundreds, '
      + 'which slows it and buries the models.';
}

function expandTerm(expr, name, dir) {
  const term = dir === 'up' ? `+${name}` : `${name}+`;
  const terms = String(expr).split(/\s+/).filter(Boolean);
  return (terms.includes(term) ? terms : terms.concat(term)).join(' ');
}

/* Walking the history from the box: past the newest is the empty line you
   started on, and past the oldest stays on the oldest. */
function historyStep(list, at, dir) {
  if (!list.length) return -1;
  const next = at + dir;
  if (next < 0) return -1;
  return next >= list.length ? list.length - 1 : next;
}

// ------------------------------------------------------ column lineage tool --
/* Where the column lineage comes from: Fusion, Collin, or Snowflake fetched on
   click (0031). Picking Snowflake starts tools/sf_lineage.py, and the script
   connects on the first column click, never before (0016). */
async function loadSidecar() {
  const was = S.sidecar && S.sidecar.state;
  try { S.sidecar = await api.get('/api/sidecar'); } catch { S.sidecar = null; }
  paintSidecar(was);
}

/* Paints what the script's state shows in, and keeps asking while it starts:
   the server starts it in the background, and nothing else would come back to
   say how the start ended. */
function paintSidecar(was) {
  paintSourceButton();
  paintProfileButton();
  const sc = S.sidecar;
  const now = sc ? sc.state : '';
  if (now !== was && S.node) renderCatalog(S.node);
  if (was === 'starting' && now === 'failed') toast('Snowflake lineage: ' + (sc.error || 'the script could not start'), 'err');
  clearTimeout(S.sidecarPoll);
  if (now === 'starting') S.sidecarPoll = setTimeout(loadSidecar, 1500);
}

const sidecarOn = () => !!(S.sidecar && S.sidecar.enabled);

/* Label, tone and tooltip of the Snowflake script for one /api/sidecar payload. */
function sidecarLabel(sc) {
  if (!sc || !sc.enabled) {
    return {
      text: 'Snowflake lineage: off', tone: 'off',
      title: 'Pick Snowflake in the column lineage menu to fetch a column\'s lineage when you click it. '
        + 'dbt-edith starts tools/sf_lineage.py with your dbt profile, and nothing connects before the first click.',
    };
  }
  const who = [sc.profile && `profile ${sc.profile}`, sc.target && `target ${sc.target}`, sc.role && `role ${sc.role}`]
    .filter(Boolean).join(', ');
  const python = sc.python ? `Python: ${sc.python}` : '';
  switch (sc.state) {
    case 'ready':
      return { text: 'Snowflake lineage: on', tone: 'on',
        title: [`Click a column to fetch its lineage${who ? ` (${who})` : ''}.`, python].filter(Boolean).join('\n') };
    case 'busy':
      return { text: 'Snowflake lineage: querying', tone: 'busy', title: ['Waiting for Snowflake.', python].filter(Boolean).join('\n') };
    case 'starting':
      return { text: 'Snowflake lineage: starting', tone: 'busy', title: 'Starting tools/sf_lineage.py' };
    case 'failed':
      return { text: 'Snowflake lineage: failed', tone: 'failed',
        title: [sc.error, python, ...(sc.log || []).slice(-8)].filter(Boolean).join('\n') };
    default:
      return { text: 'Snowflake lineage: on', tone: 'on', title: 'The script starts with the next column click.' };
  }
}

/* What to say beside the menu in the Columns tab: the script's trouble, or what
   to do next. The tooltip alone was invisible, and a toast is gone in three
   seconds. */
function columnsHint(sc, cached) {
  if (!sc || !sc.enabled) return null;
  if (sc.state === 'failed') return { text: sc.error || 'the Snowflake script could not start', tone: 'failed' };
  if (sc.state === 'starting') return { text: 'starting the Snowflake script', tone: 'busy' };
  if (sc.state === 'busy') return { text: 'querying Snowflake', tone: 'busy' };
  return cached ? null : { text: 'click a column to fetch its lineage from Snowflake', tone: 'hint' };
}

/* The same, for collin while it is the tool picked: what it is doing, why it
   failed, or that its cache is behind the manifest (0042). */
function collinHint(tool, cl, kinds) {
  if (tool !== 'collin' || !cl) return null;
  if (cl.state === 'running') return { text: 'collin is reading the compiled SQL', tone: 'busy' };
  if (cl.state === 'failed') return { text: `collin: ${cl.error || 'the run failed'}`, tone: 'failed' };
  if (!cl.found) return { text: 'collin is not installed: the column lineage menu installs it', tone: 'hint' };
  if (cl.outdated) {
    const v = shortVersion(cl.version);
    return { text: `${v ? `collin ${v} is` : 'this collin gives no version, so it is'} older than ${cl.wants}: `
      + 'the column lineage menu updates it', tone: 'hint' };
  }
  if (cl.stale) return { text: 'older than the manifest: the next column click runs collin again', tone: 'hint' };
  // Otherwise the one thing left to say is how far to trust what is drawn:
  // how many models collin read, when its report says, else the edges' share.
  const r = cl.report;
  const title = collinCaveat(kinds, r);
  const read = collinRead(r);
  if (r && r.models && read < r.models) {
    return { text: `alpha, can be wrong · SQL read for ${read} of ${r.models} models`, tone: 'alpha', title };
  }
  const { inferred, total } = inferredShare(kinds);
  const share = inferred ? Math.max(1, Math.round((100 * inferred) / total)) : 0;
  return { text: share ? `alpha, can be wrong · ${share}% guessed by name` : 'alpha, can be wrong', tone: 'alpha', title };
}

/* The models whose edges collin read from their SQL. collin counts a compiled
   file it set aside as parsed, though it matched that model's edges by name. */
function collinRead(r) {
  return r ? (r.parsed || 0) - (r.set_aside || 0) : 0;
}

/* What collin's report says about the run as a whole: how many models it read
   the SQL of, why the others were not read, where their SQL came from when the
   manifest had none, and what to do about the reasons that have a remedy here. */
function collinReadLines(r) {
  if (!r || !r.models) return [];
  const read = collinRead(r);
  const fromFiles = (r.from_files || 0) - (r.set_aside || 0);
  if (read >= r.models && !fromFiles) return [];
  const plural = (n) => `${n} model${n > 1 ? 's' : ''}`;
  const lines = read < r.models ? [`collin read the SQL of ${read} of ${r.models} models.`] : [];
  for (const x of r.reasons || []) lines.push(`${plural(x.models)}: ${x.why}`);
  if (r.set_aside) {
    lines.push(`${plural(r.set_aside)}: a file in target/compiled/ that reads a table the manifest does not give the model, `
      + 'compiled for another target or before a ref moved, so set aside');
  }
  if (fromFiles) {
    lines.push(`${plural(fromFiles)} read from target/compiled/, as the manifest has no compiled SQL for them: `
      + 'a file there can be older than the manifest.');
  }
  if (r.set_aside) lines.push('Run dbt compile with the target the manifest was written for, then Regenerate.');
  else if ((r.reasons || []).some((x) => /no compiled/.test(x.why))) {
    lines.push('Run dbt compile so target/compiled/ holds their SQL, then Regenerate.');
  }
  return lines;
}

/* The line over a model's columns when collin's report names it: why its SQL
   was not read, or how far its edges are a guess. Said on the model because
   that is where someone is about to trust an edge. */
function collinModelNote(note, columns) {
  if (!note) return null;
  const flagged = collinColumnsLine(note, columns);
  const said = collinModelSaid(note);
  return [said, flagged].filter(Boolean).join(' ') || null;
}

function collinModelSaid(note) {
  if (note.parse_error) {
    return `collin could not read this model's SQL: ${note.parse_error}. Its edges are matched by column name.`;
  }
  const file = note.sql_file && `target/${note.sql_file}`;
  if (note.set_aside) {
    return `collin set aside this model's compiled file, ${file}: it reads a table the manifest does not give this model, `
      + 'so it was compiled for another target or before a ref moved. Its edges are matched by column name.';
  }
  const said = {
    inferred: "collin did not trust this model's compiled SQL: its edges are matched by column name.",
    per_column: "This model's table is behind its compiled SQL: collin read the columns both have and matched the others by name.",
    unresolved: "collin could not resolve this model's lineage.",
  }[note.provenance] || (note.issues ? `collin noted ${note.issues} issue${note.issues > 1 ? 's' : ''} reading this model, listed in target/collin.report.json.` : null);
  // Said with whatever else is, since a file older than the manifest may be the cause.
  const from = file && `collin read its SQL from ${file}, as the manifest has none: that file can be older than the manifest.`;
  return [said, from].filter(Boolean).join(' ') || null;
}

/* collin's notes on a model's columns, by lower-cased name, each worded. */
function collinColumnNotes(note) {
  const by = new Map();
  for (const cn of (note && note.columns) || []) {
    // A compile collin could not parse produced no column it knows of, so
    // every column of the table reads as missing from it, which says nothing.
    if (note.parse_error && (cn.kind === 'missing' || cn.kind === 'unexpected')) continue;
    const text = collinColumnText(cn);
    if (!by.has(cn.column)) by.set(cn.column, []);
    if (!by.get(cn.column).includes(text)) by.get(cn.column).push(text);
  }
  return by;
}

/* `model.shop.orders` to `orders`: the report names models by unique_id. */
function shortId(id) {
  return id.split('.').slice(2).join('.') || id;
}

function nameList(names, of, short) {
  const shown = (names || []).map((x) => (short ? shortId(x) : x));
  const more = of > shown.length ? ` and ${of - shown.length} more` : '';
  return shown.join(', ') + more;
}

/* What one entry of collin's report says about a column, in a sentence that
   says what to do where there is something to do. */
function collinColumnText(cn) {
  const names = nameList(cn.names, cn.of, false);
  switch (cn.kind) {
    case 'lost':
      if (cn.detail === 'phantom') {
        return `collin lost this column's lineage: it is copied from ${names ? `the CTE ${names}` : 'a CTE'}, which does not have it, `
          + 'the shape of a select * over a parent whose column list lacks it.';
      }
      if (cn.detail === 'names_unreached' && names) {
        return `collin lost this column's lineage: its expression reads ${names}, which collin never placed.`;
      }
      return "collin lost this column's lineage: the SQL has it, and no edge came out.";
    case 'missing':
      return 'In the warehouse, but the compiled SQL does not produce it: the table is older than the code, or the code dropped it.';
    case 'unexpected':
      return 'Produced by the compiled SQL, but not in the warehouse: the table has not been rebuilt since the code changed.';
    case 'unbacked':
      return `Read from ${cn.detail ? `the CTE ${cn.detail}` : 'a CTE'}, which does not have it, so this compile cannot run as it is: `
        + 'usually a macro that read the warehouse at compile time and found a relation missing.'
        + (names ? ` ${names} under it ${cn.names.length > 1 || cn.of ? 'have' : 'has'} the name.` : '');
    case 'read_downstream': {
      const list = { warehouse: "the warehouse's", computed: "the compiled SQL's", declared: "the YAML's" }[cn.detail] || 'the';
      return `Read by ${nameList(cn.names, cn.of, true)} downstream, but absent from ${list} column list collin had for this model.`;
    }
    case 'ambiguous': {
      const from = nameList(cn.names, cn.of, true);
      if (cn.detail === 'every') return `Matched by name to each of ${from}: right for a union, wrong for a lookup join. Check it in the SQL.`;
      if (cn.detail === 'order') return `Matched by name to ${from}, the first of several parents that have it.`;
      return `Matched by name to ${from}, of several parents that have it.`;
    }
    default:
      return `collin noted ${cn.kind} on this column, in target/collin.report.json.`;
  }
}

/* The sentence over the table that points at the marks, and names the columns
   the table has no row for: a column only collin's compile produced has none. */
function collinColumnsLine(note, columns) {
  const by = collinColumnNotes(note);
  if (!by.size) return null;
  const rows = new Set((columns || []).map((c) => c.name.toLowerCase()));
  const marked = [...by.keys()].filter((c) => rows.has(c));
  const absent = [...by.keys()].filter((c) => !rows.has(c));
  const bits = [];
  if (marked.length) bits.push(`collin flags ${marked.length} column${marked.length > 1 ? 's' : ''} of this model, marked ! in the Lineage column.`);
  if (absent.length) {
    // Grouped by what is said, since one cause usually covers several.
    const groups = new Map();
    for (const c of absent) {
      const why = by.get(c).join(' ');
      if (!groups.has(why)) groups.set(why, []);
      groups.get(why).push(c);
    }
    bits.push(`It also flags ${absent.length > 1 ? `${absent.length} columns` : 'a column'} this table has no row for. `
      + [...groups].map(([why, cs]) => `${cs.join(', ')}${cs.length > 1 ? ', each' : ''}: ${why}`).join(' '));
  }
  return bits.join(' ');
}

/* `0.2.0 (a142ee5)` to `0.2.0`. */
function shortVersion(v) {
  return (v || '').split(' ')[0];
}

/* The line under Collin in the menu, once collin is installed and Collin is the
   tool picked, the one moment updating it means anything: an update when it is
   older than this dbt-edith accepts, with --force so a collin installed any
   other way is replaced, and otherwise a check, which cargo answers by
   rebuilding when GitHub has a newer commit and by "already installed" when it
   does not. dbt-edith itself never asks GitHub (0043). */
function collinUpdateRow(cl, tool) {
  if (!cl || !cl.found || tool !== 'collin') return null;
  // A version installed side by side is pinned to its tag: there is nothing
  // newer to fetch for it, and these commands reach the default one.
  if ((cl.versions || []).some((v) => v.runs && v.origin === 'installed')) return null;
  const v = shortVersion(cl.version);
  if (cl.outdated) {
    // "needs 0.2.0" over "before 0.2.0 → 0.2.0", which read as a range.
    return { label: 'Update collin', urgent: true, sub: v ? `${v}, needs ${cl.wants}` : `needs ${cl.wants}`, command: cl.update,
      title: `This dbt-edith expects collin ${cl.wants} or later, and the one found `
        + (v ? `is ${cl.version}.` : 'gives no version, so it is older than 0.2.0.')
        + '\nClick to type the command that updates it into the terminal.' };
  }
  return { label: 'Check for a newer collin', urgent: false, sub: v, command: cl.install,
    title: `collin ${cl.version}.\nClick to type the install command into the terminal: cargo rebuilds collin when `
      + 'GitHub has a newer commit, and says it is already installed when it does not.' };
}

/* How many of a cache's edges collin did not read from the SQL but matched by
   column name, because the compile could not be trusted or did not parse. */
function inferredShare(kinds) {
  let inferred = 0;
  let total = 0;
  for (const k of kinds || []) {
    total += k.edges;
    if (k.kind === 'inferred') inferred += k.edges;
  }
  return { inferred, total };
}

/* What Collin's tooltips say first: it is alpha, and how much of what it drew
   is a guess. Its edges look exactly like read ones on the canvas. */
function collinCaveat(kinds, report) {
  const lines = ['Alpha: collin is new and can be wrong. Check an edge against the compiled SQL before relying on it.'];
  const { inferred, total } = inferredShare(kinds);
  if (total && inferred === total) lines.push(`None of the ${total} edges was read from the SQL: every one is a match by column name.`);
  else if (inferred) lines.push(`${inferred} of the ${total} edges are a match by column name, not read from the SQL.`);
  const read = collinReadLines(report);
  if (read.length) lines.push('', ...read);
  return lines.join('\n');
}

/* How old a cache is, in the shortest form that is still honest. */
function cacheAge(mtime) {
  if (!mtime) return '';
  const secs = Math.max(0, Math.floor(Date.now() / 1000) - mtime);
  if (secs < 60) return 'just now';
  if (secs < 3600) return `${Math.floor(secs / 60)}min ago`;
  if (secs < 86400) return `${Math.floor(secs / 3600)}h ago`;
  return `${Math.floor(secs / 86400)}d ago`;
}

/* What one cache reads as in the menu. A cache with no source field is not
   named "unknown" but by its file, which is the only true thing about it. */
function sourceLabel(src) {
  const name = src.source || src.file.replace(/^column_lineage\.?|\.json$/g, '') || src.file;
  const bits = [src.target, cacheAge(src.mtime)].filter(Boolean);
  return { name, sub: bits.join(' · ') };
}

/* The tools the menu offers, in its order. A function rather than a constant,
   because the harness evaluates this part of the file and a const does not
   outlive its eval. `missing` is what a greyed entry says. */
function lineageToolDefs() {
  return [
    { tool: 'fusion', label: 'Fusion',
      missing: 'No column_lineage.fusion.json beside the manifest. dbt Fusion keeps its lineage as parquet under '
        + 'target/index/, which nothing here converts yet.' },
    { tool: 'collin', label: 'Collin', alpha: true,
      missing: 'No column_lineage.collin.json beside the manifest, and collin is not installed. '
        + 'Click to type the command that installs it into the terminal.' },
    { tool: 'snowflake', label: 'Snowflake', alpha: true, missing: '' },
  ];
}

/* What the menu offers, from the caches the server found, newest first, and
   what the graph holds. Each tool stands for its newest cache; every other
   file, an older one of a tool's included, stays pickable under "other
   caches" (0021). Snowflake is left out entirely while its features are off,
   and is otherwise never greyed, since it can always fetch on click (0031).
   Collin is never greyed either: installed, picking it runs it, and otherwise
   its entry offers the install command (0042). */
function lineageTools(sources, active, live, snowflake, cl) {
  const tools = [];
  const taken = new Set();
  for (const def of lineageToolDefs()) {
    const snow = def.tool === 'snowflake';
    if (snow && !snowflake) continue;
    const src = sources.find((s) => s.tool === def.tool) || null;
    if (src) taken.add(src.file);
    const onScreen = !!src && src.file === active;
    const collin = def.tool === 'collin';
    const installed = !!(cl && cl.found);
    let title = def.missing;
    let sub = src ? sourceLabel(src).sub : (snow ? 'fetch on click' : 'no cache');
    if (collin && cl && cl.state === 'running') sub = 'running';
    else if (collin && !src) sub = installed ? 'runs on pick' : 'install';
    if (collin && !src && installed) {
      title = 'Runs collin over the compiled SQL, which writes column_lineage.collin.json.';
    } else if (snow) {
      title = 'Fetches a column\'s lineage from Snowflake when you click it, under your dbt profile.'
        + (src ? `\nAdds to ${src.file}` : '');
    } else if (src) {
      title = src.file + (src.generated_at ? `\nwritten ${src.generated_at}` : '');
    }
    tools.push({
      tool: def.tool, label: def.label, src, alpha: !!def.alpha,
      available: snow || collin || !!src,
      install: collin && !src && !installed,
      active: snow ? live || onScreen : !live && (onScreen || (collin && !src && active === 'column_lineage.collin.json')),
      sub,
      title,
    });
  }
  const others = sources
    .filter((s) => !taken.has(s.file) && (snowflake || s.tool !== 'snowflake'))
    .map((src) => ({ src, active: !live && src.file === active }));
  return { tools, others };
}

/* Where the edges on screen came from, for a tooltip. Always the producer by
   name: mistaking synthetic or stale edges for warehouse truth is the
   expensive failure mode here. */
function cacheTitle(m) {
  return [
    `column lineage from ${m.cll_source || 'a cache that names no producer'}${m.cll_target ? `, target ${m.cll_target}` : ''}`,
    m.cll_file,
    m.cll_generated_at && `written ${m.cll_generated_at}`,
    m.cll_dropped && `${m.cll_dropped} row(s) dropped as unknown`,
  ].filter(Boolean).join('\n');
}

/* What the menu's button says: the tool whose edges are on screen, never what
   the menu merely offers. Snowflake picked says so before its first fetch,
   with the script's state beside it. */
function lineagePickLabel(meta, tool, live, sc, cl) {
  const m = meta || {};
  const count = m.cll_edges ? `${m.cll_edges} col edge${m.cll_edges > 1 ? 's' : ''}` : '';
  if (live) {
    const script = sidecarLabel(sc);
    const state = { starting: 'starting', busy: 'querying', failed: 'failed' }[sc && sc.state] || '';
    return { tool: 'snowflake', name: 'Snowflake', badge: 'alpha', detail: count || 'fetch on click', state, tone: script.tone,
      title: [script.title, m.cll_file && cacheTitle(m)].filter(Boolean).join('\n\n') };
  }
  const run = (tool === 'collin' && cl && cl.state) || '';
  const running = run === 'running';
  const failed = run === 'failed';
  const runState = running ? 'running' : failed ? 'failed' : '';
  const runTone = running ? 'busy' : failed ? 'failed' : '';
  if (tool === 'collin' && !m.cll_file) {
    return { tool: 'collin', name: 'Collin', badge: 'alpha', detail: running ? '' : 'no cache', state: runState,
      tone: runTone || 'off',
      title: running ? 'collin is reading the compiled SQL.' : (failed && cl.error) || 'collin has not written a cache yet.' };
  }
  if (!m.cll_file) {
    return { tool: '', name: 'column lineage: none', detail: '', state: '', tone: 'off',
      title: 'No column lineage on screen. Pick where it comes from in this menu.' };
  }
  const def = lineageToolDefs().find((d) => d.tool === tool);
  const name = def ? def.label : (m.cll_source || m.cll_file.split(/[\\/]/).pop());
  const title = [tool === 'collin' && collinCaveat(m.cll_kinds, cl && cl.report), failed && `The last run failed: ${cl.error}`, cacheTitle(m)]
    .filter(Boolean).join('\n\n');
  return { tool: def ? def.tool : '', name, badge: def && def.alpha ? 'alpha' : '', detail: count || 'no edges',
    state: runState, tone: runTone || 'on', title };
}

/* Why Snowflake's features are on or off, under the switch. */
function featureNote(f, adapter) {
  if (!f) return '';
  const now = f.snowflake ? 'on' : 'off';
  if (f.snowflake_set) return `${now}, as chosen for this project`;
  if (!adapter) return 'off: the manifest names no adapter';
  return `${now} because the manifest's adapter is ${adapter}`;
}

/* The column lineage menu's button. There are two, in the top bar and in the
   Columns tab, so it is a class and every copy is painted at once. */
function sourceMenu() {
  const b = document.createElement('button');
  b.type = 'button';
  b.className = 'btn sm cllpick';
  b.setAttribute('aria-haspopup', 'menu');
  b.addEventListener('click', (e) => {
    e.stopPropagation();
    openSourceMenu(b);
  });
  paintSourceButton(b);
  return b;
}

function paintSourceButton(only) {
  const label = lineagePickLabel(S.meta, S.cllTool, sidecarOn(), S.sidecar, S.collin);
  const span = (className, textContent) => Object.assign(document.createElement('span'), { className, textContent });
  for (const b of only ? [only] : $$('.cllpick')) {
    b.dataset.tool = label.tool;
    b.dataset.tone = label.tone;
    b.title = label.title;
    b.textContent = '';
    b.append(span('tooldot', ''), span('tname', label.name));
    if (label.badge) b.append(span('alpha', label.badge));
    if (label.detail) b.append(span('tdetail', ` · ${label.detail}`));
    if (label.state) b.append(span('tstate', ` · ${label.state}`));
    b.append(span('caret', ' ▾'));
  }
}

async function openSourceMenu(anchor) {
  if (closePopup(anchor)) return;
  // Asked again, so a cache written since the page loaded is offered too.
  try { applyLineage(await api.get('/api/meta')); } catch { /* what the page last knew still stands */ }
  const snowflake = !!(S.features && S.features.snowflake);
  const { tools, others } = lineageTools(S.cllSources || [], S.cllActive, sidecarOn(), snowflake, S.collin);
  const menu = document.createElement('div');
  menu.className = 'envmenu cllmenu';
  menu.setAttribute('role', 'menu');
  const add = ({ label, sub, tool, on, disabled, title, badge, pick, action, urgent }) => {
    const item = document.createElement('button');
    item.type = 'button';
    // An action is not one of the choices, so it carries no checked state,
    // and no dot: it is something to do with the tool above it, drawn indented
    // under that tool rather than as another tool.
    item.setAttribute('role', action ? 'menuitem' : 'menuitemradio');
    if (!action) item.setAttribute('aria-checked', String(on));
    item.dataset.tool = tool;
    if (on) item.classList.add('on');
    if (action) item.classList.add('subaction');
    if (urgent) item.dataset.urgent = 'true';
    // Not `disabled`: a disabled button swallows the hover, and its tooltip
    // is the only place that says what is missing.
    if (disabled) item.setAttribute('aria-disabled', 'true');
    if (title) item.title = title;
    if (!action) {
      item.append(
        Object.assign(document.createElement('span'), { className: 'check', textContent: on ? '✓' : '' }),
        Object.assign(document.createElement('span'), { className: 'tooldot' }),
      );
    }
    item.append(Object.assign(document.createElement('span'), { className: 'lbl', textContent: label }));
    if (badge) item.append(Object.assign(document.createElement('span'), { className: 'alpha', textContent: badge }));
    if (sub) item.append(Object.assign(document.createElement('span'), { className: 'sub', textContent: sub }));
    item.addEventListener('click', () => {
      if (disabled) return;
      closePopup();
      pick();
    });
    menu.appendChild(item);
  };
  const update = collinUpdateRow(S.collin, S.cllTool);
  const report = collinReportRow(S.collin, S.cllTool);
  const versions = collinVersionRows(S.collin, S.cllTool);
  const install = collinInstallRow(S.collin, S.cllTool);
  const compare = collinCompareRow(S.collin, S.cllTool);
  for (const t of tools) {
    add({ label: t.label, sub: t.sub, tool: t.tool, on: t.active, disabled: !t.available, title: t.title,
      badge: t.alpha ? 'alpha' : '',
      pick: t.install ? installCollin : () => switchLineage({ tool: t.tool }) });
    if (t.tool === 'collin' && report) {
      add({ label: report.label, sub: report.sub, tool: 'collin', on: false, title: report.title, action: true,
        pick: () => openReports('collin') });
    }
    if (t.tool === 'collin') {
      for (const v of versions) {
        add({ label: v.label, sub: v.sub, tool: 'collin', on: false, title: v.title, action: true,
          pick: v.runs ? () => {} : () => chooseCollinVersion(v.version) });
      }
      const gone = collinChosenGone(S.collin);
      if (gone && S.cllTool === 'collin') menu.appendChild(Object.assign(document.createElement('div'), { className: 'menunote', textContent: gone }));
      if (compare) {
        add({ label: compare.label, sub: compare.sub, tool: 'collin', on: false, title: compare.title, action: true,
          pick: () => openCollinCompare() });
      }
      if (install) {
        add({ label: install.label, sub: install.sub, tool: 'collin', on: false, title: install.title, action: true,
          pick: askCollinVersion });
      }
    }
    if (t.tool === 'collin' && update) {
      add({ label: update.label, sub: update.sub, tool: 'collin', on: false, title: update.title, action: true,
        urgent: update.urgent, pick: () => sendToTerminal(update.command) });
    }
  }
  if (others.length) {
    menu.appendChild(document.createElement('hr'));
    menu.appendChild(Object.assign(document.createElement('div'), { className: 'menuhead', textContent: 'other caches' }));
    for (const o of others) {
      const { name, sub } = sourceLabel(o.src);
      add({ label: name, sub, tool: o.src.tool || '', on: o.active, title: o.src.file,
        pick: () => switchLineage({ file: o.src.file }) });
    }
  }
  if (!snowflake) {
    menu.appendChild(Object.assign(document.createElement('div'), {
      className: 'menunote', textContent: 'Snowflake is off for this project: the settings menu turns it on.',
    }));
  }
  openPopup(anchor, menu);
}

/* One request switches the graph's column lineage and whether a click fetches
   from Snowflake, so the two can never disagree (0031). */
async function switchLineage(body) {
  try {
    applyInfo(await api.send('/api/collineage/source', 'POST', body));
    const label = lineagePickLabel(S.meta, S.cllTool, sidecarOn(), S.sidecar, S.collin);
    toast(`column lineage from ${label.name}`);
    await redrawLineage();
  } catch (e) {
    toast('column lineage: ' + e.message, 'err');
  }
}

/* Types the command that installs collin into the terminal and leaves it
   there: building it is the user's decision, and their cargo's download, never
   this server's (0042). */
function installCollin() {
  if (S.collin && S.collin.install) sendToTerminal(S.collin.install);
}

/* Runs collin and waits for it: the Regenerate button, and a column clicked
   while its cache is behind the manifest. True when the run succeeded. */
async function regenerateCollin() {
  // Painted here rather than through paintCollin: this waits for the run, so
  // nothing needs to poll for its end.
  S.collin = Object.assign({}, S.collin, { state: 'running', error: '' });
  paintSourceButton();
  if (S.node) renderCatalog(S.node);
  try {
    const info = await api.send('/api/collineage/generate', 'POST', {});
    // Settled before applyInfo sees it, so the redraw below is the only one.
    S.collin = info.collin;
    applyInfo(info);
    await redrawLineage();
    return true;
  } catch (e) {
    toast('collin: ' + e.message, 'err');
    // Forgotten first, so paintCollin does not say the failure a second time.
    S.collin = null;
    try { applyInfo(await api.get('/api/meta')); } catch { /* the toast already said it */ }
    return false;
  }
}

/* Paints what collin's state shows in, and keeps asking while a run started by
   a pick goes on, so its edges appear once it is done. */
function paintCollin(was) {
  paintSourceButton();
  paintReportsButton();
  const now = S.collin ? S.collin.state : '';
  if (now !== was && S.node) renderCatalog(S.node);
  clearTimeout(S.collinPoll);
  if (now === 'running') {
    S.collinPoll = setTimeout(async () => {
      try { applyInfo(await api.get('/api/meta')); } catch { /* asked again on the next pick */ }
    }, 1500);
  } else if (was === 'running' && now === 'done') {
    redrawLineage();
    // The run wrote its report: the button says so now rather than at the
    // next poll.
    loadReportsIndex();
  } else if (was === 'running' && now === 'failed') {
    toast('collin: ' + (S.collin.error || 'the run failed'), 'err');
  }
}

/* What each kind of finding in collin's report is, and what would remove it,
   filed under who acts: the artifacts dbt wrote, the project itself, a reader,
   or collin. The server only files the report by kind (src/collin_report.rs);
   the words are the page's, as a column's notes are. */
function collinFindingText(kind) {
  const t = {
    no_compiled_sql: ['rerun', 'No compiled SQL',
      'dbt wrote no SQL for these models, so their edges are matched by column name. Run dbt compile, then Regenerate.'],
    compiled_elsewhere: ['rerun', 'Compiled for another target',
      'Their file in target/compiled/ reads a table the manifest does not give the model, so it was set aside. '
        + 'Run dbt compile with the target the manifest was written for, then Regenerate.'],
    catalog_elsewhere: ['rerun', 'Catalog entry for another table',
      'catalog.json describes another table than the manifest names, so nothing was checked against the warehouse. '
        + 'Run dbt docs generate with the target the manifest was written for.'],
    columns_missing: ['rerun', 'Columns the warehouse has and the SQL does not produce',
      'The table is older than the code, or the code dropped them. Running the model rebuilds the table.'],
    columns_unexpected: ['rerun', 'Columns the SQL produces and the warehouse lacks',
      'The table has not been rebuilt since the code changed. Running the model rebuilds it.'],
    yaml_stale: ['project', 'YAML behind the warehouse',
      'The columns the YAML documents disagree with the table. Update the model\'s YAML.'],
    thin: ['project', 'Compile far thinner than its YAML',
      'Either the YAML is stale, or a macro that reads the warehouse at compile time found its source missing.'],
    undeclared_relation: ['project', 'Tables dbt was not told of',
      'The SQL reads a table that is neither a source nor a ref, so the lineage stops there. '
        + 'Declare it as a source, or read it through ref.'],
    unknown_parent: ['project', 'Parents with no known columns',
      'collin had no column list for them. Document their columns in YAML, or build them so the catalog has them.'],
    unbacked_read: ['project', 'Read from a CTE that lacks the column',
      'This compile cannot run as it is: usually a macro that read the warehouse at compile time and found a relation missing.'],
    merged_scope: ['project', 'Two CTEs or subqueries of one name',
      'This collin read them as one, so it did not trust the compile. Renaming one of them is enough, '
        + 'and collin reads two CTEs of one name apart from 0.4.0.'],
    relation_collision: ['project', 'Tables claimed by several nodes',
      'More than one dbt node names this table, often a source declared twice. An edge from it names the node '
        + 'the reading model depends on, and the first by name when it depends on both or neither.'],
    short_upstream: ['project', 'Columns read downstream that this model\'s list lacks',
      'A child reads them, so they exist: the table or the YAML collin had for this model is behind its code.'],
    phantom: ['project', 'Columns copied from a CTE that lacks them',
      'The shape of a select * over a parent whose column list is short: that parent\'s YAML or table is behind.'],
    parse_failed: ['check', 'SQL collin could not parse',
      'Their edges are matched by column name. If the SQL runs, the parser is at fault, and collin wants to hear of it.'],
    ambiguous: ['check', 'Matched by name to every parent that has it',
      'Right for a union, wrong for a lookup join. Check it in the SQL.'],
    engine_issue: ['check', 'Noted while reading',
      'collin\'s engine raised these and nothing else is wrong with the model. The codes are in target/collin.report.json.'],
    lost_in_expression: ['collin', 'Columns lost inside an expression',
      'The SQL reads names collin never placed, in a form it does not read yet.'],
    lost_unresolved: ['collin', 'Columns the SQL has and no edge came out for',
      'collin lost their lineage. Nothing in the project explains it.'],
  }[kind];
  // A kind from a later collin is listed rather than dropped.
  const [group, title, lever] = t || ['check', kind, 'A kind of finding this dbt-edith does not know yet.'];
  return { group, title, lever };
}

function collinGroupText(group) {
  return {
    rerun: ['Rerun dbt', 'What dbt wrote is behind the project, or from another target.'],
    project: ['Fix in the project', 'The project says less than its SQL reads, or disagrees with its warehouse.'],
    check: ['Check by hand', 'collin read these, and a person has to say whether it read them right.'],
    collin: ["collin's to fix", 'The SQL is fine and collin lost the lineage.'],
  }[group] || [group, ''];
}

/* The report's findings under their group, in the order the groups are
   worked through: rerunning dbt first, since it can clear the rest. */
function collinReportGroups(findings) {
  const groups = ['rerun', 'project', 'check', 'collin'].map((group) => {
    const [label, desc] = collinGroupText(group);
    return { group, label, desc, findings: [] };
  });
  for (const f of findings || []) {
    const t = collinFindingText(f.kind);
    groups.find((g) => g.group === t.group).findings.push(Object.assign({}, f, { title: t.title, lever: t.lever }));
  }
  return groups.filter((g) => g.findings.length);
}

/* The findings collin wrote itself, from 0.5.0, under the same groups: its
   inputs, the manifest and catalog dbt wrote, are what rerunning dbt fixes.
   Its words are kept as written (0062). An owner this page does not know is
   checked by hand. */
function collinWrittenGroups(written) {
  const groupOf = { inputs: 'rerun', project: 'project', collin: 'collin' };
  const groups = ['rerun', 'project', 'check', 'collin'].map((group) => {
    const [label, desc] = collinGroupText(group);
    return { group, label, desc, findings: [] };
  });
  for (const f of written || []) {
    const group = groupOf[f.owner] || 'check';
    groups.find((g) => g.group === group).findings.push(f);
  }
  return groups.filter((g) => g.findings.length);
}

/* The kinds whose items are columns of the entry's model, which a click can
   point at in its Columns tab. */
function collinItemsAreColumns(kind) {
  return ['columns_missing', 'columns_unexpected', 'unbacked_read', 'short_upstream', 'phantom', 'ambiguous',
    'lost_in_expression', 'lost_unresolved'].includes(kind);
}

/* What follows an entry's columns: how many were cut, and the CTE they were
   read from or copied through. */
function collinEntryTail(kind, e) {
  const shown = (e.items || []).length;
  const more = e.of > shown ? ` and ${e.of - shown} more` : '';
  const via = e.detail && { unbacked_read: `from ${e.detail}`, phantom: `through ${e.detail}` }[kind];
  return more + (via ? `, ${via}` : '');
}

/* What follows the model's name on an entry's line, as text. */
function collinEntryText(kind, e) {
  if (collinItemsAreColumns(kind)) return (e.items || []).join(', ') + collinEntryTail(kind, e);
  switch (kind) {
    case 'no_compiled_sql':
    case 'parse_failed':
      return e.detail || '';
    case 'compiled_elsewhere':
      return e.detail ? `target/${e.detail}` : '';
    case 'catalog_elsewhere':
      return e.detail ? `the catalog describes ${e.detail}` : '';
    case 'thin':
      return e.detail ? `${e.detail} columns` : '';
    default:
      return (e.items || []).join(', ') + collinEntryTail(kind, e);
  }
}

/* The figures over the report, each only when the report has its counters:
   an older collin wrote fewer. A share is rounded down, so a run one column
   short never reads as 100%. */
function collinReportTiles(t) {
  if (!t) return [];
  const has = (...keys) => keys.every((k) => typeof t[k] === 'number');
  const pct = (a, b) => (b ? `${Math.floor((1000 * a) / b) / 10}%` : '');
  const tiles = [];
  if (has('models', 'parsed')) {
    const read = t.parsed - (t.sql_from_files_set_aside || 0);
    tiles.push({ label: 'SQL read', value: `${read} of ${t.models}`, sub: t.parse_failed ? `${t.parse_failed} did not parse` : 'models' });
  }
  if (has('columns_covered', 'columns_total') && t.columns_total) {
    tiles.push({ label: 'Columns with an edge', value: pct(t.columns_covered, t.columns_total),
      sub: `${t.columns_covered} of ${t.columns_total}` });
  }
  if (has('columns_covered_independent', 'columns_total_independent') && t.columns_total_independent) {
    // From 0.5.0 collin also sets aside the columns of a model reading only
    // its own table, which have no parent in dbt either.
    tiles.push({ label: 'Against the warehouse or YAML', value: pct(t.columns_covered_independent, t.columns_total_independent),
      sub: t.columns_self ? 'columns with no parent to find aside' : 'columns built from constants aside' });
  }
  if (has('edges_parsed', 'edges_inferred')) {
    tiles.push({ label: 'Edges', value: String(t.edges_parsed + t.edges_inferred),
      sub: t.edges_inferred ? `${t.edges_inferred} matched by name` : 'all read from the SQL' });
  }
  if (has('confirmed', 'degraded', 'unchecked')) {
    tiles.push({ label: 'Checked against the warehouse', value: `${t.confirmed} confirmed`,
      sub: `${t.degraded} behind their code · ${t.unchecked} not in the catalog` });
  }
  if (has('columns_root') && t.columns_root) {
    tiles.push({ label: 'Built from constants', value: String(t.columns_root), sub: 'columns with no parent to find' });
  }
  return tiles;
}

/* What a run leaves on the Reports button until its report is shown. Only a
   run finished since the page last looked: two runs in a row both read done,
   and the counter is what tells them apart. */
function collinBadge(cl, seen) {
  if (!cl || cl.state !== 'done' || !cl.report || !cl.report.to_investigate) return null;
  if ((cl.run || 0) <= (seen || 0)) return null;
  const n = cl.report.to_investigate;
  return { text: `${n} to investigate`, short: String(n),
    title: `collin's last run names ${n} model${n > 1 ? 's' : ''} or table${n > 1 ? 's' : ''} to look at.` };
}

/* The line under Collin in the menu that opens the report, whenever Collin's
   own cache is on screen and its report says something. */
function collinReportRow(cl, tool) {
  if (!cl || tool !== 'collin' || !cl.report) return null;
  const n = cl.report.to_investigate || 0;
  return { label: "Open collin's report", sub: n ? `${n} to investigate` : 'nothing to investigate',
    title: "What collin's last run found, grouped by what would fix it, in Reports." };
}

/* A model named in the report, opened with its Columns tab, where collin's
   notes on each column already are. */
function openReportNode(id, column) {
  openFromReport(id, { dock: 'catalog', column });
}

function mountCollinTab(key) {
  const f = S.open.get(key);
  const host = $('#diff-host');
  [...host.children].forEach((c) => c.classList.add('hidden'));
  if (!f.host) {
    f.host = collinComparePane(key, f.view, f.pair);
    host.appendChild(f.host);
  }
  f.host.classList.remove('hidden');
}

/* What the head of collin's section says about the run that wrote it. */
function collinReportAbout(view) {
  const written = view.generated_at ? cacheAge(Math.floor(Date.parse(view.generated_at) / 1000)) : '';
  return [view.producer, view.project, view.adapter, view.dbt_version && `dbt ${view.dbt_version}`,
    written && `written ${written}`].filter(Boolean).join(' · ');
}

/* collin's report, the body of its section in the reports: something to work
   through model by model, each opening in the editor as the panel rolls up. */
function collinReportBody(view) {
  const el = (tag, className, textContent) => Object.assign(document.createElement(tag), className ? { className } : {},
    textContent !== undefined ? { textContent } : {});
  const body = el('div', 'reportbody');
  const tiles = el('div', 'reporttiles');
  for (const t of collinReportTiles(view.totals)) {
    const tile = el('div', 'reporttile');
    tile.append(el('div', 'rlabel', t.label), el('div', 'rvalue', t.value), el('div', 'rsub', t.sub));
    tiles.append(tile);
  }
  body.append(tiles);
  const n = view.to_investigate || 0;
  body.append(el('p', 'muted', n
    ? `${n} model${n > 1 ? 's' : ''} or table${n > 1 ? 's' : ''} to look at. A name opens the model, whose Columns tab says what collin noted on each column.`
    : 'Nothing to investigate: every model collin read came out clean.'));

  if ((view.collin_findings || []).length) {
    body.append(collinWrittenSections(view.collin_findings, el));
    return body;
  }

  for (const g of collinReportGroups(view.findings)) {
    const section = el('section', 'reportgroup');
    section.dataset.group = g.group;
    section.append(el('h3', '', g.label), el('p', 'muted', g.desc));
    for (const f of g.findings) {
      const d = el('details', 'reportfinding');
      // A short list is worth seeing at once; a long one is a count first.
      d.open = f.entries.length <= 5;
      const sum = el('summary');
      sum.append(el('span', 'rtitle', f.title), el('span', 'rcount', String(f.entries.length)));
      d.append(sum, el('p', 'rlever', f.lever));
      const list = el('ul', 'rentries');
      for (const e of f.entries) {
        const li = el('li');
        const node = f.kind !== 'relation_collision';
        const name = el(node ? 'a' : 'span', node ? 'filelink' : 'rrel', node ? shortId(e.id) : e.id);
        if (node) {
          name.title = e.id;
          name.addEventListener('click', (ev) => { ev.preventDefault(); openReportNode(e.id); });
        }
        li.append(name);
        if (f.kind === 'relation_collision') {
          li.append(el('span', 'muted', ' claimed by '));
          (e.items || []).forEach((id, i) => {
            if (i) li.append(el('span', 'muted', ', '));
            const a = el('a', 'filelink', shortId(id));
            a.title = id;
            a.addEventListener('click', (ev) => { ev.preventDefault(); openReportNode(id); });
            li.append(a);
          });
        } else if (collinItemsAreColumns(f.kind) && (e.items || []).length) {
          li.append(el('span', 'muted', ': '));
          e.items.forEach((c, i) => {
            if (i) li.append(el('span', 'muted', ', '));
            const a = el('a', 'rcol', c);
            a.title = `Open ${shortId(e.id)} with ${c} picked out in its Columns tab`;
            a.addEventListener('click', (ev) => { ev.preventDefault(); openReportNode(e.id, c); });
            li.append(a);
          });
          const tail = collinEntryTail(f.kind, e);
          if (tail) li.append(el('span', 'muted', tail));
        } else {
          const text = collinEntryText(f.kind, e);
          if (text) li.append(el('span', 'muted', ': ' + text));
        }
        list.append(li);
      }
      d.append(list);
      section.append(d);
    }
    body.append(section);
  }
  return body;
}

/* collin's own findings, as it worded them: what it saw, what that usually
   means and what to do, then each model with what its entry says. */
function collinWrittenSections(written, el) {
  const out = document.createDocumentFragment();
  for (const g of collinWrittenGroups(written)) {
    const section = el('section', 'reportgroup');
    section.dataset.group = g.group;
    section.append(el('h3', '', g.label), el('p', 'muted', g.desc));
    for (const f of g.findings) {
      const d = el('details', 'reportfinding');
      d.dataset.severity = f.severity || '';
      d.open = f.models.length <= 5;
      const sum = el('summary');
      sum.append(el('span', 'rtitle', f.title), el('span', 'rcount', f.severity || ''));
      d.append(sum);
      if (f.why) d.append(el('p', 'muted', f.why));
      if (f.action) d.append(el('p', 'rlever', f.action));
      const list = el('ul', 'rentries');
      for (const m of f.models) {
        const li = el('li');
        const a = el('a', 'filelink', m.name || shortId(m.unique_id));
        a.title = m.unique_id;
        a.addEventListener('click', (ev) => { ev.preventDefault(); openReportNode(m.unique_id); });
        li.append(a);
        if (m.detail) li.append(el('span', 'muted', ': ' + m.detail));
        list.append(li);
      }
      d.append(list);
      section.append(d);
    }
    out.append(section);
  }
  return out;
}

/* The collins there are to pick under Collin in the menu, once there is more
   than one, or a version was picked: the one beside dbt-edith or on the PATH,
   then those installed side by side (0060). The one that runs says so. */
function collinVersionRows(cl, tool) {
  if (!cl || tool !== 'collin') return [];
  const versions = cl.versions || [];
  if (versions.length < 2 && !cl.chosen) return [];
  return versions.map((v) => {
    const name = v.version ? `collin ${v.version}` : 'collin, no version given';
    const where = v.origin === 'installed' ? 'installed here' : 'beside dbt-edith or on the PATH';
    return { label: name, sub: v.runs ? 'runs' : where, runs: !!v.runs, version: v.origin === 'installed' ? v.version : '',
      title: v.runs ? `${name}, ${where}, is the one that runs for this project.`
        : `Run ${name}, ${where}, for this project. Collin's cache is written again by it.` };
  });
}

/* What the menu says when the version picked is no longer installed. */
function collinChosenGone(cl) {
  if (!cl || !cl.chosen) return null;
  const there = (cl.versions || []).some((v) => v.origin === 'installed' && v.version === cl.chosen);
  return there ? null : `collin ${cl.chosen}, picked for this project, is not installed here any more: the default one runs.`;
}

/* The line that installs another version beside the others, when there is a
   config directory to put it in. */
function collinInstallRow(cl, tool) {
  if (!cl || tool !== 'collin' || !cl.can_install) return null;
  return { label: 'Install another collin version…', sub: 'by tag',
    title: 'Type the cargo command that installs a collin version of your choice into its own folder, '
      + 'beside the one you have. Each can then be picked here.' };
}

/* A version as someone types it, `v0.4.0` or `0.4.0 `, to `0.4.0`, or '' when
   it is not three numbers. The server checks again. */
function collinVersionInput(text) {
  const v = String(text || '').trim().replace(/^v/i, '');
  return /^\d{1,5}\.\d{1,5}\.\d{1,5}$/.test(v) ? v : '';
}

/* Picks the collin that runs for this project. With Collin's cache on
   screen, the new one runs at once, and the page polls until it is done. */
async function chooseCollinVersion(version) {
  try {
    applyInfo(await api.send('/api/collin/version', 'POST', { version }));
    toast(version ? `collin ${version} runs for this project` : 'the default collin runs for this project');
  } catch (e) {
    toast('collin: ' + e.message, 'err');
  }
}

/* Asks which version to install, then types the command into the terminal,
   where the user runs it: dbt-edith never downloads anything (0042). */
async function askCollinVersion() {
  const body = document.createElement('div');
  body.append(Object.assign(document.createElement('p'), {
    textContent: 'Which collin version? It is installed from its tag into a folder of its own, beside the collin you have, '
      + 'and can then be picked under Collin. Building it takes a minute or two. Cargo then suggests adding that folder '
      + 'to your PATH: there is no need, dbt-edith finds it there.',
  }));
  const input = Object.assign(document.createElement('input'), { type: 'text', className: 'versioninput', placeholder: '0.4.0',
    spellcheck: false, autocomplete: 'off' });
  input.addEventListener('keydown', (e) => { if (e.key === 'Enter') $('#modal-actions').lastChild.click(); });
  body.append(input);
  const asked = modal({ title: 'Install a collin version', body,
    actions: [{ id: 'no', label: 'Cancel' }, { id: 'ok', label: 'Type the command', style: 'primary' }] });
  input.focus();
  if (await asked !== 'ok') return;
  const version = collinVersionInput(input.value);
  if (!version) return toast('a collin version is three numbers, such as 0.4.0', 'err');
  try {
    const { command } = await api.get('/api/collin/install?version=' + encodeURIComponent(version));
    sendToTerminal(command);
  } catch (e) {
    toast('collin: ' + e.message, 'err');
  }
}

/* The line under Collin that compares two versions, once two have a kept run
   to compare (0061). */
function collinCompareRow(cl, tool) {
  const kept = (cl && cl.kept) || [];
  if (tool !== 'collin' || kept.length < 2) return null;
  return { label: 'Compare collin versions', sub: `${kept.length} kept`,
    title: 'What two collin versions read in this project, from the last run of each: figures, findings, '
      + 'edges gained and lost, and the models where they moved.' };
}

/* The pair a comparison starts on: the newest kept version against the one
   before it, B against A. */
function collinComparePair(kept) {
  const k = kept || [];
  return k.length < 2 ? null : { a: k[1], b: k[0] };
}

/* A counter of collin's report in words, for the ones a reader looks for
   first; any other keeps its name. */
function collinTotalLabel(key) {
  return {
    models: 'Models', parsed: 'SQL parsed', parse_failed: 'SQL that did not parse',
    sql_from_files: 'SQL from target/compiled/', sql_from_files_set_aside: 'Compiled files set aside',
    confirmed: 'Confirmed by the warehouse', degraded: 'Behind their code', unchecked: 'Not in the catalog',
    edges_parsed: 'Edges read from the SQL', edges_inferred: 'Edges matched by name', edges_indirect: 'Indirect edges',
    columns_covered: 'Columns with an edge', columns_total: 'Columns', columns_covered_independent:
      'Columns with an edge, against the warehouse or YAML', columns_total_independent: 'Columns in the warehouse or YAML',
    columns_root: 'Columns built from constants', columns_lost: 'Columns lost', yaml_stale: 'YAML behind the warehouse',
    undeclared: 'Models reading a table dbt was not told of', scope_merged: 'Models with two scopes of one name',
    catalog_elsewhere: 'Catalog entries for another table', thin: 'Compiles far thinner than their YAML',
  }[key] || key.replace(/_/g, ' ');
}

/* Figures on both sides as rows, the ones that moved first, each keeping its
   order otherwise. A figure one side lacks is shown as a dash, not a zero:
   that version's report does not count it. */
function collinDeltaRows(deltas, label) {
  const rows = (deltas || []).map((d, i) => {
    const a = typeof d.a === 'number' ? d.a : null;
    const b = typeof d.b === 'number' ? d.b : null;
    const diff = a !== null && b !== null ? b - a : null;
    // A counter one report lacks and the other has at zero has not moved.
    return { key: d.key, label: label ? label(d.key) : d.key, a, b, diff, moved: (a || 0) !== (b || 0), i };
  });
  rows.sort((x, y) => (y.moved - x.moved) || (x.i - y.i));
  return rows;
}

function signed(n) {
  return n === null ? '' : n > 0 ? `+${n}` : String(n);
}

/* One edge as a line: `orders.id → customers.id`, a relation no node owns
   kept whole. */
function collinEdgeText(e) {
  const end = (id, col) => `${id.startsWith('rel:') ? id : shortId(id)}.${col}`;
  return `${end(e.from, e.from_col)} → ${end(e.to, e.to_col)}`;
}

/* What one model's line says moved. */
function collinMoveText(m) {
  const bits = [];
  if (m.a !== m.b && m.a && m.b) bits.push(`${m.a} → ${m.b}`);
  if (m.gained) bits.push(`${m.gained} gained`);
  if (m.lost) bits.push(`${m.lost} lost`);
  if (m.changed) bits.push(`${m.changed} with another role`);
  return bits.join(' · ');
}

/* A kept run older than the manifest read another project than the one on
   screen, so a difference may be the project's and not collin's. */
function collinSideNote(side, manifestAt) {
  if (!side) return '';
  const age = cacheAge(side.mtime);
  const behind = manifestAt && side.mtime && side.mtime < manifestAt ? ', before the manifest was last written' : '';
  return `collin ${side.version}: ${side.edges} edges, run ${age}${behind}${side.report ? '' : ', no report kept'}`;
}

async function openCollinCompare(pair) {
  const key = 'collin:compare';
  const p = pair || (S.open.get(key) && S.open.get(key).pair) || collinComparePair(S.collin && S.collin.kept);
  if (!p) return toast('collin: two versions need a kept run each to compare', 'err');
  let view;
  try { view = await api.get(`/api/collin/compare?a=${encodeURIComponent(p.a)}&b=${encodeURIComponent(p.b)}`); }
  catch (e) { return toast('collin compare: ' + e.message, 'err'); }
  const f = S.open.get(key);
  if (f) {
    Object.assign(f, { view, pair: p });
    if (f.host) f.host.remove();
    f.host = null;
  } else {
    S.open.set(key, { kind: 'compare', path: 'collin versions', view, pair: p, dirty: false, host: null });
    S.order.push(key);
  }
  activate(key, false);
}

function collinComparePane(key, view, pair) {
  const el = (tag, className, textContent) => Object.assign(document.createElement(tag), className ? { className } : {},
    textContent !== undefined ? { textContent } : {});
  const pane = el('div', 'diffpane reportpane');
  const head = el('div', 'diff-head');
  const title = el('div', 'diff-title');
  const dot = el('span', 'tooldot');
  dot.dataset.tool = 'collin';
  title.append(dot, el('b', '', 'collin versions'));
  const kept = (S.collin && S.collin.kept) || [pair.a, pair.b];
  const pick = (side) => {
    const s = el('select', 'versionpick');
    for (const v of kept) s.append(Object.assign(el('option', '', v), { value: v, selected: v === pair[side] }));
    s.addEventListener('change', () => openCollinCompare(Object.assign({}, pair, { [side]: s.value })));
    return s;
  };
  title.append(el('span', 'diff-state', 'A'), pick('a'), el('span', 'diff-state', 'against B'), pick('b'),
    el('div', 'grow'));
  const close = el('button', 'icon', '×');
  close.title = 'Close the comparison';
  close.addEventListener('click', () => closeFile(key));
  title.append(close);
  head.append(title);

  const body = el('div', 'reportbody');
  const manifestAt = S.meta && S.meta.manifest_mtime;
  body.append(el('p', 'muted', `A, ${collinSideNote(view.a, manifestAt)}.`), el('p', 'muted', `B, ${collinSideNote(view.b, manifestAt)}.`));
  if (view.a.version === view.b.version) body.append(el('p', 'muted', 'A and B are one run: pick two versions.'));

  const table = (rows, unit) => {
    const t = el('table', 'cmptable');
    const hr = el('tr');
    hr.append(el('th', '', unit), el('th', 'num', `A ${view.a.version}`), el('th', 'num', `B ${view.b.version}`), el('th', 'num', 'B − A'));
    t.append(hr);
    for (const r of rows) {
      const tr = el('tr', r.moved ? 'moved' : '');
      tr.append(el('td', '', r.label), el('td', 'num', r.a === null ? '-' : String(r.a)),
        el('td', 'num', r.b === null ? '-' : String(r.b)), el('td', 'num', signed(r.diff)));
      t.append(tr);
    }
    return t;
  };
  const section = (heading, desc) => {
    const s = el('section', 'reportgroup');
    s.append(el('h3', '', heading));
    if (desc) s.append(el('p', 'muted', desc));
    body.append(s);
    return s;
  };

  const edges = section('Edges', `B has ${view.gained} edge${view.gained === 1 ? '' : 's'} A lacks, A has ${view.lost} B lacks, `
    + `and ${view.changed} ${view.changed === 1 ? 'is' : 'are'} read with another role.`);
  edges.append(table(collinDeltaRows(view.kinds), 'By role'), table(collinDeltaRows(view.origins,
    (k) => ({ rel: 'a table no node owns' }[k] || k)), 'From'));

  if (view.models.length) {
    const s = section('Models that moved', `${view.models_moved} model${view.models_moved === 1 ? '' : 's'}, the most first`
      + (view.models_moved > view.models.length ? `, ${view.models.length} listed` : '') + '. A name opens the model.');
    const list = el('ul', 'rentries');
    for (const m of view.models) {
      const li = el('li');
      const a = el('a', 'filelink', shortId(m.id));
      a.title = m.id;
      a.addEventListener('click', (ev) => { ev.preventDefault(); openReportNode(m.id); });
      li.append(a, el('span', 'muted', ': ' + collinMoveText(m)));
      list.append(li);
    }
    s.append(list);
  }

  const edgeList = (heading, count, items, line) => {
    if (!count) return;
    const d = el('details', 'reportfinding');
    const sum = el('summary');
    sum.append(el('span', 'rtitle', heading), el('span', 'rcount', String(count)));
    d.append(sum);
    const list = el('ul', 'rentries');
    for (const e of items) list.append(el('li', 'rrel', line(e)));
    if (count > items.length) list.append(el('li', 'muted', `and ${count - items.length} more`));
    d.append(list);
    edges.append(d);
  };
  edgeList('Only in B', view.gained, view.gained_edges, (e) => `${collinEdgeText(e)}  ${e.kind}`);
  edgeList('Only in A', view.lost, view.lost_edges, (e) => `${collinEdgeText(e)}  ${e.kind}`);
  edgeList('Another role', view.changed, view.changed_edges, (e) => `${collinEdgeText(e)}  ${e.kind} → ${e.b_kind}`);

  if (view.findings.length) {
    section('Findings', 'Entries of each kind on the report page.')
      .append(table(collinDeltaRows(view.findings, (k) => collinFindingText(k).title), 'Kind'));
  }
  if (view.totals.length) {
    section('Figures', "Every counter in collin's report, those that moved first.")
      .append(table(collinDeltaRows(view.totals, collinTotalLabel), 'Counter'));
  }
  pane.append(head, body);
  return pane;
}

/* What a failed fetch should say. Only a connection Snowflake refused points at
   the profile: a query it rejected is about the object or the role, and a
   request this build got wrong is neither. */
function connectionAdvice(error, phase, profiles) {
  if (phase !== 'connect') return { text: `Snowflake: ${error}` };
  const text = `Snowflake refused the connection: ${error}`;
  return profiles ? { text, ask: 'check the user and account in', file: profiles } : { text };
}

/* The profile as a link, where dbt looks for it. Nothing to open when there is
   no file there. */
function profileLink() {
  const p = S.profiles;
  if (!p || !p.found) return null;
  const a = document.createElement('a');
  a.className = 'filelink';
  a.textContent = fileName(p.path);
  a.title = `${p.path}\nThe dbt profile, outside the project. Click to open it here.`;
  a.addEventListener('click', (e) => { e.preventDefault(); e.stopPropagation(); openProfiles(); });
  return a;
}

/* The profile is a fact about the project, whatever its warehouse, so it has
   a button in the top bar (0049). A missing one keeps it, marked: it is why dbt
   would fail, and where it should be is the question the user would ask. */
function paintProfileButton() {
  const b = $('#profile-btn');
  const p = S.profiles;
  b.classList.toggle('hidden', !p);
  if (!p) return;
  b.dataset.found = String(!!p.found);
  b.title = p.found
    ? `profiles.yml\n${p.path}\nThe dbt profile: click to open it here.`
    : `profiles.yml: none\nNo dbt profile at ${p.path}, where dbt looks for one.`;
}

/* An icon drawn here rather than shipped, one stroked path per shape, its
   colour the button's own. */
function strokeIcon(viewBox, className, shapes) {
  const ns = 'http://www.w3.org/2000/svg';
  const svg = document.createElementNS(ns, 'svg');
  svg.setAttribute('viewBox', viewBox);
  svg.setAttribute('class', className);
  svg.setAttribute('aria-hidden', 'true');
  for (const d of shapes) {
    const path = document.createElementNS(ns, 'path');
    path.setAttribute('d', d);
    svg.appendChild(path);
  }
  return svg;
}

/* A user and a plug: the profile is who dbt connects as, and how. */
function profileIcon() {
  return strokeIcon('0 0 20 16', 'profileicon', [
    'M8.5 5a2.7 2.7 0 1 1-5.4 0a2.7 2.7 0 1 1 5.4 0zM1.3 14.5v-.8a4.5 4.5 0 0 1 9 0v.8',
    'M14 4.5v2.2M17 4.5v2.2M12.6 6.7h5.8V8a2.9 2.9 0 0 1-5.8 0zM15.5 10.9v3.6',
  ]);
}

/* The lineage fields of an /api/meta answer, which every route that switches
   the lineage or a feature answers with. */
function applyLineage(info) {
  const was = S.sidecar && S.sidecar.state;
  S.cllSources = info.cll_sources || [];
  S.cllActive = info.cll_active || '';
  S.cllTool = info.cll_tool || '';
  S.features = info.features || null;
  S.profiles = info.profiles || null;
  if (info.sidecar) S.sidecar = info.sidecar;
  const collinWas = S.collin && S.collin.state;
  if (info.collin) S.collin = info.collin;
  paintSidecar(was);
  paintCollin(collinWas);
  paintSettingsButton();
  paintHistoryTab();
}

/* The button keeps Snowflake's colour whether the features are on or off:
   it is where they are found, and a grey one was easy to miss. Off, it is only
   muted, and its tooltip says which. */
function paintSettingsButton() {
  const on = !!(S.features && S.features.snowflake);
  const b = $('#settings-btn');
  b.dataset.on = String(on);
  b.title = `Settings for this project. Snowflake features: ${on ? 'on' : 'off'} (alpha); where its jobs are read from`;
}

/* After the graph's column lineage changed: the Columns tab's counts, and the
   canvas, which cannot stay in column mode on edges that are gone. */
async function redrawLineage() {
  if (S.node) {
    try { renderCatalog(await nodeDetail({ id: S.node.id })); } catch { /* gone with the last manifest */ }
  }
  if (S.graphMode === 'column' && !(S.meta && S.meta.cll_edges)) {
    if (S.focus) focusNode(S.focus);
  } else {
    rerender();
  }
}

/* One floating menu at a time, closed by a click outside it, Escape, a resize,
   or a second click on the button that opened it. `closed` runs once it is
   gone, however it went. */
let popup = null;

/* Below the anchor when the menu fits there, above it otherwise, and inside
   the window either way. Again when a menu drawn anew has grown. */
function placePopup(anchor, el) {
  const at = anchor.getBoundingClientRect();
  const box = el.getBoundingClientRect();
  const below = at.bottom + 4 + box.height <= window.innerHeight;
  el.style.top = `${below ? at.bottom + 4 : Math.max(4, at.top - 4 - box.height)}px`;
  el.style.left = `${Math.min(Math.max(4, at.left), window.innerWidth - box.width - 4)}px`;
}

function openPopup(anchor, el, closed) {
  closePopup();
  document.body.appendChild(el);
  placePopup(anchor, el);
  const live = () => [...el.querySelectorAll('button:not([aria-disabled="true"]), a[href]')];
  const onDown = (e) => { if (!el.contains(e.target) && !anchor.contains(e.target)) closePopup(); };
  const onKey = (e) => {
    if (e.key === 'Tab') return closePopup();
    const step = { ArrowDown: 1, ArrowUp: -1 }[e.key];
    if (e.key !== 'Escape' && !step) return;
    // Handled here only: the editor and the global shortcuts never see it.
    e.preventDefault();
    e.stopPropagation();
    if (e.key === 'Escape') {
      closePopup();
      anchor.focus();
      return;
    }
    const buttons = live();
    const i = buttons.indexOf(document.activeElement);
    if (buttons.length) buttons[(i + step + buttons.length) % buttons.length].focus();
  };
  const onResize = () => closePopup();
  document.addEventListener('mousedown', onDown, true);
  document.addEventListener('keydown', onKey, true);
  window.addEventListener('resize', onResize);
  popup = {
    el, anchor, closed,
    off: () => {
      document.removeEventListener('mousedown', onDown, true);
      document.removeEventListener('keydown', onKey, true);
      window.removeEventListener('resize', onResize);
    },
  };
  const first = live();
  (first.find((b) => b.classList.contains('on')) || first[0] || el).focus();
}

/* Closes the open menu. True when `anchor` opened it, so the click on that
   button closes it rather than opening it again. */
function closePopup(anchor) {
  if (!popup) return false;
  const same = !!anchor && popup.anchor === anchor;
  const { closed } = popup;
  popup.off();
  popup.el.remove();
  popup = null;
  if (closed) closed();
  return same;
}

/* Settings for this project. One family of features so far, Snowflake's, with
   more to come: each is a row, kept per project (0011, 0031). */
function openSettingsMenu(anchor) {
  if (closePopup(anchor)) return;
  const menu = document.createElement('div');
  menu.className = 'envmenu setmenu';
  menu.setAttribute('role', 'menu');
  paintSettingsMenu(menu);
  openPopup(anchor, menu);
}

/* A snowflake drawn here, six arms and nothing more. Not Snowflake's logo:
   their marks are for uses they approve in writing, and this repository is
   public (0014). The name in text, and the colour, say which product it is. */
/* The tests toggle above the lineage, an eye as the Snowflake button is a
   snowflake: open while the canvas draws data tests, struck through while it
   does not. Its state lives on the button, read by testsOn(). */
function testsEye(on) {
  const ns = 'http://www.w3.org/2000/svg';
  const svg = document.createElementNS(ns, 'svg');
  svg.setAttribute('viewBox', '0 0 16 16');
  svg.setAttribute('aria-hidden', 'true');
  const draw = (tag, attrs) => {
    const e = document.createElementNS(ns, tag);
    for (const [k, v] of Object.entries(attrs)) e.setAttribute(k, v);
    svg.appendChild(e);
  };
  draw('path', { d: 'M1.3 8C2.8 5.2 5.2 3.6 8 3.6S13.2 5.2 14.7 8C13.2 10.8 10.8 12.4 8 12.4S2.8 10.8 1.3 8Z' });
  draw('circle', { cx: 8, cy: 8, r: 2.1 });
  if (!on) draw('path', { d: 'M2.6 13.4 13.4 2.6' });
  return svg;
}

/* How many tests the canvas was sent, beside the word on the eye, while it is
   open: the number that decides how heavy the picture is. */
function paintTestsCount(sub) {
  const word = $('#with-tests span');
  if (word) word.textContent = testsLabel(testsOn() && sub ? sub.nodes.filter((n) => n && n.kind === 'test').length : 0);
}

function testsOn() {
  return $('#with-tests').getAttribute('aria-pressed') === 'true';
}

function foldersOn() {
  return $('#by-folder').getAttribute('aria-pressed') === 'true';
}

function paintTestsToggle(on) {
  const b = $('#with-tests');
  b.setAttribute('aria-pressed', String(on));
  b.title = testsTitle(on);
  // The eye alone reads as nothing in particular: the word says what it shows.
  b.replaceChildren(testsEye(on), Object.assign(document.createElement('span'), { textContent: 'tests' }));
}

function snowflakeIcon() {
  const ns = 'http://www.w3.org/2000/svg';
  const svg = document.createElementNS(ns, 'svg');
  svg.setAttribute('viewBox', '0 0 16 16');
  svg.setAttribute('class', 'sficon');
  svg.setAttribute('aria-hidden', 'true');
  for (const turn of [0, 60, 120]) {
    const arm = document.createElementNS(ns, 'path');
    arm.setAttribute('d', 'M8 1.4v13.2M6.2 3.1 8 4.7l1.8-1.6M6.2 12.9 8 11.3l1.8 1.6');
    arm.setAttribute('transform', `rotate(${turn} 8 8)`);
    svg.appendChild(arm);
  }
  return svg;
}

/* One row per family of features: its name, a switch, what it covers and why
   it is on or off. Snowflake's is marked alpha, since none of it has been
   seen against a real warehouse yet (docs/state.md, Waiting on a human). */
function paintSettingsMenu(menu) {
  const f = S.features || { snowflake: false, snowflake_set: false };
  const el = (tag, className, textContent = '') => Object.assign(document.createElement(tag), { className, textContent });
  menu.textContent = '';
  menu.append(el('div', 'menuhead', 'Settings for this project'));

  const row = el('div', 'setrow');
  row.dataset.tool = 'snowflake';
  const toggle = el('button', 'switch');
  toggle.type = 'button';
  toggle.setAttribute('role', 'switch');
  toggle.setAttribute('aria-checked', String(!!f.snowflake));
  toggle.setAttribute('aria-label', 'Snowflake features');
  toggle.title = f.snowflake ? 'Switch Snowflake\'s features off for this project' : 'Switch Snowflake\'s features on for this project';
  toggle.append(el('span', 'knob'));
  row.append(snowflakeIcon(), el('span', 'setlbl', 'Snowflake features'), el('span', 'alpha', 'alpha'), toggle);
  // The whole row is the target, the name included, and the switch is what
  // the keyboard reaches. A second click while the first is on its way is
  // dropped, or the two would race to the server.
  let busy = false;
  row.addEventListener('click', async () => {
    if (busy) return;
    busy = true;
    row.setAttribute('aria-busy', 'true');
    await setFeatures({ snowflake: !f.snowflake });
    paintSettingsMenu(menu);
    const again = menu.querySelector('.switch');
    if (again) again.focus();
  });

  const note = (text) => el('div', 'menunote', text);
  const warn = el('div', 'alphawarn');
  warn.append(el('b', '', 'Alpha.'), document.createTextNode(
    ' Snowflake\'s features are still being built, and have not yet been checked against a real warehouse. '
    + 'Expect rough edges, and check what they show against Snowflake itself.'));
  menu.append(row,
    note('Column lineage from Snowflake, fetched on click; the Query history tab, read from Snowflake '
      + 'when it opens; a Snowsight menu beside each relation in the Catalog; and Clone into my schema, '
      + 'under a model\'s location, which writes the statements that copy it and its direct parents '
      + 'from an environment into your schema, for you to run.'),
    note(featureNote(f, S.meta && S.meta.adapter)),
    warn);

  // The jobs' folders are a list, edited in a form of their own (0063).
  const jobs = el('div', 'setrow');
  jobs.dataset.tool = 'jobs';
  jobs.append(jobsIcon(), el('span', 'setlbl', 'Orchestration'), el('span', 'setmore', 'Sources…'));
  jobs.addEventListener('click', () => { closePopup(); openJobSources(); });
  menu.append(document.createElement('hr'), jobs,
    note('Which Airflow DAGs or dbt Cloud jobs run each model, in the Jobs tab, read from their YAML '
      + 'wherever it lives, often in another repository.'));
}

/* A clock, for the jobs that run on one. */
function jobsIcon() {
  const ns = 'http://www.w3.org/2000/svg';
  const svg = document.createElementNS(ns, 'svg');
  svg.setAttribute('viewBox', '0 0 16 16');
  svg.setAttribute('class', 'sficon');
  svg.setAttribute('aria-hidden', 'true');
  for (const [tag, attrs] of [['circle', { cx: 8, cy: 8, r: 6.2 }], ['path', { d: 'M8 4.2V8l2.6 1.6' }]]) {
    const e = document.createElementNS(ns, tag);
    for (const [k, v] of Object.entries(attrs)) e.setAttribute(k, v);
    svg.appendChild(e);
  }
  return svg;
}

async function setFeatures(body) {
  try {
    applyInfo(await api.send('/api/features', 'POST', body));
    await redrawLineage();
  } catch (e) {
    toast('settings: ' + e.message, 'err');
  }
}

/* The profile lives outside the project, so it has its own route rather than a
   hole in the one that is confined to the project (0017). */
async function openProfiles() {
  const path = (S.profiles && S.profiles.path) || '';
  if (!path) return toast('no home directory to look for profiles.yml in: set DBT_PROFILES_DIR', 'err');
  const key = 'profile:' + path;
  if (!S.open.has(key)) {
    let body;
    try { body = await api.get('/api/profiles'); }
    catch (e) { return toast('profile: ' + e.message, 'err'); }
    S.open.set(key, { kind: 'profile', path: body.path, doc: CodeMirror.Doc(body.content, 'text/x-yaml'), dirty: false, hidden: body.hidden || 0 });
    S.order.push(key);
    // The secrets never reach the page (0054): saying so here is what keeps
    // anyone from thinking the password was wiped.
    if (body.hidden) toast(hiddenNote(body.hidden), 'ok');
  }
  activate(key, false);
}

function hiddenNote(n) {
  return `${n} secret value${n === 1 ? '' : 's'} hidden: leave the placeholder to keep one, or type over it to change it`;
}

/* The script reads the profile once, when it starts, so a correction that does
   not restart it changes nothing. The server restarts it and says so. */
async function saveProfile(key, f) {
  try {
    const saved = await api.send('/api/profiles', 'PUT', { content: f.doc.getValue() });
    f.dirty = false;
    // The next connection reads the new profile, maybe for another account.
    S.sfSession = null;
    await loadSidecar();
    if (S.node) renderCatalog(S.node);
    // The caller already says it saved; this is the part it cannot know.
    if (saved.restarted) toast('Snowflake script restarted on the new profile', 'ok');
    if (S.sidecar && S.sidecar.state === 'failed') toast('Snowflake lineage: ' + S.sidecar.error, 'err');
    return true;
  } catch (e) {
    toast(`save failed for ${fileName(key)}: ${e.message}`, 'err');
    return false;
  }
}

// -------------------------------------------------------------- Snowsight --
/* Snowsight is a link, never a call: built from the organization and account
   names the session reports, and opened by the browser in a new tab with no
   referrer (0048, 0051). */

/* The account's home in Snowsight, or '' while the organization is unknown: a
   link built from the account locator alone would open the wrong page, or none. */
function snowsightHome(session) {
  if (!session || !session.org || !session.account) return '';
  const seg = (s) => encodeURIComponent(String(s).toLowerCase());
  return `https://app.snowflake.com/${seg(session.org)}/${seg(session.account)}/#`;
}

/* A query's page in Snowsight. */
function snowsightUrl(session, queryId) {
  const home = snowsightHome(session);
  return home && queryId ? `${home}/compute/history/queries/${encodeURIComponent(queryId)}/detail` : '';
}

/* Where Snowsight files an object of one materialization, as its address
   spells it. The manifest does not say what a source is, so it is asked for as
   a table, which most are. */
function snowsightKind(materialized) {
  switch (materialized) {
    case 'view': return { path: 'view', label: 'view', guess: '' };
    case 'materialized_view': return { path: 'view', label: 'materialized view', guess: '' };
    case 'dynamic_table': return { path: 'dynamic-table', label: 'dynamic table', guess: '' };
    case 'source': return { path: 'table', label: 'table', guess: 'a source can be a view too: the manifest does not say' };
    default: return { path: 'table', label: 'table', guess: '' };
  }
}

/* One relation's pages in Snowsight, the object, its schema and its database,
   each named as Snowflake keeps it: unquoted in upper case, quoted as written.
   None while the account is unknown, or for a name that is not three parts. */
function snowsightPlaces(session, parts, materialized) {
  const home = snowsightHome(session);
  if (!home || !parts || parts.length !== 3 || parts.some((p) => !p.name)) return [];
  const [database, schema, object] = parts.map((p) => (p.quoted ? p.name : p.name.toUpperCase()));
  const kind = snowsightKind(materialized);
  const inDatabase = `${home}/data/databases/${encodeURIComponent(database)}`;
  const inSchema = `${inDatabase}/schemas/${encodeURIComponent(schema)}`;
  return [
    { what: kind.label, name: object, url: `${inSchema}/${kind.path}/${encodeURIComponent(object)}`, guess: kind.guess },
    { what: 'schema', name: schema, url: inSchema, guess: '' },
    { what: 'database', name: database, url: inDatabase, guess: '' },
  ];
}

// --------------------------------------------------------- query history --
/* The user's own queries, read live from Snowflake by the script, twenty at a
   time (0048). Nothing of it is kept: closing the page forgets it, and the
   detail of a query is Snowsight's, one link away. */

/* What the role menu offers. The connected target's role comes first, then the
   roles of the targets that sign in as it does, each once with every target
   that names it, then every role. A target that signs in as someone else, or
   whose role the profile does not spell out, is listed greyed with the reason:
   its queries are not in this user's history, or not under a name we know. */
function historyRoles(targets, target, sessionRole) {
  const list = targets || [];
  const own = list.find((t) => t.name === target);
  const ownRole = (own && own.role) || sessionRole || '';
  const first = { role: null, label: ownRole || 'the target\'s role', targets: target ? [target] : [] };
  const offered = [first];
  const seen = new Map([[ownRole.toUpperCase(), first]]);
  const greyed = [];
  for (const t of list) {
    if (t.name === target) continue;
    if (!t.same_login || !t.role) {
      greyed.push({ label: t.role || 'unknown role', targets: [t.name], why: t.why || 'its queries are not in this user\'s history' });
      continue;
    }
    const key = t.role.toUpperCase();
    if (seen.has(key)) {
      seen.get(key).targets.push(t.name);
      continue;
    }
    const o = { role: t.role, label: t.role, targets: [t.name] };
    seen.set(key, o);
    offered.push(o);
  }
  return { offered, all: { role: '*', label: 'All roles', targets: [] }, greyed };
}

/* Snowflake's execution status as one of a few states. A cancelled query is
   only told apart by its message, since its status says it failed. */
function queryState(row) {
  const s = (row && row.status) || '';
  if (s === 'success') return 'ok';
  if (s.startsWith('failed')) return /SQL execution canceled/i.test(row.error || '') ? 'cancelled' : 'failed';
  if (s === 'running' || s === 'resuming_warehouse') return 'running';
  if (s === 'queued' || s === 'blocked') return 'queued';
  return s || 'unknown';
}

function queryDuration(ms) {
  if (ms == null) return '';
  if (ms < 1000) return `${ms} ms`;
  if (ms < 60000) return `${(ms / 1000).toFixed(1)} s`;
  const mins = Math.floor(ms / 60000);
  if (mins < 60) return `${mins} min ${Math.round((ms % 60000) / 1000)} s`;
  return `${Math.floor(mins / 60)} h ${mins % 60} min`;
}

/* When a query started, on the reader's clock: the time alone today, the date
   too before, never an age. The offset is asked per moment, since a week can
   cross a change of summer time, and is an argument so tests do not depend on
   the machine running them. */
function historyTime(ms, nowMs, offsetAt) {
  const local = (t) => new Date(t + offsetAt(t) * 60000);
  const d = local(ms);
  const p = (x) => String(x).padStart(2, '0');
  const day = (x) => `${x.getUTCFullYear()}-${p(x.getUTCMonth() + 1)}-${p(x.getUTCDate())}`;
  const time = `${p(d.getUTCHours())}:${p(d.getUTCMinutes())}:${p(d.getUTCSeconds())}`;
  return day(d) === day(local(nowMs)) ? time : `${day(d)} ${time}`;
}

/* The line a statement is recognised by: dbt opens most of its own with a
   comment of metadata, so leading comments and blank lines are skipped. */
function sqlHead(text) {
  let s = String(text || '');
  for (;;) {
    const t = s.replace(/^\s+/, '');
    if (t.startsWith('/*')) {
      const end = t.indexOf('*/');
      if (end < 0) return '';
      s = t.slice(end + 2);
    } else if (t.startsWith('--')) {
      const end = t.indexOf('\n');
      if (end < 0) return '';
      s = t.slice(end + 1);
    } else {
      s = t;
      break;
    }
  }
  return s.split('\n')[0].replace(/\s+/g, ' ').trim();
}

async function loadHistory({ more = false } = {}) {
  const ask = ++S.historyAsk;
  const was = S.history || { rows: [] };
  const body = { role: S.historyRole };
  const last = was.rows[was.rows.length - 1];
  if (more && last) body.before = { at: last.at, query_id: last.query_id };
  S.history = { ...was, busy: more ? 'more' : 'page', error: null };
  renderHistory();
  try {
    const page = await api.send('/api/snowflake/history', 'POST', body);
    if (ask !== S.historyAsk) return;
    if (page.session) S.sfSession = page.session;
    S.history = {
      rows: more ? was.rows.concat(page.rows || []) : (page.rows || []),
      more: !!page.more,
      session: page.session || {},
      target: page.target || '',
      targets: page.targets || [],
      fetched: new Date(),
      busy: '',
      error: null,
    };
    S.colAnswered = true;
  } catch (e) {
    if (ask !== S.historyAsk) return;
    S.history = { ...was, busy: '', error: e };
  }
  renderHistory();
  // The script may have just started for this, which the top bar shows.
  loadSidecar();
}

/* Another role reads another history: it starts again from the newest. */
function pickHistoryRole(role) {
  S.historyRole = role;
  S.history = { ...(S.history || {}), rows: [], more: false };
  loadHistory();
}

function openHistoryRoles(anchor) {
  if (closePopup(anchor)) return;
  const h = S.history || {};
  const { offered, all, greyed } = historyRoles(h.targets, h.target, h.session && h.session.role);
  const menu = document.createElement('div');
  menu.className = 'envmenu histmenu';
  menu.setAttribute('role', 'menu');
  const add = (o, disabled) => {
    const item = document.createElement('button');
    item.type = 'button';
    item.setAttribute('role', 'menuitemradio');
    const on = !disabled && o.role === S.historyRole;
    item.setAttribute('aria-checked', String(on));
    if (on) item.classList.add('on');
    if (disabled) {
      item.setAttribute('aria-disabled', 'true');
      item.title = `Not readable from this connection: ${o.why}.`;
    }
    item.append(
      Object.assign(document.createElement('span'), { className: 'check', textContent: on ? '✓' : '' }),
      Object.assign(document.createElement('span'), { className: 'lbl', textContent: o.label }),
    );
    if (o.targets.length) {
      item.append(Object.assign(document.createElement('span'), { className: 'sub', textContent: o.targets.join(', ') }));
    }
    item.addEventListener('click', () => {
      if (disabled) return;
      closePopup();
      if (o.role !== S.historyRole) pickHistoryRole(o.role);
    });
    menu.appendChild(item);
  };
  offered.forEach((o) => add(o));
  menu.appendChild(document.createElement('hr'));
  add(all);
  if (greyed.length) {
    menu.appendChild(Object.assign(document.createElement('div'), {
      className: 'menuhead', textContent: 'not in this user\'s history',
    }));
    greyed.forEach((o) => add(o, true));
  }
  openPopup(anchor, menu);
}

/* The tab is offered only while Snowflake's features are on (0031). Switched
   off, what it held is forgotten, and a tab left open falls back to the
   lineage. */
function paintHistoryTab() {
  const on = !!(S.features && S.features.snowflake);
  $('#history-tab').classList.toggle('hidden', !on);
  if (on) return;
  S.history = null;
  S.historyRole = null;
  S.sfSession = null;
  if (S.dock === 'history') showDock('lineage');
}

function renderHistory() {
  const pane = $('#dock-history');
  const h = S.history || { rows: [] };
  const el = (tag, className, textContent = '') => Object.assign(document.createElement(tag), { className, textContent });
  const wrap = el('div', 'history');
  const tools = el('div', 'histtools');
  tools.dataset.tool = 'snowflake';

  const roles = historyRoles(h.targets, h.target, h.session && h.session.role);
  const current = S.historyRole === '*' ? roles.all.label
    : S.historyRole === null ? roles.offered[0].label : S.historyRole;
  const pick = el('button', 'btn sm', `Role: ${current} ▾`);
  pick.type = 'button';
  pick.setAttribute('aria-haspopup', 'menu');
  pick.title = 'Whose queries: the role of the target dbt-edith connects with, another target\'s, or every role';
  pick.addEventListener('click', () => openHistoryRoles(pick));
  const refresh = el('button', 'btn sm', 'Refresh');
  refresh.type = 'button';
  refresh.title = 'Ask Snowflake again for the twenty newest';
  refresh.disabled = !!h.busy;
  refresh.addEventListener('click', () => loadHistory());
  const said = el('span', 'histsaid muted');
  if (h.busy === 'page') said.textContent = 'asking Snowflake';
  else if (h.fetched) {
    const p = (x) => String(x).padStart(2, '0');
    const at = `${p(h.fetched.getHours())}:${p(h.fetched.getMinutes())}:${p(h.fetched.getSeconds())}`;
    said.textContent = [h.target && `target ${h.target}`, h.session && h.session.user, `read at ${at}`].filter(Boolean).join(' · ');
  }
  tools.append(snowflakeIcon(), pick, refresh, said);
  wrap.append(tools);

  const body = el('div', 'histbody');
  if (h.error) {
    const fail = el('div', 'histnote bad');
    fail.append(el('b', '', 'Snowflake did not answer. '), document.createTextNode(h.error.message));
    // A refused connection or a script that could not start is the profile's
    // business, so the file is one click away.
    const link = (h.error.phase === 'connect' || h.error.phase === 'start') && profileLink();
    if (link) fail.append(document.createTextNode(' The connection comes from '), link, document.createTextNode('.'));
    body.append(fail);
  }
  if (!h.rows.length) {
    if (h.busy) {
      body.append(el('p', 'muted pad', S.colAnswered
        ? 'Asking Snowflake for your queries.'
        : 'Asking Snowflake for your queries. A sign-in tab may open if the session needs one.'));
    } else if (h.fetched) {
      body.append(el('p', 'muted pad', `No query of yours under ${current === roles.all.label ? 'any role' : `role ${current}`} in the last 7 days.`));
    }
    wrap.append(body);
    pane.replaceChildren(wrap);
    return;
  }

  const table = el('table', 'cols histtable');
  const head = el('tr');
  for (const [name, title, className = ''] of [
    ['Started', 'On this machine\'s clock'], ['', 'How it ended'], ['Took', 'Total elapsed time', 'h-num'], ['Role', ''],
    ['Warehouse', ''], ['Type', ''], ['Query', 'The first line, comments left out. Click a row for more.'],
    ['Rows', 'Rows produced', 'h-num'], ['', ''],
  ]) {
    const th = el('th', className, name);
    if (title) th.title = title;
    head.append(th);
  }
  table.append(head);
  const now = Date.now();
  const offsetAt = (t) => -new Date(t).getTimezoneOffset();
  for (const row of h.rows) {
    const tr = el('tr', 'h-row');
    const state = queryState(row);
    const started = Date.parse(row.start);
    const when = el('td', 'h-when', historyTime(started, now, offsetAt));
    when.title = `${exportStamp(Math.floor(started / 1000), offsetAt(started))}\n${row.start}`;
    const dot = el('td', 'h-state');
    dot.append(Object.assign(el('span', 'qdot'), { title: state === 'ok' ? 'succeeded' : state }));
    dot.dataset.state = state;
    const query = el('td', 'h-sql');
    query.append(el('div', 'h-head', sqlHead(row.text) || '(empty)'));
    if (row.error) query.append(el('div', 'h-err', row.error));
    const link = el('td', 'h-link');
    const url = snowsightUrl(h.session, row.query_id);
    if (url) {
      const a = el('a', 'filelink', 'Snowsight ↗');
      a.href = url;
      a.target = '_blank';
      a.rel = 'noopener noreferrer';
      a.title = 'Open this query in Snowsight, in a new tab';
      a.addEventListener('click', (e) => e.stopPropagation());
      link.append(a);
    }
    tr.append(when, dot, el('td', 'h-num', queryDuration(row.ms)), el('td', 'h-mono', row.role || ''),
      el('td', 'h-mono', row.warehouse || ''), el('td', 'h-type', (row.type || '').toLowerCase().replace(/_/g, ' ')),
      query, el('td', 'h-num', row.rows == null ? '' : String(row.rows)), link);
    tr.addEventListener('click', () => toggleHistoryRow(tr, row, h.session));
    table.append(tr);
  }
  body.append(table);

  const foot = el('div', 'histfoot');
  if (h.more) {
    const more = el('button', 'btn sm', h.busy === 'more' ? 'Loading…' : 'Load 20 more');
    more.type = 'button';
    more.disabled = !!h.busy;
    more.addEventListener('click', () => loadHistory({ more: true }));
    foot.append(more);
  } else {
    foot.append(el('span', 'muted', 'Nothing older: Snowflake gives back your newest 10 000 queries of the last 7 days.'));
  }
  body.append(foot);
  wrap.append(body);
  pane.replaceChildren(wrap);
}

/* A row opens under itself to show the statement as far as it was sent, and
   what it takes to find it again: its id, copied in one click. */
function toggleHistoryRow(tr, row, session) {
  const next = tr.nextElementSibling;
  if (next && next.classList.contains('h-detail')) {
    next.remove();
    tr.classList.remove('h-open');
    return;
  }
  tr.classList.add('h-open');
  const detail = document.createElement('tr');
  detail.className = 'h-detail';
  const td = document.createElement('td');
  td.colSpan = tr.children.length;
  const pre = Object.assign(document.createElement('pre'), { className: 'h-text', textContent: row.text || '' });
  const bar = Object.assign(document.createElement('div'), { className: 'h-bar' });
  const id = Object.assign(document.createElement('code'), { textContent: row.query_id });
  const copyId = Object.assign(document.createElement('button'), { className: 'btn sm', type: 'button', textContent: 'Copy id' });
  copyId.addEventListener('click', () => copyText(row.query_id, 'the query id'));
  const copySql = Object.assign(document.createElement('button'), { className: 'btn sm', type: 'button', textContent: 'Copy SQL' });
  copySql.addEventListener('click', () => copyText(row.text || '', 'the SQL'));
  bar.append(id, copyId, copySql);
  const cut = (row.text || '').length < (row.length || 0);
  if (cut || !snowsightUrl(session, row.query_id)) {
    bar.append(Object.assign(document.createElement('span'), {
      className: 'muted',
      textContent: [
        cut && `the first ${(row.text || '').length.toLocaleString()} of ${row.length.toLocaleString()} characters`,
        snowsightUrl(session, row.query_id) ? 'the rest is in Snowsight' : 'no Snowsight link: the account\'s organization is unknown',
      ].filter(Boolean).join(': '),
    }));
  }
  td.append(bar, pre);
  detail.append(td);
  tr.after(detail);
}

/* A column clicked in the Catalog, or double-clicked on the canvas. With
   Snowflake picked, its lineage is fetched from Snowflake before it is drawn;
   otherwise the cache is drawn as it is. */
async function openColumn(n, column) {
  // Collin's cache is behind the manifest: one run brings it level first.
  if (S.cllTool === 'collin' && S.collin && S.collin.found && S.collin.stale && S.collin.state !== 'running') {
    showDock('lineage');
    $('#lineage-status').textContent = 'running collin over the compiled SQL';
    if (!(await regenerateCollin())) return;
    $('#lineage-status').textContent = '';
  }
  if (!sidecarOn()) return focusColumn(n.id, column);
  const rel = lineageRelation(n, S.env);
  if (!rel.text) return toast(`no Snowflake lineage for ${column}: ${rel.reason}`, 'err');
  const ask = ++S.colAsk;
  const status = $('#lineage-status');
  showDock('lineage');
  status.title = '';
  status.textContent = `querying Snowflake for ${column} in ${rel.text}`
    + (S.colAnswered ? '' : ' (the first query of a session may open a sign-in tab)');
  S.sidecar = Object.assign({}, S.sidecar, { state: 'busy' });
  paintSourceButton();
  try {
    const res = await api.send('/api/collineage/fetch', 'POST', {
      id: n.id, column, relation: rel.text, env: rel.file, up: +$('#up').value, down: +$('#down').value,
    });
    if (ask !== S.colAsk) return;
    S.colAnswered = true;
    if (res.added) {
      applyInfo(await api.get('/api/meta'));
      if (S.node && S.node.id === n.id) {
        highlightColumn(column, n.id);
        renderCatalog(await nodeDetail({ id: n.id }));
      }
    }
    const outside = res.unmatched_total
      ? ` · ${res.unmatched_total} object${res.unmatched_total > 1 ? 's' : ''} outside the project`
      : '';
    if (!res.up && !res.down) {
      status.textContent = `Snowflake has no column lineage for ${column} in ${res.relation}${outside}`;
      status.title = res.unmatched.join('\n');
      return;
    }
    await focusColumn(n.id, column);
    if (ask !== S.colAsk) return;
    status.textContent += ` · from Snowflake, ${res.added} new${outside}`;
    status.title = res.unmatched.length ? 'Not nodes of this project:\n' + res.unmatched.join('\n') : '';
  } catch (e) {
    if (ask !== S.colAsk) return;
    const advice = connectionAdvice(e.message, e.phase, (S.sidecar && S.sidecar.profiles) || '');
    status.textContent = advice.ask ? `${advice.text}  ·  ${advice.ask} ` : advice.text;
    const link = advice.file ? profileLink() : null;
    if (link) status.appendChild(link);
    toast('Snowflake lineage: ' + e.message, 'err');
  } finally {
    await loadSidecar();
  }
}

/* Only what is actually on screen, so the legend stays short and always matches
   what is drawn.

   Model mode explains the boxes, whose colour is the materialization. Column
   mode explains the edges instead, whose colour is what happened to the column:
   that is the question the column graph exists to answer, and repeating the
   materializations there would explain something nobody is looking at.

   Pure, and shared with an exported graph, so the file and the screen explain
   the same colours. */
function legendEntries(sub) {
  const seen = new Map();
  const kinds = sub.edge_kinds || [];
  if (kinds.length) {
    // The badges too, not just the edges: `raw` and `mixed` only ever appear on
    // a box, and a legend that skipped them would leave two colours unexplained.
    for (const k of kinds.concat(Lineage.nodeRoles(sub))) {
      if (k && !seen.has(k)) seen.set(k, Lineage.roleColor(k));
    }
  } else {
    for (const n of sub.nodes) {
      const label = Lineage.matLabel(n);
      if (!seen.has(label)) seen.set(label, Lineage.nodeColor(n));
    }
  }
  return [...seen].sort((a, b) => a[0].localeCompare(b[0]));
}

function paintLegend(sub) {
  const host = $('#lineage-legend');
  host.textContent = '';
  legendEntries(sub).forEach(([label, colour]) => {
    const chip = document.createElement('span');
    const sq = document.createElement('i');
    sq.style.background = colour;
    chip.append(sq, document.createTextNode(label));
    host.appendChild(chip);
  });
}

function paintMode() {
  noteLineage('');
  const selecting = S.graphMode === 'select';
  $$('#graph-mode .segbtn').forEach((b) => {
    b.classList.toggle('active', b.dataset.mode === S.graphMode);
    if (b.dataset.mode === 'column') {
      b.disabled = !S.colFocus;
      b.title = S.colFocus
        ? 'One column\'s lineage: the columns it is built from and the ones built from it.'
        : 'One column\'s lineage. Pick a column in Catalog > Columns first.';
    }
  });
  $('#select-bar').classList.toggle('hidden', !selecting);
  // The depth boxes have nothing to say about a selection: the + in the
  // expression carries it, and two places to set one would contradict.
  $('#up').disabled = $('#down').disabled = selecting;
  const chip = $('#col-chip');
  chip.classList.toggle('hidden', !S.colFocus);
  if (S.colFocus) {
    chip.textContent = '';
    const b = document.createElement('b');
    b.textContent = S.colFocus.column;
    const x = document.createElement('span');
    x.className = 'x';
    x.textContent = '×';
    x.title = 'back to model lineage';
    x.addEventListener('click', () => { if (S.focus) focusNode(S.focus); });
    chip.append(b, x);
  }
}

/* Returns whether it moved the lineage, which is what tells a reload that
   the file it was waiting on has turned up. `keepDepth` draws the node it
   moves to at the depth the boxes hold, rather than the default. */
async function syncNode(path, { keepDepth = false } = {}) {
  noteLineage('');
  // A properties file declares many nodes, and the server answers for one of
  // them. When the lineage already shows another, most likely because it was
  // clicked in this file, coming back to the tab keeps it.
  let now = null;
  if (S.focus) {
    try {
      now = await nodeDetail({ id: S.focus });
      if (now.file === path || now.yml === path) return false;
    } catch { /* gone with the last manifest: sync as usual */ }
  }
  try {
    const detail = await nodeDetail({ file: path });
    if (detail.id === S.focus) return false;
    focusNode(detail.id, { keepDepth });
    return true;
  } catch {
    // Not a dbt node, or not one yet: the lineage stays as it is, and says
    // so. "Select a model" with a model open read as a bug, and so did the
    // last model left on screen with nothing to tell it from this file's.
    S.unsynced = path;
    if (S.graphMode !== 'model' || projectPath(S.active) !== path) return false;
    const f = S.open.get(S.active);
    const why = noNodeHint(path, S.fresh, {
      macros: !!(f && f.doc) && definesMacros(f.doc.getValue()),
      shown: S.focus && now ? now.name : '',
    });
    if (S.focus) noteLineage(why);
    else emptyHint(why || 'Select a model to see its lineage.');
    return false;
  }
}

/* A line over a canvas that is still drawn, saying what it is not: empty
   hides it. Every change of what the canvas shows clears it (paintMode). */
function noteLineage(text) {
  const el = $('#lineage-note');
  el.textContent = text;
  el.classList.toggle('hidden', !text);
}

/* The tab in front came up with no node, most often because it is newer than
   the last parse. A reload is when that can change, from the button or from
   the watcher once dbt rewrites the manifest, so the lineage follows it then
   rather than waiting for the tab to be clicked again. */
async function followActive() {
  const path = S.active && projectPath(S.active);
  if (!path || path !== S.unsynced || S.graphMode !== 'model') return false;
  return syncNode(path);
}

function relinkTools() {
  for (const el of [$('#up'), $('#down')]) {
    el.addEventListener('change', rerender);
  }
  $('#with-tests').addEventListener('click', () => { paintTestsToggle(!testsOn()); rerender(); });
  $('#by-folder').addEventListener('click', () => { $('#by-folder').setAttribute('aria-pressed', String(!foldersOn())); rerender(); });
  $('#fit-btn').addEventListener('click', () => Lineage.fit());
  $('#export-btn').addEventListener('click', exportLineage);
  $('#image-btn').addEventListener('click', copyLineageImage);
  $$('#graph-mode .segbtn').forEach((b) => b.addEventListener('click', () => {
    if (b.disabled) return;
    if (b.dataset.mode === 'select') { runSelection(); $('#select-input').focus(); return; }
    emptyHint('Select a model to see its lineage.');
    if (b.dataset.mode === 'column' && S.colFocus) focusColumn(S.colFocus.id, S.colFocus.column);
    else if (S.focus) focusNode(S.focus);
    else { S.graphMode = b.dataset.mode; paintMode(); Lineage.clear(); }
  }));

  const box = $('#select-input');
  box.addEventListener('input', () => {
    clearTimeout(selectTimer);
    selectTimer = setTimeout(runSelection, 500);
  });
  box.addEventListener('keydown', (e) => {
    if (e.key === 'Enter') { clearTimeout(selectTimer); runSelection(); return; }
    if (e.key !== 'ArrowUp' && e.key !== 'ArrowDown') return;
    e.preventDefault();
    S.selectHistAt = historyStep(S.selectHistory, S.selectHistAt, e.key === 'ArrowUp' ? 1 : -1);
    box.value = S.selectHistAt < 0 ? '' : S.selectHistory[S.selectHistAt];
  });
  $('#select-copy').addEventListener('click', () => {
    const sub = S.selectSub;
    if (!sub || !sub.names || !sub.names.length) return toast('nothing to copy', 'err');
    copyText(sub.names.join('\n'), `${sub.names.length} names`);
  });
  $('#select-ls').addEventListener('click', () => {
    const sub = S.selectSub;
    const named = sub ? sub.selector : boxSelector(box.value);
    if (named) return sendToTerminal(selectorCommand(named));
    if (!sub) return toast('run a selection first', 'err');
    sendToTerminal(lsCommand(sub.select, sub.exclude || ''));
  });
  $('#select-named').addEventListener('click', (e) => openSelectorMenu(e.currentTarget));
}

// ---------------------------------------------------------------- export --
/* What the canvas shows, as one file anyone can open with nothing installed:
   attached to a ticket and read in a browser, zoomed without blurring, printed
   to a one-page PDF, or pasted as an image (0026). Every word of it comes from
   the payload that was drawn, never from S or the boxes above the canvas: a
   rejected selector or a failed fetch leaves the last picture on screen while
   those already describe the next one.

   Pure from here to the Export button's handler, which is where the export
   harness stops reading. */

/* What the picture is of: the expression for a selection, the model for its
   neighbourhood, model.column for a column. A column box's `sub` starts with
   its owner's name, which is how the server builds it. */
function exportTitle(sub) {
  if (!sub || !sub.nodes || !sub.nodes.length) return '';
  if (sub.mode === 'select') return selectionText(sub);
  const f = sub.nodes[sub.focus];
  if (!f) return '';
  if (sub.mode !== 'column') return f.name;
  const owner = String(f.sub || '').split('  ·  ')[0];
  const column = sub.focus_column || f.name;
  return owner ? `${owner}.${column}` : column;
}

/* Short enough for a heading and a file name. A selection can run to dozens
   of terms and the command carries every one of them, so the heading keeps
   the first terms that fit and counts the rest. A first term too long to fit
   is cut itself rather than the count, which says more than its last letters
   would. An exclusion is never cut in half: it is named, or left to the
   command. */
function shortTitle(text, max) {
  const s = String(text || '').trim();
  if (s.length <= max) return s;
  const terms = s.split(/\s+/);
  const cut = terms.indexOf('--exclude');
  const inc = cut < 0 ? terms : terms.slice(0, cut);
  const rest = (k) => (k ? ` and ${k} more` : cut < 0 ? '' : ' --exclude …');
  let out = '';
  let n = 0;
  for (; n < inc.length; n++) {
    const next = out ? `${out} ${inc[n]}` : inc[n];
    if ((next + rest(inc.length - n - 1)).length > max) break;
    out = next;
  }
  if (out) return out + rest(inc.length - n);
  const tail = rest(inc.length - 1);
  return (inc[0] || s).slice(0, Math.max(1, max - tail.length - 1)) + '…' + tail;
}

/* The command that lists what the picture shows, for a reader who wants to
   check it against dbt itself (0024), in the words the dbt ls button types.
   When the set holds no test, tests are excluded outright: dbt ls would
   otherwise list the ones the tests checkbox kept off the canvas. A model's
   neighbourhood is written with dbt's own graph operators, from the depths
   actually drawn, and only for what dbt selects by a bare name. Column
   lineage has no dbt equivalent. */
function exportCommand(sub) {
  if (!sub || !sub.nodes || !sub.nodes.length) return '';
  // A named selector's own definition decides its tests, and dbt would ignore
  // an --exclude beside it anyway.
  if (sub.mode === 'select' && sub.selector) return selectorCommand(sub.selector);
  const tested = sub.mode === 'select'
    ? !!(sub.counts && sub.counts.test)
    : sub.nodes.some((n) => n.kind === 'test');
  const exclude = (ex) => (tested ? ex : [ex, 'resource_type:test'].filter(Boolean).join(' '));
  if (sub.mode === 'select') return lsCommand(sub.select || '', exclude(sub.exclude || ''));
  const f = sub.nodes[sub.focus];
  if (sub.mode !== 'model' || !f || !['model', 'seed', 'snapshot'].includes(f.kind)) return '';
  const depths = sub.nodes.map((n) => n.depth || 0);
  const up = Math.max(0, ...depths.map((d) => -d));
  const down = Math.max(0, ...depths);
  return lsCommand(`${up ? up + '+' : ''}${f.name}${down ? '+' + down : ''}`, exclude(''));
}

/* What matched but did not fit on the canvas, by name. A multiset difference,
   because a source and a model can share a name and each is a match. A box
   drawn as context matched nothing, so it accounts for no name. */
function undrawnNames(names, nodes) {
  const drawn = new Map();
  for (const n of nodes || []) if (!n.context) drawn.set(n.name, (drawn.get(n.name) || 0) + 1);
  const out = [];
  for (const name of names || []) {
    const left = drawn.get(name) || 0;
    if (left) drawn.set(name, left - 1);
    else out.push(name);
  }
  return out;
}

/* The manifest's state when the file was made, told to someone who was not
   there. The badge's own sentences speak to the person at the keyboard about
   files they can open, and carry an age, which is wrong by the time anyone
   reads this. */
function exportFreshness(f) {
  if (!f) return '';
  const n = (k) => f[k + '_n'] || 0;
  const files = (k) => `${k} file${k === 1 ? '' : 's'}`;
  const said = {
    fresh: 'The manifest then matched the project\'s files.',
    edited: `${files(n('edited'))} saved since the manifest was written had not been parsed, `
      + 'so the graph shows the project as it was before them.',
    stale: `${files(n('committed') + n('gone') + n('edited'))} had changed since the manifest was written, `
      + 'so some boxes may no longer match the code.',
  }[f.state] || '';
  const d = f.drift || {};
  const behind = d.base && d.behind
    ? `The branch was ${d.behind} commit${d.behind === 1 ? '' : 's'} behind ${d.base}.`
    : '';
  return [said, behind].filter(Boolean).join(' ');
}

/* The header's words, from the payload that was drawn and fields the caller
   fills in by name. S.meta itself never comes here: it also carries absolute
   paths, the manifest's and the column cache's, which have no business in a
   file that leaves the machine. */
function exportFacts(sub, ctx) {
  const kicker = { select: 'Custom selection', model: 'Lineage of a model', column: 'Column lineage' }[sub.mode] || 'Lineage';
  const depths = sub.nodes.map((n) => n.depth || 0);
  const up = Math.max(0, ...depths.map((d) => -d));
  const down = Math.max(0, ...depths);
  const counts = sub.mode === 'select'
    ? `${selectSummary(sub)}  ·  ${canvasStatus(sub, ctx.folders)}`
    : `${canvasStatus(sub, ctx.folders)}  ·  ${up} level${up === 1 ? '' : 's'} up, ${down} down`;
  const where = [
    ctx.project,
    ctx.dbtVersion && `dbt ${ctx.dbtVersion}`,
    ctx.manifestAt && `manifest written ${ctx.manifestAt}`,
    ctx.branch && `branch ${ctx.branch}${ctx.sha ? ` at ${ctx.sha}` : ''}`,
    sub.mode === 'column' && ctx.cllSource && `column lineage from ${ctx.cllSource}`,
  ].filter(Boolean).join('  ·  ');
  const when = [
    `Exported ${ctx.exportedAt}${ctx.version ? ` with dbt-edith ${ctx.version}` : ''}.`,
    exportFreshness(ctx.fresh),
  ].filter(Boolean).join(' ');
  return {
    kicker,
    lines: [sub.mode === 'select' && sub.description, counts, where, when].filter(Boolean),
    warnings: sub.mode === 'select' ? selectWarnings(selectNotes(sub).concat(sub.warnings || [])) : [],
    command: exportCommand(sub),
    undrawn: sub.mode === 'select' && sub.truncated ? undrawnNames(sub.names, sub.nodes) : [],
  };
}

/* An absolute moment with its offset: a reader elsewhere cannot tell 09/10
   from 10/09, and an age is wrong by the time anyone reads it. The offset is
   an argument so the answer does not depend on the machine running the tests. */
function exportStamp(secs, offsetMin) {
  const d = new Date((secs + offsetMin * 60) * 1000);
  const p = (x) => String(x).padStart(2, '0');
  const off = Math.abs(offsetMin);
  return `${d.getUTCFullYear()}-${p(d.getUTCMonth() + 1)}-${p(d.getUTCDate())} `
    + `${p(d.getUTCHours())}:${p(d.getUTCMinutes())} `
    + `UTC${offsetMin < 0 ? '-' : '+'}${p(Math.floor(off / 60))}:${p(off % 60)}`;
}

/* The day a file is named after, on the clock of whoever exported it. */
function localDay(d) {
  const p = (x) => String(x).padStart(2, '0');
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}`;
}

/* A name every system accepts. Anything outside a short safe set becomes a
   dash, which covers all Windows forbids, and the cut comes before the final
   trim so a name never ends in a dash or a dot. */
function exportFileName(title, day) {
  const slug = String(title || '')
    .replace(/[^A-Za-z0-9._+-]+/g, '-')
    .replace(/-{2,}/g, '-')
    .slice(0, 60)
    .replace(/^[-.]+|[-.]+$/g, '');
  return `lineage-${slug ? slug + '-' : ''}${day}.html`;
}

function escapeHtml(s) {
  const to = { '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' };
  return String(s ?? '').replace(/[&<>"']/g, (c) => to[c]);
}

/* The canvas rules, copied from app.css line for line, with the variables they
   and the page use. Copied rather than read from the live stylesheet at export
   time, which would hand over whatever the exporting browser made of them,
   rules it dropped included, to be opened in another browser. The export
   harness holds every copied line to app.css, so an edit there fails the
   check until it is made here too. */
function exportCanvasCss() {
  return [
    ':root { --bg: #12161d; --bg-2: #171c25; --bg-3: #1d2430; --line: #2a3340; --fg: #d7dee8; '
      + '--fg-dim: #8b97a8; --accent: #4da3ff; --warn: #e0a34b; '
      + '--mono: ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas, "Liberation Mono", monospace; '
      + '--ui: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif; }',
    '#graph text { font-family: var(--ui); user-select: none; }',
    '.edge { fill: none; stroke: var(--edge-col, #33445c); stroke-width: 1.2; }',
    '.edge.test { stroke: #5d6a7d; stroke-dasharray: 1 3; stroke-linecap: round; }',
    '.edge.hi { stroke: var(--accent); stroke-width: 2; }',
    '.edge.back { stroke-dasharray: 5 4; }',
    '.edge.test.back { stroke-dasharray: 5 3 1 3; }',
    '.nd rect.box { fill: var(--bg-3); stroke: var(--line); stroke-width: 1; rx: 5; }',
    '.nd:hover rect.box { stroke: var(--fg-dim); }',
    '.nd.focus rect.box { stroke: var(--accent); stroke-width: 2; fill: #1b2a3d; }',
    '.nd.off rect.box { stroke-dasharray: 3 3; }',
    '.nd.off .t1, .nd.off .t2 { opacity: .55; }',
    '.nd.ctx { opacity: .45; }',
    '.nd.ctx:hover { opacity: .85; }',
    '.nd.rider rect.box { fill: var(--bg-2); rx: 4; }',
    '.nd.rider .t1 { font-size: 10.5px; }',
    '.nd.fold { cursor: pointer; }',
    '.nd.fold rect.box { fill: transparent; stroke-dasharray: 3 3; }',
    '.nd.fold .t1 { fill: var(--fg-dim); }',
    '.nd.fold:hover .t1 { fill: var(--fg); }',
    '.nd.rider.singular rect.box { stroke-dasharray: 4 2; }',
    '.nd.group { cursor: pointer; }',
    '.nd .warnmark { fill: var(--warn); }',
    '.nd .warncount { font-size: 9px; }',
    '.nd.sel rect.box { stroke: #fff; }',
    '.nd .kindbar { stroke: none; rx: 3; }',
    '.nd .kindbar.seg { rx: 1.5; }',
    '.nd .t1 { fill: var(--fg); font-size: 11.5px; }',
    '.nd .t2 { fill: var(--fg-dim); font-size: 9.5px; }',
    '.nd .t2 .struck { opacity: .6; text-decoration: line-through; }',
    '.nd .more { fill: var(--fg-dim); font-size: 9px; }',
    '.band { fill: #161b24; stroke: #1f2632; stroke-width: 1; }',
    '.band-name { fill: var(--fg-dim); font-size: 13px; font-weight: 600; }',
    '.rolelabel { font: 8.5px var(--mono); letter-spacing: .4px; }',
    '.rolebadge rect { stroke-width: 1; }',
  ].join('\n');
}

/* The page around the picture, and its print. The page never scrolls, so the
   wheel over the graph always zooms. A print is one page whatever the graph:
   the body takes a fixed height that fits A4 and Letter alike, in millimetres
   because engines have measured `vh` against the screen when printing, and
   the graph gets what the header leaves. The zero page margin also keeps the
   browser from printing its own header and footer, which name where the file
   sat on the machine that printed it. */
function exportPageCss(orientation) {
  return `* { box-sizing: border-box; }
html { height: 100%; background: var(--bg); -webkit-print-color-adjust: exact; print-color-adjust: exact; }
body { height: 100%; margin: 0; display: flex; flex-direction: column; color: var(--fg); font: 13px/1.45 var(--ui); }
header { flex: 0 0 auto; padding: 10px 16px; background: var(--bg-2); border-bottom: 1px solid var(--line); }
.kicker { font: 600 10.5px var(--ui); letter-spacing: .7px; text-transform: uppercase; color: var(--fg-dim); }
h1 { margin: 1px 0 6px; font-size: 17px; font-weight: 600; overflow-wrap: anywhere; }
.cmd { display: flex; align-items: flex-start; gap: 8px; margin-bottom: 6px; }
.cmd code { flex: 1 1 auto; padding: 3px 8px; font: 12px/1.5 var(--mono); background: var(--bg-3); border: 1px solid var(--line); border-radius: 4px; white-space: pre-wrap; overflow-wrap: anywhere; }
.facts { margin: 0; padding: 0; list-style: none; font-size: 11.5px; color: var(--fg-dim); }
.facts .warn { color: var(--warn); }
.legend { display: flex; flex-wrap: wrap; gap: 4px 12px; margin-top: 6px; font: 10.5px var(--mono); color: var(--fg-dim); }
.legend i { display: inline-block; width: 8px; height: 8px; margin-right: 4px; border-radius: 2px; vertical-align: -1px; }
.tools { display: none; align-items: center; gap: 10px; margin-top: 8px; font-size: 11px; color: var(--fg-dim); }
button { display: none; padding: 2px 8px; font: 11px var(--ui); color: var(--fg); background: var(--bg-3); border: 1px solid var(--line); border-radius: 4px; cursor: pointer; }
button:hover { border-color: var(--accent); }
.js .tools { display: flex; }
.js button { display: inline-block; }
main { position: relative; flex: 1 1 0; min-height: 0; }
#graph { display: block; width: 100%; height: 100%; }
.js #graph { cursor: grab; }
.js #graph.panning { cursor: grabbing; }
#nodes { position: absolute; top: 8px; right: 8px; max-width: min(440px, 60%); max-height: calc(100% - 16px); overflow: auto; font-size: 11.5px; background: var(--bg-2); border: 1px solid var(--line); border-radius: 6px; }
#nodes summary { padding: 4px 10px; color: var(--fg-dim); cursor: pointer; }
#nodes ol { margin: 0; padding: 0 0 6px; list-style: none; }
#nodes li { padding: 3px 10px; }
.js #nodes li[data-id] { cursor: pointer; }
.js #nodes li[data-id]:hover { background: var(--bg-3); }
#nodes .nm { overflow-wrap: anywhere; }
#nodes .sub, #nodes .file, #nodes .undrawn { font: 10.5px var(--mono); color: var(--fg-dim); overflow-wrap: anywhere; }
#nodes .more { padding: 6px 10px 2px; font-size: 10.5px; letter-spacing: .6px; text-transform: uppercase; color: var(--fg-dim); }
@page { size: ${orientation}; margin: 0; }
@media print {
  html, body { height: auto; }
  body { height: ${orientation === 'portrait' ? 270 : 200}mm; padding: 6mm; overflow: hidden; }
  .tools, button, #nodes { display: none !important; }
}`;
}

/* One zoom step around a point, as a new viewBox. The point stays where it
   was on screen: with preserveAspectRatio meet, a box scaled evenly keeps its
   letterbox. Clamped through the factor, never width and height apart, so the
   ratio survives the clamp. */
function zoomViewBox(vb, ux, uy, f, minW, maxW) {
  const [x, y, w, h] = vb;
  const k = Math.min(Math.max(f, w / maxW), w / minW);
  return [ux - (ux - x) / k, uy - (uy - y) / k, w / k, h / k];
}

/* Runs inside the exported file and never here: the page is given this
   function's own source text, so it may name nothing else from this file.
   All it needs is in the page, and zoomViewBox arrives as its argument.

   Pan and zoom rewrite the viewBox, never a transform, so the page with
   scripts off still shows the whole graph fitted, and printing has one
   attribute to put back. */
function exportViewer(zoomViewBox) {
  const svg = document.getElementById('graph');
  const layer = svg && svg.querySelector('g');
  if (!layer) return;
  document.documentElement.classList.add('js');
  const home = svg.getAttribute('viewBox').split(/[\s,]+/).map(Number);
  let view = home.slice();
  const show = (v) => { view = v; svg.setAttribute('viewBox', v.join(' ')); };
  // The inner group's matrix is the viewBox mapping itself. Engines have not
  // always agreed on whether the outer svg's includes it.
  const toUser = (e) => new DOMPoint(e.clientX, e.clientY).matrixTransform(layer.getScreenCTM().inverse());

  svg.addEventListener('wheel', (e) => {
    e.preventDefault();
    // Some mice report lines in Firefox, not pixels, and a notch would then
    // barely move the zoom.
    const dy = e.deltaY * (e.deltaMode === 1 ? 16 : e.deltaMode === 2 ? 400 : 1);
    const p = toUser(e);
    show(zoomViewBox(view, p.x, p.y, Math.exp(-dy * 0.0015), 120, home[2] * 4));
  }, { passive: false });

  let picked = null;
  const pick = (id) => {
    picked = id;
    svg.querySelectorAll('.nd').forEach((g) => g.classList.toggle('sel', g.dataset.id === id));
    svg.querySelectorAll('.edge').forEach((p) => {
      const hit = !!id && (p.dataset.a === id || p.dataset.b === id);
      p.classList.toggle('hi', hit);
      // On top, as on the canvas: edges sharing a lane lie on one line.
      if (hit) p.parentNode.appendChild(p);
    });
  };

  // Listening on the window, with no pointer capture: a captured pointer
  // sends its click to the svg, and the box under it would never hear of it.
  let drag = null;
  svg.addEventListener('pointerdown', (e) => {
    if (e.button !== 0) return;
    const box = e.target.closest('.nd');
    drag = {
      x: e.clientX, y: e.clientY, from: view.slice(), k: layer.getScreenCTM().a,
      moved: false, id: box ? box.dataset.id : null,
    };
  });
  window.addEventListener('pointermove', (e) => {
    if (!drag) return;
    const dx = e.clientX - drag.x, dy = e.clientY - drag.y;
    // The canvas's own slop (`pastSlop` in lineage.js), written out: this
    // function may name nothing outside itself.
    if (!drag.moved && Math.abs(dx) <= 4 && Math.abs(dy) <= 4) return;
    drag.moved = true;
    svg.classList.add('panning');
    show([drag.from[0] - dx / drag.k, drag.from[1] - dy / drag.k, drag.from[2], drag.from[3]]);
  });
  const drop = () => { drag = null; svg.classList.remove('panning'); };
  window.addEventListener('pointerup', () => {
    if (drag && !drag.moved) pick(drag.id && drag.id !== picked ? drag.id : null);
    drop();
  });
  window.addEventListener('pointercancel', drop);
  svg.addEventListener('dblclick', (e) => { if (!e.target.closest('.nd')) show(home.slice()); });

  const fit = document.getElementById('fit');
  if (fit) fit.addEventListener('click', () => show(home.slice()));
  document.addEventListener('keydown', (e) => {
    if (e.key === '0') show(home.slice());
    if (e.key === 'Escape') pick(null);
  });

  // A row of the list selects its box and brings it to the middle, at the
  // zoom already chosen.
  document.querySelectorAll('#nodes li[data-id]').forEach((li) => li.addEventListener('click', () => {
    const g = [...svg.querySelectorAll('.nd')].find((x) => x.dataset.id === li.dataset.id);
    if (!g) return;
    pick(li.dataset.id);
    const t = g.transform.baseVal.consolidate();
    const box = g.querySelector('rect.box');
    const cx = (t ? t.matrix.e : 0) + box.width.baseVal.value / 2;
    const cy = (t ? t.matrix.f : 0) + box.height.baseVal.value / 2;
    show([cx - view[2] / 2, cy - view[3] / 2, view[2], view[3]]);
  }));

  // Copied where the clipboard is allowed; elsewhere selected, ready for the
  // keyboard.
  const copy = document.getElementById('copy');
  const code = document.querySelector('.cmd code');
  if (copy && code) copy.addEventListener('click', () => {
    const choose = () => {
      const range = document.createRange();
      range.selectNodeContents(code);
      getSelection().removeAllRanges();
      getSelection().addRange(range);
      copy.textContent = 'Selected';
    };
    if (!navigator.clipboard) return choose();
    navigator.clipboard.writeText(code.textContent).then(() => { copy.textContent = 'Copied'; }, choose);
  });

  let kept = null;
  window.addEventListener('beforeprint', () => { kept = view; show(home.slice()); });
  window.addEventListener('afterprint', () => { if (kept) show(kept); kept = null; });
}

/* The whole file. Every value from the project goes through escapeHtml; the
   SVG is the one thing inserted as it is, and XMLSerializer escaped it on the
   way out. The script is the same text whatever is drawn, so nothing from a
   project can end up running, and the policy forbids every fetch, which keeps
   the file working offline even after a later change reaches for a font. */
function exportDocument(o) {
  const e = escapeHtml;
  const f = o.facts;
  const legend = o.legend.map(([label, colour]) =>
    `<span><i style="background:${e(colour)}"></i>${e(label)}</span>`).join('');
  const facts = f.warnings.map((w) => `<li class="warn">${e(w)}</li>`)
    .concat(f.lines.map((l) => `<li>${e(l)}</li>`)).join('');
  // The struck strategy keeps its strike in the list, as it does on the picture.
  const sub = (n) => (n.parts
    ? Lineage.subJoin({ lead: e(n.parts.lead), struck: e(n.parts.struck), rest: e(n.parts.rest) }, (s) => `<s>${s}</s>`)
    : e(n.sub));
  const rows = o.nodes.map((n) => `<li data-id="${e(n.id)}"><div class="nm">${e(n.name)}</div>`
    + `<div class="sub">${sub(n)}</div><div class="file">${e(n.file)}</div></li>`).join('');
  const undrawn = f.undrawn.length
    ? `<div class="more">Matched, not drawn (${f.undrawn.length})</div>`
      + `<ol class="undrawn">${f.undrawn.map((n) => `<li>${e(n)}</li>`).join('')}</ol>`
    : '';
  const command = f.command
    ? `<div class="cmd"><code>${e(f.command)}</code><button id="copy" type="button">Copy</button></div>`
    : '';
  return `<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src 'unsafe-inline'; script-src 'unsafe-inline'">
<title>${e(o.heading)} · lineage</title>
<style>
${exportCanvasCss()}
${exportPageCss(o.frame.h > o.frame.w ? 'portrait' : 'landscape')}
</style>
</head>
<body>
<header>
<div class="kicker">${e(f.kicker)}</div>
<h1>${e(o.heading)}</h1>
${command}
<ul class="facts">${facts}</ul>
<div class="legend">${legend}</div>
<div class="tools"><button id="fit" type="button">Fit</button><span>Wheel to zoom, drag to pan, click a box for its edges, 0 to fit. Ctrl+P saves a PDF.</span></div>
</header>
<main>
${o.svg}
<details id="nodes"><summary>${e(o.noun)} (${o.nodes.length})</summary><ol>${rows}</ol>${undrawn}</details>
</main>
<script>(${exportViewer})(${zoomViewBox});</script>
</body>
</html>
`;
}

/* Twice the size for a sharp paste, less for a graph so large that a canvas
   would refuse it or eat the memory of a small machine. */
function pngScale(w, h) {
  return Math.min(2, Math.sqrt(16e6 / (w * h)), 16384 / w, 16384 / h);
}

/* The Export button. Synchronous from the click to the download: a browser
   lets a page save an .html file only while the user's gesture lasts, and an
   await would spend it. */
function exportLineage() {
  const shot = Lineage.snapshot();
  if (!shot) return toast('nothing drawn to export', 'err');
  const sub = shot.data;
  const meta = S.meta || {};
  const fresh = S.fresh || null;
  const stamp = (secs) => exportStamp(secs, -new Date(secs * 1000).getTimezoneOffset());
  const manifestAt = (fresh && fresh.manifest_at) || meta.manifest_mtime || 0;
  const facts = exportFacts(sub, {
    project: meta.project || '',
    dbtVersion: meta.dbt_version || '',
    manifestAt: manifestAt ? stamp(manifestAt) : '',
    branch: (S.git && S.git.repo && S.git.branch) || '',
    sha: (fresh && fresh.head && fresh.head.sha) || '',
    cllSource: meta.cll_source || '',
    fresh,
    version: S.version,
    exportedAt: stamp(Math.floor(Date.now() / 1000)),
    folders: shot.folders,
  });
  const heading = shortTitle(exportTitle(sub), 60);
  const nodes = sub.nodes
    .map((n) => ({ id: n.id, name: n.name, sub: Lineage.subtitle(n), parts: Lineage.subtitleParts(n), file: n.file }))
    .sort((a, b) => a.name.localeCompare(b.name));
  const html = exportDocument({
    heading, facts, nodes, legend: legendEntries(sub), svg: shot.svg, frame: shot.frame,
    noun: sub.mode === 'column' ? 'Columns' : 'Nodes',
  });
  const name = exportFileName(heading, localDay(new Date()));
  saveBlob(new Blob([html], { type: 'text/html;charset=utf-8' }), name);
  toast('saved ' + name, 'ok');
}

function saveBlob(blob, name) {
  const url = URL.createObjectURL(blob);
  const a = Object.assign(document.createElement('a'), { href: url, download: name });
  document.body.appendChild(a);
  a.click();
  a.remove();
  // Not at once: a download the browser has not started yet still needs it.
  setTimeout(() => URL.revokeObjectURL(url), 60000);
}

/* Rasterised from the standalone SVG, onto a canvas filled with the page's
   background first: an image has no page behind it. Loaded from a data: URL,
   because this page's policy takes images from data: and nowhere else (0015),
   so a blob: URL fails to load without saying why. */
function renderPng(shot, scale) {
  const bytes = new TextEncoder().encode(shot.svg);
  let bin = '';
  for (let i = 0; i < bytes.length; i += 0x8000) bin += String.fromCharCode(...bytes.subarray(i, i + 0x8000));
  return new Promise((resolve, reject) => {
    const img = new Image();
    // Caught here, or a canvas that throws would leave the promise, and the
    // clipboard waiting on it, unsettled for good.
    img.onload = () => {
      try {
        const canvas = document.createElement('canvas');
        canvas.width = Math.round(shot.frame.w * scale);
        canvas.height = Math.round(shot.frame.h * scale);
        const ctx = canvas.getContext('2d');
        ctx.fillStyle = getComputedStyle(document.documentElement).getPropertyValue('--bg').trim();
        ctx.fillRect(0, 0, canvas.width, canvas.height);
        ctx.drawImage(img, 0, 0, canvas.width, canvas.height);
        canvas.toBlob((b) => (b ? resolve(b) : reject(new Error('the browser could not encode it'))), 'image/png');
      } catch (e) { reject(e); }
    };
    img.onerror = () => reject(new Error('the browser could not draw it'));
    img.src = 'data:image/svg+xml;base64,' + btoa(bin);
  });
}

/* The Copy image button: the same picture as a PNG on the clipboard, for a
   ticket where an attachment takes a click to see. The clipboard item is made
   inside the click, holding a promise for its content, which is what Safari
   needs to count it as the user's gesture. Whatever refuses it gets the file
   instead. */
function copyLineageImage() {
  const first = Lineage.snapshot({ tight: true });
  if (!first) return toast('nothing drawn to copy', 'err');
  const scale = pngScale(first.frame.w, first.frame.h);
  const shot = Lineage.snapshot({ css: exportCanvasCss(), scale, tight: true });
  const png = renderPng(shot, scale);
  const name = exportFileName(shortTitle(exportTitle(shot.data), 60), localDay(new Date()))
    .replace(/\.html$/, '.png');
  const save = () => png.then((blob) => {
    saveBlob(blob, name);
    toast('the clipboard took no image, saved ' + name + ' instead', 'ok');
  }).catch((e) => toast('image: ' + e.message, 'err'));
  if (!window.ClipboardItem || !navigator.clipboard || !navigator.clipboard.write) return save();
  navigator.clipboard.write([new ClipboardItem({ 'image/png': png })])
    .then(() => toast('image copied: paste it into the ticket', 'ok'), save);
}

// -------------------------------------------------------------- artifacts --
/* The two files dbt leaves per node under target/: compiled/ holds the model's
   SQL with the Jinja gone, run/ the statement dbt executed to materialise it.
   They get a tab each and are never shown in each other's place, because
   "what does this model say" and "what did dbt send to the warehouse" are
   different questions and only one of them explains a table that looks wrong.

   Nothing is compiled here: dbt runs where the user runs it (0002), so a
   missing file is reported with the command that would produce it. */
async function loadArtifact(kind, id) {
  const head = $('#' + kind + '-head');
  const body = $('#' + kind + '-body');
  head.textContent = '';
  if (!id) {
    body.textContent = '';
    body.append(Object.assign(document.createElement('div'), {
      className: 'compiled-empty', textContent: 'Select a model first.' }));
    return;
  }
  const node = S.node && S.node.id === id ? S.node : null;
  const name = node ? node.name : id.split('.').pop();

  let info;
  try { info = await api.get('/api/compiled?kind=' + kind + '&id=' + encodeURIComponent(id)); }
  catch (e) { return toast(kind + ': ' + e.message, 'err'); }

  // A test node even when the catalog is showing something else: a dbt
  // unique_id says what it is, and the id is all this has when they disagree.
  const said = artifactBar(info, name, node ? node.kind === 'test' : id.startsWith('test.'));

  if (!said.found) {
    if (S.artifactCm[kind]) S.artifactCm[kind].setValue('');
    body.textContent = '';
    const box = document.createElement('div');
    box.className = 'compiled-empty';
    box.append(Object.assign(document.createElement('p'), { textContent: said.head, style: 'margin:0 0 4px' }));
    const code = document.createElement('code');
    code.textContent = said.command;
    box.append(code, document.createElement('br'));
    const send = document.createElement('button');
    send.className = 'btn sm';
    send.textContent = 'Type it in the terminal';
    send.title = 'Puts the command in the integrated terminal without running it';
    send.addEventListener('click', () => sendToTerminal(said.command));
    box.append(send);
    const det = document.createElement('details');
    det.style.marginTop = '12px';
    det.append(Object.assign(document.createElement('summary'), {
      textContent: 'paths checked', style: 'cursor:pointer;font-size:11px' }));
    const ul = document.createElement('ul');
    said.candidates.forEach((c) => ul.append(Object.assign(document.createElement('li'), { textContent: c })));
    det.append(ul);
    box.append(det);
    body.append(box);
    return;
  }

  const bar = document.createElement('div');
  bar.className = 'fresh-bar' + (said.tone === 'stale' ? ' stale' : '');
  bar.append(Object.assign(document.createElement('b'), { textContent: said.when }));
  bar.append(Object.assign(document.createElement('span'), { className: 'why', textContent: said.why }));
  bar.append(Object.assign(document.createElement('div'), { className: 'grow' }));
  const redo = document.createElement('button');
  redo.className = 'btn sm';
  redo.textContent = said.button;
  redo.title = 'Puts ' + said.command + ' in the integrated terminal without running it';
  redo.addEventListener('click', () => sendToTerminal(said.command));
  bar.append(redo, artifactPath(said));
  head.append(bar);

  body.textContent = '';
  if (!S.artifactCm[kind]) {
    S.artifactCm[kind] = CodeMirror(body, { lineNumbers: true, readOnly: true, mode: 'text/x-sql', lineWrapping: false });
  } else {
    body.appendChild(S.artifactCm[kind].getWrapperElement());
  }
  S.artifactCm[kind].setValue(info.content + (info.truncated ? '\n\n-- truncated by dbt-edith\n' : ''));
  setTimeout(() => S.artifactCm[kind].refresh(), 0);
}

/* The file's own line in the bar. A path dbt wrote is long and nobody retypes
   it, so it is a link: clicking brings the file tree back into view and walks
   it down to the row, which is the fastest way to reach the file itself, the
   ones beside it, or the folder to delete before a clean build. A target
   outside the project has no tree row, so it is shown and not linked. */
function artifactPath(said) {
  if (!said.link) {
    return Object.assign(document.createElement('div'),
      { className: 'artifact-path outside', textContent: said.path, title: said.pathTitle });
  }
  const link = document.createElement('button');
  link.className = 'artifact-path link';
  link.textContent = said.path;
  link.title = said.pathTitle;
  link.addEventListener('click', () => {
    showSide('files');
    revealInTree(said.path);
  });
  return link;
}

function humanAge(secs) {
  if (secs < 60) return secs + 's';
  if (secs < 3600) return Math.round(secs / 60) + 'min';
  if (secs < 86400) return Math.floor(secs / 3600) + 'h' + String(Math.floor((secs % 3600) / 60)).padStart(2, '0');
  return Math.floor(secs / 86400) + 'd';
}

/* Every word the Compiled or Run tab puts on screen for one file. The bar
   answers, of a single file, the question the manifest badge answers of the
   whole project: is what you are reading still what the source says?

   It leads with the date rather than the age, because that is the number you
   can compare with something. "3h ago" does not say whether this predates the
   nightly build, the pull you did at nine, or the edit still open in the
   editor; "22/09/2026 08:14" does, and the age rides beside it for the glance.

   Amber means something it was built from has moved since: the model, its
   schema file, dbt_project.yml, a macro, or for a run, a compile that happened
   after it. Never the age on its own, however large. Pure, so every state is
   testable without a DOM. */
function artifactBar(info, name, isTest) {
  const kind = info && info.kind === 'run' ? 'run' : 'compiled';
  const w = {
    compiled: {
      verb: 'compiled', dir: 'target/compiled/', what: 'compiled SQL',
      cmd: 'dbt compile', fresh: 'Compile again', stale: 'Recompile',
    },
    // A test's run file is written by `dbt test` and never by `dbt run`, which
    // would select nothing and leave the tab saying the same thing afterwards.
    // Compiling is the same command for both.
    run: isTest ? {
      verb: 'last run', dir: 'target/run/', what: 'run SQL',
      cmd: 'dbt test', fresh: 'Test again', stale: 'Test again',
    } : {
      verb: 'last run', dir: 'target/run/', what: 'run SQL',
      cmd: 'dbt run', fresh: 'Run again', stale: 'Run again',
    },
  }[kind];
  const command = w.cmd + ' --select ' + name;

  if (!info || !info.found) {
    return { kind, found: false, command,
      head: 'No ' + w.what + ' for ' + name + '. dbt has not written it into ' + w.dir + ' yet.',
      candidates: (info && info.candidates) || [] };
  }

  const age = humanAge(info.age_secs || 0) + ' ago';
  // "changed since" once, then the list. Said per entry it ran to five copies
  // of "changed after this was run" and stopped being readable at a glance,
  // which is the only thing this line is for.
  const why = [age];
  if ((info.changed || []).length) why.push('changed since: ' + info.changed.join(', '));
  (info.reasons || []).forEach((r) => why.push(r));
  // The absolute path is the tooltip either way: it is what you paste into a
  // terminal, and the only thing left to show when no link can be offered.
  return {
    kind, found: true, command,
    tone: info.stale ? 'stale' : 'ok',
    when: w.verb + ' ' + new Date((info.compiled_at || 0) * 1000).toLocaleString(),
    // The age leads this line and what moved follows it, because the age is
    // the part that is always true and the part a glance is looking for.
    why: why.join(' \u00b7 ') + (info.stale ? ' (may be out of date)' : ''),
    button: info.stale ? w.stale : w.fresh,
    link: !!info.rel,
    path: info.rel || info.path,
    pathTitle: info.rel
      ? info.path + ' \u00b7 click to show it in the file tree'
      : info.path + ' \u00b7 outside the project, so the file tree cannot reach it',
  };
}

/* What the badge beside Reload manifest shows, and what its hover card says.
   The dot's colour answers one question only, the one that decides whether the
   lineage on screen can be trusted: does this manifest still match the files
   dbt would parse? Drift from the default branch is reported in its own
   segment rather than in the colour, because a feature branch behind main
   still has a manifest that is true for the code in front of you.

   The card carries the words. "manifest 3d" and "main +10" are a reminder of
   something you already understand, not an explanation, so every number the
   badge shows is spelled out in a sentence here. Pure, so every state is
   testable without a DOM. */
function freshnessBadge(f) {
  const n = (k) => f[k + '_n'] || 0;
  const plural = (k, one, many) => n(k) + ' ' + (n(k) === 1 ? one : many);
  const tone = { fresh: 'ok', edited: 'warn', stale: 'bad', missing: 'none' }[f.state] || 'none';
  const label = f.state === 'missing' ? 'no manifest' : 'manifest ' + humanAge(f.age_secs || 0);

  const head = {
    fresh: 'The lineage matches your files',
    edited: 'Your saved edits are not in it yet',
    stale: 'The files moved under this manifest',
    missing: 'No manifest.json yet',
  }[f.state] || 'Unknown state';

  const desc = {
    fresh: 'Nothing dbt parses has changed since this manifest was written, so the graph is showing what your project actually says.',
    edited: 'You have saved changes dbt has not parsed. The graph still shows the project as it was before them.',
    stale: 'Files changed and were committed after this manifest was written. The graph may be showing models that no longer look like this.',
    missing: 'There is nothing to draw a lineage from until dbt writes one.',
  }[f.state] || '';

  const when = f.state === 'missing' ? ''
    : 'written ' + new Date((f.manifest_at || 0) * 1000).toLocaleString() + ', ' + humanAge(f.age_secs || 0) + ' ago';

  // Split so the card can lead with the file name and dim the folder: these
  // paths run to eighty characters and the last segment is the one that reads.
  const split = (p) => {
    const cut = p.lastIndexOf('/');
    return { full: p, name: cut < 0 ? p : p.slice(cut + 1), dir: cut < 0 ? '' : p.slice(0, cut).split('/').pop() };
  };
  const group = (key, heading) => {
    if (!n(key)) return null;
    const shown = (f[key] || []).map(split);
    return { head: heading, paths: shown, more: n(key) - shown.length };
  };
  const groups = [
    group('committed', plural('committed', 'file', 'files') + ' changed since, already committed'),
    group('gone', plural('gone', 'file', 'files') + ' the manifest names no longer exist'),
    group('edited', plural('edited', 'file', 'files') + ' saved since, not committed'),
  ].filter(Boolean);

  const d = f.drift || {};
  const h = f.head || {};
  const facts = [];
  if (n('committed')) facts.push('A checkout, a pull or a merge brought them in.');
  if (h.sha) facts.push('This branch sits at ' + h.sha + (h.subject ? ', "' + h.subject + '"' : '') + '.');
  let drift = '';
  if (d.base && d.behind) {
    drift = d.base.replace(/^origin\//, '') + ' +' + d.behind;
    facts.push(d.base + ' has ' + d.behind + ' commit' + (d.behind === 1 ? '' : 's')
      + ' this branch does not have, so the lineage is your branch and not production.');
  } else if (d.base) {
    facts.push('This branch has everything ' + d.base + ' has.');
  }
  if (d.base) {
    // A count of commits is only as current as the refs it was counted from,
    // so the card says when they were last refreshed rather than implying now.
    facts.push(d.fetched_at
      ? 'Remote refs last fetched ' + humanAge(Math.max(0, Math.floor(Date.now() / 1000) - d.fetched_at)) + ' ago.'
      : 'Remote refs have never been fetched in this clone, so that comparison means little.');
  }

  const advice = f.advice || '';
  return { tone, label, drift, head, when, desc, groups, facts, advice,
    hint: advice ? 'Click to run it in the terminal.' : 'Click to run dbt parse in the terminal.' };
}

/* What the canvas says when the file in front has no node, or '' when it was
   never going to have one: a properties file, a macro, anything but a .sql,
   .py or .csv. `shown` names the model still drawn, since one named like this
   file is easily read as its lineage. A manifest older than the files is the
   usual reason and the one the user can act on, so it is named; dbt parse is
   only offered as a maybe, a .sql file outside every resource folder having
   no node either. The lineage follows by itself once the manifest reloads
   (followActive), so the sentence promises no click. */
function noNodeHint(path, f, { macros = false, shown = '' } = {}) {
  const name = path.split('/').pop();
  if (macros || !/\.(sql|py|csv)$/i.test(name)) return '';
  const head = `No node in the manifest comes from ${name}`
    + (shown ? `, so the lineage still shows ${shown}.` : '.');
  if (!f || f.state === 'fresh') return head;
  if (f.state === 'missing') return head + ' There is no manifest.json yet: dbt parse writes one.';
  return head + ' The manifest is older than the project\'s files: if this one is new, run dbt parse and its lineage follows.';
}

/* Types a command into the integrated terminal. It stops at the prompt by
   default, because running it is the user's decision and dbt is theirs to
   launch (0002). `run` is for the freshness badge alone: its dbt parse reaches
   no warehouse, and a command that can is never run for the user (0071). */
function sendToTerminal(cmd, run) {
  // Always on a click, and the terminal is what it asks to see.
  closeReports();
  showDock('terminal');
  let tries = 0;
  const attempt = () => {
    if (S.ws && S.ws.readyState === 1) {
      const d = run ? cmd + '\r' : cmd;
      S.termPrompt = promptAfter(S.termPrompt, d);
      S.ws.send(JSON.stringify({ t: 'i', d }));
      S.term && S.term.focus();
      toast(run ? 'running ' + cmd + ' in the terminal' : 'command ready in the terminal, press Enter to run it');
    } else if (tries++ < 40) setTimeout(attempt, 100);
    else toast('terminal is not connected', 'err');
  };
  setTimeout(attempt, 60);
}

// ---------------------------------------------------------------- catalog --
function renderCatalog(n) {
  S.node = n;
  const host = $('#catalog');
  if (envMenu && host.contains(envMenu.anchor)) closeEnvMenu();
  if (popup && host.contains(popup.anchor)) closePopup();
  // A node that lands while the filter is being typed in redraws the table,
  // and the box with it: the next key has to reach the new one.
  const typing = document.activeElement && document.activeElement.id === 'col-filter';
  host.textContent = '';

  const head = document.createElement('div');
  head.className = 'cat-head';
  const title = document.createElement('div');
  title.className = 'cat-title';
  title.append(dot(n), document.createTextNode(n.name));
  if (n.disabled) {
    const off = document.createElement('span');
    off.className = 'offchip';
    off.textContent = 'disabled';
    off.title = 'enabled: false, so dbt leaves this model out of the graph';
    title.appendChild(off);
  }
  if (n.file) {
    const open = document.createElement('button');
    open.className = 'btn sm';
    open.textContent = 'Open file';
    open.addEventListener('click', () => {
      if (n.kind === 'test' && n.test_name) return openTest(n, { pin: true });
      pushJump();
      openFile(n.file, { focusLineage: false });
      revealInTree(n.file);
    });
    title.appendChild(open);
  }
  const crumb = document.createElement('div');
  crumb.className = 'crumb';
  crumb.innerHTML = '';
  crumb.append(document.createTextNode(n.package || ''), document.createTextNode('  /  '));
  const b = document.createElement('b');
  b.textContent = n.file || n.id;
  crumb.appendChild(b);

  const tabs = document.createElement('div');
  tabs.className = 'cat-tabs';
  for (const [key, label] of [['preview', 'Preview'], ['columns', `Columns${n.columns.length ? ' (' + n.columns.length + ')' : ''}`]]) {
    const t = document.createElement('button');
    t.className = 'ctab' + (S.catTab === key ? ' active' : '');
    t.textContent = label;
    t.addEventListener('click', () => showCatalogTab(n, key));
    tabs.appendChild(t);
  }
  head.append(title, crumb, tabs);

  const body = document.createElement('div');
  body.className = 'cat-body';
  (S.catTab === 'columns' ? catalogColumns : catalogPreview)(body, n);
  host.append(head, body);
  const filter = typing && $('#col-filter');
  if (filter) {
    filter.focus();
    filter.setSelectionRange(filter.value.length, filter.value.length);
  }
}

/* A Catalog tab opened on the node it shows, from its button, a count in
   Preview or Cmd+F alike, so none of them can come to open it differently. */
function showCatalogTab(n, key) {
  S.catTab = key;
  renderCatalog(n);
}

function h3(label) {
  const h = document.createElement('h3');
  h.textContent = label;
  return h;
}

function stat(label, value, onClick) {
  const s = document.createElement('div');
  s.className = 'stat';
  const k = document.createElement('div');
  k.className = 'k';
  k.textContent = label;
  const v = document.createElement('div');
  v.className = 'v' + (onClick ? ' link' : '');
  v.textContent = value;
  if (onClick) {
    // A div rather than a button, so the number keeps the type of the plain
    // stats beside it.
    v.setAttribute('aria-label', `${label}: ${value}`);
    linkLike(v, onClick);
  }
  s.append(k, v);
  return s;
}

// ------------------------------------------------------------------- jobs --
/* Which of the orchestrator's jobs act on a node, read from folders the user
   named outside the project (0063). The pure part runs from jobsScheduleText
   to loadJobs, which web/tests/jobs.js slices. */

/* How a job starts, in a few words. Manual is what is left when nothing
   starts it: no clock, no asset, no other job, no webhook. */
function jobsScheduleText(job) {
  const parts = jobsStartedBy(job);
  return parts.length ? parts.join(' · ') : 'manual';
}

function jobsStartedBy(job) {
  const parts = [];
  if (job.cron) parts.push(job.cron);
  if (job.assets && job.assets.length) {
    const unread = job.assets_unread || [];
    parts.push('on ' + job.assets.join(', ') + (!unread.length ? ''
      : job.assets.length === 1 ? ', which no file read updates' : `; no file read updates ${unread.join(', ')}`));
  }
  if (job.after) parts.push(job.after_unread ? `after job ${job.after}, which no file read defines` : `after ${job.after}`);
  if (job.ci) parts.push('CI');
  if (job.merge) parts.push('on merge');
  if (job.note) parts.push(job.note);
  return parts;
}

/* Scheduled runs on its own, manual is started by nothing at all, and
   between the two sits what something starts that never leads back to a
   clock: CI, a merge, an asset only a manual DAG updates, `@once`. Unread
   may run on its own, on what no file read says: a templated schedule, an
   asset no file read updates (0077). */
function jobsStart(job) {
  if (job.scheduled) return 'scheduled';
  if (job.schedule_unread) return 'unread';
  return jobsStartedBy(job).length ? 'other' : 'manual';
}

/* The jobs that run on their own first, then those that may, and the manual
   ones last, each class in the order the server sent, which is the files' own
   (0055). `job` finds the job in an item: a node's groups hold one each. */
function jobsByStart(items, job = (g) => g.job) {
  const rank = { scheduled: 0, unread: 1, other: 2, manual: 3 };
  return (items || []).map((item) => ({ item, rank: rank[jobsStart(job(item))] }))
    .sort((a, b) => a.rank - b.rank).map((d) => d.item);
}

/* How many steps not resolved could act on the node, for the tab's summary
   and the Catalog's chip alike. */
function jobsMaybeCount(maybe) {
  return (maybe || []).reduce((a, g) => a + g.steps.length, 0);
}

/* A step by its job and its own name, or its place in a dbt Cloud job: what
   a Catalog chip asks the Jobs tab to show. Not the step's index alone, which
   a read after a job file changed gives to another step; the index still tells
   apart two sources of one label holding the same file. */
function jobsStepKey(job, step) {
  return [job.source, job.file, job.ident || job.name, step.name || step.n].join('\n');
}

/* A DAG by its id, a dbt Cloud job by its name and the identifier
   dbt-jobs-as-code appends to it there. */
function jobsJobLabel(job) {
  return job.kind === 'airflow' ? 'DAG ' + job.name : 'Job ' + job.name + (job.ident ? ' [[' + job.ident + ']]' : '');
}

/* An Airflow task by its id; a dbt Cloud step by its place, since dbt Cloud
   names none. */
function jobsStepLabel(job, step) {
  if (job.kind === 'airflow') return 'task ' + (step.name || step.n);
  return `step ${step.n}/${job.steps}` + (step.implicit ? ', from run_generate_sources' : '');
}

/* A step on a Catalog chip: its job and itself, without the DAG and the
   task the Jobs tab writes in front, which the section's title says. */
function jobsChipText(job, step) {
  const own = job.kind === 'airflow' ? (step.name || 'task ' + step.n) : `step ${step.n}/${job.steps}`;
  return `${job.name} › ${own}`;
}

/* One chip per step that runs the node, scheduled jobs first, as the Jobs
   tab lists them, and each job's steps in their order. */
function jobsChips(groups) {
  return jobsByStart(groups).flatMap((g) => g.steps.map((step) => ({ job: g.job, step, text: jobsChipText(g.job, step) })));
}

/* The lines that tie a job to others. */
function jobsJobLinks(job) {
  const rows = [];
  if (job.waits_for && job.waits_for.length) rows.push('waits for ' + job.waits_for.join(', '));
  if (job.triggers && job.triggers.length) rows.push('starts ' + job.triggers.join(', ') + ' when it ends');
  const where = [job.env ? 'environment ' + job.env : '', job.target ? 'target ' + job.target : ''].filter(Boolean);
  if (where.length) rows.push(where.join(', '));
  return rows;
}

/* The lines that tie a step to others, and what in its selection reached the
   node: `+tag:daily` says why a staging model is in a marts task. */
function jobsLinkRows(step) {
  const rows = [];
  const some = (a) => a && a.length;
  if (some(step.via)) rows.push('via ' + step.via.join(', '));
  if (some(step.upstream)) rows.push('after ' + step.upstream.join(', '));
  if (some(step.downstream)) rows.push('before ' + step.downstream.join(', '));
  if (some(step.waited_by)) rows.push('awaited by ' + step.waited_by.join(', '));
  if (some(step.outlets)) {
    rows.push('updates ' + step.outlets.join(', ') + (some(step.feeds) ? ', which starts ' + step.feeds.join(', ') : ''));
  }
  return rows;
}

/* Coverage rows by folder, in the order the server sorted them. A folder is
   a level of its own only where it holds two of the rows or more: on a
   project with a folder per model, every model would otherwise be a folder to
   unfold. Each row goes under the deepest folder that gathers it with
   another, `rest` saying its path below that one. A row nothing gathers ends
   up alone, or under no folder, and the page lists it bare. */
function jobsCoverageGroups(rows) {
  const parts = (r) => (r.folder || '').split('/').filter(Boolean);
  const held = new Map();
  for (const r of rows || []) {
    const p = parts(r);
    for (let i = 1; i <= p.length; i++) {
      const f = p.slice(0, i).join('/');
      held.set(f, (held.get(f) || 0) + 1);
    }
  }
  const out = new Map();
  for (const r of rows || []) {
    const p = parts(r);
    let at = p.length;
    while (at > 0 && held.get(p.slice(0, at).join('/')) < 2) at--;
    const folder = p.slice(0, at).join('/');
    if (!out.has(folder)) out.set(folder, { folder, rows: [] });
    out.get(folder).rows.push(Object.assign({}, r, { rest: p.slice(at).join('/') }));
  }
  return [...out.values()];
}

/* The sentence over the coverage lists. */
function jobsCoverageLine(cov) {
  const unread = (cov.unread || []).length;
  const covered = cov.total - cov.never.length - cov.unscheduled.length - unread;
  const of = `${covered} of ${cov.total} models, seeds and snapshots run in a scheduled job`;
  const rest = [];
  if (unread) rest.push(`${unread} only in jobs whose schedule is not read`);
  if (cov.unscheduled.length) rest.push(`${cov.unscheduled.length} only in unscheduled jobs`);
  if (cov.never.length) rest.push(`${cov.never.length} in no job`);
  return of + (rest.length ? '; ' + rest.join(', ') : '') + '.';
}

/* What PUT /api/jobs/sources takes from what the form holds. A key left at
   the proposed shape is not sent, so a source lists only what differs. */
function jobSourcesBody(forms, defaults) {
  return {
    sources: forms.map((f) => {
      const keys = {};
      if (f.kind === 'airflow') {
        for (const [k, d] of defaults) {
          const v = String((f.keys || {})[k] || '').trim();
          if (v && v !== d) keys[k] = v;
        }
      }
      return { kind: f.kind, path: String(f.path || '').trim(), label: String(f.label || '').trim(), keys };
    }),
  };
}

function jobsPlural(n, word) {
  return `${n} ${word}${n === 1 ? '' : 's'}`;
}

/* What one source read, counted in its own words: DAGs and tasks, or jobs
   and steps. */
function jobsReadCounts(s) {
  const word = s.kind === 'airflow' ? ['DAG', 'task'] : ['job', 'step'];
  return `${jobsPlural(s.jobs, word[0])}, ${jobsPlural(s.steps, word[1])}`;
}

/* Whether a save read every source with nothing left to correct: each one
   found jobs, none failed, had a file refused or stopped at a bound. */
function jobsReadClean(sources) {
  return sources.length > 0
    && sources.every((s) => !s.error && !(s.refused || []).length && !s.capped && s.jobs > 0);
}

/* Whether a save closes the sources form. Only a clean read that was kept
   does, and only while the form still holds what was sent: the answer is
   then in the coverage. Anything else stays open on what went wrong, which
   is the answer to whether the keys are right, or on the edits made while
   the read ran, which the answer does not describe. */
function jobsSaveCloses(answer, sent, now) {
  return !!answer.saved && jobsReadClean(answer.sources || []) && JSON.stringify(sent) === JSON.stringify(now);
}

/* A save's toast, with its kind, once the form is no longer there to say it. */
function jobsSaveToast(answer) {
  const sources = answer.sources || [];
  if (!sources.length) return [answer.saved ? 'Saved: no job source left.' : 'No job source left, until dbt-edith stops.', 'ok'];
  if (!jobsReadClean(sources)) {
    return [`${answer.saved ? 'Saved' : 'Kept until dbt-edith stops'}, but not every source read cleanly: Orchestration says why.`, 'err'];
  }
  const read = sources.map(jobsReadCounts).join('; ');
  return answer.saved ? [`Saved and read: ${read}.`, 'ok'] : [`Read, but kept only until dbt-edith stops: ${read}.`, 'err'];
}

/* A small mapping as YAML lines, each list item a mapping of its own. */
function jobsYaml(value, depth) {
  const pad = ' '.repeat(depth);
  const lines = [];
  for (const [k, v] of Object.entries(value)) {
    if (Array.isArray(v)) {
      lines.push(pad + k + ':');
      for (const item of v) {
        const inner = jobsYaml(item, depth + 4);
        if (inner.length) inner[0] = pad + '  - ' + inner[0].slice(depth + 4);
        lines.push(...inner);
      }
    } else if (v && typeof v === 'object') {
      lines.push(pad + k + ':', ...jobsYaml(v, depth + 2));
    } else {
      lines.push(pad + k + ': ' + v);
    }
  }
  return lines;
}

/* The file each format is expected to look like. An Airflow one is drawn with
   the keys this source names, so the example reads in the user's own words:
   `pipeline.name` puts the id under a `pipeline:` of its own. */
function jobsExample(kind, keys, defaults) {
  if (kind === 'dbt_cloud') {
    return [
      '# dbt-jobs-as-code, as dbt Labs publishes it',
      'jobs:',
      '  daily:',
      '    name: Daily build',
      '    execute_steps:',
      '      - dbt build --select tag:daily',
      '    triggers:',
      '      schedule: true',
      '    schedule:',
      '      cron: "0 6 * * *"',
    ].join('\n');
  }
  const named = (k) => {
    const own = keys && String(keys[k] || '').trim();
    if (own) return own;
    const d = (defaults || []).find((p) => p[0] === k);
    return d ? d[1] : k;
  };
  const put = (into, path, value) => {
    const parts = path.split('.');
    let at = into;
    for (const p of parts.slice(0, -1)) {
      if (!at[p] || typeof at[p] !== 'object' || Array.isArray(at[p])) at[p] = {};
      at = at[p];
    }
    at[parts[parts.length - 1]] = value;
  };
  const task = {};
  put(task, named('task_id'), 'build_sales');
  put(task, named('command'), 'dbt run --select tag:sales');
  put(task, named('depends_on'), '[stage_sales]');
  const sensor = {};
  put(sensor, named('sensor_id'), 'wait_crm');
  put(sensor, named('external_dag_id'), 'crm_daily');
  put(sensor, named('external_task_id'), 'crm_load');
  const doc = {};
  put(doc, named('dag_id'), 'sales_daily');
  put(doc, named('schedule'), '"0 6 * * *"');
  put(doc, named('tasks'), [task]);
  put(doc, named('sensors'), [sensor]);
  return '# one file per DAG\n' + jobsYaml(doc, 0).join('\n');
}

/* The node the Jobs tab is about: the focus, or the one a Catalog chip
   opened it on, which a single click on a box puts in the Catalog without
   moving the focus, for as long as the Catalog still shows it. */
const jobsNode = () => (S.jobsFor && S.node && S.node.id === S.jobsFor ? S.jobsFor : S.focus);

/* `look` asks the server to look at the job files first, and read them again
   if one moved: done when the tab is opened, never on a click inside it. Only
   the first request of a run carries it. A Catalog chip's step is taken here,
   so a run that ends early leaves none behind for a later one to light. */
async function loadJobs({ look = false } = {}) {
  const run = ++S.jobsRun;
  const body = $('#jobs-body');
  const reveal = S.jobsReveal;
  S.jobsReveal = null;
  // What the Catalog drew came from the read this may replace.
  if (look) forgetCatalogJobs();
  paintJobsHead();
  const id = jobsNode();
  const ask = (url) => {
    const asked = look ? url + (url.includes('?') ? '&' : '?') + 'look=true' : url;
    look = false;
    return api.get(asked);
  };
  // The proposed shape is the server's, so the example cannot drift from it.
  // With no node, whether any source is set decides what the tab offers.
  let summary = null;
  if (!S.jobsKeys || !id) {
    try { summary = await ask('/api/jobs'); }
    catch (e) { if (run === S.jobsRun) toast('jobs: ' + e.message, 'err'); return; }
    if (run !== S.jobsRun) return;
    S.jobsKeys = summary.airflow_keys;
  }
  if (!id) {
    if (!summary.sources.length) return jobsNoSource(body);
    body.replaceChildren(jel('div', 'compiled-empty', 'Select a model first, or open Reports for the coverage of the whole project.'));
    return;
  }
  let view;
  try { view = await ask('/api/jobs/node?id=' + encodeURIComponent(id)); }
  catch (e) { if (run === S.jobsRun) toast('jobs: ' + e.message, 'err'); return; }
  if (run === S.jobsRun) renderJobs(body, view, id, reveal);
}

const jel = (tag, className, textContent = '') => Object.assign(document.createElement(tag), { className, textContent });

/* How a job starts, as a pill that says scheduled, not read or manual in a
   word, so they can be told apart at a glance down a list of cards. */
const JOBS_WORD = { scheduled: 'Scheduled', unread: 'Not read', manual: 'Manual' };
const JOBS_UNREAD_WHY = 'May start on its own, on what no file read says: a templated schedule or one written as a mapping, '
  + 'an asset no file read updates, a job no file read defines. Counted neither as scheduled nor as unscheduled';

function jobsWhen(job) {
  const start = jobsStart(job);
  const pill = jel('span', 'jobwhen');
  pill.dataset.start = start;
  if (JOBS_WORD[start]) pill.append(jel('b', '', JOBS_WORD[start]));
  if (start !== 'manual') pill.append(jel('span', '', jobsScheduleText(job)));
  if (start === 'unread') pill.title = JOBS_UNREAD_WHY;
  return pill;
}

/* The head of the tab: the node it is about, the coverage of every model,
   which lives in Reports (0064), and the sources. */
function paintJobsHead() {
  const head = $('#jobs-head');
  const bar = jel('div', 'jobs-bar');
  const about = jel('span', 'jobs-about', (jobsNode() || '').split('.').pop());
  bar.append(about, jel('div', 'grow'));
  if (S.reports && S.reports.jobs) {
    const coverage = jel('button', 'btn sm', 'Coverage in Reports');
    coverage.title = 'Which models every job read runs, and which none does';
    coverage.addEventListener('click', () => openReports('jobs'));
    bar.append(coverage);
  }
  const sources = jel('button', 'btn sm', 'Sources…');
  sources.title = 'Where the jobs are read from, and how';
  sources.addEventListener('click', openJobSources);
  bar.append(sources);
  head.replaceChildren(bar);
}

/* No source yet: what the Jobs tab and the coverage in Reports read, and what
   they expect to find. */
function jobsNoSource(body) {
  const box = jel('div', 'compiled-empty jobs-none');
  box.append(
    jel('p', '', 'Which jobs run a model is read from your orchestrator\'s YAML, wherever it lives, '
      + 'often in another repository. Name the folder, and say which format it is in.'),
  );
  const add = jel('button', 'btn primary', 'Add a source');
  add.addEventListener('click', () => openJobSources({ add: true }));
  box.append(add);
  // Folded, so the one thing to do stays in the middle of the tab.
  const formats = jel('details', 'jobs-formats');
  formats.append(jel('summary', '', 'The two formats read'));
  const figs = jel('div', 'jobs-figs');
  for (const [title, kind] of [['Airflow, one file per DAG; every key can be renamed', 'airflow'], ['dbt Cloud, in dbt-jobs-as-code\'s schema', 'dbt_cloud']]) {
    const fig = jel('figure', 'jobs-example');
    fig.append(jel('figcaption', '', title), jel('pre', '', jobsExample(kind, {}, (S.jobsKeys || []))));
    figs.append(fig);
  }
  formats.append(figs);
  box.append(formats);
  body.replaceChildren(box);
}

/* One job and the steps of it that act on the node, as cards. */
function jobsCard(group, { reason = false } = {}) {
  const { job } = group;
  const card = jel('div', 'jobcard');
  card.dataset.kind = job.kind;
  const head = jel('div', 'jobhead');
  head.append(
    jel('span', 'jobkind', job.kind === 'airflow' ? 'Airflow' : 'dbt Cloud'),
    jel('b', 'jobname', jobsJobLabel(job)),
    jobsWhen(job),
    jel('span', 'grow'),
  );
  const where = jel('button', 'jobfile', `${job.source} · ${job.file}:${job.line}`);
  where.title = 'Outside the project, so it does not open here. Click to copy where it is.';
  where.addEventListener('click', () => copyText(`${job.file}:${job.line}`));
  head.append(where);
  card.append(head);
  for (const row of jobsJobLinks(job)) card.append(jel('div', 'joblink', row));
  for (const step of group.steps) {
    const line = jel('div', 'jobstep');
    line.dataset.key = jobsStepKey(job, step);
    line.dataset.step = step.index;
    line.append(jel('span', 'steplbl', jobsStepLabel(job, step)));
    line.append(jel('code', 'stepcmd', step.shown));
    if (step.ls) {
      const ls = jel('button', 'btn sm', 'dbt ls');
      ls.title = 'Type the dbt ls that should list what this step acts on into the terminal, without running it';
      ls.addEventListener('click', () => sendToTerminal(step.ls));
      line.append(ls);
    }
    card.append(line);
    for (const row of reason ? [] : jobsLinkRows(step)) card.append(jel('div', 'joblink', row));
    for (const row of reason ? [step.unresolved] : step.warnings || []) card.append(jel('div', 'joblink why', row));
  }
  return card;
}

function renderJobs(body, view, id, reveal) {
  if (!view.configured) return jobsNoSource(body);
  const out = [];
  if (view.refused) {
    out.push(jel('div', 'jobs-warn', `${view.refused} file${view.refused === 1 ? '' : 's'} could not be read and may hold more: Sources… says which.`));
  }
  if (!view.groups.length) {
    out.push(jel('div', 'compiled-empty', `No step of the jobs read acts on ${id.split('.').pop()}.`));
  }
  for (const g of jobsByStart(view.groups)) out.push(jobsCard(g));
  if (view.maybe && view.maybe.length) {
    const n = jobsMaybeCount(view.maybe);
    const more = jel('details', 'jobs-maybe');
    more.append(jel('summary', '', `Not resolved, and could act on it (${n})`));
    for (const g of jobsByStart(view.maybe)) more.append(jobsCard(g, { reason: true }));
    out.push(more);
  }
  body.replaceChildren(...out);
  revealJobStep(body, id, reveal);
}

/* The step a Catalog chip was clicked for, scrolled to and lit once, or the
   steps not resolved, unfolded, and only on the node it was asked for: a
   step runs under every node it selects. By its key, and by its index among
   steps of one key, which only two sources of one label give. Within the
   tab's own scroll alone: scrollIntoView would also move the page's hidden
   overflows. */
function revealJobStep(body, id, want) {
  if (!want || want.id !== id) return;
  const maybe = want.maybe && body.querySelector(':scope > .jobs-maybe');
  if (maybe) maybe.open = true;
  const lines = [...body.querySelectorAll(':scope > .jobcard .jobstep')].filter((l) => l.dataset.key === want.key);
  const line = maybe || lines.find((l) => l.dataset.step === String(want.index)) || lines[0];
  if (!line) return;
  const at = maybe || line.closest('.jobcard');
  body.scrollTop += at.getBoundingClientRect().top - body.getBoundingClientRect().top - 8;
  if (!maybe) line.classList.add('jobs-hit');
}

/* One section of the coverage view, folders inside, open when short. */
function jobsSection(title, groups, count, open, hint = '') {
  const sec = jel('details', 'jobs-sec');
  sec.open = open;
  const head = jel('summary', '', `${title} (${count})`);
  if (hint) head.title = hint;
  sec.append(head);
  for (const g of groups) {
    // A folder of one is no level to unfold: the row is listed bare, its
    // whole folder beside it.
    if (g.rows.length === 1 || !g.folder) {
      for (const r of g.rows) sec.append(jobsRow(r, r.folder));
      continue;
    }
    const folder = jel('details', 'jobs-folder');
    folder.open = open;
    folder.append(jel('summary', '', `${g.folder} (${g.rows.length})`));
    for (const r of g.rows) folder.append(jobsRow(r, r.rest));
    sec.append(folder);
  }
  return sec;
}

function jobsRow(r, path) {
  const row = jel('button', 'jobs-row');
  row.append(dot(r.kind), jel('span', 'jobs-name', r.name));
  if (path) row.append(jel('span', 'jobs-path', path));
  row.title = `Open ${r.name}, with the Jobs tab listing its jobs`;
  row.addEventListener('click', () => openFromReport(r.id, { dock: 'jobs' }));
  return row;
}

function jobsStepList(title, refs) {
  const sec = jel('details', 'jobs-sec');
  sec.open = refs.length <= 20;
  sec.append(jel('summary', '', `${title} (${refs.length})`));
  for (const r of refs) {
    const row = jel('div', 'jobs-ref');
    row.append(jel('b', '', `${r.job} › ${r.name || 'step ' + r.n}`), jel('code', 'stepcmd', r.shown),
      jel('span', 'jobfile', `${r.source} · ${r.file}:${r.line}`));
    sec.append(row, jel('div', 'joblink why', r.why));
  }
  return sec;
}

function renderCoverage(body, cov) {
  if (!cov.configured) return jobsNoSource(body);
  const out = [jel('div', 'jobs-line', jobsCoverageLine(cov))];
  const short = (n) => n <= 50;
  out.push(jobsSection('Run by no job', jobsCoverageGroups(cov.never), cov.never.length, short(cov.never.length)));
  out.push(jobsSection('Only in unscheduled jobs', jobsCoverageGroups(cov.unscheduled), cov.unscheduled.length, short(cov.unscheduled.length),
    'In jobs, but only ones nothing starts on its own: no schedule, @once, CI or merge, or an asset or a job that never leads back to a clock'));
  const unread = cov.unread || [];
  if (unread.length) {
    out.push(jobsSection('Only in jobs whose schedule is not read', jobsCoverageGroups(unread), unread.length, short(unread.length), JOBS_UNREAD_WHY));
  }
  if (cov.ephemeral.length) {
    out.push(jobsSection('Ephemeral, so never run on their own', jobsCoverageGroups(cov.ephemeral), cov.ephemeral.length, false));
  }
  out.push(jobsStepList('Steps that select nothing they act on', cov.empty));
  out.push(jobsStepList('Steps not resolved', cov.unresolved));
  const jobs = jel('details', 'jobs-sec');
  jobs.append(jel('summary', '', `Jobs read (${cov.jobs.length})`));
  for (const j of jobsByStart(cov.jobs, (j) => j)) {
    const row = jel('div', 'jobs-ref');
    row.append(jel('b', '', jobsJobLabel(j)), jobsWhen(j),
      jel('span', 'jobfile', `${j.source} · ${j.file}:${j.line}`));
    jobs.append(row);
  }
  out.push(jobs);
  body.replaceChildren(...out);
}

/* What one source read, under its form once saved. */
function jobsReadNote(s) {
  if (s.error) return jel('div', 'jobs-warn', s.error);
  const box = jel('div', 'jobs-read');
  box.append(jel('div', '', `${jobsPlural(s.files, 'file')} read: ${jobsReadCounts(s)}`
    + (s.skipped ? `; ${s.skipped} without one` : '') + (s.capped ? '; stopped at 5 000 files' : '') + '.'));
  for (const r of s.refused || []) {
    box.append(jel('div', 'jobs-warn', `${r.file}${r.line ? ':' + r.line : ''}: ${r.message}`));
  }
  return box;
}

/* The folder picker under a source's path. A page is never told where a file
   it is handed sits, so the server lists one folder at a time, with what the
   walk would read there (0070). A click opens a folder or picks a file; Use
   writes the place the way the server spelled it, from the project when it
   is near. What it shows goes to `keep`, and comes back as `kept` to draw it
   again without asking the server. */
function browseJobSource(box, from, { use, cancel, kept, keep }) {
  let at = null;
  let picked = '';
  let asked = '';
  let ask = 0; // the last listing asked for: an answer to an earlier one is dropped
  let climbing = ''; // where ↑ goes, while its listing is on its way
  let first = null; // the row a double click began on
  const up = jel('button', 'btn sm', '↑');
  up.title = 'The folder above';
  up.setAttribute('aria-label', 'The folder above');
  up.disabled = true;
  const where = jel('input', 'grow');
  where.type = 'text';
  where.spellcheck = false;
  where.title = 'A folder, relative to the project or absolute: Enter goes there';
  const note = jel('p', 'jobsrc-note', '');
  const list = jel('div', 'jobsrc-entries');
  const as = jel('span', 'jobsrc-as', '');
  const useBtn = jel('button', 'btn sm primary', 'Use this folder');
  const cancelBtn = jel('button', 'btn sm', 'Cancel');
  const choice = () => (picked && at ? at.entries.find((e) => e.name === picked) : null);
  const paintChoice = () => {
    const file = choice();
    useBtn.textContent = file ? 'Use this file' : 'Use this folder';
    useBtn.disabled = !at;
    const written = file ? file.written : at && at.written;
    as.textContent = !at ? '' : written === '.' ? 'Saved as . (the project)' : 'Saved as ' + written;
    for (const r of list.children) r.classList.toggle('sel', !!picked && r.dataset.name === picked);
    if (keep) keep({ at, picked, asked });
  };
  const show = (next, path) => {
    at = next;
    asked = path;
    where.value = at.path;
    where.scrollLeft = where.scrollWidth; // a long path ends with the folder shown
    up.disabled = !at.up;
    note.textContent = [
      at.moved ? `${path} is not a folder this picker lists, so this is the nearest one above it.` : '',
      at.refused ? `This folder cannot be listed: ${at.refused}.` : '',
      at.capped ? `Only the first ${at.entries.length} entries are shown.` : '',
    ].filter(Boolean).join(' ');
    list.replaceChildren(...at.entries.map(row));
    if (!at.entries.length && !at.refused) list.append(jel('div', 'jobsrc-none', 'No folder and no YAML file here.'));
    paintChoice();
    list.scrollTop = 0;
    const sel = list.querySelector('.row.sel');
    if (sel) sel.scrollIntoView({ block: 'nearest' });
  };
  const go = async (path, climb = false) => {
    const mine = ++ask;
    climbing = climb ? path : '';
    let next;
    let failed = null;
    try { next = await api.get('/api/jobs/browse?path=' + encodeURIComponent(path)); }
    catch (e) { failed = e; }
    // A later listing was asked for, or this picker was closed, meanwhile.
    if (mine !== ask || !list.isConnected) return;
    climbing = '';
    if (failed) { note.textContent = failed.message; return; }
    picked = next.picked || '';
    show(next, path);
  };
  // Rows as the explorer draws them: a folder by its chevron, a file by its icon.
  const row = (e) => {
    const r = jel('div', 'row ' + (e.dir ? 'folder' : 'file'));
    r.tabIndex = 0;
    r.dataset.name = e.name;
    r.title = e.written;
    const caret = jel('span', 'caret');
    if (e.dir) caret.innerHTML = CHEVRON;
    r.append(caret);
    if (!e.dir) r.append(fileIcon(e.name));
    r.append(jel('span', 'nm', e.name));
    const act = (twice) => {
      if (e.dir) return go(e.written);
      if (twice) return use(e.written);
      picked = picked === e.name ? '' : e.name;
      paintChoice();
    };
    // The second click of a double click can land on a row of the folder the
    // first one opened, so it counts only on the row the first one hit.
    r.addEventListener('click', (ev) => { if (ev.detail > 1) return; first = r; act(false); });
    r.addEventListener('dblclick', () => { if (!e.dir && first === r) use(e.written); });
    r.addEventListener('keydown', (ev) => {
      if (ev.key !== 'Enter') return;
      ev.preventDefault();
      act(!e.dir && picked === e.name);
    });
    return r;
  };
  // Clicked again before the folder above is listed: one more up from there.
  up.addEventListener('click', () => {
    const to = climbing ? climbing + '/..' : at && at.up;
    if (to) go(to, true);
  });
  // Typing a place drops a listing on its way, which would overwrite it.
  where.addEventListener('input', () => { ask++; climbing = ''; });
  where.addEventListener('keydown', (ev) => { if (ev.key === 'Enter') { ev.preventDefault(); go(where.value); } });
  useBtn.addEventListener('click', () => { const file = choice(); if (at) use(file ? file.written : at.written); });
  cancelBtn.addEventListener('click', cancel);
  // Escape leaves the picker, not the form around it.
  box.addEventListener('keydown', (ev) => { if (ev.key === 'Escape') { ev.stopPropagation(); cancel(); } });
  const bar = jel('div', 'jobsrc-browse-bar');
  bar.append(up, where);
  const foot = jel('div', 'jobsrc-browse-foot');
  foot.append(as, cancelBtn, useBtn);
  box.replaceChildren(bar, note, list, foot);
  if (kept && kept.at) {
    picked = kept.picked;
    show(kept.at, kept.asked);
    return;
  }
  paintChoice();
  go(from || '');
}

/* The sources, edited in a modal that stays open while a read leaves
   something to correct: saving reads them at once, and what was read is the
   answer to whether the keys are right. A clean read closes it on the
   coverage (jobsSaveCloses), since a form that only repaints the same line
   looked like a save that did nothing. `add` opens it on a new source, its
   picker open, for the buttons that say so. */
async function openJobSources({ add = false } = {}) {
  closePopup();
  let summary;
  try { summary = await api.get('/api/jobs?look=true'); }
  catch (e) { return toast('jobs: ' + e.message, 'err'); }
  forgetCatalogJobs();
  const defaults = summary.airflow_keys || [];
  S.jobsKeys = defaults;
  const forms = summary.sources.map((s) => ({ kind: s.kind, path: s.path, label: s.label || '', keys: Object.assign({}, s.keys) }));
  // A source being added has its picker open, and what the picker shows is
  // kept, so that drawing the cards again leaves it as it was without asking
  // the server again.
  const fresh = (kind) => ({ kind, path: '', label: '', keys: {}, browsing: true, browsed: null });
  if (add === true) forms.push(fresh('airflow'));
  let reads = summary.sources;
  const body = jel('div', 'jobsrc');
  const list = jel('div', 'jobsrc-list');
  const foot = jel('div', 'jobsrc-foot');
  const saved = jel('p', 'muted', '');
  let closeForm = () => {};
  let open = true;

  const input = (value, placeholder, onInput, cls = '') => {
    const i = jel('input', cls);
    i.type = 'text';
    i.spellcheck = false;
    i.value = value || '';
    i.placeholder = placeholder;
    i.addEventListener('input', () => onInput(i.value));
    return i;
  };
  const paint = () => {
    list.replaceChildren();
    forms.forEach((f, i) => {
      const card = jel('div', 'jobsrc-card');
      const top = jel('div', 'jobsrc-top');
      const kind = jel('select', '');
      for (const [v, label] of [['airflow', 'Airflow'], ['dbt_cloud', 'dbt Cloud']]) {
        kind.append(Object.assign(document.createElement('option'), { value: v, textContent: label, selected: f.kind === v }));
      }
      kind.addEventListener('change', () => { f.kind = kind.value; paint(); });
      const remove = jel('button', 'btn sm', 'Remove');
      remove.addEventListener('click', () => { forms.splice(i, 1); reads = []; paint(); });
      const path = input(f.path, '../orchestration/dags, or an absolute path', (v) => { f.path = v; }, 'grow');
      const browse = jel('button', 'btn sm', 'Browse…');
      browse.title = 'Pick the folder or the file from a list';
      let picker = null;
      const closePicker = () => {
        f.browsing = false;
        f.browsed = null;
        if (picker) picker.remove();
        picker = null;
        browse.classList.remove('on');
        browse.focus();
      };
      const openPicker = () => {
        f.browsing = true;
        browse.classList.add('on');
        // A picker of its own each time, so nothing of the last one is left.
        picker = jel('div', 'jobsrc-browse');
        top.after(picker);
        browseJobSource(picker, f.path, {
          kept: f.browsed,
          keep: (k) => { f.browsed = k; },
          use: (w) => { f.path = w; path.value = w; closePicker(); },
          cancel: closePicker,
        });
      };
      browse.addEventListener('click', () => (f.browsing ? closePicker() : openPicker()));
      top.append(kind, path, browse, input(f.label, 'label', (v) => { f.label = v; }, 'jobsrc-label'), remove);
      card.append(top);
      if (f.browsing) openPicker();
      const example = jel('pre', 'jobs-example-text', jobsExample(f.kind, f.keys, defaults));
      if (f.kind === 'airflow') {
        const grid = jel('div', 'jobsrc-keys');
        for (const [k, d] of defaults) {
          grid.append(jel('label', '', k), input(f.keys[k], d, (v) => {
            f.keys[k] = v;
            example.textContent = jobsExample(f.kind, f.keys, defaults);
          }));
        }
        const cols = jel('div', 'jobsrc-cols');
        cols.append(grid, example);
        card.append(cols);
      } else {
        card.append(example);
      }
      if (reads[i]) card.append(jobsReadNote(reads[i]));
      list.append(card);
    });
    if (!forms.length) list.append(jel('p', 'muted', 'No source yet.'));
  };

  const addAirflow = jel('button', 'btn sm', '+ Airflow');
  addAirflow.addEventListener('click', () => { forms.push(fresh('airflow')); paint(); });
  const addCloud = jel('button', 'btn sm', '+ dbt Cloud');
  addCloud.addEventListener('click', () => { forms.push(fresh('dbt_cloud')); paint(); });
  const save = jel('button', 'btn sm primary', 'Save and read');
  save.addEventListener('click', async () => {
    save.disabled = true;
    const sent = jobSourcesBody(forms, defaults);
    try {
      const answer = await api.send('/api/jobs/sources', 'PUT', sent);
      reads = answer.sources;
      forgetCatalogJobs();
      // Closed while it read: a toast is all that is left to say what it did.
      if (!open) {
        toast(...jobsSaveToast(answer));
        if (S.dock === 'jobs') loadJobs();
        if (S.reportsOpen && S.reportsSection === 'jobs') return showReportSection('jobs');
        return loadReportsIndex();
      }
      if (jobsSaveCloses(answer, sent, jobSourcesBody(forms, defaults))) return closeForm(answer);
      saved.textContent = answer.saved
        ? 'Saved for this project, outside it. Only YAML files are read, never through a symlink.'
        : 'No config directory is available, so these sources last until dbt-edith stops.';
      paint();
      if (S.dock === 'jobs') loadJobs();
      loadReportsIndex();
    } catch (e) {
      toast('sources: ' + e.message, 'err');
    } finally {
      save.disabled = false;
    }
  });
  foot.append(addAirflow, addCloud, jel('div', 'grow'), save);
  body.append(
    jel('p', 'muted', 'Folders or files holding your jobs, relative to the project or absolute. '
      + 'An Airflow key is a dotted path: pipeline.name is the key name inside pipeline. Leave one empty for the shape shown.'),
    list, foot, saved);
  paint();
  const answer = await modal({ title: 'Orchestration', body, actions: [{ id: null, label: 'Close' }], wide: true,
    onOpen: (close) => { closeForm = close; } });
  open = false;
  if (S.dock === 'jobs') loadJobs();
  if (answer) {
    toast(...jobsSaveToast(answer));
    // The Jobs tab already answers for the model it shows, and Reports would
    // roll down over it; anywhere else the answer is the coverage.
    if (S.dock === 'jobs' && jobsNode() && !S.reportsOpen) return loadReportsIndex();
    return openReports('jobs');
  }
  if (S.reportsOpen && S.reportsSection === 'jobs') showReportSection('jobs');
}

// ---------------------------------------------------------------- reports --
/* Reports on the whole project, in one panel rolled down over the editor and
   the dock from the top bar (0064). The pure part runs from reportsSections to
   loadReportsIndex, which web/tests/reports.js slices. */

/* The sections there are, in the order they are worked through, each only
   while it has something to show: collin's report once a run wrote one, the
   coverage of the jobs once a source is named. Before that the coverage is
   listed all the same, `empty`, as the place a source is added from (0070),
   but it is not opened on for itself, and the button stays dimmed (0075). */
function reportsSections(index) {
  const all = [
    { key: 'run', label: 'Last run', tool: 'dbt',
      title: 'What the last dbt command ran and how each node ended, from target/run_results.json' },
    { key: 'collin', label: "collin's report", tool: 'collin',
      title: 'What collin found reading the compiled SQL, from target/collin.report.json' },
    { key: 'jobs', label: 'Job coverage', tool: 'jobs',
      title: "Which models the orchestrator's jobs run, read from the sources the Jobs tab names" },
  ];
  return all.filter((sec) => sec.key === 'jobs' || !!(index && index[sec.key]))
    .map((sec) => (index && index[sec.key] ? sec : Object.assign(sec, { empty: true })));
}

/* What is new in the reports since each was last shown, in the sections'
   order: what runBadge says of the last run, collinBadge of collin's,
   jobsBadge of the coverage. The button carries it, and nothing opens by
   itself (0059). */
function reportsNews(collin, run, jobs) {
  const news = [];
  if (run) news.push({ key: 'run', tone: 'bad', text: run.text, short: run.short, title: run.title });
  if (collin) news.push({ key: 'collin', tool: 'collin', text: collin.text, short: collin.short, title: collin.title });
  if (jobs) news.push({ key: 'jobs', tone: 'warn', text: jobs.text, short: jobs.short, title: jobs.title });
  return news;
}

/* What the chips on the button say: a report's words when it is the only one
   with news, its count alone when several are, so the button stays short;
   the tooltip has the words either way. */
function reportsChipTexts(news) {
  return (news || []).map((n) => (news.length > 1 ? n.short : n.text));
}

/* How many more models, seeds and snapshots no scheduled job runs than when
   the coverage was last shown (0066). Fewer is no news, and the page lowers
   what it last saw to match, so that a later rise counts from there. */
function jobsBadge(jobs, seen) {
  if (!jobs || typeof jobs.never !== 'number' || typeof seen !== 'number') return null;
  const now = jobs.never + (jobs.unscheduled || 0);
  if (now <= seen) return null;
  const n = now - seen;
  return { text: `${n} more unscheduled`, short: `+${n}`,
    title: `${n} more model${n > 1 ? 's' : ''}, seed${n > 1 ? 's' : ''} or snapshot${n > 1 ? 's' : ''} than when you last looked `
      + `${n > 1 ? 'are' : 'is'} run by no scheduled job: ${jobs.never} in no job, ${jobs.unscheduled || 0} only in unscheduled jobs.` };
}

/* What the last run leaves on the button until its section is shown: its
   errors and failed tests, only for a file written since the page last
   looked, as collin's badge (0059). Warnings are for the section to say. */
function runBadge(run, seen) {
  if (!run || !(run.error || run.fail)) return null;
  if ((run.at || 0) <= (seen || 0)) return null;
  const errors = run.error ? `${run.error} error${run.error > 1 ? 's' : ''}` : '';
  const what = run.command ? `dbt ${run.command}` : 'The last dbt command';
  const said = [errors, run.fail ? `${run.fail} failed test${run.fail > 1 ? 's' : ''}` : ''].filter(Boolean);
  return { text: [errors, run.fail ? `${run.fail} failed` : ''].filter(Boolean).join(', '),
    short: String((run.error || 0) + (run.fail || 0)), title: `${what} ended with ${said.join(' and ')}.` };
}

/* A dbt outcome as the section says it. One it does not know is shown as
   dbt wrote it. */
function runStatusWord(status) {
  return { ok: 'ok', error: 'error', fail: 'failed', warn: 'warned', skipped: 'skipped', partial: 'partly done',
    noop: 'nothing to do', reused: 'reused' }[status] || status;
}

/* The order outcomes are listed in: what to look at first. */
function runStatusRank(status) {
  const at = ['error', 'fail', 'warn', 'partial', 'skipped', 'ok', 'noop', 'reused'].indexOf(status);
  return at < 0 ? 99 : at;
}

function runSecs(s) {
  if (typeof s !== 'number' || !isFinite(s)) return '';
  if (s < 1) return `${s.toFixed(2)} s`;
  if (s < 10) return `${s.toFixed(1)} s`;
  if (s < 60) return `${Math.round(s)} s`;
  return `${Math.floor(s / 60)} min ${Math.round(s % 60)} s`;
}

/* `model` as a heading: `Models`; `unit_test` as `unit tests`. */
function runKindPlural(kind, capital) {
  const words = String(kind || 'node').replace(/_/g, ' ') + 's';
  return capital ? words[0].toUpperCase() + words.slice(1) : words;
}

/* What the head says of the run: the command as dbt was given it, the
   selection only, then the dbt that ran it and how long it took. */
function runAbout(run) {
  const cmd = run.command ? ['dbt ' + run.command].concat(run.selection || []).join(' ') : 'a dbt command';
  // Fusion writes no time for a command that ran nothing, as a zero.
  return [cmd, run.dbt_version && `dbt ${run.dbt_version}`, run.elapsed > 0 && runSecs(run.elapsed)].filter(Boolean).join(' · ');
}

/* What a reader has to know before trusting the section, warnings first. A
   node the run did not reach is said in a count, never listed: absent from
   the file, it was not run, which says nothing about it. */
function runNotes(run) {
  const warn = [], dim = [];
  const plural = (n, one, many) => `${n} ${n === 1 ? one : many}`;
  const ran = (run.kinds || []).reduce((a, k) => a + k.ran, 0);
  if (run.older_than_manifest) {
    warn.push('The manifest was written after this run: a parse or another command since may have changed what the project holds.');
  }
  if (run.other_target) {
    warn.push(`${plural(run.other_target, 'result names a table', 'results name tables')} the manifest names otherwise: this run was likely for another target.`);
  }
  if (run.unknown && run.unknown.of) {
    warn.push(`${plural(run.unknown.of, 'result names a node', 'results name nodes')} the manifest no longer has.`);
  }
  // Commands that build nothing, and still write the file over the last run.
  if (['compile', 'show', 'docs generate', 'run-operation'].includes(run.command)) {
    dim.push(`dbt ${run.command} wrote this file over the one before, so what the last run did is no longer in it.`);
  }
  if (!ran) dim.push('It ran no node.');
  else {
    const short = (run.kinds || []).filter((k) => k.of > k.ran);
    if (short.length) {
      dim.push(`It ran ${short.map((k) => `${k.ran} of ${k.of} ${runKindPlural(k.kind)}`).join(', ')}: `
        + 'a node it did not reach is not in it, which says nothing about that node.');
    }
  }
  if (run.command === 'test') dim.push('dbt test runs tests alone: no model was built by it.');
  return warn.map((text) => ({ tone: 'warn', text })).concat(dim.map((text) => ({ tone: '', text })));
}

/* The figures over the section: a tile per kind that ran, then the time. */
function runTiles(run) {
  const tiles = (run.kinds || []).map((k) => {
    const st = k.statuses || {};
    const sub = Object.keys(st).sort((a, b) => runStatusRank(a) - runStatusRank(b))
      .map((s) => `${st[s]} ${runStatusWord(s)}`).join(' · ');
    const tone = st.error || st.fail ? 'bad' : st.warn || st.partial ? 'warn' : '';
    return { label: runKindPlural(k.kind, true), value: k.of ? `${k.ran} of ${k.of}` : String(k.ran), sub, tone };
  });
  if (run.elapsed > 0) tiles.push({ label: 'Took', value: runSecs(run.elapsed), sub: 'for the whole command', tone: '' });
  return tiles;
}

/* What follows a node's name in a list: how it ended, how many rows a test
   found, and how long it took. */
function runEntryText(e) {
  const bits = [runStatusWord(e.status)];
  if (typeof e.failures === 'number' && e.failures > 0) bits.push(`${e.failures} row${e.failures > 1 ? 's' : ''}`);
  if (e.secs) bits.push(runSecs(e.secs));
  return bits.join(' · ');
}

/* The section the panel opens on: one with news, then the one read last this
   session unless it is empty, then the first there is, which is empty only
   when every section is. */
function reportsDefaultSection(sections, news, last) {
  const has = (key) => sections.some((sec) => sec.key === key);
  const fresh = (news || []).find((n) => has(n.key));
  if (fresh) return fresh.key;
  if (last && sections.some((sec) => sec.key === last && !sec.empty)) return last;
  return sections.length ? sections[0].key : '';
}

/* The button's tooltip: what the reports hold, then what is new in them, or
   what will fill them while nothing does, since the button is dimmed then
   rather than hidden (0075). */
function reportsButtonTitle(sections, news, key) {
  const names = sections.filter((sec) => !sec.empty).map((sec) => sec.label).join(', ');
  const none = 'Nothing yet: the last dbt run fills one, from target/run_results.json, collin\'s report another, '
    + 'and the job coverage once the Jobs tab has a source, which is added here.';
  return [`Reports on the whole project${names ? ': ' + names : ''} (${key})`]
    .concat(names ? (news || []).map((n) => n.title) : [none]).join('\n');
}

/* Whether the file behind the section on screen changed after it was read:
   the head then says so, rather than the section redrawing under the reader
   (0059). A section whose report has gone is stale too. */
function reportsStale(shown, index, key) {
  if (!shown) return false;
  return JSON.stringify(shown) !== JSON.stringify((index && index[key]) || null);
}

async function loadReportsIndex() {
  try { S.reports = await api.get('/api/reports'); }
  catch { return; }   // what the button said still stands: a poll never interrupts
  paintReportsButton();
  if (S.dock === 'jobs') paintJobsHead();
  if (S.reportsOpen) paintReportsNav();
}

function newsChip(n) {
  const chip = jel('span', 'rnew', n.text);
  if (n.tool) chip.dataset.tool = n.tool;
  if (n.tone) chip.dataset.tone = n.tone;
  chip.title = n.title || '';
  return chip;
}

function reportsNow() {
  // A run finished before the page loaded is not news to anyone.
  if (S.collinSeen === undefined && S.collin) S.collinSeen = S.collin.run || 0;
  if (S.runSeen === undefined && S.reports) S.runSeen = S.reports.run ? S.reports.run.at : 0;
  const jobs = S.reports && S.reports.jobs;
  if (jobs && typeof jobs.never === 'number') {
    const now = jobs.never + (jobs.unscheduled || 0);
    if (S.jobsSeen === undefined || now < S.jobsSeen) S.jobsSeen = now;
  }
  const run = runBadge(S.reports && S.reports.run, S.runSeen);
  return { sections: reportsSections(S.reports),
    news: reportsNews(collinBadge(S.collin, S.collinSeen), run, jobsBadge(jobs, S.jobsSeen)) };
}

function paintReportsButton() {
  const btn = $('#reports-btn');
  const { sections, news } = reportsNow();
  // In view from the first paint, so the panel is found before it has
  // anything, and dimmed until a section does, with no chip until then:
  // news of a report the index does not list yet would say the opposite of
  // its tooltip (0075).
  const some = sections.some((sec) => !sec.empty);
  btn.classList.remove('hidden');
  btn.classList.toggle('dim', !some && !S.reportsOpen);
  if (!some) news.length = 0;
  btn.classList.toggle('on', S.reportsOpen);
  btn.setAttribute('aria-expanded', String(S.reportsOpen));
  btn.title = reportsButtonTitle(sections, news, keyLabel('Mod+Alt+R', IS_MAC));
  const texts = reportsChipTexts(news);
  btn.querySelector('.rnews').replaceChildren(...news.map((n, i) => newsChip(Object.assign({}, n, { text: texts[i] }))));
}

/* Rolls the panel down over the editor and the dock, on the section asked
   for or the one reportsDefaultSection picks. The index is read first, so a
   report written since the last poll is there to pick. */
async function openReports(section, { reread = true } = {}) {
  await loadReportsIndex();
  const { sections, news } = reportsNow();
  // The coverage is always listed, empty before any source, so there is
  // always a section to open on (0070).
  const key = sections.some((sec) => sec.key === section) ? section : reportsDefaultSection(sections, news, S.reportsSection);
  if (!S.reportsOpen) {
    closeHoverCard();
    closePopup();
    S.reportsReturn = document.activeElement;
    S.reportsOpen = true;
    // Nothing under the panel takes a key or a Tab while it is down: the
    // editor and the terminal lose the focus they had, and closeReports gives
    // it back.
    for (const id of ['#editor-pane', '#gutter-dock', '#dock']) $(id).inert = true;
    const panel = $('#reports');
    panel.classList.add('open');
    panel.setAttribute('aria-hidden', 'false');
    panel.focus({ preventScroll: true });
  }
  paintReportsButton();
  // Put back by the tour as it was left (0075): the section is still drawn,
  // so it is not read again, and its Read again says when its file changed.
  if (!reread && key === S.reportsDrawn) {
    S.reportsSection = key;
    paintReportsNav();
    return;
  }
  showReportSection(key);
}

/* Rolls the panel up. `restore` gives the focus back to where it was when the
   panel came down, for a close from the keyboard or from its own buttons; a
   file opened from it is where the reader goes next instead. */
function closeReports({ restore = false } = {}) {
  if (!S.reportsOpen) return;
  S.reportsOpen = false;
  const panel = $('#reports');
  panel.classList.remove('open');
  panel.setAttribute('aria-hidden', 'true');
  for (const id of ['#editor-pane', '#gutter-dock', '#dock']) $(id).inert = false;
  const back = S.reportsReturn;
  S.reportsReturn = null;
  if (panel.contains(document.activeElement)) document.activeElement.blur();
  if (restore && back && back.focus && document.body.contains(back) && !panel.contains(back)) back.focus();
  paintReportsButton();
  // The Catalog it covered is seen again, and asks for its jobs if it has not.
  if (S.dock === 'catalog') fillCatalogJobs($('#catalog .cat-jobs'));
}

function toggleReports() {
  if (S.reportsOpen) closeReports({ restore: true });
  else openReports();
}

function paintReportsNav() {
  const { sections, news } = reportsNow();
  $('#reports-nav').replaceChildren(...sections.map((sec) => {
    const row = jel('button', 'reports-navrow' + (sec.empty ? ' empty' : ''));
    row.type = 'button';
    row.title = sec.empty ? 'No source named yet: add the folders your jobs are in' : sec.title;
    if (sec.key === S.reportsSection) row.setAttribute('aria-current', 'true');
    const dot = jel('span', 'tooldot');
    dot.dataset.tool = sec.tool;
    row.append(dot, jel('span', '', sec.label));
    if (sec.empty) row.append(jel('span', 'rhint', 'add a source'));
    // The rail is narrow: the count alone, the words being on the button.
    const n = news.find((x) => x.key === sec.key);
    if (n) row.append(newsChip(Object.assign({}, n, { text: n.short })));
    row.addEventListener('click', () => showReportSection(sec.key));
    return row;
  }));
  paintReportStale();
}

function paintReportStale() {
  const again = $('#reports-view .reports-again');
  if (!again) return;
  const stale = reportsStale(S.reportsShown, S.reports, S.reportsSection);
  again.classList.toggle('stale', stale);
  again.textContent = stale ? 'Newer results: read again' : 'Read again';
}

/* The head every section shares: what it is, what it was read from, and its
   buttons, Read again saying when the file has changed since. */
function reportHead({ title, tool, about, again, extra = [] }) {
  const head = jel('div', 'diff-head');
  const row = jel('div', 'diff-title');
  const dot = jel('span', 'tooldot');
  dot.dataset.tool = tool;
  row.append(dot, jel('b', '', title), jel('span', 'diff-state', about || ''), jel('div', 'grow'), ...extra);
  const read = jel('button', 'btn sm reports-again', 'Read again');
  read.addEventListener('click', again);
  const close = jel('button', 'icon', '×');
  close.title = `Close the reports (${keyLabel('Escape', IS_MAC)})`;
  close.setAttribute('aria-label', 'Close the reports');
  close.addEventListener('click', () => closeReports({ restore: true }));
  row.append(read, close);
  head.append(row);
  return head;
}

/* A fold of a section, known by its summary and those of the folds around
   it, without the count a run changes. */
function reportFoldKey(d) {
  const names = [];
  for (let at = d; at; at = at.parentElement ? at.parentElement.closest('details') : null) {
    const sum = at.querySelector(':scope > summary');
    const title = sum && (sum.querySelector('.rtitle') || sum);
    names.unshift(title ? title.textContent.replace(/ \(\d+\)$/, '') : '');
  }
  return names.join(' › ');
}

/* Which folds of the section on screen are open, and where its list is
   scrolled: a list of hundreds is gone through one model at a time, each
   opened from it, and coming back must find it as it was left. */
function keepReportPlace(view) {
  const scroller = view.querySelector('.reportbody, .jobs-body');
  if (!scroller) return null;
  const open = {};
  for (const d of scroller.querySelectorAll('details')) open[reportFoldKey(d)] = d.open;
  return { open, scroll: scroller.scrollTop };
}

function restoreReportPlace(view, place) {
  const scroller = view.querySelector('.reportbody, .jobs-body');
  if (!place || !scroller) return;
  for (const d of scroller.querySelectorAll('details')) {
    const key = reportFoldKey(d);
    if (key in place.open) d.open = place.open[key];
  }
  scroller.scrollTop = place.scroll;
}

/* Draws one section, read again on every showing so a run since the last
   one shows, with the folds and the scroll it was left with: every showing
   but the tour's putting back of a panel it rolled up (0075). Only the
   latest of two quick clicks is drawn. */
async function showReportSection(key) {
  const view = $('#reports-view');
  // Kept before anything replaces it, the placeholder included.
  if (S.reportsDrawn) S.reportsPlace[S.reportsDrawn] = keepReportPlace(view) || S.reportsPlace[S.reportsDrawn];
  S.reportsSection = key;
  paintReportsNav();
  const ask = ++S.reportsAsk;
  if (key !== S.reportsDrawn) {
    view.replaceChildren(jel('div', 'compiled-empty', 'Reading…'));
    S.reportsDrawn = '';
  }
  const shown = (S.reports && S.reports[key]) || null;
  const again = () => showReportSection(key);
  const drawn = key === 'run' ? await runSection(again)
    : key === 'collin' ? await collinSection(again) : await jobsCoverageSection(again);
  if (ask !== S.reportsAsk) return;
  S.reportsShown = shown;
  view.replaceChildren(...drawn);
  S.reportsDrawn = key;
  restoreReportPlace(view, S.reportsPlace[key]);
  if (key === 'collin' && S.collin) S.collinSeen = S.collin.run || 0;
  if (key === 'run' && shown) S.runSeen = shown.at;
  // The coverage is counted by the index only once the jobs were read, which
  // this section has just done: read again, so the counts it shows are seen.
  if (key === 'jobs') {
    await loadReportsIndex();
    if (ask !== S.reportsAsk) return;
    const jobs = S.reports && S.reports.jobs;
    S.reportsShown = jobs || null;
    if (jobs && typeof jobs.never === 'number') S.jobsSeen = jobs.never + (jobs.unscheduled || 0);
  }
  paintReportsButton();
  paintReportsNav();
}

async function runSection(again) {
  let run;
  try { run = await api.get('/api/reports/run'); }
  catch (e) { return [reportHead({ title: 'Last run', tool: 'dbt', about: '', again }), jel('div', 'compiled-empty', e.message)]; }
  const about = [runAbout(run), run.at ? `written ${cacheAge(run.at)}` : ''].filter(Boolean).join(' · ');
  const body = jel('div', 'reportbody');
  const tiles = jel('div', 'reporttiles');
  for (const t of runTiles(run)) {
    const tile = jel('div', 'reporttile');
    if (t.tone) tile.dataset.tone = t.tone;
    tile.append(jel('div', 'rlabel', t.label), jel('div', 'rvalue', t.value), jel('div', 'rsub', t.sub));
    tiles.append(tile);
  }
  body.append(tiles);
  for (const n of runNotes(run)) {
    const p = jel('p', 'runnote', n.text);
    if (n.tone) p.dataset.tone = n.tone;
    body.append(p);
  }
  const group = (title, list, open, tone) => {
    if (!list.of) return;
    const d = jel('details', 'reportfinding');
    d.open = open;
    if (tone) d.dataset.tone = tone;
    const sum = jel('summary', '');
    sum.append(jel('span', 'rtitle', title), jel('span', 'rcount', String(list.of)));
    const ul = jel('ul', 'rentries');
    for (const e of list.items) ul.append(runEntryRow(e));
    if (list.of > list.items.length) ul.append(jel('li', 'muted', `and ${list.of - list.items.length} more`));
    d.append(sum, ul);
    body.append(d);
  };
  group('Errors and failed tests', run.problems, true, 'bad');
  group('Warnings', run.warned, run.warned.of <= 20, 'warn');
  group('Skipped', run.skipped, run.skipped.of <= 20, '');
  group('Slowest', { items: run.slowest, of: run.slowest.length }, false, '');
  if (run.unknown.of) {
    const d = jel('details', 'reportfinding');
    const sum = jel('summary', '');
    sum.append(jel('span', 'rtitle', 'Not in the manifest'), jel('span', 'rcount', String(run.unknown.of)));
    const ul = jel('ul', 'rentries');
    for (const id of run.unknown.items) ul.append(jel('li', 'rrel', id));
    if (run.unknown.of > run.unknown.items.length) ul.append(jel('li', 'muted', `and ${run.unknown.of - run.unknown.items.length} more`));
    d.append(sum, ul);
    body.append(d);
  }
  return [reportHead({ title: 'Last run', tool: 'dbt', about, again }), body];
}

/* One node of the run: its name opens it, then how it ended, then dbt's
   message under it. */
function runEntryRow(e) {
  const li = jel('li', '');
  const known = !!e.file;
  const name = jel(known ? 'a' : 'span', known ? 'filelink' : 'rrel', e.name);
  name.title = e.id;
  if (known) name.addEventListener('click', (ev) => { ev.preventDefault(); openRunEntry(e); });
  li.append(name, jel('span', 'muted', ' · ' + runEntryText(e)));
  if (e.message) li.append(jel('div', 'rmsg', e.message));
  return li;
}

/* A test opens at its line in its YAML (0041); a model with the SQL dbt ran
   for it, which is where an error is read. */
function openRunEntry(e) {
  if (e.kind === 'test') {
    closeReports();
    openTest(e, { pin: true });
    return;
  }
  // A unit test is no node of the graph: its YAML is all there is to open.
  if (e.kind === 'unit_test') {
    closeReports();
    pushJump();
    openFile(e.file);
    revealInTree(e.file);
    return;
  }
  openFromReport(e.id, { dock: e.kind === 'model' || e.kind === 'snapshot' ? 'run' : '' });
}

async function collinSection(again) {
  const head = (about) => reportHead({ title: "collin's report", tool: 'collin', about, again });
  let view;
  try { view = await api.get('/api/collineage/report'); }
  catch (e) { return [head(''), jel('div', 'compiled-empty', e.message)]; }
  return [head(collinReportAbout(view)), collinReportBody(view)];
}

async function jobsCoverageSection(again) {
  const sources = jel('button', 'btn sm', 'Sources…');
  sources.title = 'Where the jobs are read from, and how';
  sources.addEventListener('click', openJobSources);
  const head = reportHead({ title: 'Job coverage', tool: 'jobs', about: 'every job read, against every model of the project',
    again, extra: [sources] });
  // The job files are looked at as the section opens, never on a click in it
  // (0063).
  let cov;
  try { cov = await api.get('/api/jobs/coverage?look=true'); }
  catch (e) { return [head, jel('div', 'compiled-empty', e.message)]; }
  forgetCatalogJobs();
  const body = jel('div', 'jobs-body');
  renderCoverage(body, cov);
  return [head, body];
}

/* A node named in a report, opened in the editor with the dock tab that says
   the most about it. The panel rolls up first: the file is what was asked
   for, and focusNode opens none for a node without one (0064). */
async function openFromReport(id, { dock = '', column = '' } = {}) {
  // The Catalog is about to draw this node: the one it holds asks for no jobs
  // as the panel rolls up or the tab comes forward.
  const stale = $('#catalog .cat-jobs');
  if (stale) stale.dataset.asked = '1';
  closeReports();
  pushJump();
  if (dock === 'catalog') {
    S.catTab = 'columns';
    highlightColumn(column || null, id);
    showDock('catalog');
  }
  await focusNode(id, { open: true });
  // Once the focus has moved, so the tab reads the node once, and without
  // looking at the job files again: the section looked as it opened (0063).
  if (dock === 'jobs' && S.dock !== 'jobs') showDock('jobs', { look: false });
  else if (dock && dock !== 'catalog' && dock !== 'jobs' && S.dock !== dock) showDock(dock);
}

// -------------------------------------------------------------- hover card --
/* One floating card, shared by the lineage boxes and the editor's marks. It is
   hoverable, so its text can be read and selected, but holds no focusable
   control: a surface that opens by accident should not also own a focus trap
   and an Escape contract the way the env menu has to. */
const HOVER_DELAY = 350;   // a sweep across a dense graph must open nothing
const HOVER_GRACE = 180;   // time for the pointer to cross the gap into the card
const HOVER_COLS = 8;      // the card never scrolls: the rest is counted, not listed

/* Where a box of size `box` goes next to `at`, inside `view`. Below when it
   fits, above when it does not, clamped rather than clipped when neither works.
   `view` is a parameter rather than a read of `window` so this can be checked
   without a DOM. */
function placeFloating(at, box, view, gap = 8) {
  const below = at.bottom + gap + box.height <= view.height;
  const above = at.top - gap - box.height >= 0;
  let top;
  if (below) top = at.bottom + gap;
  else if (above) top = at.top - gap - box.height;
  else top = Math.max(4, Math.min(at.bottom + gap, view.height - box.height - 4));
  // A box wider than the viewport pins to the left edge rather than going negative.
  const left = Math.max(4, Math.min(at.left, view.width - box.width - 4));
  return { top, left, above: !below && above };
}

/* The head every card shares. Returns the element so a filler can keep appending. */
/* What a test checks, for its hover card: the generic and where it applies,
   the package it was named through when dbt recorded one, and a warn
   severity. `level` is the canvas's word for the kind of test (0040). A bare
   generic may still live in a package, so without a namespace the card names
   none rather than claim the project's own. */
function testFacts(n, level) {
  if (!n || n.kind !== 'test') return [];
  const facts = [{
    text: level === 'column' ? `${n.test_name} on ${n.column}`
      : level === 'model' ? `${n.test_name} on the whole model`
        : 'A singular test: SQL of its own, no generic',
  }];
  if (n.namespace) facts.push({ text: `generic from ${n.namespace}` });
  if (n.warn) facts.push({ text: 'severity: warn', warn: true });
  return facts;
}

/* What an incremental model's strategy does, for its hover card: where it
   comes from and the key it works on, or, when the macros dbt runs insert
   every row for want of a key, why, in the warning colour and worded by the
   macro that inserts. Painted from the box and again from /api/node, which
   carry the same fields, so the line does not change under the pointer. */
const KEYED_STRATEGIES = ['merge', 'delete+insert'];
const INSERT_REASONS = {
  snowflake__get_merge_sql: 'so Snowflake runs a plain insert into',
  default__get_merge_sql: 'on FALSE, so every row is inserted',
  default__get_delete_insert_merge_sql: 'so nothing is deleted before the insert',
};
function strategyFacts(n, adapter) {
  if (!n || n.kind !== 'model' || n.materialized !== 'incremental' || n.disabled) return [];
  const name = Lineage.adapterName(adapter);
  if (!n.strategy_shown) {
    return [{ text: `${name}'s default strategy, which could not be read from its macros`, muted: true }];
  }
  const origin = n.strategy_default ? `, ${name}'s default` : '';
  if (n.strategy_insert) {
    const reason = INSERT_REASONS[n.strategy_insert] || 'so every row is inserted';
    return [{ text: `append in effect: ${n.strategy_named}${origin}${origin && ','} with no unique key, ${reason}`, warn: true }];
  }
  if (!KEYED_STRATEGIES.includes(n.strategy_shown)) return [{ text: n.strategy_shown + origin }];
  const key = n.unique_key || [];
  return [{ text: key.length ? `${n.strategy_shown} on ${key.join(', ')}${origin}` : `${n.strategy_shown}${origin}, no unique key` }];
}

/* A second line for HTML: the parts subtitleParts() gives, the struck word in
   an <s>. */
function subNodes(p) {
  if (!p.struck) return [[p.lead, p.rest].filter(Boolean).join('  ·  ')];
  return [`${p.lead} `, Object.assign(document.createElement('s'), { textContent: p.struck }), p.rest ? `  ·  ${p.rest}` : ''];
}

function hoverCardBody(el, { title, tone, sub, crumb }) {
  el.textContent = '';
  const head = document.createElement('div');
  head.className = 'hc-head';
  if (tone) head.appendChild(tone);
  head.append(Object.assign(document.createElement('span'), { className: 'hc-name', textContent: title }));
  el.appendChild(head);
  // A string, or the nodes subNodes() makes of a box's line.
  if (sub) {
    const line = Object.assign(document.createElement('div'), { className: 'hc-sub' });
    line.append(...[].concat(sub));
    el.append(line);
  }
  if (crumb) el.append(Object.assign(document.createElement('div'), { className: 'hc-crumb', textContent: crumb }));
  return el;
}

let hoverCard = null;                        // { el, at, off }
let hoverTimer = null, hoverGrace = null;
let hoverKey = '';                           // what the card shows, or is about to

function closeHoverCard() {
  clearTimeout(hoverTimer);
  clearTimeout(hoverGrace);
  hoverTimer = hoverGrace = null;
  hoverKey = '';
  if (!hoverCard) return;
  const { el, off } = hoverCard;
  hoverCard = null;
  off();
  el.remove();
}

/* Re-measured after every paint: a card that grows when the payload lands would
   otherwise keep the position its first, shorter self was given. */
function placeHoverCard() {
  if (!hoverCard) return;
  const box = hoverCard.el.getBoundingClientRect();
  const p = placeFloating(hoverCard.at, { width: box.width, height: box.height },
    { width: window.innerWidth, height: window.innerHeight });
  hoverCard.el.style.top = `${p.top}px`;
  hoverCard.el.style.left = `${p.left}px`;
}

function openHoverCard(at, fill) {
  const el = document.createElement('div');
  el.className = 'hovercard';
  el.setAttribute('role', 'tooltip');
  document.body.appendChild(el);
  hoverCard = { el, at, off: () => {} };
  fill(el);
  placeHoverCard();

  el.addEventListener('mouseenter', () => { clearTimeout(hoverGrace); hoverGrace = null; });
  el.addEventListener('mouseleave', hoverLeave);

  // Typing means the card is not being read.
  const onKey = () => closeHoverCard();
  // Deliberately no preventDefault: a click on a ref() still has to reach wireRefClicks.
  const onDown = (e) => { if (!el.contains(e.target)) closeHoverCard(); };
  const onScroll = (e) => { if (!el.contains(e.target)) closeHoverCard(); };
  const onResize = () => closeHoverCard();
  document.addEventListener('keydown', onKey, true);
  document.addEventListener('mousedown', onDown, true);
  document.addEventListener('scroll', onScroll, true);
  window.addEventListener('resize', onResize);
  hoverCard.off = () => {
    document.removeEventListener('keydown', onKey, true);
    document.removeEventListener('mousedown', onDown, true);
    document.removeEventListener('scroll', onScroll, true);
    window.removeEventListener('resize', onResize);
  };
}

/* `key` identifies what is under the pointer, so a mousemove that stays on the
   same thing neither restarts the delay nor repaints. Moving to a different one
   while a card is open swaps it without waiting again, the way a group of
   tooltips behaves. */
function hoverEnter(key, getRect, fill) {
  clearTimeout(hoverGrace);
  hoverGrace = null;
  if (key === hoverKey) return;
  const swap = !!hoverCard;
  clearTimeout(hoverTimer);
  hoverTimer = null;
  hoverKey = key;
  // The rect is read when the card opens, not now: a pan during the delay would
  // otherwise anchor it where the box used to be.
  const show = () => {
    hoverTimer = null;
    if (hoverCard) { hoverCard.off(); hoverCard.el.remove(); hoverCard = null; }
    openHoverCard(getRect(), fill);
  };
  if (swap) show();
  else hoverTimer = setTimeout(show, HOVER_DELAY);
}

function hoverLeave() {
  clearTimeout(hoverTimer);
  hoverTimer = null;
  if (!hoverCard) { hoverKey = ''; return; }
  clearTimeout(hoverGrace);
  hoverGrace = setTimeout(closeHoverCard, HOVER_GRACE);
}

/* Painted twice: once from what the caller already holds, so the card appears
   with the pointer, and again when /api/node answers with the description and
   the columns. */
function fillNodeCard(el, id, seed) {
  const paint = (n, full) => {
    if (!hoverCard || hoverCard.el !== el) return;   // closed, or swapped for another
    const tone = dot(n);
    tone.removeAttribute('title');           // no native tooltip inside the card
    hoverCardBody(el, { title: n.name || id, tone, sub: subNodes(Lineage.subtitleParts(n)), crumb: n.file || id });
    if (n.disabled) {
      const off = Object.assign(document.createElement('span'), { className: 'offchip', textContent: 'disabled' });
      el.querySelector('.hc-head').appendChild(off);
    }
    // Painted from the box and again from the detail, which carry the same
    // facts, so the card does not change under the pointer.
    const facts = testFacts(n, Lineage.testLevel(n)).concat(strategyFacts(n, S.meta && S.meta.adapter));
    for (const f of facts) {
      el.append(Object.assign(document.createElement('div'), {
        className: 'hc-note' + (f.warn ? ' warn' : '') + (f.muted ? ' muted' : ''), textContent: f.text,
      }));
    }
    if (!full) return;

    // A test keeps no description and has no columns, downstream or tests of
    // its own: only what it reads is worth a count.
    const test = n.kind === 'test';
    if (!test) {
      el.append(Object.assign(document.createElement('div'), {
        className: 'hc-desc' + (n.description ? '' : ' muted'),
        textContent: n.description || 'No description in the YAML.',
      }));
    }

    const counts = test ? [`${n.upstream_total} upstream`] : [`${n.columns.length} columns`, `${n.upstream_total} upstream`,
      `${n.downstream_total} downstream`, `${n.tests.length} tests`];
    el.append(Object.assign(document.createElement('div'), { className: 'hc-counts', textContent: counts.join('  ·  ') }));

    if (n.columns.length) {
      const list = document.createElement('div');
      list.className = 'hc-cols';
      for (const c of n.columns.slice(0, HOVER_COLS)) {
        const row = document.createElement('div');
        row.className = 'hc-col';
        row.append(Object.assign(document.createElement('span'), { className: 'c-name', textContent: c.name }));
        row.append(Object.assign(document.createElement('span'), { className: 'c-type', textContent: c.data_type || '' }));
        list.appendChild(row);
      }
      if (n.columns.length > HOVER_COLS) {
        list.append(Object.assign(document.createElement('div'), {
          className: 'hc-more', textContent: `+${n.columns.length - HOVER_COLS} more`,
        }));
      }
      el.appendChild(list);
    }

    if (n.tags && n.tags.length) {
      const tags = document.createElement('div');
      tags.className = 'hc-tags';
      for (const t of n.tags) tags.append(Object.assign(document.createElement('span'), { className: 'tagchip', textContent: t }));
      el.appendChild(tags);
    }
    placeHoverCard();
  };

  paint(seed, false);
  nodeDetail({ id }).then((n) => paint(n, true)).catch(() => {});
}

/* A variable's value. The head is painted at once and the value replaces a
   placeholder line when /api/vars answers. */
function fillVarCard(el, t) {
  const call = `${t.kind}('${t.name}')`;
  const envLabel = S.env || '';
  hoverCardBody(el, { title: call, sub: t.kind === 'env_var' ? 'environment variable' : 'project var' });
  el.append(Object.assign(document.createElement('div'), { className: 'hc-note muted', textContent: 'reading...' }));

  const show = (value, sub, crumb, tone) => {
    if (!hoverCard || hoverCard.el !== el) return;   // closed, or swapped for another
    el.textContent = '';
    hoverCardBody(el, { title: call, sub: t.kind === 'env_var' ? 'environment variable' : 'project var', crumb });
    if (value !== null) {
      el.append(Object.assign(document.createElement('div'), {
        className: 'hc-value' + (value === '' ? ' muted' : ''),
        textContent: value === '' ? '(empty)' : value,
      }));
    }
    if (sub) el.append(Object.assign(document.createElement('div'), { className: 'hc-note ' + (tone || 'muted'), textContent: sub }));
    placeHoverCard();
  };

  fetchVars(t.kind === 'env_var' ? t.name : '').then((body) => {
    if (t.kind === 'env_var') {
      const row = (body.env_vars || [])[0];
      if (!row) return show(null, 'no answer for this name', '', 'warn');
      const fallback = t.fallback ? `\ndefault written in this call: ${t.fallback}` : '';
      return show(row.value === undefined ? null : row.value, varNote(row, envLabel) + fallback,
        envLabel || 'no environment selected', row.redacted ? 'warn' : '');
    }
    const row = (body.vars || []).find((v) => v.name === t.name);
    if (!row) {
      const miss = t.fallback
        ? `not in the vars: block, so the default in this call is used: ${t.fallback}`
        : 'not in the vars: block of dbt_project.yml';
      return show(null, miss, body.file, 'warn');
    }
    const lines = [];
    // A redacted value is never shown, but the expression that produced it is
    // text from the repository, so "as written" still earns its place.
    const value = row.null || row.redacted ? null : (row.resolved !== undefined ? row.resolved : row.raw);
    if (row.null) lines.push('declared with no value, so var() returns null');
    if (row.status) lines.push(varNote(row, envLabel));
    else if (row.redacted) lines.push('hidden because the name reads as a credential');
    if (row.status && row.raw) lines.push('as written: ' + row.raw);
    show(value, lines.filter(Boolean).join('\n'), `${body.file}:${row.line}`, row.redacted ? 'warn' : '');
  }).catch(() => show(null, 'could not read dbt_project.yml', '', 'warn'));
}

/* A macro, from what its link already carries, so there is nothing to wait
   for. The package is the line that matters: a project and a package can both
   define `hub`, and this says which of the two the call reached. */
/* A folder key in dbt_project.yml. The crumb is the folder itself, which is
   what the click opens; the rest is what the key could not say on its own:
   that the config reaches no resource, or that the same chain exists under
   another root. A key the server could not place says where it looked. */
function fillDirCard(el, t) {
  const chain = t.chain.length ? t.chain.join(' > ') : t.block;
  if (!t.path) {
    hoverCardBody(el, { title: chain, sub: `${t.block} config` });
    el.append(Object.assign(document.createElement('div'), { className: 'hc-desc muted', textContent: missingKeyNote(t) }));
    return;
  }
  hoverCardBody(el, { title: chain, sub: t.dir ? `${t.block} config` : `${t.block} config · one file`, crumb: t.path });
  // Only a manifest that matches the project can turn "nothing under it" into
  // "this config reaches nothing". A folder added since the last parse is in no
  // manifest either, and saying it configures nothing would be a lie (0025).
  if (t.nodes === 0) {
    const fresh = S.fresh && S.fresh.state === 'fresh';
    el.append(Object.assign(document.createElement('div'), {
      className: 'hc-desc muted',
      textContent: fresh
        ? 'The folder is there, and no resource in the manifest sits under it: this config reaches nothing, which is what dbt warns about when it parses.'
        : 'The folder is there, and no resource in the manifest sits under it. The manifest no longer matches the project, so a folder added since the last parse is the likelier reason.',
    }));
  }
  if (t.also && t.also.length) {
    el.append(Object.assign(document.createElement('div'), {
      className: 'hc-counts',
      textContent: 'also under: ' + t.also.join(', '),
    }));
  }
}

function fillMacroCard(el, t) {
  hoverCardBody(el, { title: t.name, sub: `macro · ${t.package}`, crumb: t.file });
  el.append(Object.assign(document.createElement('div'), {
    className: 'hc-desc' + (t.description ? '' : ' muted'),
    textContent: t.description || 'No description in the YAML.',
  }));
  if (t.args && t.args.length) {
    el.append(Object.assign(document.createElement('div'), { className: 'hc-counts', textContent: 'arguments: ' + t.args.join(', ') }));
  }
}

/* The freshness card. Built from `freshnessBadge`, which decides every word:
   this only lays them out, using the same vocabulary as the node and var
   cards so the three read as one thing. */
function fillFreshCard(el, f) {
  const b = freshnessBadge(f);
  const add = (cls, text, parent) => {
    const d = Object.assign(document.createElement('div'), { className: cls, textContent: text });
    (parent || el).appendChild(d);
    return d;
  };

  const head = add('hc-head', '');
  const dot = document.createElement('span');
  dot.className = 'dot freshdot';
  dot.dataset.tone = b.tone;
  head.appendChild(dot);
  head.appendChild(Object.assign(document.createElement('span'), { className: 'hc-name', textContent: b.head }));
  if (b.when) add('hc-sub', b.when);
  if (b.desc) add('hc-desc', b.desc);

  for (const g of b.groups) {
    const box = add('hc-cols', '');
    add('hc-more', g.head, box);
    for (const p of g.paths) {
      const row = add('hc-col', '', box);
      row.title = p.full;
      row.appendChild(Object.assign(document.createElement('span'), { className: 'c-name', textContent: p.name }));
      if (p.dir) row.appendChild(Object.assign(document.createElement('span'), { className: 'c-type', textContent: p.dir }));
    }
    if (g.more > 0) add('hc-more', 'and ' + g.more + ' more', box);
  }

  if (b.facts.length) add('hc-counts', b.facts.join('\n')).style.whiteSpace = 'pre-line';
  if (b.advice) add('hc-note ' + (b.tone === 'ok' ? 'muted' : 'warn'), b.advice);
  add('hc-note muted', b.hint);
}

// ------------------------------------------------------------ environments --
/* Which .env file resolves locations. The choice lives in this tab: the stored
   one is only where a fresh tab starts, so two tabs never draw one environment's
   values under the other's name. */
/* A colour family for an environment name, so the one in use is recognisable
   at a glance. Matched on whole words, trailing digits ignored ("DEV2"), and
   pre-production is checked before production. */
function envTone(name) {
  if (!name) return 'manifest';
  const upper = name.toUpperCase();
  const words = upper.split(/[^A-Z0-9]+/).map((w) => w.replace(/\d+$/, ''));
  const has = (...list) => words.some((w) => list.includes(w));
  if (upper.replace(/[^A-Z]/g, '').includes('PREPROD') || has('UAT', 'ACC', 'ACCEPTANCE')) return 'uat';
  if (has('PROD', 'PRD', 'PRODUCTION', 'LIVE')) return 'prod';
  if (has('STG', 'STAGING', 'STAGE')) return 'stg';
  if (has('QA', 'TEST', 'TST', 'SIT')) return 'qa';
  if (has('DEV', 'DEVELOPMENT', 'LOCAL', 'SANDBOX')) return 'dev';
  if (has('CI')) return 'ci';
  return 'other';
}

function envDisplayName(file) {
  const f = S.envs && S.envs.files.find((e) => e.file === file);
  return f ? f.name : file;
}

async function loadEnvs() {
  // Re-reading the env list means the .env files were re-read too, so any
  // resolution cached against the old ones is stale.
  dropNodeCache();
  try { S.envs = await api.get('/api/envs'); } catch { S.envs = null; }
  // The stored choice is where a fresh tab starts, and nothing more: after the
  // first load this tab keeps its own.
  if (!S.envsLoaded) {
    S.envsLoaded = true;
    if (S.envs && S.envs.selected) S.env = S.envs.selected;
  }
  paintEnvSelect();
  if (S.node) renderCatalog(S.node);
}

/* True when switching environment can change something on screen. */
const envsInUse = () => !!(S.envs && S.envs.referenced.length);

function paintEnvSelect() {
  const pill = $('#status-env');
  // No location config reads an env var: the selector could change nothing.
  if (!envsInUse()) { pill.classList.add('hidden'); return; }
  const visible = S.envs.files.filter((f) => !f.hidden);
  if (S.env && !visible.some((f) => f.file === S.env)) S.env = '';
  fillEnvPill(pill, 'env');
  pill.classList.remove('hidden');
}

/* A button showing the environment in use, coloured by envTone. */
function fillEnvPill(el, prefix) {
  const name = S.env ? envDisplayName(S.env) : 'manifest';
  el.textContent = '';
  el.dataset.tone = S.env ? envTone(name) : 'manifest';
  if (prefix) el.append(Object.assign(document.createElement('span'), { className: 'envpill-k', textContent: prefix }));
  el.append(document.createTextNode(name),
    Object.assign(document.createElement('span'), { className: 'envpill-caret', textContent: '▾' }));
  el.title = S.env
    ? `Catalog locations are resolved with ${S.env}. Click to change.`
    : 'Catalog locations are shown as dbt parsed them. Click to pick an environment.';
}

function selectEnv(file) {
  S.env = file;
  paintEnvSelect();
  if (S.node) renderCatalog(S.node);
  // Remembered as the starting point for the next tab; failure is harmless.
  api.send('/api/envs/select', 'POST', { file: file || null }).catch(() => {});
}

/* The environment menu, shared by the status bar pill and the catalog header.
   Opens below its anchor, or above when there is no room (the status bar). */
let envMenu = null;

function closeEnvMenu({ refocus = false } = {}) {
  if (!envMenu) return;
  const { el, anchor, off } = envMenu;
  envMenu = null;
  off();
  el.remove();
  if (refocus && anchor.isConnected) anchor.focus();
}

function openEnvMenu(anchor) {
  const reopen = envMenu && envMenu.anchor === anchor;
  closeEnvMenu();
  if (reopen || !envsInUse()) return;   // a second click on the same anchor closes it

  const el = document.createElement('div');
  el.className = 'envmenu';
  el.setAttribute('role', 'menu');
  const item = ({ label, sub = '', tone = '', current = false, title = '', pick }) => {
    const b = document.createElement('button');
    b.type = 'button';
    b.setAttribute('role', 'menuitemradio');
    b.setAttribute('aria-checked', String(current));
    if (current) b.classList.add('on');
    if (title) b.title = title;
    const check = Object.assign(document.createElement('span'), { className: 'check', textContent: current ? '✓' : '' });
    b.append(check);
    if (tone) {
      const d = Object.assign(document.createElement('span'), { className: 'envdot' });
      d.dataset.tone = tone;
      b.append(d);
    }
    b.append(Object.assign(document.createElement('span'), { className: 'lbl', textContent: label }));
    if (sub) b.append(Object.assign(document.createElement('span'), { className: 'sub', textContent: sub }));
    b.addEventListener('click', () => { closeEnvMenu(); pick(); });
    el.appendChild(b);
    return b;
  };
  for (const f of S.envs.files.filter((e) => !e.hidden)) {
    item({
      label: f.name + (f.target_mismatch ? ' ⚠' : ''),
      sub: f.file,
      tone: envTone(f.name),
      current: S.env === f.file,
      title: f.target_mismatch ? `DBT_TARGET in ${f.file} is ${f.target}, which does not match the name` : '',
      pick: () => selectEnv(f.file),
    });
  }
  item({
    label: 'manifest', sub: 'as dbt parsed it', tone: 'manifest', current: !S.env,
    title: 'the config with the env vars dbt had loaded when it parsed this manifest',
    pick: () => selectEnv(''),
  });
  el.appendChild(document.createElement('hr'));
  item({ label: 'Manage environments...', pick: openEnvPanel });
  document.body.appendChild(el);

  const at = anchor.getBoundingClientRect();
  const box = el.getBoundingClientRect();
  const below = at.bottom + 4 + box.height <= window.innerHeight;
  el.style.top = `${below ? at.bottom + 4 : Math.max(4, at.top - 4 - box.height)}px`;
  el.style.left = `${Math.min(Math.max(4, at.left), window.innerWidth - box.width - 4)}px`;

  const buttons = [...el.querySelectorAll('button')];
  const onKey = (e) => {
    if (e.key === 'Tab') return closeEnvMenu();
    const i = buttons.indexOf(document.activeElement);
    const step = { ArrowDown: 1, ArrowUp: -1 }[e.key];
    if (e.key !== 'Escape' && !step) return;
    // Handled here only: the editor and the global shortcuts never see it.
    e.preventDefault();
    e.stopPropagation();
    if (e.key === 'Escape') closeEnvMenu({ refocus: true });
    else buttons[(i + step + buttons.length) % buttons.length].focus();
  };
  const onDown = (e) => { if (!el.contains(e.target) && !anchor.contains(e.target)) closeEnvMenu(); };
  // Scrolling the anchor away leaves the menu floating in the wrong place. Only
  // a container of the anchor counts: a terminal printing output scrolls too.
  const onScroll = (e) => {
    const t = e.target;
    if (t === document || (t instanceof Node && t.contains(anchor))) closeEnvMenu();
  };
  const onResize = () => closeEnvMenu();
  document.addEventListener('keydown', onKey, true);
  document.addEventListener('mousedown', onDown, true);
  document.addEventListener('scroll', onScroll, true);
  window.addEventListener('resize', onResize);
  envMenu = {
    el, anchor,
    off: () => {
      document.removeEventListener('keydown', onKey, true);
      document.removeEventListener('mousedown', onDown, true);
      document.removeEventListener('scroll', onScroll, true);
      window.removeEventListener('resize', onResize);
    },
  };
  (buttons.find((b) => b.classList.contains('on')) || buttons[0]).focus();
}

/* The named selectors, in a menu beside the Selection box. The environment
   menu's rows with a filter on top, since a project can name dozens. Fetched
   on every open rather than kept: the manifest reloads itself when dbt
   rewrites it, and the list is short. A selector this build cannot resolve is
   listed anyway, with the reason, so dbt ls can still settle it. */
let selectorMenu = null;
let selectorAsk = 0;

function closeSelectorMenu({ refocus = false } = {}) {
  if (!selectorMenu) return;
  const { el, anchor, off } = selectorMenu;
  selectorMenu = null;
  off();
  el.remove();
  if (refocus && anchor.isConnected) anchor.focus();
}

async function openSelectorMenu(anchor) {
  const reopen = selectorMenu && selectorMenu.anchor === anchor;
  closeSelectorMenu();
  const ask = ++selectorAsk;
  if (reopen) return;                         // a second click on the button closes it
  let body;
  try { body = await api.get('/api/selectors'); } catch (e) { toast(e.message, 'err'); return; }
  if (ask !== selectorAsk) return;
  const all = body.selectors || [];

  const el = document.createElement('div');
  el.className = 'envmenu selmenu';
  el.setAttribute('role', 'menu');
  el.setAttribute('aria-label', 'Named selectors');
  const filter = Object.assign(document.createElement('input'), {
    type: 'text', spellcheck: false, autocomplete: 'off',
    placeholder: `Filter ${all.length} selector${all.length === 1 ? '' : 's'}`,
  });
  filter.setAttribute('aria-label', 'Filter the named selectors');
  const rows = Object.assign(document.createElement('div'), { className: 'selrows' });
  el.append(filter, rows);
  for (const note of selectorsNote(body)) {
    el.append(Object.assign(document.createElement('div'), { className: 'selnote', textContent: note }));
  }

  let hits = [];
  let at = 0;
  const pick = (s) => {
    closeSelectorMenu();
    $('#select-input').value = `--selector ${s.name}`;
    clearTimeout(selectTimer);
    runSelection();
  };
  const mark = () => {
    [...rows.children].forEach((b, i) => b.classList.toggle('cur', i === at));
    const cur = rows.children[at];
    if (cur) cur.scrollIntoView({ block: 'nearest' });
  };
  const paint = () => {
    hits = filterSelectors(all, filter.value);
    rows.textContent = '';
    if (!hits.length) {
      const empty = all.length ? 'No selector matches.' : 'This manifest records no selectors.';
      rows.append(Object.assign(document.createElement('div'), { className: 'selempty', textContent: empty }));
      return;
    }
    hits.forEach((s, i) => {
      const b = document.createElement('button');
      b.type = 'button';
      b.setAttribute('role', 'menuitem');
      if (s.unsupported) b.classList.add('off');
      b.title = s.unsupported ? `Cannot be resolved here: ${s.unsupported}` : (s.description || s.name);
      b.append(Object.assign(document.createElement('span'), { className: 'lbl', textContent: s.name }));
      if (s.default) b.append(Object.assign(document.createElement('span'), { className: 'tag', textContent: 'default' }));
      const sub = s.unsupported || s.description;
      if (sub) b.append(Object.assign(document.createElement('span'), { className: 'sub', textContent: sub }));
      b.addEventListener('click', () => pick(s));
      b.addEventListener('mousemove', () => { if (at !== i) { at = i; mark(); } });
      rows.appendChild(b);
    });
    mark();
  };
  filter.addEventListener('input', () => { at = 0; paint(); });
  document.body.appendChild(el);
  paint();
  const place = placeFloating(anchor.getBoundingClientRect(), el.getBoundingClientRect(),
    { width: window.innerWidth, height: window.innerHeight }, 4);
  el.style.top = `${place.top}px`;
  el.style.left = `${place.left}px`;

  const onKey = (e) => {
    if (e.key === 'Tab') return closeSelectorMenu();
    const step = { ArrowDown: 1, ArrowUp: -1 }[e.key];
    if (e.key !== 'Escape' && e.key !== 'Enter' && !step) return;
    // Handled here only: the box, the editor and the global shortcuts never see it.
    e.preventDefault();
    e.stopPropagation();
    if (e.key === 'Escape') closeSelectorMenu({ refocus: true });
    else if (e.key === 'Enter') { if (hits[at]) pick(hits[at]); }
    else if (hits.length) { at = (at + step + hits.length) % hits.length; mark(); }
  };
  const onDown = (e) => { if (!el.contains(e.target) && !anchor.contains(e.target)) closeSelectorMenu(); };
  const onScroll = (e) => {
    const t = e.target;
    if (t === document || (t instanceof Node && t.contains(anchor))) closeSelectorMenu();
  };
  const onResize = () => closeSelectorMenu();
  document.addEventListener('keydown', onKey, true);
  document.addEventListener('mousedown', onDown, true);
  document.addEventListener('scroll', onScroll, true);
  window.addEventListener('resize', onResize);
  selectorMenu = {
    el, anchor,
    off: () => {
      document.removeEventListener('keydown', onKey, true);
      document.removeEventListener('mousedown', onDown, true);
      document.removeEventListener('scroll', onScroll, true);
      window.removeEventListener('resize', onResize);
    },
  };
  filter.focus();
}

/* The Manage panel. Only real overrides are stored: a name equal to the
   automatic one, or a visibility equal to what detection would decide, is sent
   as "automatic" so later detection changes still apply. */
async function openEnvPanel() {
  await loadEnvs();
  if (!S.envs) return toast('environments are unavailable', 'err');
  const refs = S.envs.referenced;
  const nodesFor = (v) => (refs.find((r) => r.var === v) || { nodes: 0 }).nodes;

  const body = document.createElement('div');
  body.className = 'envpanel';
  const table = document.createElement('table');
  table.className = 'loc';
  const head = document.createElement('tr');
  for (const [label, tip] of [
    ['file', ''],
    ['name', 'leave empty for the automatic name, the uppercased file suffix'],
    ['shown', 'hidden files do not appear in the status bar menu'],
    ['DBT_TARGET', 'read from the file as a cross-check against the name'],
    ['variables', `how many of the ${refs.length} variables your location config reads this file defines`],
    ['matches manifest', 'how often this file reproduces what dbt parsed. The file dbt had loaded scores 100%.'],
  ]) {
    const th = document.createElement('th');
    th.textContent = label;
    if (tip) th.title = tip;
    head.appendChild(th);
  }
  table.appendChild(head);

  const inputs = [];
  for (const f of S.envs.files) {
    const tr = document.createElement('tr');
    const td = (content, cls) => {
      const cell = document.createElement('td');
      if (cls) cell.className = cls;
      cell.append(content);
      return cell;
    };

    const name = document.createElement('input');
    name.type = 'text';
    name.maxLength = 32;
    name.placeholder = f.auto_name;
    name.value = f.name === f.auto_name ? '' : f.name;

    const shown = document.createElement('input');
    shown.type = 'checkbox';
    shown.checked = !f.hidden;
    const shownCell = td(shown);
    if (f.hidden_reason) shownCell.title = f.hidden_reason;

    const target = document.createElement('span');
    target.textContent = f.target || '-';
    if (f.target_mismatch) {
      target.className = 'env-placeholder';
      target.title = `DBT_TARGET is ${f.target}, which does not match ${f.file}`;
    }

    const cov = f.coverage;
    const vars = document.createElement('span');
    vars.textContent = `${cov.defined}/${refs.length}`;
    const notes = [];
    if (cov.missing.length) notes.push(`missing: ${cov.missing.join(', ')}`);
    if (cov.placeholders.length) {
      notes.push(`placeholder: ${cov.placeholders.map((v) => `${v} (${nodesFor(v)} nodes)`).join(', ')}`);
      vars.className = 'env-placeholder';
    }
    if (cov.missing.length && !cov.placeholders.length) vars.className = 'env-missing';
    if (notes.length) vars.title = notes.join('\n');

    const a = f.agreement;
    const match = document.createElement('span');
    match.textContent = a.checked ? `${Math.round((1000 * a.equal) / a.checked) / 10}%` : '-';
    match.title = a.checked ? `${a.equal} of ${a.checked} env-driven values equal what dbt parsed` : 'no env-driven values to compare';

    tr.append(td(f.file, 'k'), td(name), shownCell, td(target), td(vars), td(match));
    table.appendChild(tr);
    inputs.push({ f, name, shown });
  }
  body.appendChild(table);
  body.appendChild(Object.assign(document.createElement('p'), {
    className: 'muted',
    textContent: S.envs.persist
      ? `Saved in ${S.envs.settings_path}, outside the project.`
      : 'No config directory is available, so these settings cannot be saved.',
  }));

  const actions = [{ id: null, label: 'Cancel' }];
  if (S.envs.persist) actions.push({ id: 'save', label: 'Save', style: 'primary' });
  if (await modal({ title: 'Environments', body, actions, wide: true }) !== 'save') return;

  const envs = {};
  for (const { f, name, shown } of inputs) {
    const typed = name.value.trim();
    const hidden = !shown.checked;
    envs[f.file] = {
      name: typed && typed !== f.auto_name ? typed : null,
      hidden: hidden === !!f.auto_hidden_reason ? null : hidden,
    };
  }
  try {
    await api.send('/api/envs', 'PUT', { envs });
    await loadEnvs();
    toast('environments saved', 'ok');
  } catch (e) {
    toast('could not save: ' + e.message, 'err');
  }
}

/* One row per location key, across the stages dbt goes through. `envFile`
   picks a .env resolution when the node carries one; without it, the resolved
   column is the config as dbt parsed it. Rows empty at every stage are dropped. */
function locationRows(loc, envFile) {
  if (!loc) return [];
  const parsed = loc.parsed || loc.resolved || {};   // payloads before the rename
  const env = envFile && loc.envs ? loc.envs[envFile] : null;
  return ['database', 'schema', 'alias']
    .map((key) => {
      const cell = env && env.status ? env.status[key] : null;
      const row = {
        key,
        written: loc.written[key] || '',
        parsed: parsed[key] || '',
        resolved: (env ? env.place : parsed)[key] || '',
        built: loc.built[key] || '',
        status: cell ? cell.kind : '',
        vars: cell && cell.vars ? cell.vars : [],
        branch: !!(cell && cell.branch),
      };
      row.templated = /\{[{%]/.test(row.written);
      // Moved compares parsed with built, never the selected environment with
      // built: both come from the same parse, so the answer does not change
      // when another environment is picked.
      row.redirected = !!row.parsed && !!row.built && !sameIdent(row.parsed, row.built);
      return row;
    })
    .filter((row) => row.written || row.parsed || row.resolved || row.built);
}

/* Same warehouse identifier: Snowflake folds unquoted names, and dbt Fusion
   drops a pair of literal quotes that the written config may still carry. */
function sameIdent(a, b) {
  const bare = (s) => (s.length > 1 && s[0] === '"' && s[s.length - 1] === '"' ? s.slice(1, -1) : s);
  return bare(a).toLowerCase() === bare(b).toLowerCase();
}

/* Text, class and tooltip for one resolved cell. `file` is the chosen .env
   file, empty in manifest mode. */
function resolvedCell(row, envName, file) {
  const vars = row.vars.join(', ');
  const branchNote = row.branch
    ? '\nPicked the branch dbt took when it parsed; the condition itself is not evaluated.'
    : '';
  const empty = { text: 'default', cls: 'nul', title: 'not set here, inherited from the target' };
  switch (row.status) {
    case 'missing':
      return { text: `missing ${vars}`, cls: 'env-missing', title: `${vars} is not defined in ${file}` };
    case 'placeholder':
      return {
        text: `${row.resolved || 'empty'} (placeholder)`,
        cls: 'env-placeholder',
        title: `${vars} is set to a placeholder in ${file}, not a real name`,
      };
    case 'unevaluated':
      return {
        text: row.resolved || 'not evaluated',
        cls: 'env-unevaluated',
        title: 'Contains Jinja that dbt-edith does not evaluate.' + branchNote,
      };
    case 'env':
      return { text: row.resolved, cls: '', title: `${vars} from ${file}` + branchNote };
    case 'literal':
      return row.resolved
        ? { text: row.resolved, cls: '', title: `written as a plain value, the same in every environment${branchNote}` }
        : empty;
    case 'parsed':
      return { text: row.resolved, cls: '', title: 'nothing written for this key: value as dbt parsed it' };
    default:
      return row.resolved
        ? { text: row.resolved, cls: '', title: `as dbt parsed it for ${envName || 'this manifest'}` }
        : empty;
  }
}

/* A relation name split into its parts, each remembering whether it was
   quoted. Inside quotes, "" stands for one literal quote. */
function splitRelation(text) {
  const parts = [];
  let name = '', quoted = false, inside = false;
  for (let i = 0; i < text.length; i++) {
    const ch = text[i];
    if (inside) {
      if (ch === '"' && text[i + 1] === '"') { name += '"'; i++; }
      else if (ch === '"') inside = false;
      else name += ch;
    } else if (ch === '"') { inside = true; quoted = true; }
    else if (ch === '.') { parts.push({ name, quoted }); name = ''; quoted = false; }
    else name += ch;
  }
  parts.push({ name, quoted });
  return parts;
}

/* The relation the resolved column points at, quoted part by part the way the
   built relation is, or the reason it cannot be written. Database and schema
   must come from the config: left unset, they fall back to the target profile,
   which dbt-edith does not read. An unset alias is the one dbt built, since dbt
   derives it from the node, not from the environment. */
function resolvedRelation(rows, builtRelation, file) {
  const names = [];
  for (const key of ['database', 'schema', 'alias']) {
    const r = rows.find((row) => row.key === key) || { resolved: '', built: '', status: '', vars: [] };
    const vars = r.vars.join(', ');
    if (r.status === 'missing') return { text: '', reason: `${vars} is not defined in ${file}` };
    if (r.status === 'placeholder') return { text: '', reason: `${key} is a placeholder in ${file}, not a real name` };
    if (r.status === 'unevaluated') return { text: '', reason: `${key} contains Jinja that dbt-edith does not evaluate` };
    if (r.resolved) names.push(r.resolved);
    else if (key === 'alias' && r.built) names.push(r.built);
    else return { text: '', reason: `${key} is not set in the config, so it comes from the target profile, which dbt-edith does not read` };
  }
  const parts = builtRelation ? splitRelation(builtRelation) : [];
  const bare = (s) => (s.length > 1 && s[0] === '"' && s[s.length - 1] === '"' ? s.slice(1, -1) : s);
  const write = (name, i) => (parts.length === 3 && parts[i].quoted
    ? `"${bare(name).replace(/"/g, '""')}"`
    : name);
  return { text: names.map(write).join('.'), reason: '' };
}

/* The relation Snowflake is asked about for a column's lineage, the .env file
   actually used, or why there is none. With no file chosen it is where this
   manifest's target built the node, since that object exists. A chosen file
   gives the relation the Location table resolves, and a file this node was
   not resolved against falls back to the manifest, as that table does. */
function lineageRelation(n, env) {
  if (!n.relation) {
    return { text: '', file: '', reason: n.materialized === 'ephemeral'
      ? `${n.name} is ephemeral, so it is not in the warehouse`
      : `${n.name} has no relation in the warehouse` };
  }
  const file = env && n.location && n.location.envs && n.location.envs[env] ? env : '';
  if (!file) return { text: n.relation, file: '', reason: '' };
  const rel = resolvedRelation(locationRows(n.location, file), n.relation, file);
  return { text: rel.text, file, reason: rel.reason };
}

function catalogLocation(body, n) {
  // A chosen file this node was not resolved against (none discovered at the
  // time, or the file was removed) quietly falls back to manifest mode.
  const file = S.env && n.location && n.location.envs && n.location.envs[S.env] ? S.env : '';
  const envName = file ? envDisplayName(file) : '';
  const rows = locationRows(n.location, file);
  if (!rows.length) return;
  // In a sandbox manifest every model is built away from its config, so a move
  // is expected there and only a model built outside the sandbox stands out.
  const sandbox = (S.meta && S.meta.sandbox) || '';
  const builtAt = n.location ? `${n.location.built.database}.${n.location.built.schema}`.toLowerCase() : '';
  const inSandbox = !!sandbox && builtAt === sandbox;
  const title = h3('location');
  // Beside the heading rather than in the relation row, which an ephemeral
  // model lacks: copying what it reads is just what developing one needs
  // (0058). Under the table, it sat a whole table lower. A sibling of the
  // heading rather than inside it, so the heading still reads "location"
  // alone.
  if (S.features && S.features.snowflake && ['model', 'seed', 'snapshot'].includes(n.kind) && !n.disabled) {
    const b = document.createElement('button');
    b.type = 'button';
    b.className = 'btn sm sfopen clonebtn';
    b.dataset.tool = 'snowflake';
    b.title = `write the statements that copy ${n.name} and its direct parents from an environment into your schema, for you to run`;
    b.append(snowflakeIcon(), document.createTextNode('Clone into my schema'));
    b.addEventListener('click', () => openClonePlan(n));
    const row = document.createElement('div');
    row.className = 'sechead';
    row.append(title, b);
    body.appendChild(row);
  } else {
    body.appendChild(title);
  }

  const table = document.createElement('table');
  table.className = 'loc';
  const head = document.createElement('tr');
  // The environment switch sits in the header of the column it changes. It
  // shows only when this node follows the choice, so its label never differs
  // from the values underneath.
  const pill = envsInUse() && file === S.env;
  for (const [label, tip] of [
    ['', ''],
    ['as written', 'the config as it appears in YAML, SQL or dbt_project.yml'],
    file
      ? [pill ? 'resolved' : `resolved (${envName})`, `the config evaluated with ${file} alone, before the generate_*_name macros`]
      : [pill ? 'resolved' : 'resolved (manifest)', 'the config with the env vars dbt had loaded when it parsed this manifest, '
        + 'before the generate_*_name macros. Not necessarily any particular environment.'],
    ['built (manifest)', 'where the target that produced this manifest actually built it'],
  ]) {
    const th = document.createElement('th');
    th.textContent = label;
    if (tip) th.title = tip;
    head.appendChild(th);
  }
  if (pill) {
    const b = document.createElement('button');
    b.type = 'button';
    b.className = 'envpill sm';
    b.setAttribute('aria-haspopup', 'menu');
    fillEnvPill(b, '');
    b.addEventListener('click', () => openEnvMenu(b));
    head.children[2].append(b);
  }
  table.appendChild(head);

  const cell = (value, cls) => {
    const td = document.createElement('td');
    if (value) {
      td.textContent = value;
      if (cls) td.className = cls;
    } else {
      td.textContent = 'default';
      td.className = 'nul';
      td.title = 'not set here, inherited from the target';
    }
    return td;
  };
  for (const r of rows) {
    const tr = document.createElement('tr');
    const k = document.createElement('td');
    k.className = 'k';
    k.textContent = r.key;
    const built = cell(r.built, r.redirected ? (inSandbox ? 'moved expected' : 'moved') : '');
    if (r.redirected) built.title = `dbt parsed ${r.parsed}, built in ${r.built}`;
    const rc = resolvedCell(r, envName, file);
    const resolved = document.createElement('td');
    resolved.textContent = rc.text;
    if (rc.cls) resolved.className = rc.cls;
    resolved.title = rc.title;
    tr.append(k, cell(r.written, r.templated ? 'tpl' : ''), resolved, built);
    table.appendChild(tr);
  }
  // Ephemeral models have no relation, in any environment.
  if (n.relation) {
    const tr = document.createElement('tr');
    tr.className = 'rel';
    const k = document.createElement('td');
    k.className = 'k';
    k.textContent = 'relation';
    const rel = resolvedRelation(rows, n.relation, file);
    tr.append(k, document.createElement('td'),
      relationCell(rel.text, rel.reason,
        file ? `resolved with ${file}, before the generate_*_name macros` : 'as dbt parsed it, before the generate_*_name macros',
        n.materialized),
      relationCell(n.relation, '', 'where this manifest\'s target built it', n.materialized));
    table.appendChild(tr);
  }
  body.appendChild(table);

  const foot = document.createElement('div');
  foot.className = 'loc-foot';
  const unresolved = rows.filter((r) => r.status === 'missing' || r.status === 'placeholder');
  if (unresolved.length) {
    foot.append(Object.assign(document.createElement('span'), {
      className: 'moved-note',
      textContent: `${file} does not give a usable value for ${unresolved.map((r) => r.key).join(' and ')}.`,
    }));
  } else if (inSandbox) {
    foot.append(Object.assign(document.createElement('span'), {
      className: 'sandbox-note',
      textContent: `Sandbox manifest: ${S.meta.sandbox_models} models are built into ${sandbox}. `
        + (file
          ? `Resolved shows ${envName}; built still shows where this manifest's target built it.`
          : 'Resolved is the config with the env vars dbt had when it parsed. Pick an environment in the status bar to see another.'),
    }));
  } else if (rows.some((r) => r.redirected)) {
    foot.append(Object.assign(document.createElement('span'), {
      className: 'moved-note',
      textContent: 'Built outside its configured location: the generate_*_name macros redirect it for this target.',
    }));
  }
  if (foot.children.length) body.appendChild(foot);
}

/* A full relation name with its own Copy button, and with Snowflake's
   features on its Snowsight menu, or why there is none. */
function relationCell(text, reason, what, materialized) {
  const td = document.createElement('td');
  if (!text) {
    td.className = 'nul';
    td.textContent = 'not available';
    td.title = reason;
    return td;
  }
  const wrap = document.createElement('div');
  wrap.className = 'relcell';
  const name = document.createElement('span');
  name.textContent = text;
  name.title = what;
  const copy = document.createElement('button');
  copy.type = 'button';
  copy.className = 'btn sm';
  copy.textContent = 'Copy';
  copy.title = 'copy ' + text;
  copy.addEventListener('click', () => copyText(text));
  wrap.append(name, copy);
  if (S.features && S.features.snowflake) {
    const open = document.createElement('button');
    open.type = 'button';
    open.className = 'btn sm sfopen';
    open.dataset.tool = 'snowflake';
    open.setAttribute('aria-haspopup', 'menu');
    open.title = `open ${text} in Snowsight: the ${snowsightKind(materialized).label}, its schema or its database`;
    open.append(snowflakeIcon(), document.createTextNode('Snowsight ▾'));
    open.addEventListener('click', () => openSnowsightMenu(open, text, materialized));
    wrap.append(open);
  }
  td.appendChild(wrap);
  return td;
}

/* Where one relation opens in Snowsight. The account comes from the script's
   session, so the first menu of a page may connect, and open a sign-in tab:
   it waits for this click (0016, 0051). Its entries are links the browser
   opens, never requests of ours. */
async function openSnowsightMenu(anchor, relation, materialized) {
  if (closePopup(anchor)) return;
  const menu = document.createElement('div');
  menu.className = 'envmenu sfmenu';
  menu.dataset.tool = 'snowflake';
  menu.setAttribute('role', 'menu');
  if (S.sfSession) {
    paintSnowsightMenu(menu, relation, materialized, {});
    return openPopup(anchor, menu);
  }
  paintSnowsightMenu(menu, relation, materialized, { busy: true });
  openPopup(anchor, menu);
  let error = null;
  try {
    const body = await api.send('/api/snowflake/session', 'POST', {});
    S.sfSession = body.session || {};
    S.colAnswered = true;
  } catch (e) {
    error = e;
  }
  // Closed while Snowflake was asked, or another menu opened since.
  if (!popup || popup.el !== menu) return;
  paintSnowsightMenu(menu, relation, materialized, { error });
  // Placed again for its new size. The script may have started for this, which
  // the top bar shows once the menu is gone: repainting the Catalog now would
  // close it.
  openPopup(anchor, menu, loadSidecar);
}

function paintSnowsightMenu(menu, relation, materialized, { busy, error }) {
  const note = (text, className = 'menunote') => Object.assign(document.createElement('div'), { className, textContent: text });
  menu.replaceChildren(note('open in Snowsight', 'menuhead'));
  if (busy) {
    menu.append(note('Asking Snowflake which account the profile reaches.'
      + (S.colAnswered ? '' : ' A sign-in tab may open if the session needs one.')));
    return;
  }
  if (error) {
    menu.append(note(`Snowflake did not answer: ${error.message}`));
    // A refused connection or a script that could not start is the profile's
    // business, so the file is one click away.
    const link = (error.phase === 'connect' || error.phase === 'start') && profileLink();
    if (link) {
      const where = note('The connection comes from ');
      where.append(link, document.createTextNode('.'));
      menu.append(where);
    }
    return;
  }
  const places = snowsightPlaces(S.sfSession, splitRelation(relation), materialized);
  if (!places.length) {
    menu.append(note(snowsightHome(S.sfSession)
      ? `${relation} is not a database.schema.object name.`
      : 'No Snowsight link: the account\'s organization is unknown.'));
    return;
  }
  for (const place of places) {
    const a = document.createElement('a');
    a.className = 'menulink';
    a.setAttribute('role', 'menuitem');
    a.href = place.url;
    a.target = '_blank';
    a.rel = 'noopener noreferrer';
    a.title = [place.url, place.guess].filter(Boolean).join('\n');
    a.append(
      Object.assign(document.createElement('span'), { className: 'lbl', textContent: place.name }),
      Object.assign(document.createElement('span'), { className: 'sub', textContent: place.what }),
    );
    // After the click, so the link is still in the page when the browser follows it.
    a.addEventListener('click', () => setTimeout(closePopup));
    menu.append(a);
  }
}

// ------------------------------------------------------------ clone (0058) --
/* Statements that copy a model and its direct parents from an environment into
   the developer's own schema. Written here, from the relations the Location
   table resolves, and run by the user: dbt-edith sends them nowhere. The
   functions down to openClonePlan are pure, and web/tests/clone.js slices them
   out by those two names. */

/* What one materialization is copied as. A table-like object is cloned, which
   is instant and costs nothing until either side changes. A view becomes a
   table, since a cloned view would go on reading the environment it came from,
   and the object of that kind a local run may have left is dropped first: a
   CREATE OR REPLACE TABLE does not replace an object of another kind. */
function cloneShape(materialized, mode) {
  const m = materialized || '';
  if (m === 'semantic_view') return { action: 'skip', drop: '', reason: 'a semantic view holds no rows to copy' };
  if (['table', 'incremental', 'snapshot', 'seed'].includes(m)) {
    return { action: mode === 'ctas' ? 'ctas' : 'clone', drop: '', reason: '' };
  }
  const drop = { view: 'view', materialized_view: 'materialized view', dynamic_table: 'dynamic table' }[m] || '';
  return { action: 'ctas', drop, reason: '' };
}

/* Where one node is copied from. Under a chosen file it is the relation the
   Location table resolves with that file, and never the manifest's instead: a
   copy from the wrong environment is worse than none. A database or schema
   the config leaves unset is the target's, which the file does not say. With
   no file, it is where this manifest's target built the node. */
function cloneSource(item, file) {
  if (!item.relation) return { text: '', reason: `${item.name} has no relation in the warehouse` };
  if (!file) return { text: item.relation, reason: '' };
  const env = item.location && item.location.envs ? item.location.envs[file] : null;
  if (!env) return { text: '', reason: `${item.name} was not resolved against ${file}` };
  // The table's own refusals first: a missing variable is the one to fix.
  const rel = resolvedRelation(locationRows(item.location, file), item.relation, file);
  if (!rel.text) return rel;
  for (const key of ['database', 'schema']) {
    if (env.status && env.status[key] && env.status[key].kind === 'parsed') {
      return { text: '', reason: `${key} is not set in the config, so it comes from the target profile, not from ${file}` };
    }
  }
  return rel;
}

/* The destination as typed: two names, each quoted or not. */
function cloneInto(text) {
  const t = (text || '').trim();
  if (!t) return { text: '', reason: 'type the database.schema to clone into' };
  const parts = splitRelation(t);
  if (t.length > 255 || /[\u0000-\u001f\u007f]/.test(t) || parts.length !== 2 || parts.some((p) => !p.name.trim())) {
    return { text: '', reason: 'the destination is a database and a schema: database.schema' };
  }
  return { text: t, reason: '' };
}

/* The destination before the user types one, and where it came from: what
   they typed last time, then the profile's active target, then the sandbox
   this manifest was built into, which only a manifest parsed with their own
   target has. */
function cloneIntoDefault(into) {
  const i = into || {};
  const p = i.profile || {};
  if (i.remembered) return { text: i.remembered, from: 'what you typed last time' };
  if (p.database && p.schema) {
    const picked = p.target_from === 'DBT_TARGET' ? ', picked by DBT_TARGET' : '';
    return { text: `${p.database}.${p.schema}`, from: `profile ${p.profile}, target ${p.target}${picked}` };
  }
  const why = p.why || 'the profile gives no target';
  if (i.sandbox) return { text: i.sandbox, from: `where this manifest built nearly every model, since ${why}: confirm it` };
  return { text: '', from: why };
}

/* One statement. Always TRANSIENT: dbt creates tables transient unless told
   otherwise, Snowflake refuses to clone a transient table into a permanent
   one, and a copy in a sandbox needs no Fail-safe. */
function cloneStatement(source, target, shape) {
  const drop = shape.drop ? `drop ${shape.drop} if exists ${target};\n` : '';
  if (shape.action === 'clone') return `${drop}create or replace transient table ${target}\n    clone ${source};`;
  return `${drop}create or replace transient table ${target} as\nselect * from ${source};`;
}

/* Two relations naming one object: an unquoted name is upper-cased by
   Snowflake, a quoted one kept as written. */
function cloneSameObject(a, b) {
  const key = (t) => splitRelation(t).map((p) => (p.quoted ? p.name : p.name.toUpperCase())).join('\u0000');
  return key(a) === key(b);
}

/* One row per node of the plan, with its statement or why there is none. The
   copy keeps the source's own name, quotes included, so a local ref() finds it. */
function cloneRows(plan, file, dest, mode) {
  return plan.items.map((item) => {
    const shape = cloneShape(item.materialized, mode);
    const row = {
      id: item.id, name: item.name, kind: item.kind, materialized: item.materialized,
      why: item.why, via: item.via || '', changed: !!item.changed,
      action: shape.action, source: '', target: '', sql: '', reason: '',
    };
    if (item.disabled) return Object.assign(row, { action: 'refused', reason: `${item.name} is disabled, so it is not built` });
    if (shape.action === 'skip') return Object.assign(row, { reason: shape.reason });
    const src = cloneSource(item, file);
    if (!src.text) return Object.assign(row, { action: 'refused', reason: src.reason });
    row.source = src.text;
    if (!dest.text) return row;
    const last = splitRelation(src.text).pop();
    row.target = `${dest.text}.${last.quoted ? `"${last.name.replace(/"/g, '""')}"` : last.name}`;
    if (cloneSameObject(row.source, row.target)) {
      return Object.assign(row, { action: 'refused', reason: 'it already is where the copy would go' });
    }
    row.sql = cloneStatement(row.source, row.target, shape);
    return row;
  });
}

/* What the user should know before running them, only where it applies. */
function cloneNotes(plan, rows, ctx) {
  const notes = [];
  const names = (list) => list.map((r) => r.name).join(', ');
  const changed = rows.filter((r) => r.changed);
  if (changed.length) {
    notes.push(`${names(changed)} changed after this manifest was written: run dbt parse, or the parents listed may not be the ones it reads now.`);
  } else if (ctx.stale) {
    notes.push(`${ctx.stale}: the parents listed are the ones this manifest knew.`);
  }
  if (plan.ephemeral.includes(plan.name)) {
    notes.push(`${plan.name} is ephemeral, so nothing of it is copied: only what it reads.`);
  }
  const through = plan.ephemeral.filter((e) => e !== plan.name);
  if (through.length) {
    notes.push(`${through.join(', ')} ${through.length === 1 ? 'is' : 'are'} ephemeral, with no object of ${through.length === 1 ? 'its' : 'their'} own: what ${through.length === 1 ? 'it reads' : 'they read'} is copied instead.`);
  }
  const copied = rows.filter((r) => r.sql);
  if (copied.some((r) => cloneShape(r.materialized, 'clone').drop === 'view')) {
    notes.push('A view is copied as a table of its rows today: a cloned view would go on reading the environment it came from.');
  }
  if (copied.some((r) => r.materialized === 'incremental')) {
    notes.push('An incremental model is copied with the rows it holds, so a local dbt run adds to the copy rather than rebuilding it.');
  }
  if (copied.some((r) => r.materialized === 'snapshot')) {
    notes.push('A snapshot is copied with its history.');
  }
  if (ctx.file) {
    notes.push(`Locations are the config resolved with ${ctx.file}, before the generate_*_name macros: check them if your macros move schemas for that target.`);
  } else if (ctx.sandbox && rows.some((r) => r.action === 'refused' && r.source && cloneSameObject(r.source, r.target))) {
    notes.push('This manifest was built into the destination: pick an environment to copy from.');
  }
  if (ctx.dotenvTarget && ctx.profileTarget && ctx.dotenvTarget !== ctx.profileTarget) {
    notes.push(`Your .env sets DBT_TARGET=${ctx.dotenvTarget}, which dbt reads only once your shell exports it: the destination comes from target ${ctx.profileTarget}.`);
  }
  notes.push('Copies are transient and not managed by dbt: a local dbt run into the same schema replaces them. '
    + 'Running the statements needs a role that reads the source databases and writes the destination.');
  return notes;
}

/* The comment the script opens with. No date, so the same plan reads the same. */
function cloneHead(plan, file, dest) {
  const from = file ? `where ${file} puts them` : 'where this manifest built them';
  return [
    `-- ${plan.name} and its direct parents, from ${from}, into ${dest.text || 'a destination still to type'}.`,
    '-- Written by dbt-edith, which runs none of it: read it, then run it yourself.',
  ];
}

/* The whole script: the picked statements, parents first and the model last,
   and a line for every node that cannot be copied, so nothing goes missing
   unnoticed. A node the user unticked is simply left out. */
function cloneScript(rows, picked, head, blocked) {
  const out = [...head, ''];
  if (blocked) return out.concat(`-- ${blocked}`).join('\n') + '\n';
  for (const r of rows) {
    if (!r.sql) out.push(`-- skipped ${r.name}: ${r.reason}`, '');
    else if (picked.has(r.id)) out.push(r.sql, '');
  }
  if (!rows.some((r) => r.sql && picked.has(r.id))) out.push('-- nothing to run');
  return out.join('\n').replace(/\n+$/, '') + '\n';
}

/* The plan, asked again on every opening so it follows the manifest and the
   .env files, in a modal the user reads and copies from. The destination they
   type is kept for the project once the modal closes (0011). */
async function openClonePlan(n) {
  let plan;
  try {
    plan = await api.get('/api/snowflake/clone?id=' + encodeURIComponent(n.id));
  } catch (e) {
    toast('clone: ' + e.message, 'err');
    return;
  }
  const files = S.envs ? S.envs.files.filter((f) => !f.hidden).map((f) => f.file) : [];
  const was = S.cloneFrom;
  const file = was !== null && (was === '' || files.includes(was)) ? was : (files.includes(S.env) ? S.env : '');
  const first = cloneIntoDefault(plan.into);
  const st = { plan, file, mode: S.cloneMode, into: first.text, from: first.from, unpicked: new Set(), script: '' };

  const body = document.createElement('div');
  body.className = 'envpanel clonepanel';
  const seg = (choices, current, pick) => {
    const box = document.createElement('div');
    box.className = 'seg';
    for (const [value, label, tip] of choices) {
      const b = document.createElement('button');
      b.type = 'button';
      b.className = 'segbtn' + (value === current() ? ' active' : '');
      b.textContent = label;
      if (tip) b.title = tip;
      b.addEventListener('click', () => {
        pick(value);
        box.querySelectorAll('.segbtn').forEach((x) => x.classList.toggle('active', x === b));
        paintClonePlan(view, st);
      });
      box.append(b);
    }
    return box;
  };
  const line = (label, ...parts) => {
    const row = document.createElement('div');
    row.className = 'clonehead';
    row.append(Object.assign(document.createElement('span'), { className: 'k', textContent: label }), ...parts);
    return row;
  };
  const sources = files.map((f) => [f, envDisplayName(f), `copy from where ${f} puts each node`]);
  sources.push(['', 'built (manifest)', 'copy from where the target that produced this manifest built each node']);
  const input = Object.assign(document.createElement('input'), { type: 'text', value: st.into, spellcheck: false });
  input.setAttribute('aria-label', 'destination, database.schema');
  input.placeholder = 'DATABASE.schema';
  const from = Object.assign(document.createElement('span'), { className: 'muted' });
  input.addEventListener('input', () => { st.into = input.value; paintClonePlan(view, st); });
  body.append(
    line('from', seg(sources, () => st.file, (v) => { st.file = v; })),
    line('into', input, from),
    line('as', seg([
      ['clone', 'Clone', 'zero-copy clones for tables, seeds, snapshots and incremental models'],
      ['ctas', 'Create as select', 'a real copy of the rows of every node'],
    ], () => st.mode, (v) => { st.mode = v; })),
  );
  const view = {
    from,
    table: document.createElement('div'),
    sources: Object.assign(document.createElement('p'), { className: 'muted' }),
    notes: document.createElement('div'),
    sql: Object.assign(document.createElement('pre'), { className: 'clonesql' }),
  };
  const copy = Object.assign(document.createElement('button'), { type: 'button', className: 'btn sm', textContent: 'Copy statements' });
  // Straight from the click, before anything awaits, or the browser may
  // refuse the clipboard.
  copy.addEventListener('click', () => copyText(st.script, 'the statements'));
  body.append(view.table, view.sources, view.notes, line('sql', copy), view.sql);
  paintClonePlan(view, st);

  await modal({ title: `Clone ${n.name} into your schema`, body, wide: true, actions: [{ id: 'close', label: 'Close' }] });
  S.cloneFrom = st.file;
  S.cloneMode = st.mode;
  if (!plan.into.persist) return;
  const typed = st.into.trim();
  if (typed && cloneInto(typed).reason) return;
  const p = plan.into.profile || {};
  // The profile's own value, typed back, means "follow the profile" again.
  const keep = !typed || (p.database && typed === `${p.database}.${p.schema}`) ? null : typed;
  if (keep === (plan.into.remembered || null)) return;
  try { await api.send('/api/snowflake/clone/into', 'POST', { into: keep }); } catch (e) { toast('clone: ' + e.message, 'err'); }
}

/* The parts of the modal that follow the choices: the destination's origin,
   the table of nodes, the sources, the notes and the script. */
function paintClonePlan(view, st) {
  const dest = cloneInto(st.into);
  const rows = cloneRows(st.plan, st.file, dest, st.mode);
  view.from.textContent = dest.reason && st.into.trim() ? dest.reason : (st.into.trim() === cloneIntoDefault(st.plan.into).text ? st.from : 'typed here');
  view.from.className = dest.reason && st.into.trim() ? 'env-missing' : 'muted';

  const table = document.createElement('table');
  table.className = 'loc';
  const head = document.createElement('tr');
  for (const label of ['', 'node', 'materialized', 'from', 'action']) head.append(Object.assign(document.createElement('th'), { textContent: label }));
  table.append(head);
  for (const r of rows) {
    const tr = document.createElement('tr');
    const box = Object.assign(document.createElement('input'), { type: 'checkbox' });
    box.disabled = !r.source || r.action === 'refused' || r.action === 'skip';
    box.checked = !box.disabled && !st.unpicked.has(r.id);
    box.setAttribute('aria-label', 'copy ' + r.name);
    box.addEventListener('change', () => {
      if (box.checked) st.unpicked.delete(r.id); else st.unpicked.add(r.id);
      paintClonePlan(view, st);
    });
    const pick = document.createElement('td');
    pick.append(box);
    const name = document.createElement('td');
    name.textContent = r.name;
    name.title = r.why === 'model' ? 'the node asked for' : r.why === 'via' ? `read through ${r.via}, which is ephemeral` : 'a direct parent';
    if (r.why !== 'parent') name.append(Object.assign(document.createElement('span'), { className: 'why', textContent: r.why === 'via' ? ` via ${r.via}` : ' (this one)' }));
    const mat = Object.assign(document.createElement('td'), { className: 'k', textContent: r.materialized });
    const src = document.createElement('td');
    if (r.source && r.action !== 'refused') src.textContent = r.source;
    else { src.textContent = r.reason; src.className = r.action === 'skip' ? 'nul' : 'env-missing'; }
    const act = Object.assign(document.createElement('td'), { className: 'k' });
    act.textContent = { clone: 'clone', ctas: 'create as select', skip: 'nothing to copy', refused: 'not copied' }[r.action] || r.action;
    tr.append(pick, name, mat, src, act);
    table.append(tr);
  }
  view.table.replaceChildren(table);

  const srcs = st.plan.sources.map((s) => {
    const at = cloneSource(s, st.file);
    return `${s.name} (${at.text || at.reason})`;
  });
  view.sources.textContent = srcs.length ? `Read where they are, never copied: ${srcs.join('; ')}.` : '';
  if (st.plan.other.length) view.sources.textContent += ` Also among the parents, with nothing to copy: ${st.plan.other.join(', ')}.`;

  const dotenv = S.envs && S.envs.files.find((f) => f.file === '.env');
  const p = st.plan.into.profile || {};
  const fresh = S.fresh && S.fresh.state && S.fresh.state !== 'fresh' && S.fresh.state !== 'missing' ? freshnessBadge(S.fresh).head : '';
  const notes = cloneNotes(st.plan, rows, {
    file: st.file, stale: fresh, sandbox: st.plan.into.sandbox || '',
    dotenvTarget: dotenv ? dotenv.target || '' : '', profileTarget: p.database ? p.target : '',
  });
  view.notes.replaceChildren(...notes.map((t) => Object.assign(document.createElement('p'), { className: 'clonenote', textContent: t })));

  const picked = new Set(rows.filter((r) => r.sql && !st.unpicked.has(r.id)).map((r) => r.id));
  st.script = cloneScript(rows, picked, cloneHead(st.plan, st.file, dest), dest.reason);
  view.sql.textContent = st.script;
}

/* dbt's own materializations that use no incremental strategy, and the kinds
   of node that have none; a snapshot uses a key and its own `strategy`. */
const UNUSED_BY = ['view', 'table', 'ephemeral', 'seed', 'test', 'snapshot', 'materialized_view'];

/* `a view`, `an ephemeral`. */
function article(word) {
  return (/^[aeiou]/i.test(word) ? 'an ' : 'a ') + word;
}

/* A column name or a key entry as Snowflake compares them: case aside and
   quotes off, since it upper-cases what the YAML declares in lower case. */
function columnKey(name) {
  const s = String(name || '').trim();
  const q = s.match(/^"(.*)"$|^`(.*)`$|^\[(.*)\]$/);
  return (q ? q[1] ?? q[2] ?? q[3] : s).toLowerCase();
}

/* Each entry of a unique key, in the key's order, with the column it names,
   or null for an expression or a name no column of the node has: an
   expression is not parsed, so `order_id || '-' || line_number` marks none. */
function keyColumns(key, columns) {
  const by = new Map(columns.map((c) => [columnKey(c.name), c]));
  return (key || []).map((entry) => ({ entry, column: by.get(columnKey(entry)) || null }));
}

/* Marks a column for the Columns table of node `id` to scroll to, once, the
   next time that node's table is drawn with the Catalog in front. Tied to the
   node, so a table drawn meanwhile, the one a report's link opens over or one
   with a column of the same name, neither takes the scroll nor loses its own. */
function highlightColumn(column, id) {
  S.colHighlight = column;
  S.colReveal = column ? id : '';
}

/* Strategy, as the box reads it: `append merge` with `merge` struck when the
   macros dbt runs insert every row, and the adapter's default said under it. */
function strategyStat(n) {
  const adapter = S.meta && S.meta.adapter;
  const s = stat('Strategy', n.strategy_shown || 'default');
  const v = s.querySelector('.v');
  if (n.strategy_named) v.replaceChildren(...subNodes({ lead: n.strategy_shown, struck: n.strategy_named, rest: '' }));
  if (!n.strategy_shown) v.classList.add('none');
  const under = !n.strategy_shown ? 'not read from the macros'
    : n.strategy_default ? `${Lineage.adapterName(adapter)}'s default` : '';
  if (under) s.append(Object.assign(document.createElement('div'), { className: 'under', textContent: under }));
  const fact = strategyFacts(n, adapter)[0];
  if (fact) s.title = fact.text;
  return s;
}

/* Unique key, one entry per line in the key's order, so a key of long names
   does not stretch the row. Past three, the first two and a link to the rest
   in the Columns tab. Each entry that names a column opens it there. */
function keyStat(n) {
  const key = n.unique_key || [];
  const s = stat('Unique key', key.length ? '' : 'none');
  const v = s.querySelector('.v');
  if (!key.length) {
    v.classList.add('none');
    return s;
  }
  v.classList.add('keys');
  const entries = keyColumns(key, n.columns);
  for (const { entry, column } of entries.length > 3 ? entries.slice(0, 2) : entries) {
    const line = Object.assign(document.createElement('div'), { textContent: entry });
    if (column) {
      line.className = 'keylink';
      line.title = `show ${column.name} in the Columns tab`;
      linkLike(line, () => { highlightColumn(column.name, n.id); showCatalogTab(n, 'columns'); });
    } else {
      line.title = /^["`[]?\w+["`\]]?$/.test(entry.trim())
        ? 'no column of this name, neither declared in the YAML nor in catalog.json'
        : 'an expression, not a column: it is not read';
    }
    v.append(line);
  }
  if (entries.length > 3) {
    const more = Object.assign(document.createElement('div'), {
      className: 'keymore', textContent: `+${entries.length - 2} more, in Columns`,
      title: ['unique_key, in order'].concat(key).join('\n'),
    });
    linkLike(more, () => { S.colSort = 'key'; showCatalogTab(n, 'columns'); });
    v.append(more);
  }
  return s;
}

/* A div that answers a click and the keys a button does, the role and the
   keys giving a keyboard what a button would, so a stat's number or a key
   name keeps the type of the stat it sits in. */
function linkLike(node, onClick) {
  node.tabIndex = 0;
  node.setAttribute('role', 'button');
  node.addEventListener('click', onClick);
  node.addEventListener('keydown', (e) => {
    if (e.key !== 'Enter' && e.key !== ' ') return;
    e.preventDefault();
    onClick(e);
  });
}

function catalogPreview(body, n) {
  const desc = document.createElement('p');
  desc.className = 'cat-desc';
  if (n.description) desc.textContent = n.description;
  else { desc.classList.add('muted'); desc.textContent = 'No description in the YAML.'; }
  body.appendChild(desc);

  body.appendChild(h3('context'));
  const stats = document.createElement('div');
  stats.className = 'stats';
  // The strategy and the key where they are used, beside Mat. type; elsewhere
  // they were inherited, and "details" says so.
  // A disabled model is never run, so nothing is said of how.
  const incremental = n.kind === 'model' && n.materialized === 'incremental' && !n.disabled;
  const keyed = incremental || (n.kind === 'snapshot' && !n.disabled);
  stats.append(
    stat('Mat. type', n.materialized || n.kind),
    ...(incremental ? [strategyStat(n)] : []),
    ...(keyed ? [keyStat(n)] : []),
    stat('Columns', String(n.columns.length), n.columns.length ? () => showCatalogTab(n, 'columns') : null),
    // Each sets the box it names and leaves the other: the Catalog shows
    // another node than the canvas's only when a box was clicked on it.
    stat('Upstream models', `${n.parents.length}`, () => { $('#up').value = 1; focusNode(n.id, { keepDepth: true }); showDock('lineage'); }),
    stat('Downstream models', `${n.children.length}`, () => { $('#down').value = 1; focusNode(n.id, { keepDepth: true }); showDock('lineage'); }),
    stat('Tests', String(n.tests.length), n.tests.length ? () => {
      const h = [...body.querySelectorAll('h3')].find((e) => e.textContent.startsWith('tests on'));
      if (h) h.scrollIntoView({ block: 'start' });
    } : null),
  );
  const totals = document.createElement('div');
  totals.className = 'muted';
  totals.style.cssText = 'font-size:11px;margin-top:8px';
  totals.textContent = `${n.upstream_total} models upstream in total, ${n.downstream_total} downstream (whole graph).`;
  body.append(stats, totals);
  catalogLocation(body, n);
  catalogJobs(body, n);

  if (n.tags.length) {
    body.appendChild(h3('tags'));
    const box = document.createElement('div');
    box.className = 'links';
    n.tags.forEach((t) => {
      const c = document.createElement('span');
      c.className = 'tagchip';
      c.textContent = t;
      box.appendChild(c);
    });
    body.appendChild(box);
  }

  const kv = document.createElement('dl');
  kv.className = 'kvline';
  const add = (k, v, dim) => {
    if (!v) return;
    const dt = document.createElement('dt'); dt.textContent = k;
    const dd = document.createElement('dd'); dd.textContent = v;
    if (dim) dt.className = dd.className = 'muted';
    kv.append(dt, dd);
  };
  // A folder's config reaches every node under it. What a node of dbt's own
  // materializations does not use stays said, dimmed, since a selection by it
  // still picks the node; a custom materialization may read either, so its
  // line says what is set and no more.
  const key = (n.unique_key || []).join(', ');
  const mat = n.materialized || n.kind;
  const unused = UNUSED_BY.includes(mat) && !n.disabled ? `, unused by ${article(mat)}` : '';
  if (!incremental && n.strategy) add('incremental_strategy', n.strategy + unused, !!unused);
  if (!keyed && key) add('unique_key', key + unused, !!unused);
  add('model file', n.file);
  add('schema file', n.yml);
  add('unique id', n.id);
  if (kv.children.length) { body.appendChild(h3('details')); body.appendChild(kv); }

  const refs = (label, items, text) => {
    if (!items.length) return;
    body.appendChild(h3(`${label} (${items.length})`));
    const box = document.createElement('div');
    box.className = 'links';
    // Same gestures as a box in the graph: click moves the lineage and the
    // catalog to that node, double-click also opens its file.
    items.slice(0, 300).forEach((r) => {
      const a = document.createElement('span');
      a.className = 'link';
      a.append(dot(r), document.createTextNode(text ? text(r) : r.name));
      const test = r.kind === 'test' && r.test_name;
      a.title = `${r.id}\nclick: show it here and in the lineage`
        + (r.file ? `\ndouble-click: also open ${test ? 'where it is declared' : 'its file'}` : '');
      a.addEventListener('click', () => focusNode(r.id));
      a.addEventListener('dblclick', () => {
        if (!r.file) return;
        if (test) return openTest(r, { pin: true });
        pushJump();
        openFile(r.file, { focusLineage: false });
        revealInTree(r.file);
      });
      box.appendChild(a);
    });
    body.appendChild(box);
  };
  refs('upstream', n.parents);
  refs('downstream', n.children);
  const guards = splitTests(n.tests);
  // The model's own tests come first, and they are the point of this section:
  // guarding no column, they have no row in the Columns table and are named
  // nowhere else at all. The column ones follow for completeness, each saying
  // which column, because a relationships test counts on both models it joins
  // and reads as a stranger on the one that did not declare it.
  refs('tests on the model', guards.model, (r) => r.test_name || r.name);
  refs('tests on a column', guards.column, (r) => `${r.test_name || r.name} · ${r.column}`);
}

/* The steps that run the node, one chip each, a click away from their card
   in the Jobs tab. The jobs are asked for only once the Catalog is seen, as
   the tab asks once it is opened (0063): one drawn behind another tab, or
   under Reports, is filled in by showDock or closeReports. An answer is kept
   per node, so drawing the Catalog again asks nothing until the jobs may have
   changed (forgetCatalogJobs). Nothing shows until a source is named. */
function catalogJobs(body, n) {
  const box = jel('div', 'cat-jobs');
  box.dataset.node = n.id;
  body.appendChild(box);
  fillCatalogJobs(box);
}

const catalogSeen = () => S.dock === 'catalog' && !S.reportsOpen;

async function fillCatalogJobs(box) {
  if (!box || box.dataset.asked) return;
  const id = box.dataset.node;
  const known = S.catJobs.get(id);
  if (known) {
    box.dataset.asked = '1';
    return paintCatalogJobs(box, known);
  }
  if (!catalogSeen()) return;
  box.dataset.asked = '1';
  const gen = S.catJobsGen;
  let view;
  // What went wrong is the Jobs tab's to say, with the sources at hand; the
  // next time the Catalog is seen asks again.
  try { view = await api.get('/api/jobs/node?id=' + encodeURIComponent(id)); }
  catch { if (gen === S.catJobsGen) delete box.dataset.asked; return; }
  if (gen !== S.catJobsGen) return;
  S.catJobs.set(id, view);
  if (box.isConnected) paintCatalogJobs(box, view);
}

/* The jobs may have changed: the sources were saved, a look at their files
   may have read them again, or a new manifest moved what each step selects.
   What the Catalog shows stays until its next answer replaces it. */
function forgetCatalogJobs() {
  S.catJobs.clear();
  S.catJobsGen++;
  const box = $('#catalog .cat-jobs');
  if (!box) return;
  delete box.dataset.asked;
  fillCatalogJobs(box);
}

function paintCatalogJobs(box, view) {
  if (!view.configured) return box.replaceChildren();
  const id = box.dataset.node;
  const list = jel('div', 'links');
  const chip = (cls, text, title, reveal) => {
    const b = jel('button', cls, text);
    b.type = 'button';
    b.title = title;
    b.addEventListener('click', () => showJobsFor(id, reveal));
    list.append(b);
    return b;
  };
  const chips = jobsChips(view.groups);
  for (const c of chips) {
    const start = jobsStart(c.job);
    const word = { scheduled: 'scheduled, ', unread: 'schedule not read, ' }[start] || '';
    chip('jobchip', c.text,
      `${jobsJobLabel(c.job)}: ${word}${jobsScheduleText(c.job)}\n${c.step.shown}\nclick: show it in the Jobs tab`,
      { key: jobsStepKey(c.job, c.step), index: c.step.index }).dataset.start = start;
  }
  const maybe = jobsMaybeCount(view.maybe);
  if (maybe) {
    chip('jobchip more', `${maybe} not resolved`,
      `${jobsPlural(maybe, 'step')} the tool could not resolve, which could act on it\nclick: show them in the Jobs tab`,
      { maybe: true });
  }
  // A refused file may run the node too, which the tab says above its cards.
  if (view.refused) {
    chip('jobchip more', `${jobsPlural(view.refused, 'file')} not read`,
      `${jobsPlural(view.refused, 'file')} could not be read and may hold more\nclick: open the Jobs tab, whose Sources… says which`, null);
  }
  if (!chips.length) list.prepend(jel('span', 'muted', 'No job read runs it.'));
  box.replaceChildren(h3('jobs'), list);
}

/* The Jobs tab on a node, which may not be the focus, at the step a chip
   names. */
function showJobsFor(id, reveal) {
  S.jobsFor = id === S.focus ? null : id;
  S.jobsReveal = reveal ? Object.assign({ id }, reveal) : null;
  showDock('jobs');
}

/* Which tests a Tests cell shows, and what the +N stands for. Pure, so the cap
   and its edge cases are checked without a DOM (web/tests/testchips.js). Each
   entry is one test node, so two `relationships` on one column are two entries:
   the cell used to hold names, and a name cannot tell them apart. */
function testChips(tests, max, chars) {
  const all = tests || [];
  // Both caps live here rather than beside the cell, so the harness pins the
  // numbers the cell actually uses: a `const` next to it would not survive the
  // slice the harness evaluates.
  const cap = max || 3;
  // Counting chips is not enough. Three of dbt_expectations' names are longer
  // than the Description column and wrap the row to three lines, while three of
  // not_null, unique and one more sit on one. The budget is what keeps every
  // row the same height, whatever the names happen to be.
  const room = chars || 48;
  let used = 0;
  let n = 0;
  while (n < all.length && n < cap) {
    used += (all[n].label || '').length;
    // The first chip is shown however long its name: a cell that says only +4
    // names nothing at all.
    if (n && used > room) break;
    n++;
  }
  if (n >= all.length) return { shown: all, hidden: [] };
  return { shown: all.slice(0, n), hidden: all.slice(n) };
}

/* A node's tests, split by what each one guards. A test naming a column is
   already a chip in that column's row; one naming none guards the whole table
   and has no row it could ever appear in, which is why the two are listed apart
   rather than in one list where the difference is a missing suffix. */
function splitTests(tests) {
  const all = tests || [];
  return { model: all.filter((t) => !t.column), column: all.filter((t) => t.column) };
}

/* Every test guarding one column, behind the +N chip. Unlike fillNodeCard there
   is nothing to fetch: /api/node already sent all of them. */
function fillTestsCard(el, n, c) {
  hoverCardBody(el, { title: c.name, sub: `${c.tests.length} tests`, crumb: n.name });
  const list = document.createElement('div');
  list.className = 'hc-tests';
  for (const t of c.tests) {
    const row = document.createElement('div');
    row.className = 'hc-test';
    row.append(Object.assign(document.createElement('span'), { className: 't-label', textContent: t.label }));
    // dbt's generated name carries what the generic was given: which table a
    // relationships points at, which column it joins on.
    row.append(Object.assign(document.createElement('span'), { className: 't-name', textContent: t.name }));
    list.appendChild(row);
  }
  el.appendChild(list);
  // The list is taller than the head the card was placed with.
  placeHoverCard();
}

/* Fills one Tests cell, collapsed or expanded. Repaints itself rather than
   keeping the expansion in S: sorting or reopening the node rebuilds the table
   anyway, and a key in S would only be there to go stale. */
function paintTests(td, n, c, expanded) {
  td.textContent = '';
  const all = c.tests || [];
  if (!all.length) {
    td.append(Object.assign(document.createElement('span'), { className: 'nul', textContent: '–' }));
    return;
  }
  const cut = expanded ? { shown: all, hidden: [] } : testChips(all);
  for (const t of cut.shown) {
    const chip = document.createElement('span');
    chip.className = 'testchip' + (t.label === 'not_null' ? ' nn' : t.label === 'unique' ? ' un' : '');
    chip.textContent = t.label;
    td.appendChild(chip);
    if (!t.file) { chip.title = `${t.name}\n${t.id}`; continue; }
    // A chip opens its test where it is declared (0041): a chip hangs on the
    // model whose YAML declares it, the one this Catalog shows.
    chip.title = `${t.name}\n${t.id}\nclick: open where it is declared`;
    chip.tabIndex = 0;
    chip.setAttribute('role', 'button');
    const go = (e) => {
      e.stopPropagation();
      openTest({ kind: 'test', file: t.file, host: n.name, column: c.name, test_name: t.label, name: t.name });
    };
    chip.addEventListener('click', go);
    chip.addEventListener('keydown', (e) => {
      if (e.key !== 'Enter' && e.key !== ' ') return;
      e.preventDefault();
      go(e);
    });
  }
  if (!cut.hidden.length && !expanded) return;

  const more = document.createElement('span');
  more.className = 'testchip more';
  more.textContent = expanded ? '−' : `+${cut.hidden.length}`;
  more.tabIndex = 0;
  more.setAttribute('role', 'button');
  more.setAttribute('aria-expanded', String(expanded));
  more.setAttribute('aria-label', expanded
    ? `show fewer tests on ${c.name}`
    : `show all ${all.length} tests on ${c.name}`);
  // No native title here: one under a hover card is two tooltips.
  const open = () => hoverEnter(`tests:${n.id}:${c.name}`, () => more.getBoundingClientRect(), (el) => fillTestsCard(el, n, c));
  more.addEventListener('mouseenter', open);
  more.addEventListener('mouseleave', hoverLeave);
  // A hover is no affordance on a touch screen and no route from a keyboard, so
  // the click expands the chips in the cell itself and focus opens the card for
  // whoever tabbed here. Reopening the card on a second click cannot work: the
  // card closes on any outside mousedown, so the click would close and reopen it
  // forever rather than toggle.
  more.addEventListener('focus', open);
  more.addEventListener('blur', hoverLeave);
  const toggle = (e) => {
    e.stopPropagation();
    closeHoverCard();
    paintTests(td, n, c, !expanded);
    const next = td.querySelector('.testchip.more');
    if (next && e.type !== 'click') next.focus();
  };
  more.addEventListener('click', toggle);
  more.addEventListener('keydown', (e) => {
    if (e.key !== 'Enter' && e.key !== ' ') return;
    e.preventDefault();
    toggle(e);
  });
  td.appendChild(more);
}

/* Whether a column answers the Columns filter: its name holds the text, in any
   case. The name alone, because that is what the filter is for finding, and a
   description mentioning the same word would bury the column called that
   under every column that talks about it. */
function columnMatches(c, query) {
  return !query || c.name.toLowerCase().includes(query.toLowerCase());
}

/* Cmd+F in the Catalog. The columns are what there is to find there, so the
   Preview tab gives way to the Columns one. */
function focusColumnFilter() {
  const n = S.node;
  if (!n || !n.columns.length) return false;
  if (S.catTab !== 'columns') showCatalogTab(n, 'columns');
  const box = $('#col-filter');
  if (!box) return false;
  box.focus();
  box.select();
  return true;
}

function catalogColumns(body, n) {
  if (!n.columns.length) {
    if (S.colReveal === n.id && S.dock === 'catalog') S.colReveal = '';
    const p = document.createElement('p');
    p.className = 'muted';
    p.textContent = 'No column is declared for this node. Document them in the YAML, or run dbt docs generate so catalog.json lists the warehouse columns.';
    body.appendChild(p);
    return;
  }

  const tools = document.createElement('div');
  tools.className = 'coltools';
  // Kept from one node to the next, so a column can be followed through the
  // lineage: click the next model and the filter is already asking for it.
  const filter = document.createElement('input');
  filter.id = 'col-filter';
  filter.className = 'colfilter';
  filter.placeholder = 'Filter by name';
  filter.value = S.colFilter;
  filter.spellcheck = false;
  filter.autocomplete = 'off';
  filter.title = `Filter the columns by name (${keyLabel('Mod+F', IS_MAC)}). ${keyLabel('Escape', IS_MAC)} clears it.`;
  // The count sits inside the box, so it cannot be read as part of the typed
  // count beside it.
  const shown = document.createElement('span');
  shown.className = 'colcount';
  const box = document.createElement('span');
  box.className = 'colfind';
  box.append(filter, shown);
  tools.append(box);
  const untyped = n.columns.filter((c) => !c.data_type).length;
  const note = document.createElement('span');
  note.textContent = untyped === n.columns.length
    ? 'no types: run dbt compile --write-catalog to pull them from Snowflake'
    : `${n.columns.length - untyped}/${n.columns.length} typed`;
  tools.append(note, sourceMenu());
  // This table can only ever show the tests that name a column. Saying how many
  // it cannot show is what stops it from reading as the whole story.
  const onModel = splitTests(n.tests).model.length;
  if (onModel) {
    const span = document.createElement('span');
    span.className = 'colhint modeltests';
    span.textContent = `${onModel} guard the model itself`;
    span.title = 'a singular test, or a generic naming no column: no column row can hold it.\nclick to list them in Preview';
    span.addEventListener('click', () => showCatalogTab(n, 'preview'));
    tools.appendChild(span);
  }
  const hint = columnsHint(S.sidecar, n.columns.some((c) => c.up || c.down))
    || collinHint(S.cllTool, S.collin, S.meta && S.meta.cll_kinds);
  if (hint) {
    const span = document.createElement('span');
    span.className = 'colhint';
    span.dataset.tone = hint.tone;
    span.textContent = hint.text;
    if (hint.title) span.title = hint.title;
    if (hint.tone === 'failed') span.title = (((sidecarOn() ? S.sidecar : S.collin) || {}).log || []).join('\n');
    tools.appendChild(span);
  }
  if (S.cllTool === 'collin' && S.collin && S.collin.found) {
    const again = document.createElement('button');
    again.className = 'btn sm';
    again.textContent = 'Regenerate';
    again.disabled = S.collin.state === 'running';
    again.title = 'Run collin again over the compiled SQL. Run dbt compile first for SQL that changed.';
    again.addEventListener('click', () => regenerateCollin());
    tools.appendChild(again);
  }
  // The key's columns, in the key's order, on the two nodes that use a key.
  const usesKey = ((n.kind === 'model' && n.materialized === 'incremental') || n.kind === 'snapshot') && !n.disabled;
  // By its place in the whole key, expressions included; a column named twice
  // keeps its first place.
  const keyed = new Map();
  if (usesKey) {
    keyColumns(n.unique_key, n.columns).forEach((k, i) => {
      if (k.column && !keyed.has(k.column.name)) keyed.set(k.column.name, i);
    });
  }
  const keyLength = (n.unique_key || []).length;
  // Kept from node to node like the others; a node without a key has no Key
  // button, and sorts A-Z until the next one that has.
  const sort = S.colSort === 'key' && !keyed.size ? 'az' : S.colSort;
  tools.append(Object.assign(document.createElement('div'), { className: 'grow' }));
  tools.appendChild(document.createTextNode('Sort by'));
  for (const [key, label] of [['az', 'A-Z'], ['tests', 'Tests'], ...(keyed.size ? [['key', 'Key']] : [])]) {
    const b = document.createElement('button');
    b.className = 'btn sm';
    b.textContent = label;
    if (key === 'key') b.title = 'the unique key\'s columns first, in its order, then the rest A-Z';
    if (sort === key) b.style.borderColor = 'var(--accent)';
    b.addEventListener('click', () => { S.colSort = key; renderCatalog(n); });
    tools.appendChild(b);
  }
  body.appendChild(tools);
  const said = S.cllTool === 'collin' && collinModelNote(n.collin, n.columns);
  const flags = S.cllTool === 'collin' ? collinColumnNotes(n.collin) : new Map();
  if (said) body.appendChild(Object.assign(document.createElement('div'), { className: 'colnote', textContent: said }));

  const cols = [...n.columns];
  // A column with no test has no `tests` at all: the payload skips it empty.
  if (sort === 'tests') cols.sort((a, b) => (b.tests || []).length - (a.tests || []).length || a.name.localeCompare(b.name));
  else if (sort === 'key') {
    const at = (c) => (keyed.has(c.name) ? keyed.get(c.name) : Infinity);
    cols.sort((a, b) => at(a) - at(b) || a.name.localeCompare(b.name));
  } else cols.sort((a, b) => a.name.localeCompare(b.name));

  // With Snowflake picked, every column can be asked about, not just the cached ones.
  const live = sidecarOn();
  const linked = live || n.columns.some((c) => c.up || c.down || flags.has(c.name.toLowerCase()));
  const table = document.createElement('table');
  table.className = 'cols';
  const head = document.createElement('tr');
  (linked ? ['Column', 'Type', 'Description', 'Tests', 'Lineage'] : ['Column', 'Type', 'Description', 'Tests']).forEach((t) => {
    const th = document.createElement('th');
    th.textContent = t;
    head.appendChild(th);
  });
  table.appendChild(head);

  const nul = () => {
    const s = document.createElement('span');
    s.className = 'nul';
    s.textContent = "\u2013";
    return s;
  };
  const rows = [];
  let focused = null;
  for (const c of cols) {
    const tr = document.createElement('tr');
    if (c.undeclared) tr.className = 'c-undeclared';
    const name = document.createElement('td');
    name.className = 'c-name';
    // The filter rewrites the name to mark what matched; the KEY tag sits
    // beside it, out of reach of that rewrite.
    const text = Object.assign(document.createElement('span'), { textContent: c.name });
    name.append(text);
    if (keyed.has(c.name)) {
      name.append(Object.assign(document.createElement('span'), {
        className: 'keytag', textContent: 'KEY', title: `in the unique key, entry ${keyed.get(c.name) + 1} of ${keyLength}`,
      }));
    }
    rows.push({ tr, name: text, c });
    if (c.undeclared) name.title = 'in the warehouse but not declared in YAML';
    const type = document.createElement('td');
    type.className = 'c-type';
    type.append(c.data_type || nul());
    const desc = document.createElement('td');
    desc.className = 'c-desc';
    desc.append(c.description || nul());
    const tests = document.createElement('td');
    tests.className = 'c-tests';
    paintTests(tests, n, c, false);
    tr.append(name, type, desc, tests);
    if (linked) {
      const lin = document.createElement('td');
      lin.className = 'c-lin';
      if (c.up || c.down) {
        const u = document.createElement('b'); u.textContent = `\u2190${c.up}`;
        const dn = document.createElement('b'); dn.textContent = `${c.down}\u2192`;
        lin.append(u, document.createTextNode('  '), dn);
      } else if (live) {
        const ask = document.createElement('span');
        ask.className = 'c-ask';
        ask.textContent = 'fetch';
        lin.append(ask);
      } else {
        lin.append(nul());
      }
      const flagged = flags.get(c.name.toLowerCase());
      if (flagged) {
        // In the Lineage cell because it is the lineage it qualifies; its own
        // title, so hovering it says why rather than what a click does.
        const mark = document.createElement('span');
        mark.className = 'c-flag';
        mark.textContent = '!';
        mark.title = flagged.join('\n');
        lin.append(mark);
        tr.classList.add('c-flagged');
      }
      if (c.up || c.down || live) {
        tr.classList.add('c-linked');
        tr.title = live ? `fetch the lineage of ${c.name} from Snowflake` : `column lineage for ${c.name}`;
        // The name cell is left out of the click target on purpose: copying a
        // column name is the more common thing to want, and a click target
        // makes the text impossible to select. The Tests cell is left out
        // because its +N chip owns the click. The rest of the row still opens
        // the lineage, so column mode stays reachable from here.
        tr.addEventListener('click', (e) => {
          if (e.target.closest('.c-name, .c-tests')) return;
          openColumn(n, c.name);
        });
      }
      tr.appendChild(lin);
    }
    // Compared as Snowflake compares names, since a report, a column box or
    // Snowflake's own lineage may spell it in another case.
    if (S.colHighlight && columnKey(c.name) === columnKey(S.colHighlight)) {
      tr.classList.add('c-focus');
      focused = rows[rows.length - 1];
    }
    table.appendChild(tr);
  }
  body.appendChild(table);
  const none = document.createElement('p');
  none.className = 'muted';
  body.appendChild(none);

  // Rows are hidden in place rather than the table redrawn, so the box keeps
  // the focus and a +N chip opened further down stays open.
  const apply = () => {
    const q = S.colFilter.trim();
    let kept = 0;
    for (const r of rows) {
      const hit = columnMatches(r.c, q);
      r.tr.classList.toggle('hidden', !hit);
      if (!hit) continue;
      kept++;
      const [before, match, after] = splitMatch(r.c.name, q);
      r.name.textContent = before;
      if (match) r.name.append(Object.assign(document.createElement('mark'), { textContent: match }));
      r.name.append(after);
    }
    shown.textContent = q ? `${kept} of ${rows.length}` : '';
    none.textContent = q && !kept ? `No column name contains "${q}".` : '';
    none.classList.toggle('hidden', !none.textContent);
  };
  filter.addEventListener('input', () => {
    S.colFilter = filter.value;
    apply();
  });
  filter.addEventListener('keydown', (e) => {
    if (e.key !== 'Escape') return;
    e.preventDefault();
    if (filter.value) {
      filter.value = S.colFilter = '';
      apply();
    } else {
      filter.blur();
    }
  });
  apply();
  // A column just asked for is brought into view, once, on the node it was
  // asked on: a filter hiding it is cleared, and typing in the filter
  // afterwards is left alone. It waits for the Catalog to be in front, and is
  // spent on that node even when the table has no such row.
  if (S.colReveal === n.id && S.dock === 'catalog') {
    S.colReveal = '';
  } else {
    focused = null;
  }
  if (focused) {
    if (focused.tr.classList.contains('hidden')) {
      filter.value = S.colFilter = '';
      apply();
    }
    // The row on screen at the next frame: a second draw before it replaces
    // this table.
    requestAnimationFrame(() => {
      const row = $('#catalog .cols tr.c-focus');
      if (row) row.scrollIntoView({ block: 'nearest' });
    });
  }
}

// --------------------------------------------------------------- terminal --
/* What a key in the terminal does to the clipboard, or '' to leave it to the
   shell. On a Mac, Cmd+C and Cmd+V already reach the browser, and Ctrl+C has to
   stay an interrupt. Elsewhere xterm.js sends both to the shell, as ^C and ^V,
   so neither copies nor pastes. This is what Windows Terminal does instead:
   Ctrl+C copies when something is selected and interrupts otherwise, Ctrl+V
   pastes, and Ctrl+Shift+C and Ctrl+Shift+V always copy and paste, the first
   of which would otherwise open the browser's element picker. Ctrl with Alt is
   AltGr on Windows, a character on the layouts that have one (0044). */
function termClipboard(e, mac, selected) {
  if (mac || e.type !== 'keydown' || !e.ctrlKey || e.altKey || e.metaKey) return '';
  const key = (e.key || '').toLowerCase();
  if (key === 'c' && (selected || e.shiftKey)) return 'copy';
  if (key === 'v') return 'paste';
  return '';
}

/* What a line key sends the shell in place of what xterm.js sends, or ''.
   `at` says where it lands: `os` as KEYS_OS, `shell` as /api/meta names it,
   `prompt` as S.termPrompt, `screen`, xterm.js's buffer type, and
   `appCursor`, whether a program asked for application cursor keys.
   On a Mac, xterm.js drops every key held with Cmd, so Cmd+Left and Cmd+Right
   did nothing, and Cmd+Backspace deleted one character. They get the bytes VS
   Code's terminal sends: Ctrl+A, Ctrl+E and Ctrl+U, start of line, end of line,
   and the line deleted back to its start (all of it, in zsh). Option+Left and
   Option+Right already move by word, as ESC b and ESC f; Option+Delete, the
   forward one, gets ESC d, where xterm.js sends a sequence zsh leaves unbound.
   On the Windows VM, Ctrl+Left, Ctrl+Right and Home did nothing in Git Bash.
   xterm.js sends them as CSI sequences, which ConPTY turns into keys and back
   into sequences for bash, whose readline looks them up in Git's inputrc.
   To bash they now go as keys readline binds by itself: Ctrl+A and Ctrl+E for
   Home and End, ESC b and ESC f for Ctrl+Left and Ctrl+Right, and VS Code's
   Ctrl+W and ESC d for Ctrl+Backspace and Ctrl+Delete. The ESC ones still
   cross ConPTY as escape sequences, read there as Alt with a letter.
   A program reading the keys itself gets xterm.js's: one on the alternate
   screen, and anything bash started, a pager less -X keeps on the normal
   screen, PowerShell typed at its prompt, or a prompt of its own. On Windows,
   when bash never reported its prompt, application cursor keys stand in for
   that, as less asks for them and Git Bash's prompt does not. Not on a Mac,
   where oh-my-zsh asks for them at every prompt, and where a pager reading
   Ctrl+E as a line down is little worse than Cmd doing nothing, as in VS
   Code. PowerShell as the shell, and Linux, keep xterm.js's keys (0073). */
function termLineKey(e, at) {
  if (e.type !== 'keydown' || e.shiftKey || e.isComposing) return '';
  if (at.screen !== 'normal' || at.prompt === false) return '';
  if (at.os === 'mac') {
    if (e.ctrlKey) return '';
    if (e.metaKey && !e.altKey) return { ArrowLeft: '\x01', ArrowRight: '\x05', Backspace: '\x15' }[e.key] || '';
    if (e.altKey && !e.metaKey && e.key === 'Delete') return '\x1bd';
    return '';
  }
  // Ctrl with Alt is AltGr, a character on the layouts that have one (0044).
  if (at.os !== 'windows' || at.shell !== 'bash' || e.altKey || e.metaKey) return '';
  if (at.prompt === undefined && at.appCursor) return '';
  if (e.ctrlKey) return { ArrowLeft: '\x1bb', ArrowRight: '\x1bf', Backspace: '\x17', Delete: '\x1bd' }[e.key] || '';
  return { Home: '\x01', End: '\x05' }[e.key] || '';
}

/* Whether bash is still at its prompt after `d` is typed: Enter hands the line
   to a command, which reads the keys itself until the next prompt's OSC says
   bash has them back. A bracketed paste keeps its newlines in the line. */
function promptAfter(prompt, d) {
  if (prompt !== true || d.startsWith('\x1b[200~')) return prompt;
  return /[\r\n]/.test(d) ? false : prompt;
}

/* The OSC number pty.rs has bash print its VIRTUAL_ENV under. */
const VENV_OSC = 6973;

function initTerm() {
  if (S.term) return;
  S.term = new Terminal({
    fontFamily: 'ui-monospace, SFMono-Regular, Menlo, Consolas, monospace',
    fontSize: 12.5, cursorBlink: true, scrollback: 5000,
    theme: { background: '#0e1219', foreground: '#d7dee8', cursor: '#4da3ff', selectionBackground: '#2b3a4d' },
  });
  S.fit = new FitAddon.FitAddon();
  S.term.loadAddon(S.fit);
  S.term.open($('#term'));
  S.fit.fit();
  // bash says which venv it has before each prompt (0069). Any program can
  // print the same sequence, which is why it only ever changes words on screen.
  S.term.parser.registerOscHandler(VENV_OSC, (data) => {
    // Sent before each of bash's prompts, so bash reads its line again (0073).
    S.termPrompt = true;
    if (data.length <= 4096 && !/[\x00-\x1f]/.test(data)) heardVenv(data);
    return true;
  });
  connectTerm();
  S.term.onData((d) => {
    S.termPrompt = promptAfter(S.termPrompt, d);
    if (S.ws && S.ws.readyState === 1) S.ws.send(JSON.stringify({ t: 'i', d }));
  });
  S.term.attachCustomKeyEventHandler((e) => {
    const act = termClipboard(e, IS_MAC, S.term.hasSelection());
    // A paste is the browser's to do: refused by xterm.js, the key fires a
    // paste event on its textarea, which it reads as typing, bracketed paste
    // included. Reading the clipboard from here would ask for a permission.
    if (act === 'paste') return false;
    if (act === 'copy') {
      e.preventDefault();
      copyTermSelection();
      return false;
    }
    const sent = termLineKey(e, {
      os: KEYS_OS, shell: S.shellName, prompt: S.termPrompt, screen: S.term.buffer.active.type,
      appCursor: S.term.modes.applicationCursorKeysMode,
    });
    if (sent) {
      e.preventDefault();
      // Fires onData, which sends it, and not onKey: no key of these is Enter,
      // the one onKey listens for.
      S.term.input(sent);
      return false;
    }
    return true;
  });
  S.term.onKey(({ domEvent }) => {
    if (domEvent.key === 'Enter' && (!S.ws || S.ws.readyState > 1)) { S.term.reset(); connectTerm(); }
  });
  new ResizeObserver(() => {
    if ($('#dock-terminal').classList.contains('hidden')) return;
    try { S.fit.fit(); } catch {}
    if (S.ws && S.ws.readyState === 1) S.ws.send(JSON.stringify({ t: 'r', cols: S.term.cols, rows: S.term.rows }));
  }).observe($('#dock-terminal'));
}

/* Without a toast, as in any terminal. The selection goes, so that the next
   Ctrl+C interrupts. */
function copyTermSelection() {
  const text = S.term.getSelection();
  S.term.clearSelection();
  if (!text) return;
  if (!navigator.clipboard) return toast('clipboard unavailable', 'err');
  navigator.clipboard.writeText(text).catch(() => toast('clipboard unavailable', 'err'));
}

function connectTerm() {
  const proto = location.protocol === 'https:' ? 'wss' : 'ws';
  S.ws = new WebSocket(`${proto}://${location.host}/ws/pty?cols=${S.term.cols}&rows=${S.term.rows}`);
  S.ws.binaryType = 'arraybuffer';
  S.ws.onmessage = (e) => {
    if (typeof e.data === 'string') S.term.write(e.data);
    else S.term.write(new Uint8Array(e.data));
  };
  const ws = S.ws;
  ws.onclose = () => {
    // A socket already replaced, closing late, says nothing of the shell now.
    if (S.ws !== ws) return;
    S.term.write(S.keyless
      ? '\r\n\x1b[90m[no shell: this tab has no key for this server, see the bar at the top]\x1b[0m\r\n'
      : '\r\n\x1b[90m[disconnected - press Enter to start a new shell]\x1b[0m\r\n');
    // No shell any more: back to what the next one would be handed.
    S.termVenv = undefined;
    S.termPrompt = undefined;
    paintVenv();
  };
}

// ---------------------------------------------------------------- palette --
let palIndex = 0, palHits = [];
function openPalette() {
  closeCrumbMenu();
  $('#palette').classList.remove('hidden');
  const input = $('#palette-input');
  input.value = ''; input.focus();
  runPalette('');
}
function closePalette() { $('#palette').classList.add('hidden'); }

/* Nodes first, then the files that are not already represented by one: a model
   would otherwise appear twice, once as a node and once as its own .sql. */
function mergeHits(nodes, paths) {
  const taken = new Set();
  for (const n of nodes) {
    if (n.file) taken.add(n.file);
  }
  return [
    ...nodes.map((n) => ({ kind: 'node', node: n, label: n.name, sub: n.file })),
    ...paths.filter((p) => !taken.has(p)).map((p) => ({ kind: 'file', path: p, label: base(p), sub: p })),
  ];
}

/* One box for everything: dbt nodes from the manifest, and every file in the
   project. A generic test definition or a dotfile is not a node, and used to be
   unfindable. */
async function runPalette(q) {
  const [nodes, paths] = await Promise.all([
    api.get(`/api/search?q=${encodeURIComponent(q)}&kind=model,source,seed,snapshot,exposure&limit=40`).catch(() => []),
    api.get(`/api/files?q=${encodeURIComponent(q)}&limit=40`).catch(() => []),
  ]);
  palHits = mergeHits(nodes, paths);
  palIndex = 0;

  const box = $('#palette-results');
  box.textContent = '';
  if (!palHits.length) {
    box.append(Object.assign(document.createElement('div'), {
      className: 'git-empty',
      textContent: q.trim() ? `Nothing matches "${q.trim()}".` : 'Type to search models, sources and every file in the project.',
    }));
    return;
  }
  palHits.forEach((h, i) => {
    const r = document.createElement('div');
    r.className = 'pres' + (i === 0 ? ' on' : '') + (h.kind === 'node' && h.node.disabled ? ' off' : '');
    const p1 = document.createElement('span');
    p1.className = 'p1';
    p1.textContent = h.label;
    const p2 = document.createElement('span');
    p2.className = 'p2';
    p2.textContent = h.sub;
    r.append(h.kind === 'node' ? dot(h.node) : fileIcon(h.path), p1, p2);
    r.addEventListener('click', () => choosePalette(i));
    box.appendChild(r);
  });
}

function movePalette(delta) {
  const rows = $$('#palette-results .pres');
  if (!rows.length) return;
  rows[palIndex].classList.remove('on');
  palIndex = (palIndex + delta + rows.length) % rows.length;
  rows[palIndex].classList.add('on');
  rows[palIndex].scrollIntoView({ block: 'nearest' });
}

function choosePalette(i) {
  const h = palHits[i ?? palIndex];
  if (!h) return;
  closePalette();
  // A node without a file opens nothing, so activate() would not close them.
  closeReports();
  pushJump();
  if (h.kind === 'file') {
    openFile(h.path, { preview: true });
    revealInTree(h.path);
    return;
  }
  focusNode(h.node.id);
  if (h.node.file) { openFile(h.node.file, { focusLineage: false, preview: true }); revealInTree(h.node.file); }
}

// ------------------------------------------------------------------ tabs --
/* Search across file contents, which the path index cannot answer. The server
   finds the lines; the match inside one is found here, because the browser knows
   the query and a byte offset from Rust would not survive into a UTF-16 string. */
function splitMatch(text, query) {
  const at = text.toLowerCase().indexOf(query.toLowerCase());
  if (!query || at < 0) return [text, '', ''];
  return [text.slice(0, at), text.slice(at, at + query.length), text.slice(at + query.length)];
}

/* What the status line says about a result. Pure, so the wording is checkable. */
function grepSummary(result, query) {
  if (!query) return '';
  if (query.length < 3) return 'three letters or more';
  if (!result) return 'searching…';
  const files = result.files.length;
  if (!files) return `no match for "${query}"`;
  const hits = result.total;
  const bits = [`${hits} match${hits > 1 ? 'es' : ''} in ${files} file${files > 1 ? 's' : ''}`];
  if (result.capped) bits.push('showing the first found');
  if (result.skipped) bits.push(`${result.skipped} file${result.skipped > 1 ? 's' : ''} skipped`);
  return bits.join(' · ');
}

let grepTimer = null;
let grepRun = 0;

function paintGrep(result, query) {
  $('#grep-status').textContent = grepSummary(result, query);
  const host = $('#grep-results');
  host.textContent = '';
  if (!result) return;
  for (const f of result.files) {
    const head = document.createElement('div');
    head.className = 'grep-file';
    head.append(Object.assign(document.createElement('span'), { className: 'nm', textContent: base(f.path) }));
    head.append(Object.assign(document.createElement('span'), { className: 'dir', textContent: dirOf(f.path) }));
    head.title = f.path;
    head.addEventListener('click', () => openAt(f.path, f.hits[0].line));
    host.appendChild(head);
    for (const h of f.hits) {
      const row = document.createElement('div');
      row.className = 'grep-hit';
      row.append(Object.assign(document.createElement('span'), { className: 'ln', textContent: h.line }));
      const [before, hit, after] = splitMatch(h.text, query);
      const body = document.createElement('span');
      body.className = 'tx';
      body.append(document.createTextNode(before));
      if (hit) body.append(Object.assign(document.createElement('mark'), { textContent: hit }));
      body.append(document.createTextNode(after));
      row.appendChild(body);
      row.addEventListener('click', () => openAt(f.path, h.line));
      host.appendChild(row);
    }
    if (f.more) {
      host.append(Object.assign(document.createElement('div'), {
        className: 'grep-more', textContent: `+${f.more} more in this file`,
      }));
    }
  }
}

function runGrep() {
  const query = $('#grep-input').value.trim();
  clearTimeout(grepTimer);
  if (query.length < 3) {
    paintGrep(null, query);
    return;
  }
  paintGrep(null, query);
  const run = ++grepRun;
  // A full scan of a large project is well under a second, so a short pause is
  // enough to keep a burst of typing down to one request.
  grepTimer = setTimeout(() => {
    api.get('/api/grep?q=' + encodeURIComponent(query))
      .then((result) => { if (run === grepRun) paintGrep(result, query); })
      .catch((e) => { if (run === grepRun) $('#grep-status').textContent = 'search failed: ' + e.message; });
  }, 250);
}

/* Opens a file and puts the cursor on one of its lines, which is what a search
   result is for. Preview, like a single click in the explorer: browsing results
   should not leave a dozen pinned tabs behind. */
async function openAt(path, line) {
  await openFile(path, { focusLineage: false, preview: true });
  if (!S.cm || S.active !== path) return;
  gotoPos(line - 1, 0);
}

function wireTabs() {
  $$('[data-side]').forEach((b) => b.addEventListener('click', () => showSide(b.dataset.side)));
  $$('[data-dock]').forEach((b) => b.addEventListener('click', () => showDock(b.dataset.dock)));
  // Each tab's name and description, read once: foldDockTabs() writes the
  // tooltip from them.
  $$('#dock > .tabs .tab').forEach((t) => {
    t.dataset.name = t.querySelector('span').textContent;
    t.dataset.about = t.title;
  });
  // The row's width moves with the window, the sidebar and the dock. Only the
  // row is observed for size: the fold never changes it, so the observer
  // never loops on its own change.
  new ResizeObserver(() => foldDockTabs()).observe($('#dock > .tabs'));
  // What the tools hold moves too, the eye's count, the column chip, a mode,
  // and it changes while another tab is open, when the tools are hidden and a
  // hidden element reports no resize. So it is watched as markup. A change of
  // the tools' own class is left out: showDock hides and shows them, which
  // moves nothing the fold measures, and foldDockTabs shows them for its
  // measure, which would call it again.
  const tools = $('#lineage-tools');
  const watch = new MutationObserver((records) => {
    if (records.some((r) => r.target !== tools || r.type !== 'attributes')) foldDockTabs();
  });
  watch.observe(tools, { subtree: true, childList: true, characterData: true, attributes: true });
  watch.observe($('#history-tab'), { attributes: true, attributeFilter: ['class'] });
}

/* Pulled out of wireTabs because a link into the file tree has to bring the
   tree back into view before there is anything to reveal. */
function showSide(which) {
  $$('[data-side]').forEach((x) => x.classList.toggle('active', x.dataset.side === which));
  $('#side-files').classList.toggle('hidden', which !== 'files');
  $('#side-models').classList.toggle('hidden', which !== 'models');
  $('#side-git').classList.toggle('hidden', which !== 'git');
  $('#side-search').classList.toggle('hidden', which !== 'search');
  if (which === 'models' && !$('#model-list').children.length) refreshModels();
  if (which === 'git') refreshGit();
  if (which === 'search') $('#grep-input').focus();
}

/* `look` is off for a tab brought forward by a click in a report that has
   just looked at the job files itself (0063), and for the tour's Jobs step
   and its putting back of the dock, which ask for none (0075). The reports
   are left as they are: a column's lineage brings the canvas forward when
   its answer lands, seconds later with Snowflake, and must not roll up a
   panel opened since. What the user asks for rolls them up itself. */
function showDock(which, { look = true } = {}) {
  let loading = null;
  S.dock = which;
  $$('[data-dock]').forEach((x) => x.classList.toggle('active', x.dataset.dock === which));
  $('#dock-lineage').classList.toggle('hidden', which !== 'lineage');
  $('#dock-terminal').classList.toggle('hidden', which !== 'terminal');
  $('#dock-catalog').classList.toggle('hidden', which !== 'catalog');
  $('#dock-compiled').classList.toggle('hidden', which !== 'compiled');
  $('#dock-run').classList.toggle('hidden', which !== 'run');
  $('#dock-history').classList.toggle('hidden', which !== 'history');
  $('#dock-jobs').classList.toggle('hidden', which !== 'jobs');
  if (which === 'compiled' || which === 'run') loadArtifact(which, S.focus);
  if (which === 'jobs') loading = loadJobs({ look });
  if (which === 'catalog') fillCatalogJobs($('#catalog .cat-jobs'));
  // A column box clicked with the lineage in front asked for its row.
  if (which === 'catalog' && S.node && S.colReveal === S.node.id && S.catTab === 'columns') renderCatalog(S.node);
  // Read when the tab is first opened, never before: that click is what may
  // bring a sign-in tab (0016).
  if (which === 'history' && !S.history) loadHistory();
  $('#lineage-tools').classList.toggle('hidden', which !== 'lineage');
  if (which === 'terminal') { initTerm(); setTimeout(() => { try { S.fit.fit(); } catch {} S.term.focus(); }, 30); }
  if (which === 'lineage') setTimeout(() => Lineage.fit(), 30);
  // The tour waits on it, to point at the tab once it is filled.
  return loading;
}

/* The dock's tabs share their row with the lineage's tools, and the glyphs
   made each tab wider. Once the row cannot hold every label on one line, the
   tabs keep their glyph alone, named by their tooltip, rather than the tools
   breaking onto a second line the 30px row cannot show. It is measured, never
   set at a width, because the room moves with the sidebar, Query history, the
   eye's count and the column chip. It is measured with the tools shown, and
   the open tab folds with the others, so that a click on a tab never moves
   the tabs under the pointer: wireTabs() calls it when the row or the tools
   change, and never on a tab change. Only the tools are asked whether they
   broke, since a tab cannot, and a tab taller than the row, under a large
   minimum font, would keep the labels folded at any width. Past that, the
   tools still break: docs/state.md has the rest. */
function foldDockTabs() {
  const row = $('#dock > .tabs');
  const tools = $('#lineage-tools');
  const away = tools.classList.contains('hidden');
  const was = row.classList.contains('folded');
  row.classList.remove('folded');
  tools.classList.remove('hidden');
  const h = row.getBoundingClientRect().height;
  const fits = row.scrollWidth <= row.clientWidth
    && $$('#lineage-tools > *').every((e) => e.getBoundingClientRect().height <= h);
  tools.classList.toggle('hidden', away);
  row.classList.toggle('folded', !fits);
  if (was === !fits) return;
  $$('#dock > .tabs .tab').forEach((t) => {
    const { name, about } = t.dataset;
    if (!fits) t.title = about ? `${name}: ${about}` : name;
    else if (about) t.title = about;
    else t.removeAttribute('title');
  });
}

// -------------------------------------------------------------- splitters --
function wireSplitters() {
  const drag = (gutter, onMove) => {
    gutter.addEventListener('mousedown', (e) => {
      e.preventDefault();
      gutter.classList.add('drag');
      const move = (ev) => onMove(ev);
      const up = () => {
        gutter.classList.remove('drag');
        window.removeEventListener('mousemove', move);
        window.removeEventListener('mouseup', up);
        S.cm && S.cm.refresh();
        if (S.fit && !$('#dock-terminal').classList.contains('hidden')) { try { S.fit.fit(); } catch {} }
        Lineage.fit();
      };
      window.addEventListener('mousemove', move);
      window.addEventListener('mouseup', up);
    });
  };
  drag($('#gutter-side'), (e) => {
    const w = Math.min(520, Math.max(150, e.clientX));
    document.documentElement.style.setProperty('--side-w', w + 'px');
  });
  drag($('#gutter-dock'), (e) => {
    const body = $('#body').getBoundingClientRect();
    const pct = Math.min(85, Math.max(10, ((body.bottom - e.clientY) / body.height) * 100));
    document.documentElement.style.setProperty('--dock-h', pct + '%');
  });
}

// ------------------------------------------------------------------ tour --
/* "Give me a tour", from the shortcut list, and offered once on its own
   (0074). Each step points at a part of the page and says what it is for,
   on a model the server picks with parents and children. It runs nothing and
   changes no setting: it opens a file as a preview, moves the lineage, the
   dock and the Custom selection box, and puts them back as they were when it
   ends. The pure part runs from tourSteps to tourPlace, which
   web/tests/tour.js slices. */

/* The steps, from what the page knows when the tour starts: `model` from
   /api/tour, `manifest` whether one was read, `jobs` how many job sources
   there are, `reports` the labels of
   the reports with something to show, `cll` the column lineage tool on
   screen, '' for none, and `snowflake` whether its features are on. A step
   about a model is left out without one; a step about a feature that is off
   says how to turn it on. Text between backticks is set as code, and a key
   between braces as a key: ⌘` holds a backtick of its own. */
function tourSteps(c) {
  const m = c.model;
  const key = (combo) => '{' + keyLabel(combo, c.mac) + '}';
  const plural = (n, one, many) => `${n} ${n === 1 ? one : many}`;
  const steps = [];
  const leftOut = 'so the editor, the lineage, the Catalog and the Jobs tab are left out.';
  // Without a model the welcome rings the ? button, where the tour is taken
  // again: a list nobody has opened is no place to send anyone by name.
  const again = `then take the tour again from the shortcut list, behind this button or ${key('?')}: Give me a tour `
    + `heads it. ${key('Escape')} ends it.`;
  steps.push({ key: 'welcome', target: m ? '' : '#keys-btn', title: 'A tour of dbt-edith',
    text: m
      ? (m.package ? 'This project has no model of its own, so it opens' : 'It opens')
        + ` \`${m.name}\`, a model of ${m.package ? `the package \`${m.package}\`` : 'this project'} with `
        + `${plural(m.parents, 'parent', 'parents')} and ${plural(m.children, 'child', 'children')}, and points at `
        + `what each part of the page does with it. It runs nothing and changes no setting, and ${key('Escape')} ends `
        + 'it at any step.'
      : c.manifest
        ? `This manifest has no model to show, ${leftOut} Once the project has one, run \`dbt parse\` in the `
          + `terminal, ${again}`
        : `There is no \`manifest.json\` to read yet, ${leftOut} Run \`dbt parse\` in the terminal, which writes `
          + `one, ${again}` });
  // Right after the welcome: every step after it is only as true as this file.
  const read = 'Everything here is read from `manifest.json`, which only dbt writes: the lineage, the Catalog, every '
    + 'selection, what each job runs.';
  const parse = 'A click types `dbt parse` in the terminal, leaving Enter to you, and the page reads the new '
    + 'manifest on its own.';
  steps.push({ key: 'manifest', target: '#fresh-badge', title: 'manifest.json, and how fresh it is',
    text: c.manifest
      ? `${read} So the page is as true as the last \`dbt parse\`. This says how old the manifest is, and its dot `
        + 'whether a file dbt reads changed since: green, none has; amber, your own saved edits; red, a pull, a '
        + `checkout or a merge. Hovering it says which files. ${parse}`
      : `${read} There is none yet, so this says no manifest. ${parse}` });
  if (m) {
    steps.push({ key: 'editor', target: '#editor-pane', title: `${m.name}, in the editor`,
      text: 'Every `ref()`, `source()` and macro call is a link to the file behind it, and hovering one says what it is. '
        + 'Each part of the breadcrumb above opens a menu of what sits beside it.' });
    steps.push({ key: 'config', target: ['#crumbs .cfg', '.crumbmenu.config'], title: 'Where dbt_project.yml configures it',
      text: `The violet button at the end of the breadcrumb lists every key of \`dbt_project.yml\` that reaches \`${m.name}\`, `
        + 'broadest first, with what each one sets. Picking one opens `dbt_project.yml` at that key. A `config()` in the '
        + 'model, or in its properties YAML, still wins over them.' });
    steps.push({ key: 'lineage', target: '#dock-lineage', title: 'Lineage',
      text: `\`${m.name}\` and its neighbours, as many levels up and down as the two boxes say, read from \`manifest.json\`: `
        + 'nothing here runs dbt. Click a box to read its file, double-click to centre the lineage on it, and a `+N` '
        + 'badge pulls in one more level. Opening a model\'s file moves the lineage onto it.' });
    steps.push({ key: 'select', target: '#select-bar', title: 'Custom selection',
      text: 'Any set of nodes, written the way dbt reads a selection: names, `tag:`, `path:`, `+` and `@`, or a named '
        + `selector from \`selectors.yml\`. This one, \`+${m.name}+\`, is everything it is built from and everything `
        + 'built from it. dbt ls types the same selection into the terminal, for you to check against dbt itself.' });
    steps.push({ key: 'share', target: ['#export-btn', '#image-btn', '#select-copy'], title: 'Export, or copy as an image',
      text: 'Whatever the canvas shows, this selection included. Export saves it as one HTML file that opens offline, '
        + 'zooms without blurring and prints on one page. Copy image puts the same picture on the clipboard as a PNG, '
        + 'for a ticket. Copy, in the bar of the selection, copies the names instead, one per line.' });
    steps.push({ key: 'catalog', target: '#dock-catalog', title: 'Catalog',
      text: `What the manifest says of \`${m.name}\`: its materialization, description, tags and tests, where it lands `
        + 'in each environment, the steps of your jobs that run it once the Jobs tab has a source, and its columns '
        + 'under Columns. Its upstream and downstream counts move the lineage, as a box does.' });
  }
  const columns = c.cll
    ? `On screen: ${c.cll}. A click on a column in Catalog > Columns follows it from model to model, and this menu `
      + 'picks where the edges come from.'
    : 'None yet: this menu picks where a column\'s lineage comes from. Collin reads the compiled SQL, and picking '
      + 'it runs it, or offers the command that installs it'
      + (c.snowflake ? '; Snowflake fetches a column\'s lineage when you click it.' : '.')
      + ' Then a click on a column in Catalog > Columns follows it from model to model.'
      + (c.snowflake ? '' : ' Snowflake joins the list once its features are on, from the snowflake button in the top bar.');
  steps.push({ key: 'columns', target: '#cll-pick-host .cllpick', title: 'Column lineage', text: columns });
  if (m) {
    steps.push({ key: 'jobs', target: '#dock-jobs', title: 'Jobs',
      text: c.jobs
        ? `Which of your orchestrator's jobs run \`${m.name}\`: an Airflow DAG and its task, or a dbt Cloud job and `
          + 'its step, with its schedule and its dbt command, resolved here as Custom selection is. Sources… says '
          + 'where the jobs are read from.'
        : 'Which of your orchestrator\'s jobs run a model, read from their YAML, often in another repository: '
          + 'Airflow DAGs or dbt Cloud jobs. None is read yet: Add a source names the folder they are in, and '
          + 'nothing outside the project is read before.' });
  }
  const reportsKey = key('Mod+Alt+R');
  steps.push({ key: 'reports', target: '#reports-btn', title: 'Reports',
    text: c.reports.length
      ? `Reports on the whole project: ${c.reports.join(', ')}. A chip on the button says what is new in them `
        + `since you last looked, and ${reportsKey} opens them from anywhere.`
      : 'Reports on the whole project, dimmed until one has something to show: the last dbt run, from '
        + '`run_results.json`, collin\'s findings, and the job coverage once the Jobs tab has a source. Meanwhile '
        + `it opens on the coverage, where a source is added, and ${reportsKey} opens it from anywhere.` });
  steps.push({ key: 'terminal', target: '#dock [data-dock=terminal]', title: 'Terminal',
    text: 'A shell in the project folder, where dbt runs: this page never runs it. A button that offers a dbt '
      + 'command, such as dbt ls above the lineage, types it there and leaves Enter to you. '
      + `${key('Mod+`')} brings it forward from anywhere.` });
  steps.push({ key: 'search', target: '#palette-btn', title: 'Search',
    text: `Every model, source and file of the project by its name, from ${key('Mod+K')} anywhere. The Search tab `
      + 'in the sidebar looks inside the files instead.' });
  steps.push({ key: 'end', target: '#keys-btn', title: 'Every shortcut, and this tour',
    text: `${key('?')} or ${key('F1')} lists every shortcut, and Give me a tour at the top of that list starts this `
      + 'again. That is the tour.' });
  return steps;
}

/* The ring around a target: its box grown by `pad`, kept inside the view so
   a pane against the window's edge still shows all four sides. A target of
   several elements is rung as one box around them all. */
function tourRing(at, view, pad = 4) {
  const left = Math.max(2, at.left - pad), top = Math.max(2, at.top - pad);
  const right = Math.min(view.width - 2, at.right + pad), bottom = Math.min(view.height - 2, at.bottom + pad);
  return { left, top, width: Math.max(0, right - left), height: Math.max(0, bottom - top) };
}

/* Where the card goes beside a ring: below it, centred on it, when it fits,
   then above, and otherwise inside its lower edge, for a target as tall as
   the window. Never off screen. */
function tourPlace(ring, box, view, gap = 12) {
  const left = Math.max(8, Math.min(ring.left + ring.width / 2 - box.width / 2, view.width - box.width - 8));
  const bottom = ring.top + ring.height;
  if (bottom + gap + box.height <= view.height - 8) return { left, top: bottom + gap };
  if (ring.top - gap - box.height >= 8) return { left, top: ring.top - gap - box.height };
  return { left, top: Math.max(8, Math.min(bottom - box.height - gap, view.height - box.height - 8)) };
}

/* Starts the tour, unless one is on screen. The overlay goes up at once, so
   nothing can be done under it while the model is picked, and what the tour
   will move is noted in the same moment, to be put back when it ends. */
async function startTour() {
  if (S.tour) return;
  closeShortcuts();
  closePalette();
  const reports = S.reportsOpen;
  const reportsReturn = S.reportsReturn;
  closeReports();
  closePopup();
  closeHoverCard();
  for (const offer of $$('.tour-offer')) offer.remove();
  const t = { at: -1, ask: 0, chain: Promise.resolve() };
  S.tour = t;
  t.saved = {
    active: S.active, preview: S.preview, order: S.order.slice(), dock: S.dock, mode: S.graphMode,
    focus: S.focus, colFocus: S.colFocus, jobsFor: S.jobsFor, unsynced: S.unsynced,
    select: $('#select-input').value, history: S.selectHistory.slice(), catTab: S.catTab,
    up: $('#up').value, down: $('#down').value,
    node: S.node, catalog: [...$('#catalog').childNodes], back: document.activeElement,
    reports, reportsSection: S.reportsSection, reportsReturn,
  };
  buildTour(t);
  // The index too, so the Reports step says what the button shows even when
  // the page asked for it a moment ago and has not heard back.
  const [{ model, sources: jobs }] = await Promise.all([
    api.get('/api/tour').then((r) => ({ model: r.model || null, sources: r.sources || 0 }), () => ({ model: null, sources: 0 })),
    loadReportsIndex(),
  ]);
  if (S.tour !== t || t.ending) return;
  const pick = lineagePickLabel(S.meta, S.cllTool, sidecarOn(), S.sidecar, S.collin);
  t.model = model;
  t.steps = tourSteps({
    model, mac: IS_MAC, jobs,
    manifest: !!(S.meta && S.meta.manifest_mtime),
    reports: reportsNow().sections.filter((sec) => !sec.empty).map((sec) => sec.label),
    // A tool picked is not yet edges on screen: collin running or failed has none.
    cll: (S.meta && S.meta.cll_file) || sidecarOn() ? pick.name : '',
    snowflake: !!(S.features && S.features.snowflake),
  });
  tourGo(0);
}

/* The overlay: a ring that dims the rest of the page through its shadow, and
   a card, which says the model is being picked until the first step is
   ready. The page under it takes no click and no key while it is up. */
function buildTour(t) {
  const root = jel('div', 'centre');
  root.id = 'tour';
  root.setAttribute('role', 'dialog');
  root.setAttribute('aria-modal', 'true');
  root.setAttribute('aria-labelledby', 'tour-title');
  const ring = jel('div', 'tour-ring hidden');
  const card = jel('div', 'tour-card');
  card.tabIndex = -1;
  const head = jel('div', 'tour-head');
  const close = jel('button', 'icon', '×');
  close.type = 'button';
  close.title = `End the tour (${keyLabel('Escape', IS_MAC)})`;
  close.addEventListener('click', endTour);
  head.append(jel('span', 'tour-count'), jel('div', 'grow'), close);
  const title = jel('h2', '', 'A tour of dbt-edith');
  title.id = 'tour-title';
  const back = jel('button', 'btn sm tour-back', 'Back');
  back.type = 'button';
  back.disabled = true;
  back.addEventListener('click', () => tourStep(-1));
  const next = jel('button', 'btn sm primary tour-next', 'Next');
  next.type = 'button';
  next.disabled = true;
  next.addEventListener('click', () => tourStep(1));
  const actions = jel('div', 'tour-actions');
  const keys = jel('span', 'tour-keys', '← →');
  keys.title = 'The arrow keys walk the tour too';
  actions.append(keys, jel('div', 'grow'), back, next);
  card.append(head, title, jel('p', 'tour-text', 'Picking a model of this project…'), actions);
  root.append(ring, card);
  document.body.append(root);
  for (const id of ['#topbar', '#body', '#statusbar']) $(id).inert = true;
  t.root = root;
  card.focus();
  window.addEventListener('keydown', tourKey, true);
  window.addEventListener('resize', paintTour);
}

/* The keys the app answers from anywhere, which wait for the tour to end,
   with what the browser would do with them meanwhile: Cmd+S would offer to
   save the page, and F1 open the browser's help. */
function tourHeld() {
  return new Set(shortcutSheet().filter((g) => g.title === 'Anywhere' || g.title === 'Find')
    .flatMap((g) => g.keys.flatMap(([combos]) => combos)).filter((k) => /Mod|Alt|(^|\+)F\d|^\?$/.test(k)));
}

/* The arrows and Enter walk the tour, Escape ends it, and every other key
   waits for it to end. Enter on a button is the button's own, and Tab moves
   between the card's buttons, the rest of the page being inert. */
function tourKey(e) {
  if (!S.tour) return;
  if (S.tour.ending) {
    // The page is being put back: nothing reaches it, and nothing walks on.
    if (e.key !== 'Tab') e.preventDefault();
    e.stopPropagation();
    return;
  }
  const onButton = e.target && e.target.tagName === 'BUTTON';
  if (e.key === 'Escape') endTour();
  else if (e.key === 'ArrowRight' || (e.key === 'Enter' && !onButton)) tourStep(1);
  else if (e.key === 'ArrowLeft') tourStep(-1);
  else {
    if (tourHeld().has(keyCombo(e, IS_MAC))) e.preventDefault();
    e.stopPropagation();
    return;
  }
  e.preventDefault();
  e.stopPropagation();
}

function tourStep(by) {
  const t = S.tour;
  if (!t || !t.steps || t.ending) return;
  const to = t.at + by;
  if (to >= t.steps.length) return endTour();
  if (to >= 0) tourGo(to);
}

/* Sets the page up for a step, then draws the ring and the card over it.
   One step sets the page up at a time, and one passed meanwhile, by a key
   held down or an end, stops at its next look: finishing over the next step,
   or after the end, it would leave its canvas or its menu behind. */
async function tourGo(i) {
  const t = S.tour;
  if (!t || !t.steps || t.ending) return;
  t.at = i;
  const ask = ++t.ask;
  const step = t.steps[i];
  const live = () => S.tour === t && t.ask === ask;
  t.chain = t.chain.then(async () => {
    if (!live()) return;
    closeCrumbMenu();
    try { await tourEnter(step.key, t, live); }
    catch (e) { toast('tour: ' + e.message, 'err'); }
  });
  await t.chain;
  // The dock fits its canvas 30 ms after it is shown, and a target can still
  // be moving, or not drawn yet: the ring waits for it to stand still.
  const wait = (ms) => new Promise((done) => setTimeout(done, ms));
  await wait(60);
  for (let n = 0, was = null; step.target && n < 12 && live(); n++) {
    const r = tourTarget(step.target);
    const now = r ? [r.left, r.top, r.right, r.bottom].join() : null;
    if (now !== null && now === was) break;
    was = now;
    await wait(50);
  }
  if (!live()) return;
  paintTour();
  t.root.querySelector('.tour-next').focus();
}

/* What each step needs on screen. Each sets up all of it, whichever step it
   is reached from, so Back works as Next does, and stops as soon as `live`
   says the tour has moved on. */
async function tourEnter(key, t, live) {
  const m = t.model;
  // The model's file in front, and with `canvas`, its lineage on the canvas:
  // a step that draws a selection there has no use for it first.
  const onModel = async ({ canvas = true } = {}) => {
    if (!m) return;
    if (S.active !== m.file) {
      // A tab already open is brought forward as it is, a preview left one.
      if (S.open.has(m.file)) activate(m.file, false);
      else await openFile(m.file, { preview: true, focusLineage: false });
    }
    if (canvas && live() && (S.graphMode !== 'model' || S.focus !== m.id)) await focusNode(m.id);
  };
  const selection = async () => {
    showDock('lineage');
    if (S.graphMode === 'select' && $('#select-input').value === `+${m.name}+`) return;
    $('#select-input').value = `+${m.name}+`;
    await runSelection();
  };
  switch (key) {
    case 'editor':
      await onModel();
      break;
    case 'lineage':
      await onModel();
      if (live()) showDock('lineage');
      break;
    case 'select':
    case 'share':
      await onModel({ canvas: false });
      if (live()) await selection();
      break;
    case 'catalog':
      await onModel();
      if (!live()) break;
      if (S.catTab !== 'preview') showCatalogTab(S.node, 'preview');
      showDock('catalog');
      break;
    case 'columns':
      if (!m) break;
      await onModel();
      if (!live()) break;
      if (S.catTab !== 'columns') showCatalogTab(S.node, 'columns');
      showDock('catalog');
      break;
    case 'jobs':
      await onModel();
      // The jobs as last read, which /api/tour has just read if nothing had:
      // the tour never asks to look at the job files again (0063, 0075).
      if (live()) await showDock('jobs', { look: false });
      break;
    case 'config': {
      await onModel();
      const button = live() && $('#crumbs .cfg');
      if (!button) break;
      // Opened, so the step shows the keys themselves, but not to be used
      // from the tour: a key picked would open dbt_project.yml under it.
      await openCrumbMenu(button, { kind: 'config', file: S.active });
      if (!live()) closeCrumbMenu();
      else if (crumbMenu) crumbMenu.el.inert = true;
      break;
    }
  }
}

/* The box around a step's target, one selector or several: every element
   found and drawn, as one box. Null when none is on screen. */
function tourTarget(target) {
  const boxes = [].concat(target).map((sel) => document.querySelector(sel)).filter(Boolean)
    .map((el) => el.getBoundingClientRect()).filter((r) => r.width > 0 && r.height > 0);
  if (!boxes.length) return null;
  return {
    left: Math.min(...boxes.map((r) => r.left)), top: Math.min(...boxes.map((r) => r.top)),
    right: Math.max(...boxes.map((r) => r.right)), bottom: Math.max(...boxes.map((r) => r.bottom)),
  };
}

function paintTour() {
  const t = S.tour;
  if (!t || t.at < 0 || t.ending) return;
  const step = t.steps[t.at];
  const root = t.root;
  root.querySelector('.tour-count').textContent = `${t.at + 1} of ${t.steps.length}`;
  root.querySelector('#tour-title').textContent = step.title;
  const text = root.querySelector('.tour-text');
  text.textContent = '';
  for (const [, plain, code, key] of step.text.matchAll(/([^`{]+)|`([^`]*)`|\{([^}]*)\}/g)) {
    if (plain) text.append(plain);
    else text.append(Object.assign(document.createElement(code !== undefined ? 'code' : 'kbd'), { textContent: code !== undefined ? code : key }));
  }
  root.querySelector('.tour-back').disabled = t.at === 0;
  root.querySelector('.tour-next').disabled = false;
  root.querySelector('.tour-next').textContent = t.at === t.steps.length - 1 ? 'Done' : 'Next';
  const at = step.target ? tourTarget(step.target) : null;
  const view = { width: window.innerWidth, height: window.innerHeight };
  const shown = !!at;
  root.classList.toggle('centre', !shown);
  const ring = root.querySelector('.tour-ring');
  const card = root.querySelector('.tour-card');
  ring.classList.toggle('hidden', !shown);
  if (!shown) {
    card.style.left = card.style.top = '';
    return;
  }
  const r = tourRing(at, view);
  Object.assign(ring.style, { left: r.left + 'px', top: r.top + 'px', width: r.width + 'px', height: r.height + 'px' });
  const box = card.getBoundingClientRect();
  const place = tourPlace(r, { width: box.width, height: box.height }, view);
  card.style.left = place.left + 'px';
  card.style.top = place.top + 'px';
}

/* Puts back what the tour moved, once every step still setting the page up
   has stopped. A page with no tab open keeps the model, in the editor, the
   lineage and the Catalog, as a place to start from. Any other gets back its
   tabs in their order, a preview the tour's took the place of included, the
   tab in front, the canvas, the node the dock's tabs speak of, the Catalog,
   and the Reports panel if it was down. */
async function endTour() {
  const t = S.tour;
  if (!t || t.ending) return;
  // The overlay stays, inert under it, until the page is back: a click while
  // it is put back would be undone by it, and a tour started meanwhile would
  // note a page half restored. Every step still setting the page up stops at
  // its next look.
  t.ending = true;
  t.ask++;
  if (t.root) {
    t.root.classList.add('centre');
    t.root.querySelector('.tour-ring').classList.add('hidden');
    const card = t.root.querySelector('.tour-card');
    card.style.left = card.style.top = '';
    t.root.querySelector('.tour-text').textContent = 'Putting the page back…';
    for (const b of t.root.querySelectorAll('button')) b.disabled = true;
  }
  await t.chain;
  closeCrumbMenu();
  const s = t.saved;
  const m = t.model;
  if (s.active) {
    if (m && !s.order.includes(m.file) && S.open.has(m.file) && !S.open.get(m.file).dirty) dropTab(m.file);
    if (s.preview && !S.open.has(s.preview)) await openFile(s.preview, { preview: true, focusLineage: false });
    S.order = s.order.filter((p) => S.open.has(p)).concat(S.order.filter((p) => !s.order.includes(p)));
    if (S.open.has(s.active)) activate(s.active, false);
    else settleTabs();
    $('#select-input').value = s.select;
    // A tour that moved to a model of its own drew it at the default depth.
    const depth = $('#up').value !== s.up || $('#down').value !== s.down;
    $('#up').value = s.up;
    $('#down').value = s.down;
    if (s.mode === 'select') await runSelection();
    else if (s.mode === 'column' && s.colFocus) await focusColumn(s.colFocus.id, s.colFocus.column);
    else if (s.focus) {
      if (S.focus !== s.focus || S.graphMode !== 'model' || depth) await focusNode(s.focus, { keepDepth: true });
    } else {
      S.graphMode = 'model';
      paintMode();
      Lineage.clear();
      emptyHint('Select a model to see its lineage.');
      $('#lineage-status').textContent = '';
    }
    // Whatever the canvas shows, Jobs, Compiled and Run speak of these.
    Object.assign(S, { focus: s.focus, colFocus: s.colFocus, jobsFor: s.jobsFor, unsynced: s.unsynced });
    if ((S.node && S.node.id) !== (s.node && s.node.id)) {
      S.node = s.node;
      $('#catalog').replaceChildren(...s.catalog);
    }
  } else if (m && S.graphMode !== 'model') {
    // An end on a selection leaves the model's lineage, as the editor does.
    await focusNode(m.id);
  }
  $('#select-input').value = s.select;
  S.selectHistory = s.history;
  S.selectHistAt = -1;
  if (S.catTab !== s.catTab && S.node) showCatalogTab(S.node, s.catTab);
  else S.catTab = s.catTab;
  showDock(s.dock, { look: false });
  S.tour = null;
  window.removeEventListener('keydown', tourKey, true);
  window.removeEventListener('resize', paintTour);
  if (t.root) t.root.remove();
  for (const id of ['#topbar', '#body', '#statusbar']) $(id).inert = false;
  if (s.reports) {
    await openReports(s.reportsSection, { reread: false });
    // Where Escape in the panel sends the focus back, as before the tour.
    S.reportsReturn = s.reportsReturn;
  } else if (s.back && s.back.focus && document.body.contains(s.back)) s.back.focus();
  // A manifest that landed during the tour was not asked about (0078).
  if (S.tourOfferHeld && !S.tour) offerTour();
}

/* Once on a machine, the first page to ask with a model to show offers the
   tour beside the ? button, which is where it stays (0074). Nothing opens: it
   waits for an answer, and the server has already noted that it was made.
   Without a model the server notes nothing and the offer is held, to be asked
   for again when a manifest lands (0078). Never while a tour is on screen:
   the server notes the offer as it answers, and a card nobody can see would
   spend it, so the tour's end asks instead. */
async function offerTour() {
  if (S.tour) {
    S.tourOfferHeld = true;
    return;
  }
  const was = S.tourOfferHeld;
  S.tourOfferHeld = false;
  const at = S.meta && S.meta.manifest_mtime;
  let r;
  try { r = await api.send('/api/tour/offer', 'POST', {}); } catch {
    // Not asked, so still to ask at the next manifest.
    S.tourOfferHeld = was;
    return;
  }
  if (r && r.held) {
    S.tourOfferHeld = true;
    // A manifest that landed while the server answered was not asked about.
    if (S.meta && S.meta.manifest_mtime !== at) offerTour();
    return;
  }
  if (!r || !r.offer || S.tour) return;
  const box = jel('div', 'tour-offer');
  box.setAttribute('role', 'dialog');
  box.setAttribute('aria-label', 'A tour of dbt-edith');
  const go = jel('button', 'btn sm primary', 'Give me a tour');
  go.type = 'button';
  const later = jel('button', 'btn sm', 'Not now');
  later.type = 'button';
  // Under the ? button, its arrow on the button's middle, until it goes:
  // answered, or taken away by a tour started from the shortcut list.
  const place = () => {
    if (!box.isConnected) return window.removeEventListener('resize', place);
    const at = $('#keys-btn').getBoundingClientRect();
    const width = box.offsetWidth;
    const left = Math.max(8, Math.min(at.left + at.width / 2 - width + 24, window.innerWidth - width - 8));
    box.style.left = left + 'px';
    box.style.top = (at.bottom + 10) + 'px';
    box.style.setProperty('--arrow-x', (at.left + at.width / 2 - left) + 'px');
  };
  const close = () => {
    box.remove();
    window.removeEventListener('resize', place);
  };
  go.addEventListener('click', () => { close(); startTour(); });
  later.addEventListener('click', close);
  box.addEventListener('keydown', (e) => {
    if (e.key !== 'Escape') return;
    e.stopPropagation();
    close();
  });
  const actions = jel('div', 'tour-actions');
  actions.append(jel('div', 'grow'), later, go);
  box.append(
    jel('b', '', 'New to dbt-edith?'),
    jel('p', '', 'A tour of what it does, on this project. It runs nothing and changes no setting.'),
    actions,
    jel('p', 'tour-later', `Not now leaves it in the shortcut list, under ${keyLabel('?', IS_MAC)}.`),
  );
  document.body.append(box);
  place();
  window.addEventListener('resize', place);
}

// ------------------------------------------------------------------ keys --
/* A keydown, named the way the shortcut list names it: Mod+Alt+S, Shift+Enter,
   ?. Mod is Cmd or Ctrl, as it always was here. Shift is named for a letter or
   a named key only: for a symbol it is already in the character, which is how
   `?` stays `?` on a layout that types it with Shift and on one that does not.

   On a Mac, Option makes a letter into another character, Option+W into ∑, so
   there the physical key is read instead. Not when Option alone typed a
   letter, though: that is someone writing Â on a French keyboard, and closing
   their tab would be the wrong answer to it. */
function keyCombo(e, mac) {
  let key = e.key || '';
  const letter = /^Key([A-Z])$/.exec(e.code || '');
  if (mac && e.altKey && letter && (e.metaKey || e.ctrlKey || !/^\p{L}$/u.test(key))) key = letter[1];
  if (/^[a-z]$/.test(key)) key = key.toUpperCase();
  if (key === ' ') key = 'Space';
  const mods = [];
  if (e.metaKey || e.ctrlKey) mods.push('Mod');
  if (e.altKey) mods.push('Alt');
  if (e.shiftKey && (/^[A-Z]$/.test(key) || key.length > 1)) mods.push('Shift');
  return mods.concat(key).join('+');
}

/* How a key reads on this machine: ⌥⌘S on a Mac, in Apple's order, and
   Ctrl+Alt+S elsewhere. */
function keyLabel(combo, mac) {
  const parts = combo.split('+');
  const key = parts.pop();
  const k = {
    ArrowUp: '↑', ArrowDown: '↓', ArrowLeft: '←', ArrowRight: '→', Escape: 'Esc', Enter: mac ? '↩' : 'Enter',
    Backspace: mac ? '⌫' : 'Backspace', Delete: mac ? '⌦' : 'Delete',
  }[key] || key;
  if (!mac) return parts.map((m) => (m === 'Mod' ? 'Ctrl' : m)).concat(k).join('+');
  const glyphs = [['Ctrl', '⌃'], ['Alt', '⌥'], ['Shift', '⇧'], ['Mod', '⌘']];
  const mods = glyphs.filter(([m]) => parts.includes(m)).map(([, g]) => g).join('');
  // A mouse action is a word, which reads apart from the glyphs: ⌥ click.
  return mods + (mods && /^[a-z]/.test(k) ? ' ' : '') + k;
}

/* Every shortcut there is, for the list ? opens: each row is the keys, any of
   which does it, and what it does. The keys wireKeys answers to are checked
   against this by web/tests/keys.js, so none is bound without appearing here;
   the rest are answered by CodeMirror or by one box, and are listed by hand.
   `os` is KEYS_OS: the terminal's line keys differ on each (0073). */
function shortcutSheet(os) {
  return [
    { title: 'Anywhere', keys: [
      [['Mod+K'], 'Search models, sources and every file'],
      [['Mod+S'], 'Save the file'],
      [['Mod+Alt+S'], 'Save every modified file'],
      [['Mod+Alt+X', 'Alt+W'], 'Close the tab'],
      [['Mod+Alt+Shift+X'], 'Close every tab'],
      [['Mod+Alt+P'], 'Back to the link you followed'],
      [['Mod+Alt+N'], 'Forward again'],
      [['Mod+`'], 'Show the terminal'],
      [['Mod+Alt+R'], 'Open or close the reports'],
      [['?', 'F1'], 'This list: ? outside a text box, F1 from anywhere'],
    ] },
    { title: 'Reports', keys: [
      [['Escape'], 'Close them, from inside'],
      [['Mod+Alt+X', 'Alt+W'], 'Close them too, rather than the tab under them'],
    ] },
    { title: 'Find', keys: [
      [['Mod+F'], 'In a file, or in Compiled or Run: find in that text'],
      [['Mod+F'], 'In the Catalog: filter the columns by name'],
      [['Enter', 'Shift+Enter'], 'Next and previous match, from the find box'],
      [['Mod+G', 'Mod+Shift+G'], 'Next and previous match, from the text too'],
      [['F3', 'Shift+F3'], 'The same, as in VS Code'],
      [['Escape'], 'Close the find bar, or empty the column filter'],
    ] },
    { title: 'Editor', keys: [
      [['Mod+Z', 'Mod+Shift+Z'], 'Undo and redo'],
      [['Mod+['], 'Indent less'],
      [['Mod+]'], 'Indent more'],
      [['Mod+D'], 'Delete the line, where VS Code would select the next match'],
      [['Alt+click'], 'Put the cursor on a link without following it'],
    ] },
    { title: 'Tabs', keys: [
      [['Middle-click'], 'Close a tab'],
      [['Double-click'], 'Keep a preview tab, the one in italics'],
    ] },
    { title: 'Search box, branches and menus', keys: [
      [['ArrowUp', 'ArrowDown'], 'Move'],
      [['Enter'], 'Open or pick'],
      [['Escape'], 'Close'],
    ] },
    { title: 'Custom selection box, above the lineage', keys: [
      [['Enter'], 'Draw it now'],
      [['ArrowUp', 'ArrowDown'], 'Earlier expressions'],
    ] },
    { title: 'Git', keys: [
      [['Mod+Enter'], 'Commit, from the message box'],
    ] },
    { title: 'Terminal', keys: [
      [['Mod+C'], 'Copy the selection; Ctrl+C with nothing selected interrupts'],
      [['Mod+V'], 'Paste'],
      ...termLineRows(os),
    ] },
    { title: 'Catalog', keys: [
      [['Enter', 'Space'], 'On a +N chip: every test on that column'],
    ] },
  ];
}

/* The terminal's line keys, as termLineKey answers them, for the shortcut
   list: on a Mac in any shell, on Windows in Git Bash, and none elsewhere,
   where the shell reads xterm.js's own. */
function termLineRows(os) {
  if (os === 'mac') return [
    [['Alt+ArrowLeft', 'Alt+ArrowRight'], 'A word back and forward'],
    [['Mod+ArrowLeft', 'Mod+ArrowRight'], 'Start and end of the command'],
    [['Mod+Backspace'], 'Delete the command back to its start, all of it in zsh'],
    [['Alt+Backspace', 'Alt+Delete'], 'Delete the word before and after'],
  ];
  if (os === 'windows') return [
    [['Mod+ArrowLeft', 'Mod+ArrowRight'], 'A word back and forward, in Git Bash'],
    [['Home', 'End'], 'Start and end of the command, in Git Bash'],
    [['Mod+Backspace', 'Mod+Delete'], 'Delete the word before and after, in Git Bash'],
  ];
  return [];
}

/* Whether a key pressed here is text: a box, the editor, the terminal. `?` is
   a shortcut only outside all three. */
function isTyping(el) {
  if (!el || !el.tagName) return false;
  if (el.isContentEditable || el.tagName === 'TEXTAREA' || el.tagName === 'SELECT') return true;
  return el.tagName === 'INPUT' && !/^(checkbox|radio|button|submit|range)$/.test(el.type);
}

/* The shortcut list: ? outside a text box, F1 anywhere, or the button in the
   top bar. Cmd+H was the first idea and cannot work: on a Mac, the system
   hides the browser before any page sees the key (0030). */
function toggleShortcuts() {
  const built = $('#shortcuts');
  if (built && !built.classList.contains('hidden')) return closeShortcuts();
  const box = built || buildShortcuts();
  S.keysReturn = document.activeElement;
  box.classList.remove('hidden');
  box.querySelector('.keys-box').focus();
}

/* Focus goes back to where the keys were typed, the editor usually. */
function closeShortcuts() {
  const box = $('#shortcuts');
  if (!box || box.classList.contains('hidden')) return;
  box.classList.add('hidden');
  const back = S.keysReturn;
  S.keysReturn = null;
  if (back && back.focus && document.body.contains(back)) back.focus();
}

function buildShortcuts() {
  const box = document.createElement('div');
  box.id = 'shortcuts';
  box.className = 'hidden';
  const panel = document.createElement('div');
  panel.className = 'keys-box' + (IS_MAC ? '' : ' pc');
  panel.tabIndex = -1;
  panel.setAttribute('role', 'dialog');
  panel.setAttribute('aria-label', 'Keyboard shortcuts');
  const head = document.createElement('div');
  head.className = 'keys-head';
  const close = document.createElement('button');
  close.className = 'icon';
  close.textContent = '×';
  close.title = `Close (${keyLabel('Escape', IS_MAC)})`;
  close.addEventListener('click', closeShortcuts);
  // The tour lives here, where someone looking for what the page can do looks (0074).
  const tour = document.createElement('button');
  tour.type = 'button';
  tour.className = 'btn sm primary';
  tour.textContent = 'Give me a tour';
  tour.title = 'What each part of the page is for, on a model of this project. It runs nothing.';
  tour.addEventListener('click', startTour);
  head.append(Object.assign(document.createElement('h2'), { textContent: 'Keyboard shortcuts' }),
    Object.assign(document.createElement('div'), { className: 'grow' }), tour, close);
  const grid = document.createElement('div');
  grid.className = 'keys-grid';
  for (const group of shortcutSheet(KEYS_OS)) {
    const sec = document.createElement('section');
    sec.append(h3(group.title));
    for (const [combos, what] of group.keys) {
      const row = document.createElement('div');
      row.className = 'keys-row';
      const keys = document.createElement('span');
      keys.className = 'keys-k';
      for (const combo of combos) keys.append(Object.assign(document.createElement('kbd'), { textContent: keyLabel(combo, IS_MAC) }));
      row.append(keys, Object.assign(document.createElement('span'), { className: 'keys-what', textContent: what }));
      sec.append(row);
    }
    grid.append(sec);
  }
  // The question the list raises first: why not the key every other app uses.
  const note = document.createElement('p');
  note.className = 'keys-note';
  note.textContent = IS_MAC
    ? '⌘W and ⌘H belong to the browser and to macOS, never to a page: one closes the browser tab, '
      + 'the other hides the window. Hence ⌥W to close a tab here, which a French keyboard cannot '
      + 'type at all: there the same command answers to ⌥⌘X, closing every tab to ⌥⇧⌘X, and back '
      + 'and forward ask for ⌘ for the same reason (0037). ? or F1 opens this list.'
    : 'Ctrl+W belongs to the browser, never to a page: it closes the browser tab. Hence Alt+W to close a tab here.';
  panel.append(head, grid, note);
  box.append(panel);
  box.addEventListener('mousedown', (e) => { if (e.target === box) closeShortcuts(); });
  // Only the list closes: the search box or a dialog under it stays open.
  panel.addEventListener('keydown', (e) => {
    if (e.key !== 'Escape') return;
    e.preventDefault();
    e.stopPropagation();
    closeShortcuts();
  });
  document.body.append(box);
  return box;
}

/* What a close key closes: the reports when they are down, since the tab
   under them cannot be seen, and the tab otherwise. */
function closeFront() {
  if (S.reportsOpen) closeReports({ restore: true });
  else if (S.active) closeFile(S.active);
}

function wireKeys() {
  window.addEventListener('keydown', (e) => {
    switch (keyCombo(e, IS_MAC)) {
      case 'Mod+Alt+S': e.preventDefault(); saveAll(); return;
      case 'Mod+S': e.preventDefault(); save(); return;
      /* X for the cross on the tab, and in the same place on every layout.
         Alt+W stays bound for the keyboards it reaches, but a French Mac is
         not one: there the key marked W is `KeyZ`, and the key at `KeyW` is
         marked Z and writes a letter under Option, which keyCombo leaves alone
         on purpose (0030, 0037). Alt+Shift+W is not kept beside it: it sat one
         modifier from Shift+Cmd+W, which closes the browser window. */
      case 'Mod+Alt+X': e.preventDefault(); closeFront(); return;
      case 'Mod+Alt+Shift+X': e.preventDefault(); closeAll(); return;
      case 'Alt+W': e.preventDefault(); closeFront(); return;
      /* P for previous and N for next, and Cmd is not decoration: Option alone
         with a letter is left as typed, since on some layouts it writes one
         (keyCombo). Option+B and Option+N were tried and wrote into the file on
         a keyboard where both produce a letter. Cmd is what makes keyCombo read
         the physical key, as it already does for Mod+Alt+S. Not Alt+Left and
         Alt+Right either, which CodeMirror binds in both its keymaps, nor
         Mod+Alt+Left and Mod+Alt+Right, which Chrome takes for its tabs. */
      case 'Mod+Alt+P': e.preventDefault(); jumpBack(); return;
      case 'Mod+Alt+N': e.preventDefault(); jumpForward(); return;
      case 'Mod+K': e.preventDefault(); openPalette(); return;
      case 'Mod+`': e.preventDefault(); closeReports(); showDock('terminal'); return;
      /* R for reports, in the same place on every layout, with Cmd so that
         keyCombo reads the key and not what Option typed (0037). */
      case 'Mod+Alt+R': e.preventDefault(); toggleReports(); return;
      // An editor with the focus has answered this already, and said so.
      case 'Mod+F':
        if (!e.defaultPrevented && findInPane(paneOf(e.target) || S.lastPane)) e.preventDefault();
        return;
      // Not from the terminal, where F1 belongs to whatever runs in it.
      case 'F1':
        if (e.target.closest && e.target.closest('#term')) return;
        e.preventDefault();
        toggleShortcuts();
        return;
      case '?':
        if (isTyping(e.target)) return;
        e.preventDefault();
        toggleShortcuts();
        return;
    }
    if (e.key === 'Escape' && !$('#palette').classList.contains('hidden')) closePalette();
  });
  // What Cmd+F searches when nothing has the focus: the pane last used, never a
  // dialog or the top bar.
  const lastPane = (e) => {
    const pane = paneOf(e.target);
    if (pane && pane !== 'other') S.lastPane = pane;
  };
  document.addEventListener('pointerdown', lastPane, true);
  document.addEventListener('focusin', lastPane);
  $('#keys-btn').addEventListener('click', toggleShortcuts);
  $('#palette-input').addEventListener('input', (e) => runPalette(e.target.value));
  $('#palette-input').addEventListener('keydown', (e) => {
    if (e.key === 'ArrowDown') { e.preventDefault(); movePalette(1); }
    else if (e.key === 'ArrowUp') { e.preventDefault(); movePalette(-1); }
    else if (e.key === 'Enter') { e.preventDefault(); choosePalette(); }
  });
  $('#palette').addEventListener('click', (e) => { if (e.target.id === 'palette') closePalette(); });
  $('#palette-btn').addEventListener('click', openPalette);
  $('#model-filter').addEventListener('input', () => {
    clearTimeout(modelTimer);
    modelTimer = setTimeout(refreshModels, 120);
  });
  $('#branch-btn').addEventListener('click', openBranches);
  $('#status-branch').addEventListener('click', openBranches);
  $('#git-fetch').addEventListener('click', async () => {
    const r = await gitPost('fetch');
    if (r && !r.ok) showGitFailure('Fetch failed', r); else toast('fetched', 'ok');
  });
  $('#git-pull').addEventListener('click', doPull);
  $('#git-push').addEventListener('click', doPush);
  $('#commit-btn').addEventListener('click', doCommit);
  $('#commit-msg').addEventListener('keydown', (e) => {
    if ((e.metaKey || e.ctrlKey) && e.key === 'Enter') { e.preventDefault(); doCommit(); }
  });
  $('#branch-filter').addEventListener('input', (e) => paintBranches(e.target.value));
  $('#branch-filter').addEventListener('keydown', (e) => {
    if (e.key === 'ArrowDown') { e.preventDefault(); moveBranch(1); }
    else if (e.key === 'ArrowUp') { e.preventDefault(); moveBranch(-1); }
    else if (e.key === 'Enter') {
      e.preventDefault();
      const b = (branchHits.visible || [])[branchIndex];
      if (b) chooseBranch(b);
    } else if (e.key === 'Escape') $('#branches').classList.add('hidden');
  });
  $('#branches').addEventListener('click', (e) => { if (e.target.id === 'branches') e.currentTarget.classList.add('hidden'); });
  $('#save-all-btn').addEventListener('click', (e) => { e.stopPropagation(); saveAll(); });
  $('#close-all-btn').addEventListener('click', (e) => { e.stopPropagation(); closeAll(); });
  $('#collapse-btn').addEventListener('click', collapseTree);
  for (const sel of ['#tabbar', '#oe-list']) {
    $(sel).addEventListener('contextmenu', (e) => {
      const el = e.target.closest('[data-key]');
      if (!el) return;
      e.preventDefault();
      openTabMenu(el, el.dataset.key);
    });
  }
  $('#reveal-btn').addEventListener('click', revealActive);
  $('#oe-head').addEventListener('click', () => {
    const box = $('#open-editors');
    box.classList.toggle('collapsed');
    $('#oe-caret').textContent = box.classList.contains('collapsed') ? '\u25b8' : '\u25be';
  });
  $('#reload-btn').addEventListener('click', async () => {
    try {
      const meta = await api.send('/api/reload', 'POST', {});
      applyMeta(meta);
      toast(`manifest reloaded in ${meta.load_ms} ms`, 'ok');
      if (!await followActive()) rerender();
      refreshFreshness();
    } catch (e) { toast('reload failed: ' + e.message, 'err'); }
  });
  // The badge says the manifest is behind; the command that fixes that is the
  // one thing the user would type next. dbt still runs in their terminal, under
  // their environment, never from the server (0002).
  $('#fresh-badge').addEventListener('click', () => sendToTerminal('dbt parse', true));
  $('#reports-btn').addEventListener('click', toggleReports);
  // On the panel rather than the document: a dialog over it answers its own
  // Escape there, and must not take the panel with it.
  $('#reports').addEventListener('keydown', (e) => {
    if (e.key !== 'Escape' || e.defaultPrevented) return;
    e.preventDefault();
    e.stopPropagation();
    closeReports({ restore: true });
  });
  // Keyed on the state, so a card being read is not rebuilt under the pointer
  // by a poll that changed nothing.
  $('#fresh-badge').addEventListener('mouseenter', (e) => {
    if (!S.fresh) return;
    const at = e.currentTarget;
    hoverEnter('fresh:' + S.freshKey, () => at.getBoundingClientRect(), (el) => fillFreshCard(el, S.fresh));
  });
  $('#fresh-badge').addEventListener('mouseleave', hoverLeave);
}

// ------------------------------------------------------------------ boot --
function applyMeta(meta) {
  // Every caller of this has just learned the graph changed, which is exactly
  // when a cached /api/node payload stops being true. Here rather than at each
  // call site: forgetting one is how a stale count survives a column fetch.
  dropNodeCache();
  // What a step selects follows the manifest alone: a switch of column lineage,
  // a feature or a collin poll brings the same one again.
  const moved = !S.meta || S.meta.manifest_mtime !== meta.manifest_mtime;
  S.meta = meta;
  $('#project').textContent = meta.project || '(no manifest)';
  $('#tree-title').textContent = meta.project || 'Project';
  const c = meta.counts || {};
  $('#counts').textContent = '';
  for (const k of ['model', 'source', 'seed', 'snapshot', 'test']) {
    if (!c[k]) continue;
    const s = document.createElement('span');
    s.className = 'chip';
    s.textContent = `${c[k]} ${k}${c[k] > 1 ? 's' : ''}`;
    $('#counts').appendChild(s);
  }
  const when = meta.manifest_mtime ? new Date(meta.manifest_mtime * 1000).toLocaleString() : 'missing';
  $('#status-manifest').textContent = `dbt ${meta.dbt_version || '?'} · manifest ${when}`;
  // A reload can change the count of edges the menu's button names.
  paintSourceButton();
  // And which nodes each job's selection reaches.
  if (S.dock === 'jobs') loadJobs();
  if (moved) forgetCatalogJobs();
  // And the coverage the reports would show, which the index stamps with it.
  loadReportsIndex();
  // A first manifest may have a model to show the tour on (0078).
  if (moved && S.tourOfferHeld) offerTour();
}

/* A whole /api/meta answer: the graph's facts, then where its column lineage
   comes from, which the menu in the top bar names in place of a count. */
function applyInfo(info) {
  applyMeta(info.meta);
  applyLineage(info);
}

/* One click on a box opens its file as the preview, the way one click in the
   tree does: walking the graph leaves no tab behind, and the canvas stays where
   it is. Two clicks pin it and centre the lineage on it (onOpen). The move is
   recorded, so Back returns to a pinned tab it left, but it pins nothing: a
   preview it leaves is replaced, as the next click in the tree would. */
function previewBox(n) {
  // A generic test lands on its line, in the file already shown too (0041).
  if (n.kind === 'test' && n.test_name) return openTest(n, { focus: false });
  if (!n.file || n.file === S.active) return;
  pushJump({ pin: false });
  openFile(n.file, { focusLineage: false, preview: true });
  revealInTree(n.file);
}

/* Two clicks on a box, a model's or one of its columns': the first already
   opened the file as the preview and recorded where it came from, so this one
   pins it in place. */
function pinBoxFile(n) {
  if (!n.file) return;
  if (S.active !== n.file) pushJump();
  openFile(n.file, { focusLineage: false });
  revealInTree(n.file);
}

/* What the status bar offers for an /api/update answer (0068): nothing until
   a later release is known, then a pill naming it, and a menu with its notes
   and, for a binary built from a clone on this machine, the command that
   pulls and rebuilds it, or why there is none. Pure, for web/tests/update.js. */
function updateOffer(u) {
  if (!u || !u.newer) return null;
  const how = u.how || null;
  const lines = [];
  if (how) {
    lines.push(how.kind === 'build'
      ? `It pulls main into ${how.source} and builds it again with cargo build --release, a minute or two.`
      : `It pulls main into ${how.source} and installs it again with cargo install, a minute or two.`);
    if (how.renames) {
      lines.push(`${how.exe} steps aside as ${how.exe}.old first, since Windows will not replace a running .exe, `
        + 'and comes back if the build fails.');
    }
    lines.push(`Then stop dbt-edith and start it again: until then, ${u.current} serves this page.`);
  } else if (u.why === 'locked') {
    lines.push('This dbt-edith runs from its clone\'s target/release, which a build cannot replace on Windows '
      + 'while it runs: stop it, then pull and build it again.');
  } else if (u.why === 'shell') {
    lines.push('The update is typed for sh, bash, zsh and PowerShell, and this terminal runs another shell: '
      + 'pull the clone and install it again by hand, as the README says.');
  } else if (u.why === 'path') {
    lines.push('A path in the update holds a character a shell would read inside quotes, so none is typed: '
      + 'pull the clone and install it again by hand, as the README says.');
  } else {
    lines.push('This dbt-edith is neither in cargo\'s bin nor in the target/release of a clone on this machine, '
      + 'so there is no command to offer: pull and rebuild it where it was built, as the README says.');
  }
  return {
    label: `${u.latest} available`,
    title: `dbt-edith ${u.latest} is out, and this is ${u.current}`,
    head: `dbt-edith ${u.latest} · this is ${u.current}`,
    notes: u.notes || '',
    command: how ? how.command : '',
    lines,
  };
}

/* When a look was made or is due, as the menu says it: today, yesterday or
   tomorrow and the time, or the date and the time. `t` and `now` in seconds
   since the epoch, read in the machine's own time zone, so a look the server
   times in Paris reads in local time. Pure, for web/tests/update.js. */
function whenLooked(t, now) {
  const at = new Date(t * 1000);
  const two = (n) => String(n).padStart(2, '0');
  const time = `${two(at.getHours())}:${two(at.getMinutes())}`;
  const midnight = (d) => new Date(d.getFullYear(), d.getMonth(), d.getDate()).getTime();
  // Rounded, since a day the clocks change on lasts 23 or 25 hours.
  const days = Math.round((midnight(new Date(now * 1000)) - midnight(at)) / 86400000);
  if (days === 0) return `today at ${time}`;
  if (days === 1) return `yesterday at ${time}`;
  if (days === -1) return `tomorrow at ${time}`;
  const month = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'][at.getMonth()];
  return `${at.getDate()} ${month} at ${time}`;
}

/* What the menu says of the looks for a newer dbt-edith (0068, 0079): one on
   its way, when the answer came and what it was, what git said when a later
   one got none, and when the next is due, which the server works out from the
   Paris clock. Pure, for web/tests/update.js. */
function updateLook(u, now) {
  if (!u) return '';
  if (u.off) return 'Started with --no-update-check, so no release is looked for.';
  if (u.checking) return 'Looking for a newer release on GitHub…';
  const said = [];
  if (u.failed) said.push(`The look ${whenLooked(u.failed, now)} got no answer from GitHub: ${u.error || 'git said nothing'}.`);
  if (u.checked) {
    const when = whenLooked(u.checked, now);
    if (u.newer) said.push(`GitHub listed ${u.latest} ${when}.`);
    else if (u.latest === u.current) said.push(`${u.current} is the newest release, as GitHub listed it ${when}.`);
    // A build from a branch, or a release not yet tagged when GitHub answered.
    else said.push(`This build is ahead of ${u.latest}, the newest release GitHub listed ${when}.`);
  } else if (!u.failed) {
    said.push('Not looked for yet.');
  }
  if (u.next > now) said.push(`The next look is ${whenLooked(u.next, now)}.`);
  else if (u.next) said.push('The next look is due now.');
  return said.join(' ');
}

/* The releases since the running one, as the menu lists them (0079): a head
   counting them all, one row per release with its pull request's title, and
   a last row for the older ones the server left out. Null without the list,
   when the menu links the newest release's notes alone. Pure, for
   web/tests/update.js. */
function releaseRows(u) {
  if (!u || !u.newer || !(u.releases || []).length) return null;
  const all = u.releases.length + (u.older || 0);
  return {
    head: `${all} release${all === 1 ? '' : 's'} since yours`,
    rows: u.releases.map((r) => ({ version: r.version, title: r.title, href: r.notes, tip: `dbt-edith ${r.version}: ${r.title}` })),
    older: u.older && u.older_notes
      ? { label: `and ${u.older} older`, href: u.older_notes, tip: 'Every release, on GitHub' } : null,
  };
}

/* Edith's menu, from the name at the top left and from the status bar's pill
   alike: which build serves the page, a newer release and how to install it
   once one is known (0068), and the looks for one, with a button that makes
   one at once (0079). */
function openUpdateMenu(anchor) {
  if (closePopup(anchor)) return;
  const menu = document.createElement('div');
  menu.className = 'envmenu updmenu';
  menu.setAttribute('role', 'menu');
  S.updateMenu = menu;
  paintUpdateMenu(menu);
  openPopup(anchor, menu);
}

/* Drawn again in place when an answer comes in while the menu is open. */
function paintUpdateMenu(menu) {
  const el = (tag, className, textContent = '') => Object.assign(document.createElement(tag), { className, textContent });
  const u = S.update || { current: S.version };
  const o = updateOffer(u);
  // Drawn anew under the focus, at a click and every two seconds of a look:
  // it goes back to the same row, found by its link since an answer can add
  // rows above it, or to the button, without scrolling the menu to it.
  const rows = () => [...menu.querySelectorAll('[role=menuitem]')];
  const had = menu.contains(document.activeElement) ? document.activeElement : null;
  const same = (r) => (had.getAttribute('href') ? r.getAttribute('href') === had.getAttribute('href') : r.className === had.className);
  menu.replaceChildren(el('div', 'menuhead', o ? o.head : `dbt-edith ${S.version || '?'}`));
  if (S.build) menu.append(el('div', 'menunote', S.build));
  if (o) {
    menu.append(document.createElement('hr'));
    if (o.command) {
      const run = el('button', '');
      run.type = 'button';
      run.setAttribute('role', 'menuitem');
      run.title = o.command;
      run.append(el('span', 'lbl', 'Update in the terminal'));
      run.addEventListener('click', () => { closePopup(); sendToTerminal(o.command); });
      menu.append(run);
    }
    const link = (href, className, title) => {
      const a = el('a', className);
      a.setAttribute('role', 'menuitem');
      a.href = href;
      a.target = '_blank';
      a.rel = 'noopener noreferrer';
      if (title) a.title = title;
      // After the click, so the link is still in the page when the browser follows it.
      a.addEventListener('click', () => setTimeout(closePopup));
      return a;
    };
    const listed = releaseRows(u);
    if (listed) {
      menu.append(el('div', 'menuhead', listed.head));
      for (const r of listed.rows) {
        const a = link(r.href, 'menulink relrow', r.tip);
        a.append(el('span', 'ver', r.version), el('span', 'lbl', r.title));
        menu.append(a);
      }
      if (listed.older) {
        const a = link(listed.older.href, 'menulink relrow', listed.older.tip);
        a.append(el('span', 'ver', ''), el('span', 'lbl', listed.older.label), el('span', 'sub', 'GitHub'));
        menu.append(a);
      }
      // The list ends here, and what follows says how to install it.
      menu.append(document.createElement('hr'));
    } else if (o.notes) {
      const a = link(o.notes, 'menulink');
      a.append(el('span', 'lbl', 'What changed'), el('span', 'sub', 'GitHub'));
      menu.append(a);
    }
    for (const line of o.lines) menu.append(el('div', 'menunote', line));
  }
  menu.append(document.createElement('hr'));
  // What the looks found, beside the button that makes one now.
  const foot = el('div', 'updfoot');
  foot.append(el('div', 'menunote', updateLook(u, Math.floor(Date.now() / 1000))));
  if (!u.off) {
    const look = el('button', 'btn updcheck', u.checking ? 'Looking…' : 'Check for updates');
    look.type = 'button';
    look.setAttribute('role', 'menuitem');
    // Not `disabled`, which takes no focus: the focus goes back to it once
    // the menu is drawn anew.
    if (u.checking) look.setAttribute('aria-disabled', 'true');
    look.addEventListener('click', () => { if (!u.checking) checkForUpdate(); });
    foot.append(look);
  }
  menu.append(foot);
  const back = had && (rows().find(same) || menu.querySelector('.updcheck') || rows().pop());
  if (back) back.focus({ preventScroll: true });
}

/* The menu's button. The server answers once GitHub has, so the menu says it
   is looking until then, and is drawn again with the answer. */
async function checkForUpdate() {
  S.update = Object.assign({}, S.update, { checking: true });
  paintUpdate();
  try {
    S.update = await api.send('/api/update/check', 'POST', {});
  } catch (e) {
    S.update = Object.assign({}, S.update, { checking: false });
    toast('looking for a newer dbt-edith: ' + e.message, 'err');
  }
  paintUpdate();
  // Another look was on its way, whose answer this one waits for.
  if (S.update.checking) refreshUpdate();
}

/* The pill in the status bar, the name at the top left, and the menu if it
   is open, from the last /api/update answer. */
function paintUpdate() {
  const o = updateOffer(S.update);
  const pill = $('#status-update');
  pill.classList.toggle('hidden', !o);
  if (o) {
    pill.textContent = o.label;
    pill.title = o.title;
  }
  const brand = $('#brand');
  brand.classList.toggle('newer', !!o);
  brand.title = o ? `${o.title}\nClick for what changed and how to update.`
    : `dbt-edith ${S.version || '?'}${S.build ? '\n' + S.build : ''}\nClick to check for updates.`;
  if (S.updateMenu && S.updateMenu.isConnected) {
    paintUpdateMenu(S.updateMenu);
    if (popup && popup.el === S.updateMenu) placePopup(popup.anchor, S.updateMenu);
  }
}

/* Asked at boot, then every few seconds while a look is on its way, then
   every ten minutes: the server looks on the hour through the Paris night, and
   the page shows the answer soon after. */
async function refreshUpdate() {
  clearTimeout(S.updatePoll);
  try { S.update = await api.get('/api/update'); } catch {
    // A request lost to sleep or the network: asked again in a minute.
    S.updatePoll = setTimeout(refreshUpdate, 60 * 1000);
    return;
  }
  paintUpdate();
  // Switched off, there will be nothing to hear.
  if (S.update.off) return;
  S.updatePoll = setTimeout(refreshUpdate, S.update.checking ? 2000 : 600 * 1000);
}

async function boot() {
  initEditor();
  Lineage.init($('#graph'), {
    onSelect: (n) => {
      const { node: nodeId, column } = splitColId(n.id);
      highlightColumn(column, nodeId);
      if (column) S.catTab = 'columns';
      nodeDetail({ id: nodeId }).then(renderCatalog).catch(() => {});
      // A column's box opens its model's file too: following a column from
      // model to model is reading each one in turn.
      previewBox(n);
    },
    onHover: (n, g) => {
      const { node: nodeId, column } = splitColId(n.id);
      if (column) return;                  // in column mode the box already says everything
      hoverEnter('node:' + nodeId, () => g.getBoundingClientRect(), (el) => fillNodeCard(el, nodeId, n));
    },
    onHoverOut: hoverLeave,
    onHoverClose: closeHoverCard,
    // A fold chip redrew the canvas in place: only the line under it changes.
    onRefold: (d, drawn) => { $('#lineage-status').textContent = canvasStatus(d, drawn); },
    onOpen: (n) => {
      const { node: nodeId, column } = splitColId(n.id);
      if (column) {
        pinBoxFile(n);
        if (sidecarOn()) {
          return api.get('/api/node?id=' + encodeURIComponent(nodeId))
            .then((detail) => openColumn(detail, column))
            .catch((e) => toast('column lineage: ' + e.message, 'err'));
        }
        return focusColumn(nodeId, column);
      }
      focusNode(nodeId, { keepDepth: true });
      if (n.kind === 'test' && n.test_name) return openTest(n, { pin: true });
      pinBoxFile(n);
    },
    onExpand: (dir, n) => {
      if (S.graphMode === 'select') return expandSelection(dir, n);
      const input = dir === 'up' ? $('#up') : $('#down');
      input.value = Math.min(20, +input.value + 1);
      rerender();
    },
    // Where the folder names can sit: below the selector bar, which floats
    // over the top of the canvas in that mode.
    headTop: () => {
      const bar = $('#select-bar');
      return bar.classList.contains('hidden') ? 0 : bar.offsetTop + bar.offsetHeight;
    },
  });
  wireTabs(); wireKeys(); wireSplitters(); relinkTools();
  // The markup says ⌘K, which on the Windows machine this is built for is Ctrl+K.
  const palKey = keyLabel('Mod+K', IS_MAC);
  $$('.k-palette').forEach((k) => { k.textContent = palKey; });
  $('#palette-btn').title = `Search models and files (${palKey})`;
  $('#grep-input').addEventListener('input', runGrep);
  renderTabs();
  paintMode();
  $('#editor-host').style.display = 'none';

  $('#cll-pick-host').append(sourceMenu());
  $('#settings-btn').append(snowflakeIcon());
  $('#profile-btn').append(profileIcon());
  $('#profile-btn').addEventListener('click', () => {
    const p = S.profiles;
    if (p && p.found) openProfiles();
    else if (p) toast(`no profiles.yml at ${p.path}`, 'err');
  });
  paintTestsToggle(false);
  $('#settings-btn').addEventListener('click', (e) => openSettingsMenu(e.currentTarget));
  const info = await api.get('/api/meta');
  applyInfo(info);
  $('#status-shell').textContent = info.shell;
  S.shellName = info.shell_name || '';
  // Which build drew this page. A release binary embeds web/ (0005), so this is
  // what tells a stale binary from a frontend change that really did nothing.
  const build = $('#status-build');
  S.version = info.version || '';
  S.build = info.build || '';
  build.textContent = 'v' + (info.version || '?');
  build.title = `dbt-edith ${info.version || '?'}\n${info.build || 'no build stamp'}`;
  $('#status-update').addEventListener('click', (e) => openUpdateMenu(e.currentTarget));
  $('#brand').addEventListener('click', (e) => openUpdateMenu(e.currentTarget));
  paintUpdate();
  refreshUpdate();
  S.venv = info.venv || {};
  paintVenv();
  // The second click of a double click would type the line twice, or close the
  // menu the first one opened.
  $('#status-venv').addEventListener('click', (e) => { if (e.detail < 2) openVenvMenu(e.currentTarget); });
  document.title = `${info.meta.project || 'dbt-edith'} · dbt-edith`;
  $('#status-env').addEventListener('click', (e) => openEnvMenu(e.currentTarget));
  loadEnvs();
  await loadDir($('#tree'), '', 0);
  refreshGit();
  refreshFreshness();
  // Each round costs the server a git status and a walk of the model folders,
  // for a page nobody sees while its tab is hidden. It waits there instead, and
  // catches up the moment the tab is shown again.
  const poll = () => { if (!document.hidden) { refreshGit(); refreshFreshness(); refreshOpenFiles(); } };
  setInterval(poll, 5000);
  document.addEventListener('visibilitychange', poll);
  window.addEventListener('resize', () => Lineage.fit());
  offerTour();
}

/* What bash said at its prompt (0069), said again at every prompt. A folder no
   venv read is may be one made since startup, so the server reads the project
   again, once per folder; nothing is sent to it. */
function heardVenv(data) {
  if (data === S.termVenv) return;
  S.termVenv = data;
  paintVenv();
  if (!data || venvMatch(S.venv || {}, data) || S.venvAsked === data) return;
  S.venvAsked = data;
  api.get('/api/venv').then((v) => { S.venv = v; paintVenv(); }).catch(() => {});
}

function paintVenv() {
  const { text, title, state, offer, line } = venvStatus(S.venv || {}, S.termVenv);
  const el = $('#status-venv');
  el.textContent = text;
  el.title = !offer.length ? title : title + '\n' + (line
    ? 'click to type the line that activates it in the terminal'
    : 'click to pick one to activate in the terminal');
  el.dataset.state = state;
  el.classList.toggle('act', offer.length > 0);
  // A button only while a click does something: green or grey, it is a status.
  el.tabIndex = offer.length ? 0 : -1;
  el.setAttribute('aria-disabled', String(!offer.length));
  if (offer.length && !line) el.setAttribute('aria-haspopup', 'menu');
  else el.removeAttribute('aria-haspopup');
}

/* A click on an inactive venv types the line that activates it, and Enter
   stays the user's (0071, 0072). The project's venvs are read again first,
   for one made in the terminal since startup. */
function openVenvMenu(anchor) {
  if (closePopup(anchor) || !venvStatus(S.venv || {}, S.termVenv).offer.length) return;
  api.get('/api/venv').then((v) => { S.venv = v; paintVenv(); }).catch(() => {}).then(() => {
    const { offer, line } = venvStatus(S.venv || {}, S.termVenv);
    if (line) return sendToTerminal(line);
    if (offer.length) openPopup(anchor, venvMenu(offer));
  });
}

/* The venvs a click offers when it does not type one at once. None is marked:
   none is active, and the first is only the one the bar names. */
function venvMenu(offer) {
  const el = (tag, className, textContent = '') => Object.assign(document.createElement(tag), { className, textContent });
  const menu = el('div', 'envmenu');
  menu.setAttribute('role', 'menu');
  menu.append(el('div', 'menuhead', 'Activate in the terminal'));
  for (const e of offer) {
    const b = el('button', '');
    b.type = 'button';
    b.setAttribute('role', 'menuitem');
    b.title = [e.activation, e.dbt && 'dbt: ' + e.dbt].filter(Boolean).join('\n');
    b.append(el('span', 'lbl', e.name), el('span', 'sub', e.dbt ? e.dbt.split(', ')[0] : e.python ? 'py ' + e.python : ''));
    b.addEventListener('click', () => { closePopup(); sendToTerminal(e.activation); });
    menu.append(b);
  }
  return menu;
}

/* One spelling per folder, to tell whether the venv the terminal names is one
   the server read. On Windows, Git Bash says /c/Users/... where Windows says
   C:\Users\... or C:/Users/..., and case does not count; elsewhere a path is
   compared as it is. */
function venvKey(path, win) {
  if (!win) return String(path).replace(/\/+$/, '');
  let p = String(path).replace(/\\/g, '/').replace(/^\/\/\?\//, '').replace(/\/+$/, '');
  const drive = /^\/(?:cygdrive\/)?([a-zA-Z])(?=\/|$)/.exec(p);
  if (drive) p = drive[1] + ':' + p.slice(drive[0].length);
  return p.toLowerCase();
}

/* The venv the server read that the terminal names, if any: by its folder, or
   by the folder its activate script sets, which is how bash spells it when the
   project root is reached through a mapped drive, a subst or a symlink. The
   server's own paths say whether it runs on Windows. */
function venvMatch(v, reported) {
  const known = [v, ...(v.found || [])].filter((e) => e.path);
  const key = venvKey(reported, venvWin(v));
  return known.find((e) => venvKeys(v, e).includes(key)) || null;
}

/* Whether the server runs on Windows, from its own paths. */
function venvWin(v) {
  return [v, ...(v.found || [])].some((e) => e.path && /^(?:[a-zA-Z]:[\\/]|\\\\)/.test(e.path));
}

/* The spellings `e`, one of the venvs `v` lists, is known by: its folder, and
   the folder its activate script sets. */
function venvKeys(v, e) {
  const win = venvWin(v);
  return [e.path, e.activates].filter(Boolean).map((p) => venvKey(p, win));
}

/* What the status bar says about Python. Until the terminal's shell has said
   anything, that is the venv dbt-edith was started in, else the best one found
   in the project. bash says at each prompt (0069): `reported` is then its
   VIRTUAL_ENV, '' for none, and undefined while nothing has been heard. A venv
   the server did not read, outside the project or tracked by git, is only
   named: its path never goes to the server. `state` colours it: 'on' for an
   active venv, 'off' for one found but not active, 'none' for no venv.
   `offer` is what a click offers to type, and `line` the one it types at
   once, '' when it lists them instead. */
function venvStatus(v, reported) {
  const found = v.found || [];
  const py = (e) => (e.python ? ' · py ' + e.python : '');
  const also = (shown) => {
    const names = found.filter((e) => !shown || e.path !== shown.path).map((e) => e.name);
    return names.length ? 'also found: ' + names.join(', ') : '';
  };
  const lines = (...parts) => [...parts, committedNote(v)].filter(Boolean).join('\n');
  if (reported) {
    const e = venvMatch(v, reported);
    if (!e) {
      const name = reported.replace(/[\\/]+$/, '').split(/[\\/]/).pop();
      return { text: 'venv ' + name, state: 'on', offer: [], line: '',
        title: lines(reported, 'activated in the terminal; not a venv dbt-edith read, so its versions are not shown', also(null)) };
    }
    return { text: 'venv ' + e.name + py(e), state: 'on', offer: [], line: '',
      title: lines(e.path, e.dbt && ('dbt: ' + e.dbt), 'activated in the terminal', also(e)) };
  }
  if (!v.name) {
    return { text: 'no venv', state: 'none', offer: [], line: '', title: lines(reported === ''
      ? 'none activated in the terminal, and none found in the project'
      : 'no VIRTUAL_ENV when dbt-edith started, and none found in the project') };
  }
  const active = reported === undefined && v.source === 'activated';
  const offer = active ? [] : venvOffer(v);
  return {
    text: (active ? 'venv ' : 'venv (inactive) ') + v.name + py(v),
    state: active ? 'on' : 'off',
    // Typed at once only when it is the venv the bar names; any other is picked.
    offer, line: offer.length === 1 && offer[0] === v ? v.activation : '',
    title: lines(v.path, v.dbt && ('dbt: ' + v.dbt),
      active ? 'activated when dbt-edith started'
        : reported === '' ? 'not activated in the terminal; source its activate script there'
          : 'found in the project, not activated when dbt-edith started; source its activate script in the terminal',
      also(v)),
  };
}

/* What a click on an inactive venv offers to type: the venv the bar names
   first, then the others found, each with the line the server made for the
   terminal's shell (0072). One with no script for that shell has no line and
   is left out, and a venv git tracks is never found (0053). The venv
   dbt-edith was started in can also be among those found, under the
   project's spelling of its folder, and is offered once. */
function venvOffer(v) {
  const named = venvKeys(v, v);
  const others = (v.found || []).filter((e) => !venvKeys(v, e).some((k) => named.includes(k)));
  return [v, ...others].filter((e) => e.path && e.activation);
}

/* A venv git tracks came with the repository, so whoever wrote the repository
   wrote its python too: the server never runs it, and this says why it is not
   the one shown (0053). */
function committedNote(v) {
  const names = v.committed || [];
  return names.length ? `committed to the repository, so never run by dbt-edith: ${names.join(', ')}` : '';
}

// Without the key the bar at the top already says why, once.
boot().catch((e) => { if (!S.keyless) toast('startup failed: ' + e.message, 'err'); });
})();
