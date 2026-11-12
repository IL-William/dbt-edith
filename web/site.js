/* The static site's server: the answers /api/lineage, /api/node and /api/search give,
   worked out in the browser from the data `dbt-edith --site` wrote (0177).

   Each walk below is a port of the Rust one, line by line, and the golden fixture
   tests/fixtures/site_case.js holds Rust's answers for web/tests/site.js to compare
   them with. A rule changed in src/graph.rs or src/api.rs changes that fixture
   (DBT_EDITH_BLESS=1 cargo test site_case) and fails here until this file follows.

   Pure: nothing here touches the page. `make` takes the data and a way to load a
   bucket of cards, so the harness runs it under jsc; the browser's own is made once
   below, from window.EDITH_SITE, and read as EdithSite.get / .send / .meta.
   Nothing here asks for a network: a bucket is a <script> of the folder itself. */
// biome-ignore lint/correctness/noUnusedVariables: the global app.js reads; a plain script has no export
const EdithSite = (() => {
  const BUCKET_URL = (k) => 'data/cards/' + String(k).padStart(4, '0') + '.js';
  const NO_ROUTE = 'not in this site';
  const LOOK_KEY = 'dbt-edith-site-look';
  const LOOK_DEFAULT = { theme: 'night', follow: false, apparitions: true, boxes: 'tombstone' };

  /* The kinds /api/search accepts, as api.rs parse_kinds spells them: an unknown word is
     dropped, not refused. */
  const SEARCH_KINDS = ['model', 'source', 'seed', 'snapshot', 'test', 'exposure'];

  /* Rust's str::trim: the White_Space property. JavaScript's trim() also strips U+FEFF
     and leaves U+0085, so the two would disagree on a query that begins with either. */
  const RUST_SPACE = '\\t\\n\\v\\f\\r \\u0085\\u00a0\\u1680\\u2000-\\u200a\\u2028\\u2029\\u202f\\u205f\\u3000';
  const TRIM_LEFT = new RegExp('^[' + RUST_SPACE + ']+');
  const TRIM_RIGHT = new RegExp('[' + RUST_SPACE + ']+$');

  function rustTrim(s) { return s.replace(TRIM_LEFT, '').replace(TRIM_RIGHT, ''); }

  /* A string's length in UTF-8 bytes, which is what Rust's str::len counts and the search
     ranks by. TextEncoder is not in every shell the harness runs under. */
  function utf8Len(s) {
    let n = 0;
    for (let i = 0; i < s.length; i++) {
      const c = s.charCodeAt(i);
      if (c < 0x80) n += 1;
      else if (c < 0x800) n += 2;
      else if (c >= 0xd800 && c < 0xdc00 && i + 1 < s.length) { n += 4; i++; }
      else n += 3;
    }
    return n;
  }

  /* Rust compares strings by their bytes, which is by code point. JavaScript compares code
     units, which puts an astral character (an emoji) before U+E000 to U+FFFF. */
  function cmpCodePoints(a, b) {
    let i = 0;
    let j = 0;
    while (i < a.length && j < b.length) {
      const x = a.codePointAt(i);
      const y = b.codePointAt(j);
      if (x !== y) return x < y ? -1 : 1;
      i += x > 0xffff ? 2 : 1;
      j += y > 0xffff ? 2 : 1;
    }
    const left = a.length - i;
    const right = b.length - j;
    return left === right ? 0 : left < right ? -1 : 1;
  }

  function decode(s) {
    try { return decodeURIComponent(s.replace(/\+/g, ' ')); } catch { return s; }
  }

  /* A query string as serde_urlencoded reads it: `+` is a space, the last of a repeated
     key wins here where the server would refuse it. */
  function parseQuery(qs) {
    const out = Object.create(null);
    for (const part of qs ? qs.split('&') : []) {
      if (!part) continue;
      const eq = part.indexOf('=');
      out[decode(eq < 0 ? part : part.slice(0, eq))] = eq < 0 ? '' : decode(part.slice(eq + 1));
    }
    return out;
  }

  /* An unsigned number, as the routes' own parameters are: anything else is refused where
     the server answers 400. */
  function unsigned(params, name, fallback, max) {
    if (!(name in params)) return fallback;
    const text = params[name];
    if (!/^\d+$/.test(text) || Number(text) > max) throw new Error('invalid ' + name + ': ' + text);
    return Number(text);
  }

  function parseKinds(spec) {
    return spec.split(',').filter((s) => s && SEARCH_KINDS.includes(s));
  }

  function make(data, loadBucket) {
    const { base, parents, children, tests, rank, key, att, lv, cuts, limits, meta, tree, placed } = data;
    const total = base.length;
    const numbers = new Map();
    for (let i = 0; i < total; i++) numbers.set(base[i].id, i);
    const cards = new Map();            // node number -> its card, once its bucket arrived
    const pending = new Map();          // bucket -> { promise, resolve, reject }

    /* Tests and operations are hung under a model, never walked through. */
    const isGraph = (n) => base[n].kind !== 'test' && base[n].kind !== 'operation';
    const byRank = (a, b) => rank[a] - rank[b];

    /* graph.rs Graph::lineage. The cap stops only the neighbour loop it is met in: the
       rest of the frontier and the other direction still run, each stopping at its own
       first new node, which is why `truncated` is not a reason to leave the pass. */
    function lineage(focus, up, down, withTests, maxNodes) {
      const depth = new Map([[focus, 0]]);
      let truncated = false;
      for (const [limit, upstream] of [[up, true], [down, false]]) {
        let frontier = [focus];
        for (let step = 1; step <= limit; step++) {
          const next = [];
          for (const cur of frontier) {
            for (const nb of (upstream ? parents : children)[cur]) {
              if (!isGraph(nb) || depth.has(nb)) continue;
              if (depth.size >= maxNodes) { truncated = true; break; }
              depth.set(nb, upstream ? -step : step);
              next.push(nb);
            }
          }
          if (next.length === 0 || truncated) break;
          frontier = next;
        }
      }

      if (withTests) {
        // One test per model a turn, each model's by name: a cap reached halfway leaves
        // every model some of its tests.
        const models = [...depth.keys()].sort(byRank);
        const lists = models.map((m) => tests[m].slice().sort(byRank));
        const turns = lists.reduce((most, l) => Math.max(most, l.length), 0);
        fill: for (let turn = 0; turn < turns; turn++) {
          for (let k = 0; k < models.length; k++) {
            const t = lists[k][turn];
            if (t === undefined || depth.has(t)) continue;
            if (depth.size >= maxNodes) { truncated = true; break fill; }
            depth.set(t, depth.get(models[k]));
          }
        }
      }

      const members = [...depth.keys()].sort((a, b) => a - b);
      const pos = new Map(members.map((n, p) => [n, p]));
      let edges = [];
      members.forEach((n, p) => {
        for (const c of children[n]) if (pos.has(c)) edges.push([p, pos.get(c)]);
        if (withTests) for (const t of tests[n]) if (pos.has(t)) edges.push([p, pos.get(t)]);
      });
      edges.sort((a, b) => a[0] - b[0] || a[1] - b[1]);
      edges = edges.filter((e, i) => i === 0 || e[0] !== edges[i - 1][0] || e[1] !== edges[i - 1][1]);

      return {
        focus: pos.get(focus),
        truncated,
        mode: 'model',
        nodes: members.map((n) => {
          // What a walk decides, over what the node says whatever the walk (box_base).
          const box = Object.assign({}, base[n], {
            depth: depth.get(n),
            hidden_up: parents[n].filter((p) => !pos.has(p)).length,
            hidden_down: children[n].filter((c) => !pos.has(c)).length,
          });
          if (att[n] >= 0 && pos.has(att[n])) box.attached = pos.get(att[n]);
          return box;
        }),
        edges,
      };
    }

    /* graph.rs fold_levels, for the plain walk: a model's level has no cap to be cut by. */
    function foldLevels(tallies) {
      while (tallies.length && tallies[tallies.length - 1].total === 0) tallies.pop();
      let more = false;
      if (tallies.length > limits.max_levels) { tallies.length = limits.max_levels; more = true; }
      const levels = tallies.map((t) => {
        t.kinds.sort((a, b) => b[1] - a[1] || cmpCodePoints(a[0], b[0]));
        const level = { kinds: t.kinds };
        if (t.total === 1) level.name = base[t.first].name;
        return level;
      });
      return [levels, more];
    }

    /* graph.rs Graph::levels: what each level around the focus adds, however far the graph
       goes, one level past the last listed to know whether it goes on. */
    function levels(focus) {
      const side = (upstream) => {
        const seen = new Set([focus]);
        let frontier = [focus];
        const tallies = [];
        for (let round = 0; round <= limits.max_levels; round++) {
          const next = [];
          const tally = { kinds: [], first: -1, total: 0 };
          for (const cur of frontier) {
            for (const nb of (upstream ? parents : children)[cur]) {
              if (!isGraph(nb) || seen.has(nb)) continue;
              seen.add(nb);
              next.push(nb);
              tally.total++;
              if (tally.total === 1) tally.first = nb;
              const found = tally.kinds.find((k) => k[0] === lv[nb]);
              if (found) found[1]++;
              else tally.kinds.push([lv[nb], 1]);
            }
          }
          if (next.length === 0) break;
          tallies.push(tally);
          frontier = next;
        }
        return foldLevels(tallies);
      };
      const [up, upMore] = side(true);
      const [down, downMore] = side(false);
      return { up, down, up_more: upMore, down_more: downMore };
    }

    /* api.rs lineage_answer: the clamps, and the levels the depth menu lists. */
    function lineageAnswer(focus, up, down, withTests, max) {
      const answer = lineage(focus, Math.min(up, limits.depth_max), Math.min(down, limits.depth_max), withTests,
        Math.min(max === undefined ? limits.max : max, limits.max_max));
      answer.levels = levels(focus);
      return answer;
    }

    /* api.rs hit: the few fields a search answer carries, which a box already holds. */
    function hit(n) {
      const b = base[n];
      const out = { id: b.id, name: b.name, kind: b.kind, file: b.file, schema: b.schema, materialized: b.materialized,
        tests: b.tests, disabled: b.disabled };
      if (b.test_name) out.test_name = b.test_name;
      if (b.column) out.column = b.column;
      return out;
    }

    /* graph.rs Graph::search: 0 for the name itself, 10 for its start, 20 for a part of it,
       30 for a part of its path, 50 for everything when there is no query; then the shorter
       name, then the name's order. A name's length counts bytes, as the server's does. */
    function search(query, kinds, limit) {
      const q = rustTrim(query).toLowerCase();
      const hits = [];
      for (let n = 0; n < total; n++) {
        if (kinds.length && !kinds.includes(base[n].kind)) continue;
        let score;
        if (q === '') {
          score = 50;
        } else {
          const cut = key[n].indexOf('\u0000');
          const nameKey = cut < 0 ? key[n] : key[n].slice(0, cut);
          const pathKey = cut < 0 ? '' : key[n].slice(cut + 1);
          if (nameKey === q) score = 0;
          else if (nameKey.startsWith(q)) score = 10;
          else if (nameKey.includes(q)) score = 20;
          else if (pathKey.includes(q)) score = 30;
          else continue;
        }
        hits.push([score * 1000000 + Math.min(utf8Len(base[n].name), 999) * 1000, n]);
      }
      hits.sort((a, b) => a[0] - b[0] || rank[a[1]] - rank[b[1]]);
      return hits.slice(0, limit).map((h) => hit(h[1]));
    }

    /* The bucket a node's card is in: the last run that starts at or before it. */
    function bucketOf(n) {
      let lo = 0;
      let hi = cuts.length - 1;
      while (lo < hi) {
        const mid = (lo + hi + 1) >> 1;
        if (cuts[mid] <= n) lo = mid; else hi = mid - 1;
      }
      return lo;
    }

    /* What a bucket's script calls once it runs. */
    function received(start, list) {
      list.forEach((card, i) => cards.set(start + i, card));
      const k = cuts.indexOf(start);
      const wait = pending.get(k);
      if (wait) wait.resolve();
    }

    /* One promise per bucket, so a hover and the click after it are one load. A bucket that
       fails to load is forgotten, so the next ask tries again. */
    function bucket(k) {
      if (pending.has(k)) return pending.get(k).promise;
      const wait = {};
      wait.promise = new Promise((resolve, reject) => { wait.resolve = resolve; wait.reject = reject; });
      pending.set(k, wait);
      wait.promise.catch(() => { if (pending.get(k) === wait) pending.delete(k); });
      Promise.resolve(loadBucket(k, BUCKET_URL(k))).then(() => {
        // A script that ran and never gave its cards is a file that is not one of ours.
        if (cards.has(cuts[k])) wait.resolve();
        else wait.reject(new Error('the cards of ' + BUCKET_URL(k) + ' did not arrive'));
      }, (e) => wait.reject(e));
      return wait.promise;
    }

    function card(n) {
      if (cards.has(n)) return Promise.resolve(cards.get(n));
      return bucket(bucketOf(n)).then(() => {
        if (!cards.has(n)) throw new Error('the card of ' + base[n].id + ' is not in its bucket');
        return cards.get(n);
      });
    }

    /* The editor's tree, of the files the manifest builds from (src/site.rs tree_of): a folder's entries
       as /api/dir lists them, in the order Rust wrote them, so nothing is sorted or lower-cased here.
       A file's kind and materialization come from its node's box, by the server's own rule. */
    function dir(path) {
      const list = tree[path.replace(/^\/+|\/+$/g, '')];
      if (!list) throw new Error('not a folder in this site');
      const at = path.replace(/^\/+|\/+$/g, '');
      return list.map(([name, n]) => {
        const entry = { name, path: at ? at + '/' + name : name, dir: n < 0, size: 0 };
        if (n >= 0) {
          const box = base[n];
          entry.kind = box.kind;
          if (box.kind === 'model' && box.materialized) entry.materialized = box.materialized;
          entry.id = box.id;
        }
        return entry;
      });
    }

    /* Where a node's row is in that tree: where its package was put when it has no file of its own
       to be found, else its file, which is a row only if a model, a seed or a snapshot is built from it. */
    function treePath(id) {
      const n = number(id);
      return placed[n] || base[n].file;
    }

    function number(id) {
      if (!numbers.has(id)) throw new Error('unknown node');
      return numbers.get(id);
    }

    function route(path) {
      const mark = path.indexOf('?');
      return [mark < 0 ? path : path.slice(0, mark), parseQuery(mark < 0 ? '' : path.slice(mark + 1))];
    }

    function readLook() {
      try {
        const kept = JSON.parse(localStorage.getItem(LOOK_KEY));
        if (kept && typeof kept === 'object') return Object.assign({}, LOOK_DEFAULT, kept);
      } catch { /* nothing kept, or storage refused: the default stands */ }
      return Object.assign({}, LOOK_DEFAULT);
    }

    /* The routes the page asks, answered as a Promise of the same JSON. A route this site
       has no data for is refused, which the page treats as any failed request. */
    function get(path) {
      return new Promise((resolve) => {
        const [name, params] = route(String(path));
        if (name === '/api/lineage') {
          const focus = number(params.id);
          resolve(lineageAnswer(focus, unsigned(params, 'up', limits.depth, 4294967295), unsigned(params, 'down', limits.depth, 4294967295),
            unsigned(params, 'tests', 0, 255) === 1, 'max' in params ? unsigned(params, 'max', undefined, Number.MAX_SAFE_INTEGER) : undefined));
        } else if (name === '/api/node') {
          // By its unique_id only: the editor's file is not in this site.
          if (!('id' in params)) throw new Error(NO_ROUTE);
          resolve(card(number(params.id)));
        } else if (name === '/api/search') {
          resolve(search(params.q || '', parseKinds(params.kind || ''),
            Math.min(unsigned(params, 'limit', limits.search, Number.MAX_SAFE_INTEGER), limits.search_max)));
        } else if (name === '/api/dir') {
          resolve(dir(params.path || ''));
        } else if (name === '/api/look') {
          resolve(readLook());
        } else if (name === '/api/macros/search' || name === '/api/files') {
          resolve([]);
        } else {
          throw new Error(NO_ROUTE);
        }
      });
    }

    /* The one write the page makes here: its look, kept in this browser. Storage that is
       refused (a private window) leaves the look for the session, as the server does when
       it has no folder to keep it in. */
    function send(path, method, body) {
      return new Promise((resolve) => {
        if (route(String(path))[0] !== '/api/look' || method !== 'POST') throw new Error(NO_ROUTE);
        try { localStorage.setItem(LOOK_KEY, JSON.stringify(body)); } catch { /* kept for this page only */ }
        resolve(body);
      });
    }

    return { get, send, received, meta, limits, number, treePath, lineage: lineageAnswer, search, parseKinds, levels };
  }

  /* The page's own, once the data has loaded: a bucket is a script appended to the page,
     which calls EDITH_SITE_CARDS. Absent without the data, so a harness loading this file
     alone, and the page before its data script ran, get the factory only. */
  const out = { make, rustTrim, utf8Len, cmpCodePoints, parseQuery, parseKinds };
  if (typeof window !== 'undefined' && window.EDITH_SITE) {
    const loadScript = (k, url) => new Promise((resolve, reject) => {
      const el = document.createElement('script');
      el.src = url;
      el.onload = () => { el.remove(); resolve(); };
      el.onerror = () => { el.remove(); reject(new Error('cannot load ' + url)); };
      document.head.appendChild(el);
    });
    const site = make(window.EDITH_SITE, loadScript);
    window.EDITH_SITE_CARDS = site.received;
    Object.assign(out, site);
  }
  return out;
})();
