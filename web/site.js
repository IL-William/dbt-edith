/* The static site's server: the answers /api/lineage, /api/node, /api/search, /api/site/code,
   /api/collineage and /api/colreach give, worked out in the browser from the data `dbt-edith --site`
   wrote (0177, 0180, 0182).

   Each walk below is a port of the Rust one, line by line, and the golden fixture
   tests/fixtures/site_case.js holds Rust's answers for web/tests/site.js to compare
   them with. A rule changed in src/graph.rs or src/api.rs changes that fixture
   (DBT_EDITH_BLESS=1 cargo test site_case) and fails here until this file follows.

   Pure: nothing here touches the page. `make` takes the data and a way to load a
   bucket of cards or of SQL, so the harness runs it under jsc; the browser's own is made once
   below, from window.EDITH_SITE, and read as EdithSite.get / .send / .meta.
   Nothing here asks for a network: a bucket is a <script> of the folder itself. */
// biome-ignore lint/correctness/noUnusedVariables: the global app.js reads; a plain script has no export
const EdithSite = (() => {
  const BUCKET_URL = (k) => 'data/cards/' + String(k).padStart(4, '0') + '.js';
  const SQL_URL = (k) => 'data/sql/' + String(k).padStart(4, '0') + '.js';
  const CLL_URL = 'data/cll.js';
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

  /* The address of a node's file in the repository, at the commit the site was written for (0180):
     the base Rust wrote, `file_base`, and the node's path with each segment escaped as the page
     writes a query. Only a file of the root project is in the repository; a package's is not, and
     a path that is not plain (an empty segment, `.` or `..`) is no link at all. The root is
     `meta.root`, which Rust reads as the tree does, not the manifest's project name, which a
     manifest may lack. '' for none. */
  function fileUrl(meta, pkg, file) {
    const base = meta && meta.file_base;
    if (!base || !/^https:\/\//.test(base) || !meta.root || pkg !== meta.root || !file) return '';
    const parts = file.split('/');
    if (parts.some((p) => p === '' || p === '.' || p === '..')) return '';
    return base + parts.map(encodeURIComponent).join('/');
  }

  function make(data, loadBucket) {
    const { base, parents, children, tests, rank, key, att, lv, cuts, sql_cuts: sqlCuts, limits, meta, tree, placed } = data;
    const total = base.length;
    const numbers = new Map();
    for (let i = 0; i < total; i++) numbers.set(base[i].id, i);

    /* Buckets of one kind of item, the cards or the SQL: each a script of the folder that hands its
       items to `received`, found by the node's number among the cuts the index lists. One promise
       per bucket, so a hover and the click after it are one load; a bucket that fails to load is
       forgotten, so the next ask tries again. */
    function buckets(cutList, urlOf, one, many) {
      const held = new Map();           // node number -> its item, once its bucket arrived
      const pending = new Map();        // bucket -> { promise, resolve, reject }

      /* The bucket an item is in: the last run that starts at or before it. */
      function bucketOf(n) {
        let lo = 0;
        let hi = cutList.length - 1;
        while (lo < hi) {
          const mid = (lo + hi + 1) >> 1;
          if (cutList[mid] <= n) lo = mid; else hi = mid - 1;
        }
        return lo;
      }

      /* What a bucket's script calls once it runs. */
      function received(start, list) {
        list.forEach((item, i) => held.set(start + i, item));
        const wait = pending.get(cutList.indexOf(start));
        if (wait) wait.resolve();
      }

      function bucket(k) {
        if (pending.has(k)) return pending.get(k).promise;
        const wait = {};
        wait.promise = new Promise((resolve, reject) => { wait.resolve = resolve; wait.reject = reject; });
        pending.set(k, wait);
        wait.promise.catch(() => { if (pending.get(k) === wait) pending.delete(k); });
        Promise.resolve(loadBucket(k, urlOf(k))).then(() => {
          // A script that ran and never gave its items is a file that is not one of ours.
          if (held.has(cutList[k])) wait.resolve();
          else wait.reject(new Error('the ' + many + ' of ' + urlOf(k) + ' did not arrive'));
        }, (e) => wait.reject(e));
        return wait.promise;
      }

      function get(n) {
        if (held.has(n)) return Promise.resolve(held.get(n));
        return bucket(bucketOf(n)).then(() => {
          if (!held.has(n)) throw new Error('the ' + one + ' of ' + base[n].id + ' is not in its bucket');
          return held.get(n);
        });
      }
      return { get, received };
    }
    const cardBuckets = buckets(cuts, BUCKET_URL, 'card', 'cards');
    // A site written before the SQL had none: its Code tab finds nothing, and says so.
    const sqlBuckets = buckets(sqlCuts || [], SQL_URL, 'SQL', 'SQL');

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

    /* graph.rs fold_levels: the tallies of one side as the levels the depth menu reads. `cutAt` is the
       step a column walk's cap ended on, null for a model's walk, which has none to be cut by; `nameOf`
       names the one node a level adds when it adds only one. */
    function foldLevels(tallies, cutAt, nameOf) {
      while (tallies.length && tallies[tallies.length - 1].total === 0) tallies.pop();
      let more = cutAt !== null;
      if (tallies.length > limits.max_levels) { tallies.length = limits.max_levels; more = true; }
      const listed = tallies.length;
      const levels = tallies.map((t, i) => {
        t.kinds.sort((a, b) => b[1] - a[1] || cmpCodePoints(a[0], b[0]));
        const level = { kinds: t.kinds };
        if (t.total === 1) {
          const name = nameOf(t.first);
          if (name !== '') level.name = name;
        }
        if (cutAt !== null && cutAt === i + 1 && i + 1 === listed) level.cut = true;
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
        return foldLevels(tallies, null, (first) => base[first].name);
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

    /* ---- Column lineage (0182): collin's cache, laid out whole in data/cll.js and loaded when a column is
       first asked about. A column is a number, where its node starts plus its slot, so Rust's `ColRef`
       order is the number's. Everything below is a port of src/graph.rs's `ColLineage` and of the two
       routes api.rs answers with it, and the golden fixture holds it to Rust's answers. */

    let cll = null;               // what `buildCll` made of the file, once it arrived
    let cllRaw = null;            // what EDITH_SITE_CLL handed over, until it is built
    let cllLoad = null;           // the one promise of a load in flight, forgotten if it fails

    function receivedCll(raw) { cllRaw = raw; }

    /* The file's arrays as the walks read them: the edges' `from` made absolute, the edges leaving a column
       as a run (they are held sorted by it), and the edges reaching a column as a stable counting sort by
       target, which is the order `by_to` keeps and so the order a walk upstream meets its neighbours in. */
    function buildCll(raw) {
      const at = raw.at;
      const columns = at[at.length - 1];
      const edges = raw.to.length;
      const colNode = new Int32Array(columns);
      for (let n = 0; n + 1 < at.length; n++) colNode.fill(n, at[n], at[n + 1]);
      const from = new Int32Array(edges);
      let sum = 0;
      for (let i = 0; i < edges; i++) { sum += raw.from[i]; from[i] = sum; }
      const to = Int32Array.from(raw.to);
      const ek = Uint8Array.from(raw.ek);
      const outStart = new Int32Array(columns + 1);
      for (let i = 0; i < edges; i++) outStart[from[i] + 1]++;
      for (let c = 0; c < columns; c++) outStart[c + 1] += outStart[c];
      const inStart = new Int32Array(columns + 1);
      for (let i = 0; i < edges; i++) inStart[to[i] + 1]++;
      for (let c = 0; c < columns; c++) inStart[c + 1] += inStart[c];
      const inEdge = new Int32Array(edges);
      const fill = inStart.slice(0, columns);
      for (let i = 0; i < edges; i++) inEdge[fill[to[i]]++] = i;
      const rowKind = new Uint8Array(256);
      for (const k of raw.row) rowKind[k] = 1;
      const pairs = (list) => { const m = new Map(); for (let i = 0; i < list.length; i += 2) m.set(list[i], list[i + 1]); return m; };
      const rules = new Map(raw.rules);
      return {
        kinds: raw.kinds, rank: raw.rank, rowKind, hasIndirect: raw.row.length > 0,
        at, columns, colNode, edges, from, to, ek, outStart, inStart, inEdge,
        names: raw.names, nm: Int32Array.from(raw.nm), types: raw.types, ty: Int32Array.from(raw.ty),
        tests: pairs(raw.tests), roots: new Map(raw.roots), lost: new Map(raw.lost), outside: new Set(raw.outside),
        expr: new Map(raw.expr.map(([c, kind, text, steps, cut]) => [c, { kind, text, steps, cut: cut === 1 }])),
        rules,
      };
    }

    /* One load per file: a hover and the click after it are one, a file that fails to load is forgotten so
       the next ask tries again, and a script that ran and gave nothing is a file that is not ours. */
    function loadCll() {
      if (cll) return Promise.resolve(cll);
      if (!cllLoad) {
        const load = Promise.resolve(loadBucket(null, CLL_URL)).then(() => {
          if (!cllRaw) throw new Error('the column lineage of ' + CLL_URL + ' did not arrive');
          const built = buildCll(cllRaw);
          cllRaw = null;
          cll = built;
          return built;
        });
        cllLoad = load;
        load.catch(() => { if (cllLoad === load) cllLoad = null; });
      }
      return cllLoad;
    }

    const colName = (C, c) => C.names[C.nm[c]];
    const crossed = (C, e, indirect) => indirect || !C.rowKind[C.ek[e]];

    /* graph.rs row_models: the models the indirect edges leaving `c` reach, each with the strongest role
       among those edges. An edge into `c`'s own model is left out. A column's edges are sorted by the column
       they reach, so a model's are together and only the last one found can be the model of the next. */
    function rowModels(C, c) {
      const out = [];
      const own = C.colNode[c];
      for (let e = C.outStart[c]; e < C.outStart[c + 1]; e++) {
        const k = C.ek[e];
        if (!C.rowKind[k]) continue;
        const m = C.colNode[C.to[e]];
        if (m === own) continue;
        const last = out.length ? out[out.length - 1] : null;
        if (last && last[0] === m) { if (C.rank[k] > C.rank[last[1]]) last[1] = k; } else out.push([m, k]);
      }
      return out;
    }

    /* graph.rs ColLineage::degree: an edge from a column to itself is no lineage. */
    function degree(C, c, indirect) {
      let up = 0;
      let down = 0;
      for (let i = C.inStart[c]; i < C.inStart[c + 1]; i++) {
        const e = C.inEdge[i];
        if (C.from[e] !== c && crossed(C, e, indirect)) up++;
      }
      for (let e = C.outStart[c]; e < C.outStart[c + 1]; e++) if (C.to[e] !== c && crossed(C, e, indirect)) down++;
      return [up, down];
    }

    /* graph.rs ColLineage::walk, the level loop of `column_lineage` with no payload. The cap turns a column
       away and goes on to the next neighbour rather than out, `truncated` is kept from the upstream side
       into the downstream one, no stop is added once it is set, and a stop reached around the cut is taken
       back whatever the cap. A box is refused by the cap as a column is, and then has no edge either. */
    function cllWalk(C, focus, up, down, maxNodes, cut, rowEdges) {
      const toModels = rowEdges === 'models';
      const indirect = rowEdges !== 'hidden';
      const shown = (e) => crossed(C, e, indirect);
      const follows = (e) => shown(e) && !(toModels && C.rowKind[C.ek[e]]);
      const depth = new Map([[focus, 0]]);
      const stopped = new Set();
      const walked = new Map([[focus, 0]]);
      const boxes = new Map();
      const boxEdges = new Map();       // (column, model) -> [column, model, role]
      let truncated = false;
      const cutAt = [null, null];
      const room = () => depth.size + boxes.size >= maxNodes;
      for (const [limit, upstream] of [[up, true], [down, false]]) {
        let frontier = [focus];
        for (let step = 1; step <= limit; step++) {
          const next = [];
          for (const cur of frontier) {
            const neighbours = [];
            if (upstream) {
              for (let i = C.inStart[cur]; i < C.inStart[cur + 1]; i++) {
                const e = C.inEdge[i];
                if (shown(e)) neighbours.push(C.from[e]);
              }
            } else {
              for (let e = C.outStart[cur]; e < C.outStart[cur + 1]; e++) if (follows(e) && !cut.includes(C.ek[e])) neighbours.push(C.to[e]);
            }
            for (const nb of neighbours) {
              if (stopped.delete(nb)) { walked.set(nb, step); next.push(nb); continue; }
              if (depth.has(nb)) continue;
              if (room()) { truncated = true; continue; }
              depth.set(nb, upstream ? -step : step);
              if (!upstream) walked.set(nb, step);
              next.push(nb);
            }
          }
          if (!upstream && cut.length && !truncated) {
            stop: for (const cur of frontier) {
              for (let e = C.outStart[cur]; e < C.outStart[cur + 1]; e++) {
                if (!follows(e) || !cut.includes(C.ek[e])) continue;
                const t = C.to[e];
                if (depth.has(t)) continue;
                if (room()) { truncated = true; break stop; }
                depth.set(t, step);
                stopped.add(t);
              }
            }
          }
          if (toModels && !upstream) {
            for (const cur of frontier) {
              for (const [model, kind] of rowModels(C, cur)) {
                if (!boxes.has(model)) {
                  if (room()) { truncated = true; continue; }
                  boxes.set(model, step);
                }
                boxEdges.set(cur + ':' + model, [cur, model, kind]);
              }
            }
          }
          if (truncated) cutAt[upstream ? 0 : 1] = step;
          if (next.length === 0 || truncated) break;
          frontier = next;
        }
      }
      return { depth, stopped, walked, boxes, boxEdges, truncated, cutUp: cutAt[0], cutDown: cutAt[1] };
    }

    /* graph.rs ColLineage::cut_kinds: the kinds, of the first 256, a role named in the cut stands for. */
    function cutKinds(C, names) {
      const out = [];
      for (let k = 0; k < Math.min(C.kinds.length, 256); k++) if (names.includes(C.kinds[k])) out.push(k);
      return out;
    }

    /* graph.rs Graph::col_slot: a column's position in its node, matched by its lower-cased name. -1 for none. */
    function colSlot(C, node, name) {
      const wanted = name.toLowerCase();
      for (let c = C.at[node]; c < C.at[node + 1]; c++) if (colName(C, c).toLowerCase() === wanted) return c - C.at[node];
      return -1;
    }

    /* graph.rs Graph::column_lineage. */
    function colLineage(C, focus, up, down, maxNodes, cutNames, rowEdges) {
      const toModels = rowEdges === 'models';
      const indirect = rowEdges !== 'hidden';
      const cut = cutKinds(C, cutNames);
      const shown = (e) => crossed(C, e, indirect);
      const W = cllWalk(C, focus, up, down, maxNodes, cut, rowEdges);

      const members = [...W.depth.keys()].sort((a, b) => a - b);
      const pos = new Map(members.map((c, p) => [c, p]));
      // The boxes follow the columns, by model, and sit past the deepest column that reaches each.
      const boxModels = [...W.boxes.keys()].sort((a, b) => a - b);
      const boxPos = new Map(boxModels.map((m, k) => [m, members.length + k]));
      const boxDepth = new Map(W.boxes);
      for (const [c, m] of W.boxEdges.values()) boxDepth.set(m, Math.max(boxDepth.get(m), W.depth.get(c) + 1));

      // Nothing leaves a column the walk stopped at, and under models an indirect edge between two columns
      // is drawn only into the focus's side: past it, its model's box is the edge.
      const drawn = (e) => shown(e) && pos.has(C.from[e]) && pos.has(C.to[e]) && !W.stopped.has(C.from[e])
        && !(toModels && C.rowKind[C.ek[e]] && W.depth.get(C.to[e]) > 0);
      const boxed = (e) => toModels && C.rowKind[C.ek[e]] && W.boxEdges.has(C.from[e] + ':' + C.colNode[C.to[e]]);

      const pairs = [];
      members.forEach((c, p) => {
        for (let e = C.outStart[c]; e < C.outStart[c + 1]; e++) if (drawn(e)) pairs.push([p, pos.get(C.to[e]), C.ek[e]]);
      });
      for (const [c, m, kind] of W.boxEdges.values()) pairs.push([pos.get(c), boxPos.get(m), kind]);
      pairs.sort((a, b) => a[0] - b[0] || a[1] - b[1] || a[2] - b[2]);
      const kept = pairs.filter((p, i) => i === 0 || p[0] !== pairs[i - 1][0] || p[1] !== pairs[i - 1][1]);
      const edges = kept.map((p) => [p[0], p[1]]);

      // The roles of the edges that left a stop where it is, not every role ticked.
      const stoppedAt = [];
      for (const c of W.stopped) {
        for (let i = C.inStart[c]; i < C.inStart[c + 1]; i++) {
          const e = C.inEdge[i];
          const left = W.walked.get(C.from[e]);
          if (!cut.includes(C.ek[e]) || !drawn(e) || left === undefined || left >= down) continue;
          const role = C.kinds[C.ek[e]] === undefined ? '' : C.kinds[C.ek[e]];
          if (!stoppedAt.includes(role)) stoppedAt.push(role);
        }
      }
      stoppedAt.sort(cmpCodePoints);

      const nodes = members.map((c) => {
        const owner = base[C.colNode[c]];
        const name = colName(C, c);
        let [upN, downN] = degree(C, c, indirect);
        if (toModels) downN = degree(C, c, false)[1] + rowModels(C, c).length;
        const type = C.types[C.ty[c]];
        // Past the walk, an indirect edge to a model counts once for the model, and none for a model whose box
        // this column's own edge reaches: another column's box does not draw it.
        let hiddenDown = 0;
        const models = [];
        for (let e = C.outStart[c]; e < C.outStart[c + 1]; e++) {
          if (!shown(e) || drawn(e)) continue;
          if (!(toModels && C.rowKind[C.ek[e]])) hiddenDown++;
          else {
            const m = C.colNode[C.to[e]];
            if (m !== C.colNode[c] && !W.boxEdges.has(c + ':' + m) && !models.includes(m)) models.push(m);
          }
        }
        let hiddenUp = 0;
        let hiddenUpRows = 0;
        for (let i = C.inStart[c]; i < C.inStart[c + 1]; i++) {
          const e = C.inEdge[i];
          if (shown(e) && !drawn(e) && !boxed(e)) { hiddenUp++; if (C.rowKind[C.ek[e]]) hiddenUpRows++; }
        }
        const box = {
          id: owner.id + '::' + name, name, kind: owner.kind, file: owner.file, schema: owner.schema,
          materialized: owner.materialized, disabled: owner.disabled, depth: W.depth.get(c),
          tests: C.tests.get(c) || 0, parents: upN, children: downN, hidden_up: hiddenUp, hidden_down: hiddenDown + models.length,
          at: c,
        };
        box.sub = type === '' ? owner.name : owner.name + '  ·  ' + type;
        if (hiddenUpRows) box.hidden_up_rows = hiddenUpRows;
        if (C.roots.has(c)) box.generated = C.roots.get(c);
        if (C.expr.has(c)) {
          const x = C.expr.get(c);
          box.expression = { kind: x.kind, text: x.text, steps: x.steps };
          if (x.cut) box.expression.cut = true;
        }
        if (C.lost.has(c)) box.partial = C.lost.get(c);
        // Without roots the page badges every start `raw`, so the mark would say nothing.
        if (C.roots.size && C.outside.has(c)) box.outside = true;
        return box;
      });
      // A model's box (graph.rs whole_model_node): the node as it is drawn on its own, a leaf with no +N.
      for (const m of boxModels) {
        const b = Object.assign({}, base[m], { depth: boxDepth.get(m), hidden_up: 0, hidden_down: 0 });
        // What only a test says of itself: a cache may name a column of any node, and whole_model_node says none of it.
        for (const k of ['test_name', 'column', 'namespace', 'warn', 'host']) delete b[k];
        b.sub = b.materialized ? b.materialized + '  ·  whole model' : 'whole model';
        nodes.push(b);
      }

      const answer = { focus: pos.get(focus), truncated: W.truncated, mode: 'column', nodes, edges };
      const edgeKinds = kept.map((p) => C.kinds[p[2]] === undefined ? '' : C.kinds[p[2]]);
      if (edgeKinds.length) answer.edge_kinds = edgeKinds;
      if (stoppedAt.length) answer.stopped_at = stoppedAt;
      if (C.roots.size) answer.roots_known = true;
      if (rowEdges === 'hidden' && C.hasIndirect) answer.indirect_hidden = true;
      const focusName = colName(C, focus);
      if (focusName !== '') answer.focus_column = focusName;
      return answer;
    }

    /* graph.rs rows_by, rows_of and rules_read, filled by `name_rows_by` for the page's card: each the first
       ROWS_BY in order, and how many more. */
    function rowsBy(C, c) {
      if (!C.hasIndirect) return [[], 0];
      const named = [];
      for (let i = C.inStart[c]; i < C.inStart[c + 1]; i++) {
        const e = C.inEdge[i];
        if (C.from[e] === c || !C.rowKind[C.ek[e]]) continue;
        named.push([base[C.colNode[C.from[e]]].name + '.' + colName(C, C.from[e]), C.kinds[C.ek[e]] === undefined ? '' : C.kinds[C.ek[e]]]);
      }
      named.sort((a, b) => cmpCodePoints(a[0], b[0]) || cmpCodePoints(a[1], b[1]));
      const unique = named.filter((p, i) => i === 0 || p[0] !== named[i - 1][0] || p[1] !== named[i - 1][1]);
      return [unique.slice(0, limits.rows_by), Math.max(unique.length - limits.rows_by, 0)];
    }

    function rowsOf(C, c) {
      if (!C.hasIndirect) return [[], 0];
      const named = rowModels(C, c).map(([m, k]) => [base[m].name, C.kinds[k] === undefined ? '' : C.kinds[k]]);
      named.sort((a, b) => cmpCodePoints(a[0], b[0]) || cmpCodePoints(a[1], b[1]));
      return [named.slice(0, limits.rows_by), Math.max(named.length - limits.rows_by, 0)];
    }

    function rulesRead(C, c) {
      if (!C.hasIndirect || !C.rules.size) return [[], 0];
      const models = rowModels(C, c).map((x) => x[0]).sort((a, b) => cmpCodePoints(base[a].name, base[b].name) || a - b);
      const named = [];
      let more = 0;
      for (const m of models) {
        for (const [text, cols] of C.rules.get(m) || []) {
          if (!cols.includes(c)) continue;
          if (named.length < limits.rows_by) named.push([base[m].name, text]); else more++;
        }
      }
      return [named, more];
    }

    function nameRowsBy(C, answer) {
      for (const n of answer.nodes) {
        if (n.at === undefined) continue;
        const [by, byMore] = rowsBy(C, n.at);
        const [of, ofMore] = rowsOf(C, n.at);
        const [rules, rulesMore] = rulesRead(C, n.at);
        if (by.length) n.rows_by = by;
        if (byMore) n.rows_by_more = byMore;
        if (of.length) n.rows_of = of;
        if (ofMore) n.rows_of_more = ofMore;
        if (rules.length) n.rules = rules;
        if (rulesMore) n.rules_more = rulesMore;
      }
      // The column's place in the file is a walk's, never the page's.
      for (const n of answer.nodes) delete n.at;
    }

    /* graph.rs Graph::column_levels: what each level around a column adds, from the walk the canvas makes,
       each side walked one level past the last listed. A column counts at its distance, a stop included, and
       a model's box at the step the walk first boxed it. */
    function colLevels(C, focus, up, maxNodes, cutNames, rowEdges) {
      const cut = cutKinds(C, cutNames);
      const deep = limits.max_levels + 1;
      // A box has no column: its tally names the model, as a model's level does.
      const nameOf = (first) => first < 0 ? base[-first - 1].name : base[C.colNode[first]].name + '.' + colName(C, first);
      const add = (t, label, first) => {
        t.total++;
        if (t.total === 1) t.first = first;
        const found = t.kinds.find((k) => k[0] === label);
        if (found) found[1]++; else t.kinds.push([label, 1]);
      };
      const tally = (W, upstream) => {
        const tallies = Array.from({ length: limits.max_levels + 1 }, () => ({ kinds: [], first: -1, total: 0 }));
        for (const [c, d] of [...W.depth.entries()].sort((a, b) => a[0] - b[0])) {
          const level = upstream ? -d : d;
          if (level >= 1) add(tallies[level - 1], 'column', c);
        }
        if (!upstream) {
          for (const [m, step] of [...W.boxes.entries()].sort((a, b) => a[0] - b[0] || a[1] - b[1])) {
            if (tallies[step - 1]) add(tallies[step - 1], 'model', -m - 1);
          }
        }
        return tallies;
      };
      const above = cllWalk(C, focus, deep, 0, maxNodes, cut, rowEdges);
      const below = cllWalk(C, focus, Math.min(up, limits.max_levels), deep, maxNodes, cut, rowEdges);
      const [upLevels, upMore] = foldLevels(tally(above, true), above.cutUp, nameOf);
      const [downLevels, downMore] = foldLevels(tally(below, false), below.cutDown, nameOf);
      // The 21st level is walked only to learn the graph goes on, so a cap met there cut nothing listed.
      const within = (k) => k !== null && k <= limits.max_levels;
      const out = { up: upLevels, down: downLevels, up_more: upMore, down_more: downMore };
      if (within(above.cutUp)) out.up_cut = true;
      if (within(below.cutDown)) out.down_cut = true;
      return out;
    }

    /* api.rs col_lineage, after the query is read: "no column lineage" is the page's own meta, so it is
       said before anything is loaded, then the node, and the column once the file is here. */
    function collineage(params) {
      for (const name of ['id', 'column']) if (!(name in params)) throw new Error('missing ' + name);
      const up = unsigned(params, 'up', limits.col_depth, 4294967295);
      const down = unsigned(params, 'down', limits.col_depth, 4294967295);
      const asked = 'max' in params ? unsigned(params, 'max', undefined, Number.MAX_SAFE_INTEGER) : undefined;
      const indirect = unsigned(params, 'indirect', 0, 255);
      if (!meta.cll) throw new Error('no column lineage');
      const node = number(params.id);
      return loadCll().then((C) => {
        const slot = colSlot(C, node, params.column);
        if (slot < 0) throw new Error('unknown column');
        const focus = C.at[node] + slot;
        const max = Math.min(asked === undefined ? limits.col_drawn : asked, limits.col_most);
        const cut = (params.cut || '').split(',').filter((r) => r);
        const rowEdges = indirect === 1 ? 'models' : 'hidden';
        const answer = colLineage(C, focus, Math.min(up, limits.col_depth_max), Math.min(down, limits.col_depth_max), max, cut, rowEdges);
        answer.most = limits.col_most;
        answer.levels = colLevels(C, focus, Math.min(up, limits.col_depth_max), max, cut, rowEdges);
        nameRowsBy(C, answer);
        return answer;
      });
    }

    /* graph.rs ColLineage::reach_all: how many columns lie on one side of each start, the whole chain, in one
       pass. The columns reached are cut into strongly connected parts (Tarjan's, without recursion), a cycle
       being one part, and the starts that reach each part are carried as bits from part to part in
       topological order, so each edge is crossed once. A count is the size of what is reached, which does
       not depend on the order it is visited in. */
    function reachAll(C, starts, upstream, indirect) {
      const at = new Map();
      const cols = [];
      for (const s of starts) if (!at.has(s)) { at.set(s, cols.length); cols.push(s); }
      const first = [0];
      const next = [];
      for (let i = 0; i < cols.length; i++) {
        const c = cols[i];
        const side = [];
        if (upstream) {
          for (let k = C.inStart[c]; k < C.inStart[c + 1]; k++) { const e = C.inEdge[k]; if (crossed(C, e, indirect)) side.push(C.from[e]); }
        } else {
          for (let e = C.outStart[c]; e < C.outStart[c + 1]; e++) if (crossed(C, e, indirect)) side.push(C.to[e]);
        }
        for (const nb of side) {
          if (nb === c) continue;
          let n = at.get(nb);
          if (n === undefined) { n = cols.length; at.set(nb, n); cols.push(nb); }
          next.push(n);
        }
        first.push(next.length);
      }

      const n = cols.length;
      const index = new Int32Array(n).fill(-1);
      const low = new Int32Array(n);
      const part = new Int32Array(n).fill(-1);
      const open = [];
      const onOpen = new Uint8Array(n);
      let counter = 0;
      let parts = 0;
      for (let root = 0; root < n; root++) {
        if (index[root] !== -1) continue;
        const calls = [[root, 0]];
        index[root] = counter; low[root] = counter; counter++;
        open.push(root); onOpen[root] = 1;
        while (calls.length) {
          const call = calls[calls.length - 1];
          const v = call[0];
          const slot = first[v] + call[1];
          if (slot < first[v + 1]) {
            call[1]++;
            const w = next[slot];
            if (index[w] === -1) {
              index[w] = counter; low[w] = counter; counter++;
              open.push(w); onOpen[w] = 1;
              calls.push([w, 0]);
            } else if (onOpen[w]) {
              low[v] = Math.min(low[v], index[w]);
            }
            continue;
          }
          calls.pop();
          if (calls.length) { const u = calls[calls.length - 1][0]; low[u] = Math.min(low[u], low[v]); }
          if (low[v] === index[v]) {
            for (;;) {
              const w = open.pop();
              onOpen[w] = 0;
              part[w] = parts;
              if (w === v) break;
            }
            parts++;
          }
        }
      }

      // Each part's starts, as bits (32 to a word), handed on to the parts it reaches: a part's number is
      // higher than any it reaches, so from the highest down it has every start that reaches it.
      const words = Math.max(Math.ceil(starts.length / 32), 1);
      const bits = new Int32Array(parts * words);
      const size = new Int32Array(parts);
      for (let v = 0; v < n; v++) size[part[v]]++;
      const own = starts.map((s) => part[at.get(s)]);
      own.forEach((p, i) => { bits[p * words + (i >> 5)] |= 1 << (i & 31); });
      const byPart = new Int32Array(parts + 1);
      for (let v = 0; v < n; v++) byPart[part[v] + 1]++;
      for (let p = 0; p < parts; p++) byPart[p + 1] += byPart[p];
      const order = new Int32Array(n);
      const put = byPart.slice(0, parts);
      for (let v = 0; v < n; v++) order[put[part[v]]++] = v;
      for (let k = n - 1; k >= 0; k--) {
        const v = order[k];
        const p = part[v];
        for (let s = first[v]; s < first[v + 1]; s++) {
          const q = part[next[s]];
          if (q !== p) for (let w = 0; w < words; w++) bits[q * words + w] |= bits[p * words + w];
        }
      }
      // A start counts every column of each part it reaches, but itself.
      const counts = new Array(starts.length).fill(0);
      for (let p = 0; p < parts; p++) {
        for (let w = 0; w < words; w++) {
          let word = bits[p * words + w];
          while (word !== 0) {
            const low1 = word & -word;
            const i = w * 32 + (31 - Math.clz32(low1));
            counts[i] += size[p] - (own[i] === p ? 1 : 0);
            word ^= low1;
          }
        }
      }
      return counts;
    }

    /* api.rs col_reach: the whole chain on each side of each column of one node that has an edge, for the
       Columns table alone. The default canvas's figures: the indirect edges are left out. */
    function colreach(params) {
      if (!('id' in params)) throw new Error('missing id');
      const node = number(params.id);
      if (!meta.cll) return [];
      return loadCll().then((C) => {
        const first = C.at[node];
        const count = C.at[node + 1] - first;
        const deg = Array.from({ length: count }, (_, i) => degree(C, first + i, false));
        const side = (upstream) => {
          const idx = [];
          for (let i = 0; i < count; i++) if (deg[i][upstream ? 0 : 1] > 0) idx.push(i);
          const counts = reachAll(C, idx.map((i) => first + i), upstream, false);
          const all = new Array(count).fill(0);
          idx.forEach((i, k) => { all[i] = counts[k]; });
          return all;
        };
        const up = side(true);
        const down = side(false);
        const rows = [];
        for (let i = 0; i < count; i++) {
          if (deg[i][0] + deg[i][1] > 0) rows.push({ name: colName(C, first + i), up: up[i], down: down[i] });
        }
        return rows;
      });
    }

    function card(n) {
      return cardBuckets.get(n);
    }

    /* One node's code as the manifest holds it (0180), with the file it is in and the link to it on the
       repository's host. A node with nothing to show has an empty source and no compiled text; a text
       missing from the manifest is null, which is what the page says "not compiled" for. */
    function code(n) {
      return Promise.all([card(n), sqlBuckets.get(n)]).then(([c, entry]) => ({
        raw: entry ? entry.raw : '',
        compiled: entry && typeof entry.compiled === 'string' ? entry.compiled : null,
        lang: entry && entry.lang ? entry.lang : 'sql',
        file: c.file,
        url: fileUrl(meta, c.package, c.file) || null,
      }));
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
        } else if (name === '/api/site/code') {
          resolve(code(number(params.id)));
        } else if (name === '/api/collineage') {
          resolve(collineage(params));
        } else if (name === '/api/colreach') {
          resolve(colreach(params));
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

    return {
      get, send, received: cardBuckets.received, receivedSql: sqlBuckets.received, receivedCll, cllHeld: () => cll !== null,
      meta, limits, number, treePath, lineage: lineageAnswer, search, parseKinds, levels,
    };
  }

  /* The page's own, once the data has loaded: a bucket is a script appended to the page,
     which calls EDITH_SITE_CARDS. Absent without the data, so a harness loading this file
     alone, and the page before its data script ran, get the factory only. */
  const out = { make, rustTrim, utf8Len, cmpCodePoints, parseQuery, parseKinds, fileUrl };
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
    window.EDITH_SITE_SQL = site.receivedSql;
    window.EDITH_SITE_CLL = site.receivedCll;
    Object.assign(out, site);
  }
  return out;
})();
