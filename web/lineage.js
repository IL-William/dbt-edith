/* Lineage canvas: layered layout + hand-rolled SVG renderer.
   No graph library (0006): a Sugiyama-style layout written out here. Columns
   come from the drawn edges by longest path, an edge that skips columns gets a
   lane in each one it crosses, median sweeps keep crossings down, and Brandes
   and Kopf sets the heights so long edges run level, unless that costs too much
   height, when a compact isotonic step does (0027). */
const Lineage = (() => {
  const NS = 'http://www.w3.org/2000/svg';
  const HGAP = 80, VGAP = 14;
  // A lane carries no text, so two of them only need telling apart.
  const LANE = 10;
  // Column mode draws a role tag in the 14 px above a box: a lane beside a box
  // keeps this much further off, or the tag's fill hides it.
  const TAG_ROOM = 6;
  // From this many long edges out of one node they share a lane per kind, or a
  // date spine read across the graph becomes a band of hundreds of lines.
  const BUNDLE = 4;
  // Re-assigned per mode in render(): column boxes are smaller than model boxes.
  let W = 200, H = 48;
  /* Colour carries the materialization, which is what you actually reason about
     when reading a DAG: what exists in the warehouse, and what gets rebuilt. */
  const MAT = {
    view: '#4da3ff',
    table: '#4ec78d',
    incremental: '#e0a34b',
    ephemeral: '#6f7d90',
    materialized_view: '#3fc7c7',
    dynamic_table: '#3fc7c7',
  };
  const KIND = {
    source: '#b98cf0', snapshot: '#ef7a9b',
    test: '#7a879a', exposure: '#ef7a9b', other: '#7a879a',
    /* Lighter than the rest, because at their lightness no hue stands far
       enough from all of them: the green a seed had there drew it as a table. */
    seed: '#c3e143',
  };
  /* Anything else is a custom materialization, and deserves to be noticed. */
  const CUSTOM = '#ff6ec7';
  /* A test hung under its model (0034): a one-line box, `indent` in from the
     model's left edge so the stem joining them shows, `top` below the model and
     `gap` apart. Past `fold` rows a model shows its first `fold` and a chip for
     the rest, which opens them (0035). A model's tests of one generic share a
     row that opens into one per test, `sub` further in (0040). */
  const RIDE = { h: 22, gap: 4, top: 6, indent: 18, sub: 12, fold: 5 };
  /* What a test checks, by what dbt recorded of it: a generic on the whole
     model, SQL of its own, or a generic on a column. Greys, all three, so a
     test reads as a test beside the coloured boxes, lighter in the order the
     rows go: the column tests, most of any project, keep the grey every test
     had. They stand apart from each other on the scale colours.js holds a
     seed to, and each has a shape of its own as well (0040). Not from every
     box colour: no light grey can, a cyan reading as one to a red-green
     colour-blind eye, and the grey already sat close to an ephemeral
     model's. A row on a stem under its model is what says test. */
  const TEST = { model: '#eceff3', singular: '#b2bac6', column: '#7a879a' };

  /* Column mode colours the edge rather than the box. Where a column is stored
     is not what you are reading that graph for: what happened to it between two
     models is.

     Its own palette, sharing no colour with the materializations: in column mode
     the boxes still carry the owning model's materialization, so both channels
     are on screen at once and a shared colour would read as a relationship that
     is not there. The ramp runs quiet to loud, because that is the order you
     care about: a passthrough is not news, a transform is. `inferred` is the
     dullest of all, being the one the producer did not read out of the SQL. */
  const ROLE = {
    passthrough: '#c3ccd6',
    rename: '#9fb0c4',
    cast: '#c9b08a',
    aggregate: '#cf7a3f',
    window: '#8b46c9',
    transform: '#d94f4f',
    inferred: '#5f6a78',
    /* Not an edge kind: the badge a column with no incoming edge in view gets.
       That column is where the graph starts, which is raw rather than unknown. */
    raw: '#7fb069',
  };
  /* The plain edge colour, also used for a role this build does not know: a
     cache from another producer may use words that did not exist when this
     shipped, and a wrong hue would claim more than a neutral one. */
  const EDGE_PLAIN = '#33445c';

  function roleColor(kind) {
    return ROLE[String(kind || '').toLowerCase()] || EDGE_PLAIN;
  }

  /* What produced each column, read off its incoming edges, so the badge sits on
     the thing it describes instead of making you trace a line back.

     A column fed by several edges of different roles is `mixed`, which has no
     colour of its own on purpose: it falls back to the neutral, because naming
     one of the roles would be picking a winner at random. A column with nothing
     feeding it inside the current view is where the graph starts, and that is
     `raw` rather than unknown. */
  function nodeRoles(d) {
    const found = d.nodes.map(() => '');
    (d.edges || []).forEach(([, to], i) => {
      const kind = (d.edge_kinds && d.edge_kinds[i]) || '';
      if (!kind) return;
      found[to] = !found[to] ? kind : (found[to] === kind ? kind : 'mixed');
    });
    return found.map((role, i) => role || (d.nodes[i].hidden_up ? '' : 'raw'));
  }

  /* Which of the three a test is. Every payload that carries a test carries
     `test_name` and `column` when it has them, so a test with neither has no
     generic behind it: a singular test. */
  function testLevel(n) {
    return n.column ? 'column' : n.test_name ? 'model' : 'singular';
  }

  function nodeColor(n) {
    if (n.kind === 'test') return TEST[testLevel(n)];
    if (n.kind && n.kind !== 'model') return KIND[n.kind] || KIND.other;
    const m = (n.materialized || '').toLowerCase();
    if (!m) return KIND.other;
    return MAT[m] || CUSTOM;
  }

  function matLabel(n) {
    if (n.kind === 'test') return `${testLevel(n)} test`;
    if (n.kind && n.kind !== 'model') return n.kind;
    return (n.materialized || 'unknown').toLowerCase();
  }

  /* The area an exported picture shows: the layout's box plus room for what is
     drawn outside the boxes, a +N badge reaching 27 beside one and a role tag
     14 above one. A page gets at least a screenful at natural size, because it
     scales its picture to fit the window: one box alone would arrive six times
     too big, which the canvas avoids by capping its own zoom at 1.1. An image
     has a size of its own and takes the tight frame. */
  function frameOf(b, minW = 1000, minH = 560) {
    const pad = 40;
    const w = Math.max(minW, b.x1 - b.x0 + pad * 2);
    const h = Math.max(minH, b.y1 - b.y0 + pad * 2);
    return { x: (b.x0 + b.x1 - w) / 2, y: (b.y0 + b.y1 - h) / 2, w, h };
  }

  /* Every tie in the layout falls back on this rank, name then id, so a graph
     comes out the same whatever order the server listed it in. */
  function dagRank(nodes) {
    const byRank = nodes.map((_, i) => i);
    byRank.sort((a, b) => {
      let A = String(nodes[a].name), B = String(nodes[b].name);
      if (A === B) { A = String(nodes[a].id); B = String(nodes[b].id); }
      return A < B ? -1 : A > B ? 1 : a - b;
    });
    const rank = new Int32Array(nodes.length);
    byRank.forEach((v, r) => { rank[v] = r; });
    return { rank, byRank };
  }

  /* Numbers inside a name compare as numbers, so `9_marts` sorts before
     `10_audit` and the folders a project numbers keep their numbers. */
  function dagNatural(a, b) {
    const A = String(a).match(/\d+|\D+/g) || [], B = String(b).match(/\d+|\D+/g) || [];
    for (let i = 0; i < A.length && i < B.length; i++) {
      if (A[i] === B[i]) continue;
      if (/^\d/.test(A[i]) && /^\d/.test(B[i]) && +A[i] !== +B[i]) return +A[i] - +B[i];
      return A[i] < B[i] ? -1 : 1;
    }
    return A.length - B.length;
  }

  /* The kinds filed under the model paths, which set the level the folders are
     read at. A seed, a snapshot or a test lives under a path of its own, and
     letting one vote would lift the level to the top of the project, where
     every model shares `models`. */
  const ANCHORS = new Set(['model', 'source', 'exposure']);

  /* The folder bands: which folder each drawn node is filed under, and the
     order the folders go in, left to right (0029).

     The folder is the first one below where the drawn models and sources stop
     sharing a path. On a canvas holding `models/core/20_clean/...` and
     `models/core/30_vault/...` that is `20_clean` and `30_vault`; with
     everything drawn in 20_clean, its own sub-folders. Read from what is
     drawn, not from the whole project, so a canvas whose models split
     somewhere always gets bands. A node outside that level, a seed say, goes
     under the first folder of its own path that leaves it, `seeds`, and a
     model filed above it under its own folder, `models`. A node from another
     package goes under the package, since its path is relative to the package
     and would otherwise land in any folder of this project sharing the name.

     The order comes from the edges between folders, never from how their
     names sort, so `staging`, `intermediate`, `marts` works. A name that
     starts with a number is the exception: whoever numbered `20_clean` and
     `30_vault` has said which comes first, and one model in 30_vault read by
     one in 20_clean must not swap the two on the canvas that happens to
     draw that edge and no other. A numbered folder never goes before a lower
     number, and the edges place everything else. Two folders feeding each
     other have no order, and then the one fed least by the folders not yet
     placed goes next: the edges it leaves behind are the ones drawn against
     the order, dashed. Where the edges leave a choice, the folder whose nodes
     sit further left without the bands goes first, `plain` being those
     columns: a seed read by nothing drawn then starts the canvas, as it would
     have, rather than ending it because `seeds` sorts after every number.
     Names break the ties left. A test goes with the latest of the nodes it
     tests, so it never makes a folder of its own nor points an edge back.

     `band` is the position of each node's folder in that order. `key` is the
     folder's path, or `@` and the package, and `name` what is written on it. */
  function dagFolders(nodes, edges, project, plain) {
    const n = nodes.length;
    const pkg = nodes.map((x) => String(x.id || '').split('.')[1] || '');
    const own = (v) => !project || pkg[v] === project;
    const dirs = nodes.map((x) => String(x.file || '').split('/').slice(0, -1));
    // Down one folder at a time while every model and source going that deep
    // shares it. One filed higher up, a `models/sources.yml` say, keeps its own
    // folder rather than lifting the level for all the others.
    const deep = [], root = [];
    for (let v = 0; v < n; v++) if (ANCHORS.has(nodes[v].kind) && own(v)) deep.push(dirs[v]);
    for (let l = 0; ; l++) {
      const reach = deep.filter((d) => d.length > l);
      if (!reach.length || reach.some((d) => d[l] !== reach[0][l])) break;
      root.push(reach[0][l]);
    }
    const fed = new Uint8Array(n);
    for (const [a, b] of edges) if (a !== b) fed[b] = 1;
    const key = nodes.map((x, v) => {
      if (x.kind === 'test' && fed[v]) return null;
      if (!own(v)) return '@' + pkg[v];
      const d = dirs[v];
      let k = 0;
      while (k < root.length && root[k] === d[k]) k++;
      return d.slice(0, k + 1).join('/');
    });
    const nameOf = (k) => (k[0] === '@' ? k.slice(1) : k.slice(k.lastIndexOf('/') + 1) || '/');
    const keys = [...new Set(key.filter((k) => k !== null))];
    keys.sort((a, b) => dagNatural(nameOf(a), nameOf(b)) || (a < b ? -1 : a > b ? 1 : 0));
    const B = keys.length, at = new Map(keys.map((k, i) => [k, i]));
    if (!B) return null;
    // How many drawn pairs of nodes lead from one folder into another.
    const w = new Float64Array(B * B), seen = new Set();
    for (const [a, b] of edges) {
      const x = at.get(key[a]), y = at.get(key[b]);
      if (x === undefined || y === undefined || x === y || seen.has(a * n + b)) continue;
      seen.add(a * n + b);
      w[x * B + y]++;
    }
    const numbered = keys.map((k) => k[0] !== '@' && /^\d/.test(nameOf(k)));
    const mean = new Float64Array(B), count = new Float64Array(B);
    if (plain) {
      for (let v = 0; v < n; v++) {
        if (key[v] === null) continue;
        const x = at.get(key[v]);
        mean[x] += plain[v]; count[x]++;
      }
      for (let x = 0; x < B; x++) mean[x] /= count[x];
    }
    const left = new Uint8Array(B).fill(1), place = new Int32Array(B);
    for (let s = 0; s < B; s++) {
      // The keys are in name order, so the first numbered one left is the lowest.
      let pick = -1, least = Infinity, lowest = -1;
      for (let y = 0; y < B && lowest < 0; y++) if (left[y] && numbered[y]) lowest = y;
      for (let y = 0; y < B; y++) {
        if (!left[y] || (numbered[y] && y !== lowest)) continue;
        let into = 0;
        for (let x = 0; x < B; x++) if (left[x]) into += w[x * B + y];
        if (into < least || (into === least && mean[y] < mean[pick])) { least = into; pick = y; }
      }
      left[pick] = 0; place[pick] = s;
    }
    const band = new Int32Array(n).fill(-1);
    for (let v = 0; v < n; v++) if (key[v] !== null) band[v] = place[at.get(key[v])];
    for (const [a, b] of edges) if (key[b] === null && band[a] > band[b]) band[b] = band[a];
    for (let v = 0; v < n; v++) if (band[v] < 0) band[v] = 0;
    const bands = new Array(B);
    keys.forEach((k, i) => {
      bands[place[i]] = { key: k, name: nameOf(k), size: 0, package: k[0] === '@' };
    });
    for (let v = 0; v < n; v++) bands[band[v]].size++;
    return { band, bands, root: root.join('/') };
  }

  /* Columns by longest path over the drawn edges, so every edge points right.
     Never n.depth: that is the server's distance from the focus, which the
     export turns into dbt's graph operators, and a model reached by a short
     path and by a long one sits at the short one, one of its parents then
     landing to its right (0027).

     Column lineage may hold cycles, an incremental model reading itself
     through the access history, so the edges a depth-first walk finds pointing
     back are turned round for the layering only, and self-loops stay out of
     it. A node with more readers than parents is then pulled right to just
     before its first reader, which shortens more edges than it stretches: the
     usual case is an input only a late model reads, which would otherwise sit
     in the first column at the end of a long edge.

     `band`, when the canvas is drawn by folder, gives each folder its own run
     of columns, starting one column after the folder before it ends. An edge
     from a later folder to an earlier one is turned round before the walk,
     since the folders now fix that order, which leaves the walk only the loops
     inside one folder to find. A node is pulled right no further than its
     folder's last column. `span` is each folder's first and last column. */
  function dagLayers(n, edges, R, band) {
    const { rank } = R;
    const out = Array.from({ length: n }, () => []);
    const index = new Map(), pairs = [], edgePair = [];
    for (const [a, b] of edges) {
      if (a === b) { edgePair.push(-1); continue; }
      const key = a * n + b;
      if (!index.has(key)) { index.set(key, pairs.length); out[a].push(pairs.length); pairs.push([a, b]); }
      edgePair.push(index.get(key));
    }
    out.forEach((l) => l.sort((x, y) => rank[pairs[x][1]] - rank[pairs[y][1]]));
    const indeg = new Int32Array(n), state = new Uint8Array(n), flip = new Uint8Array(pairs.length);
    let against = 0;
    if (band) pairs.forEach(([a, b], k) => { if (band[a] > band[b]) { flip[k] = 1; against++; } });
    pairs.forEach(([, b], k) => { if (!flip[k]) indeg[b]++; });
    // Roots first, so that in a cycle hanging off a root it is the edge closing
    // the loop that turns.
    for (const s of R.byRank.filter((v) => !indeg[v]).concat(R.byRank)) {
      if (state[s]) continue;
      const stack = [s], at = [0];
      state[s] = 1;
      while (stack.length) {
        const t = stack.length - 1, u = stack[t];
        if (at[t] < out[u].length) {
          const k = out[u][at[t]++], w = pairs[k][1];
          if (flip[k]) continue;
          if (state[w] === 1) flip[k] = 1;
          else if (!state[w]) { state[w] = 1; stack.push(w); at.push(0); }
        } else { state[u] = 2; stack.pop(); at.pop(); }
      }
    }
    const succ = Array.from({ length: n }, () => []), pred = Array.from({ length: n }, () => []);
    const deg = new Int32Array(n), layer = new Int32Array(n);
    pairs.forEach((p, k) => {
      const a = p[flip[k]], b = p[1 - flip[k]];
      succ[a].push(b); pred[b].push(a); deg[b]++;
    });
    const topo = R.byRank.filter((v) => !deg[v]);
    for (let h = 0; h < topo.length; h++) for (const w of succ[topo[h]]) if (!--deg[w]) topo.push(w);
    // Still an order every edge agrees with, since no edge left points to an
    // earlier folder: sort() is stable.
    if (band) topo.sort((x, y) => band[x] - band[y]);
    const span = [];
    for (let h = 0, start = 0; h < topo.length; h++) {
      const v = topo[h];
      if (band && (!h || band[v] !== band[topo[h - 1]])) {
        if (h) start = span[band[topo[h - 1]]][1] + 1;
        span[band[v]] = [start, start];
      }
      let l = band ? start : 0;
      for (const u of pred[v]) if (layer[u] + 1 > l) l = layer[u] + 1;
      layer[v] = l;
      if (band && l > span[band[v]][1]) span[band[v]][1] = l;
    }
    // Backwards, so every reader already sits in its final column.
    for (let h = topo.length - 1; h >= 0; h--) {
      const v = topo[h];
      if (!succ[v].length || pred[v].length >= succ[v].length) continue;
      const l = Math.min(...succ[v].map((w) => layer[w])) - 1;
      layer[v] = band ? Math.min(l, span[band[v]][1]) : l;
    }
    const lo = Math.min(0, ...layer);
    for (let i = 0; i < n; i++) layer[i] -= lo;
    return { pairs, flip, edgePair, layer, against, span };
  }

  /* The whole layout from the payload alone, which it never touches: the same
     object is counted in the status line and listed in an export, so the lanes
     live only in here. `dim` carries the sizes, which render() picks per mode.

     The heights come from Brandes and Kopf while its level lanes cost little,
     and from the compact isotonic step once they cost too much: its packing
     stacks blocks in a staircase, and with the tests of a 112 model
     neighbourhood ticked it drew 15 615 px where the tallest column, lanes
     included, needs 8 092: twice the zoom out on a graph that tall. Level
     lanes may spend half again the tallest column's height, no more. The
     isotonic heights are no promise either, only usually shorter, so they
     replace Brandes and Kopf only where they are.

     `folders`, when the canvas is drawn by folder, is the project's package,
     or '' when it is not known, which files every node as the project's own:
     the layout then gives each folder a band of columns (0029). Left out, or
     null, it draws as it always has. */
  function dagLayout(d, dim, folders, open) {
    // Tests with a model to hang under leave the grid: the rest is laid out as
    // it always was, each host made taller by what hangs under it (0034).
    // `open` holds the ids of the models whose folded tests are shown (0035),
    // and the keys of the groups turned from how they start (0040).
    const cut = dagRiders(d), c = cut ? cut.core : d, ride = dim.ride || RIDE;
    const n = c.nodes.length, R = dagRank(c.nodes);
    const F = folders == null ? null : dagFolders(c.nodes, c.edges, folders, dagLayers(n, c.edges, R).layer);
    const lay = dagLayers(n, c.edges, R, F && F.band);
    const hosts = cut && dagHosts(d, cut, lay.layer, R.rank, ride.fold, open, d.focus);
    const g = dagProper(n, lay, R.rank, c.edge_kinds || [], dim.bundle);
    if (hosts) g.extra = dagStacks(hosts, n, ride);
    dagOrder(g, R);
    let y = bkCoords(g, dim);
    if (dagSpan(g, y, dim) > 1.5 * dagTallest(g, dim)) {
      const iso = isoCoords(g, dim);
      if (dagSpan(g, iso, dim) < dagSpan(g, y, dim)) y = iso;
    }
    const L = dagDraw(c, g, lay, y, dim);
    if (F) dagBands(L, F, lay, dim);
    return hosts ? dagMount(d, cut, hosts, L, dim) : L;
  }

  /* Which tests hang under a model rather than take a column of their own
     (0034): every test with a drawn parent that is no test. A test read by
     nothing but tests, or by nothing at all, stays in the grid. `core` is the
     payload without them, for the layout proper, and `coreOf` finds a node's
     place in it. Null when no test hangs anywhere, which leaves every other
     canvas laid out exactly as before. */
  function dagRiders(d) {
    const n = d.nodes.length, parents = Array.from({ length: n }, () => []);
    const isTest = (v) => d.nodes[v].kind === 'test';
    d.edges.forEach(([a, b]) => { if (a !== b) parents[b].push(a); });
    const rider = new Uint8Array(n);
    let any = false;
    for (let v = 0; v < n; v++) if (isTest(v) && parents[v].some((p) => !isTest(p))) { rider[v] = 1; any = true; }
    if (!any) return null;
    const keep = [], coreOf = new Int32Array(n).fill(-1);
    for (let v = 0; v < n; v++) if (!rider[v]) { coreOf[v] = keep.length; keep.push(v); }
    const edges = [], kinds = [], edgeOf = [];
    d.edges.forEach(([a, b], i) => {
      if (rider[a] || rider[b]) return;
      edgeOf.push(i); edges.push([coreOf[a], coreOf[b]]);
      if (d.edge_kinds) kinds.push(d.edge_kinds[i]);
    });
    const core = { mode: d.mode, nodes: keep.map((v) => d.nodes[v]), edges };
    if (d.edge_kinds) core.edge_kinds = kinds;
    return { rider, parents, isTest, keep, coreOf, edgeOf, core };
  }

  /* The model each test hangs under: the one whose YAML declares it, which the
     server sends as `attached`, when it is drawn and read by the test. A
     singular test has no YAML, so it takes the parent in the furthest column,
     the last of its inputs to be built, then the first by name. Under a host,
     the tests go in rows, which `dagRows` makes.

     Past `fold` rows a host keeps its first `fold` and a chip counting the
     tests in the rest, `hidden`, which are not drawn at all, their edges with
     them: a model with a hundred tests would otherwise stand thousands of
     pixels tall. A host whose id is in `open` shows them all, and a chip to
     fold them again. The tests of a closed group are in `into`, under the row
     that stands for them. Nothing here changes an answer, only how much of it
     is drawn (0035, 0040). */
  function dagHosts(d, cut, layer, rank, fold, open, pin) {
    const host = new Int32Array(d.nodes.length).fill(-1), lists = new Map();
    for (let v = 0; v < d.nodes.length; v++) {
      if (!cut.rider[v]) continue;
      const own = d.nodes[v].attached;
      let h = own != null && cut.coreOf[own] >= 0 && !cut.isTest(own) && cut.parents[v].includes(own) ? cut.coreOf[own] : -1;
      if (h < 0) {
        for (const p of cut.parents[v]) {
          if (cut.isTest(p)) continue;
          const q = cut.coreOf[p];
          if (h < 0 || layer[q] > layer[h] || (layer[q] === layer[h] && rank[q] < rank[h])) h = q;
        }
      }
      host[v] = h;
      if (!lists.has(h)) lists.set(h, []);
      lists.get(h).push(v);
    }
    const stacks = new Map(), hidden = new Set(), chips = new Map(), into = new Map();
    const limit = fold == null ? Infinity : fold;
    lists.forEach((list, h) => {
      const id = d.nodes[cut.keep[h]].id;
      const R = dagRows(d, list, id, open, limit, pin);
      stacks.set(h, R.rows);
      R.hidden.forEach((v) => hidden.add(v));
      R.into.forEach((g, v) => into.set(v, g));
      if (R.chip) chips.set(h, Object.assign({ id }, R.chip));
    });
    return { host, stacks, hidden, chips, into };
  }

  /* One host's tests as rows (0040). The tests of one generic, at one level,
     named through one package, share a row once there are two of them, which
     opens into a row per test; a test alone is its own row, and so is every
     singular test, which has no generic to share. Generics on the whole model
     come first, then the singular tests, then the column generics, each by
     name: the checks of the model as a whole before those of its columns.

     A host whose rows, every group opened, fit in `fold` starts with its
     groups open, so a model with three tests reads as three; past that they
     start closed. A key in `open` turns its group from how it starts. Past
     `fold` top-level rows the rest fold under the chip, which counts tests.
     `pin`, the focus, is never folded away: its row stays, and in a closed
     group it is drawn under the header alone. */
  function dagRows(d, list, id, open, limit, pin) {
    const by = (a, b) => (a < b ? -1 : a > b ? 1 : 0);
    const named = (a, b) => by(String(d.nodes[a].name), String(d.nodes[b].name)) || by(String(d.nodes[a].id), String(d.nodes[b].id)) || a - b;
    const ORDER = { model: 0, singular: 1, column: 2 };
    const parts = new Map(), entries = [];
    for (const v of list) {
      const n = d.nodes[v], level = testLevel(n);
      if (level === 'singular') { entries.push({ level, generic: '', ns: '', tests: [v] }); continue; }
      const key = groupKey(id, level, n.namespace, n.test_name);
      if (!parts.has(key)) {
        const e = { key, level, generic: String(n.test_name), ns: String(n.namespace || ''), tests: [] };
        parts.set(key, e);
        entries.push(e);
      }
      parts.get(key).tests.push(v);
    }
    entries.forEach((e) => {
      e.tests.sort(e.level === 'column'
        ? (a, b) => by(String(d.nodes[a].column), String(d.nodes[b].column)) || named(a, b)
        : named);
    });
    entries.sort((a, b) => ORDER[a.level] - ORDER[b.level] || by(a.generic, b.generic) || by(a.ns, b.ns) || named(a.tests[0], b.tests[0]));
    const whole = entries.reduce((t, e) => t + (e.tests.length > 1 ? 1 + e.tests.length : 1), 0);
    const opened = whole <= limit;
    const top = entries.map((e) => {
      if (e.tests.length === 1) return { type: 'test', v: e.tests[0], level: e.level };
      const cols = new Set(e.tests.map((v) => String(d.nodes[v].column)));
      return {
        type: 'group', key: e.key, level: e.level, generic: e.generic, ns: e.ns, members: e.tests,
        cols: e.level === 'column' ? cols.size : 0,
        warn: e.tests.filter((v) => d.nodes[v].warn).length,
        open: opened !== !!(open && open.has(e.key)),
      };
    });
    const holds = (r) => (r.type === 'group' ? r.members.includes(pin) : r.v === pin);
    const testsOf = (r) => (r.type === 'group' ? r.members : [r.v]);
    let shown = top, chip = null;
    const hidden = [];
    if (top.length > limit) {
      const all = !!(open && open.has(id));
      if (all) {
        chip = { open: true, count: top.slice(limit).reduce((t, r) => t + testsOf(r).length, 0) };
      } else {
        shown = top.slice(0, limit);
        const kept = top.slice(limit).find(holds);
        if (kept) shown.push(kept);
        top.slice(limit).forEach((r) => { if (r !== kept) hidden.push(...testsOf(r)); });
        chip = { open: false, count: hidden.length };
      }
    }
    const rows = [], into = new Map();
    for (const r of shown) {
      rows.push(r);
      if (r.type !== 'group') continue;
      const twice = r.level === 'column' && r.cols < r.members.length;
      for (const v of r.members) {
        if (r.open || v === pin) rows.push({ type: 'member', v, level: r.level, of: r, byName: twice || r.level === 'model' });
        else into.set(v, r);
      }
    }
    return { rows, hidden, into, chip };
  }

  // A group's key, unique across hosts and apart from any host id, which
  // shares `open` with it. Id characters only, so it can sit in an attribute.
  function groupKey(id, level, ns, generic) {
    return `${id}#${level}:${ns ? ns + '.' : ''}${generic}`;
  }

  /* What a row says, and how much of its end clipping must leave: the column
     after a lone column test, and the count and chevron after a group. A
     member is told apart from its siblings by its column, or, where that does
     not, by dbt's name without the part they all share. */
  function rowLabel(r, d, host) {
    if (r.type === 'group') {
      const n = r.members.length;
      const what = r.level === 'column' ? `  ${n} ${r.cols === n ? 'cols' : 'tests'}` : ` ×${n}`;
      const tail = `${what} ${r.open ? '▾' : '▸'}`;
      return { text: r.generic + tail, keep: tail.length };
    }
    const n = d.nodes[r.v];
    if (r.type === 'member') {
      if (!r.byName) return { text: String(n.column), keep: 0 };
      return { text: memberName(n, host), keep: 0 };
    }
    if (r.level === 'column') {
      const tail = ` · ${n.column}`;
      return { text: n.test_name + tail, keep: tail.length };
    }
    return { text: String(r.level === 'model' ? n.test_name : n.name), keep: 0 };
  }

  /* dbt's name for a test without what every test of its group starts with,
     the package, the generic and the model, which is what is left to tell six
     row-count checks on one model apart. The underscores after each go too,
     since a name given in the YAML often joins its parts with two. A name dbt
     shortened with a hash does not start that way and is kept whole. */
  function memberName(n, host) {
    let s = String(n.name);
    for (const head of [n.namespace, n.test_name, host]) {
      if (!head || !s.startsWith(head + '_')) continue;
      const rest = s.slice(head.length).replace(/^_+/, '');
      if (rest) s = rest;
    }
    return s;
  }

  /* At most `max` characters, never cutting the last `keep` while six of
     what comes before can still show: a column or a count is what a row is
     read for, and the generic is what it is read with. Past that the end is
     cut too, after ten of the generic, or all of it when it is that short. */
  function rowClip(text, keep, max) {
    text = String(text);
    if (text.length <= max) return text;
    if (keep <= 0) return text.slice(0, max - 1) + '…';
    const head = text.slice(0, text.length - keep), tail = text.slice(text.length - keep);
    if (keep + 1 + Math.min(6, head.length) <= max) return head.slice(0, max - keep - 1) + '…' + tail;
    const h = head.length <= 10 ? head : head.slice(0, 10) + '…';
    return h + tail.slice(0, max - h.length - 1) + '…';
  }

  // What a fold chip says: how many more there are, or, opened, how many it
  // folds away again, a chevron up saying which way, as a group's row does.
  function foldLabel(c) {
    const noun = c.count === 1 ? 'test' : 'tests';
    return c.open ? `▴ hide ${c.count} ${noun}` : `+${c.count} more ${noun}`;
  }

  // How much taller each host is for what hangs under it, a chip counting as a row.
  function dagStacks(hosts, n, ride) {
    const extra = new Float64Array(n);
    hosts.stacks.forEach((list, h) => {
      const rows = list.length + (hosts.chips.has(h) ? 1 : 0);
      extra[h] = ride.top + rows * ride.h + (rows - 1) * ride.gap;
    });
    return extra;
  }

  /* Half a node's height: nothing for a lane, half a box for a box, and half
     of what hangs under it besides for a host, whose box sits at the top of
     the room it takes. */
  function dagHalf(g, v, dim) {
    if (g.dummy[v]) return 0;
    return (dim.h + (g.extra && v < g.n ? g.extra[v] : 0)) / 2;
  }

  // Where an edge meets a node: a lane's line, or the middle of the box itself.
  function dagAnchor(g, y, v, dim) {
    return g.dummy[v] ? y[v] : y[v] - dagHalf(g, v, dim) + dim.h / 2;
  }

  /* The whole payload again, from the core's layout: every core box and edge
     where it was drawn, and each hanging test under its host with a stem from
     the host's lower edge. An edge from another parent reaches the test from
     the left, the way every edge reaches its reader; from the same column it
     loops out on the right, and from a later column it runs back, dashed, as a
     turned edge does. A closed group's row stands for its tests: its own stem
     for their host's, and one edge from each other parent they share, the one
     into the first of them (0040). The frame grows by every control point, so
     it holds the curves too. */
  function dagMount(d, cut, hosts, L, dim) {
    const ride = dim.ride || RIDE, n = d.nodes.length;
    const boxes = new Array(n), paths = new Array(d.edges.length), back = new Array(d.edges.length).fill(0);
    cut.keep.forEach((v, c) => { boxes[v] = L.boxes[c]; });
    const bb = L.bbox ? [L.bbox.x0, L.bbox.y0, L.bbox.x1, L.bbox.y1] : [Infinity, Infinity, -Infinity, -Infinity];
    const grow = (x, yy) => {
      bb[0] = Math.min(bb[0], x); bb[1] = Math.min(bb[1], yy);
      bb[2] = Math.max(bb[2], x); bb[3] = Math.max(bb[3], yy);
    };
    const mid = (b) => b.y + b.h / 2;
    const stem = (A, B) => {
      const x = A.x + ride.indent / 2;
      return `M${x},${A.y + A.h} L${x},${mid(B)} L${B.x},${mid(B)}`;
    };
    const chips = [], groups = [], heads = new Map();
    hosts.stacks.forEach((list, h) => {
      const hb = L.boxes[h], name = d.nodes[cut.keep[h]].name;
      const slot = (i) => ({ x: hb.x + ride.indent, y: hb.y + hb.h + ride.top + i * (ride.h + ride.gap), w: hb.w - ride.indent, h: ride.h });
      list.forEach((r, i) => {
        const b = slot(i), say = rowLabel(r, d, name);
        grow(b.x, b.y); grow(b.x + b.w, b.y + b.h);
        if (r.type === 'group') {
          const g = Object.assign(b, {
            host: cut.keep[h], key: r.key, open: r.open, count: r.members.length, level: r.level, warn: r.warn,
            generic: r.generic, ns: r.ns, cols: r.cols,
            label: say.text, keep: say.keep, members: r.members, stem: stem(hb, b),
          });
          groups.push(g);
          heads.set(r, g);
          return;
        }
        if (r.type === 'member') { b.x += ride.sub; b.w -= ride.sub; }
        boxes[r.v] = Object.assign(b, {
          rider: true, test: r.level, member: r.type === 'member', warn: !!d.nodes[r.v].warn, label: say.text, keep: say.keep,
        });
      });
      const chip = hosts.chips.get(h);
      if (chip) {
        const b = Object.assign(slot(list.length), { host: cut.keep[h], id: chip.id, open: chip.open, count: chip.count });
        chips.push(b);
        grow(b.x, b.y); grow(b.x + b.w, b.y + b.h);
      }
    });
    // A folded test is not drawn, and neither is any edge into it. Nor is a
    // test in a closed group, whose row the edges into it reach instead.
    hosts.hidden.forEach((v) => { boxes[v] = null; });
    hosts.into.forEach((r, v) => { boxes[v] = null; });
    // Of the edges from one parent into the tests of one closed group, the
    // one into the first of them in the group's order, whatever order the
    // server listed the edges in.
    const shared = new Map();
    heads.forEach((g, r) => {
      if (r.open) return;
      const host = g.host, from = new Map();
      for (const v of r.members) {
        if (!hosts.into.has(v)) continue;
        for (const p of cut.parents[v]) if (p !== host && !from.has(p)) from.set(p, v);
      }
      shared.set(r, from);
    });
    cut.edgeOf.forEach((i, c) => { paths[i] = L.paths[c]; back[i] = L.back[c]; });
    const curve = (pts) => {
      pts.forEach(([x, yy]) => grow(x, yy));
      const [p0, p1, p2, p3] = pts;
      return `M${p0[0]},${p0[1]} C${p1[0]},${p1[1]} ${p2[0]},${p2[1]} ${p3[0]},${p3[1]}`;
    };
    d.edges.forEach(([a, v], i) => {
      if (paths[i] !== undefined) return;
      const r = hosts.into.get(v);
      const A = boxes[a], B = r ? heads.get(r) : boxes[v];
      if (!A || !B) { paths[i] = null; return; }
      if (cut.rider[v] && cut.keep[hosts.host[v]] === a) {
        paths[i] = r ? null : stem(A, B);
        return;
      }
      if (r && shared.get(r).get(a) !== v) { paths[i] = null; return; }
      const ay = mid(A), by = mid(B);
      if (A.x + A.w < B.x) {
        const dx = Math.max(34, (B.x - A.x - A.w) * 0.45);
        paths[i] = curve([[A.x + A.w, ay], [A.x + A.w + dx, ay], [B.x - dx, by], [B.x, by]]);
      } else if (A.x > B.x + B.w) {
        const dx = Math.max(34, (A.x - B.x - B.w) * 0.45);
        paths[i] = curve([[A.x, ay], [A.x - dx, ay], [B.x + B.w + dx, by], [B.x + B.w, by]]);
        back[i] = 1;
      } else {
        const ear = Math.max(A.x + A.w, B.x + B.w) + 30;
        paths[i] = curve([[A.x + A.w, ay], [ear, ay], [ear, by], [B.x + B.w, by]]);
      }
    });
    const out = { boxes, paths, back, bbox: n ? { x0: bb[0], y0: bb[1], x1: bb[2], y1: bb[3] } : null };
    if (L.bands) { out.bands = L.bands; out.against = L.against; }
    if (chips.length) { out.chips = chips; out.folded = hosts.hidden.size; }
    if (groups.length) out.groups = groups;
    return out;
  }

  /* Where each band is drawn: its columns, plus a margin that keeps the +N
     badges and the loops inside it and leaves a gutter between two bands. The
     frame grows to hold them, and above the boxes by the room an exported file
     writes their names in. Heights are left to whoever draws them: on the
     canvas a band runs the whole height, so its name can sit at the top of the
     window wherever that is. */
  function dagBands(L, F, lay, dim) {
    const step = dim.w + dim.hgap, margin = dim.hgap * 3 / 8;
    L.bands = F.bands.map((b, i) => Object.assign({}, b, {
      x0: lay.span[i][0] * step - margin,
      x1: lay.span[i][1] * step + dim.w + margin,
    }));
    L.against = lay.against;
    if (!L.bbox) return;
    L.bbox.x0 = Math.min(L.bbox.x0, L.bands[0].x0);
    L.bbox.x1 = Math.max(L.bbox.x1, L.bands[L.bands.length - 1].x1);
    L.bbox.y0 -= 20;
  }

  /* How far apart two neighbours in a column sit, centre to centre. Two boxes
     keep VGAP between them; two lanes only need telling apart, so they sit
     closer. A lane beside a box keeps clear of the role tag column mode draws
     above each box. */
  function dagGap(g, a, b, dim) {
    const da = g.dummy[a], db = g.dummy[b];
    return dagHalf(g, a, dim) + dagHalf(g, b, dim) + (da && db ? dim.lane : dim.vgap) + (da !== db ? dim.badge : 0);
  }

  // The height of the tallest column stacked tight: no layout can be shorter.
  function dagTallest(g, dim) {
    let most = 0;
    for (const arr of g.layers) {
      if (!arr.length) continue;
      let h = dagHalf(g, arr[0], dim) + dagHalf(g, arr[arr.length - 1], dim);
      for (let i = 1; i < arr.length; i++) h += dagGap(g, arr[i - 1], arr[i], dim);
      most = Math.max(most, h);
    }
    return most;
  }

  function dagSpan(g, y, dim) {
    let lo = Infinity, hi = -Infinity;
    for (let v = 0; v < g.N; v++) {
      const half = dagHalf(g, v, dim);
      lo = Math.min(lo, y[v] - half); hi = Math.max(hi, y[v] + half);
    }
    return hi - lo;
  }

  /* An edge spanning several columns gets a dummy node in each column it
     crosses, a lane, so the ordering can route it between the boxes rather than
     behind them. A node with several such edges of one kind shares one lane its
     readers branch off: one lane per edge turns a date spine read across the
     whole graph into a band of hundreds of parallel lines. A lane has one
     colour, hence one kind. An edge turned round for a cycle keeps a lane of
     its own, even beside its forward twin, or the loop would vanish into one
     line. Pairs are walked in rank order so the dummies, and every tie they
     take part in, do not depend on the order the server listed the edges in. */
  function dagProper(n, lay, rank, kinds, bundle) {
    const layer = Array.from(lay.layer), dummy = [], up = [], down = [], tip = [];
    for (let i = 0; i < n; i++) { dummy.push(0); up.push([]); down.push([]); tip.push(i); }
    const kindOf = [], long = new Int32Array(n), linked = new Set(), shared = new Map();
    lay.edgePair.forEach((k, e) => { if (k >= 0 && kindOf[k] === undefined) kindOf[k] = String(kinds[e] || ''); });
    const ends = (k) => (lay.flip[k] ? [lay.pairs[k][1], lay.pairs[k][0]] : lay.pairs[k]);
    lay.pairs.forEach((_, k) => {
      const [a, b] = ends(k);
      if (!lay.flip[k] && layer[b] - layer[a] > 1) long[a]++;
    });
    const link = (a, b) => {
      const key = `${a} ${b}`;
      if (linked.has(key)) return;
      linked.add(key); down[a].push(b); up[b].push(a);
    };
    const order = lay.pairs.map((_, k) => k);
    order.sort((x, y) => {
      const p = lay.pairs[x], q = lay.pairs[y];
      return rank[p[0]] - rank[q[0]] || rank[p[1]] - rank[q[1]];
    });
    const chainOf = new Array(lay.pairs.length);
    for (const k of order) {
      const [a, b] = ends(k);
      const key = !lay.flip[k] && bundle && long[a] >= bundle ? `${a} ${kindOf[k]} ` : '';
      const chain = [a];
      let prev = a;
      for (let l = layer[a] + 1; l < layer[b]; l++) {
        let z = key ? shared.get(key + l) : undefined;
        if (z === undefined) {
          z = layer.length;
          layer.push(l); dummy.push(1); up.push([]); down.push([]); tip.push(b);
          if (key) shared.set(key + l, z);
        }
        link(prev, z); chain.push(z); prev = z;
      }
      link(prev, b); chain.push(b);
      chainOf[k] = chain;
    }
    down.forEach((l) => l.sort((x, y) => rank[tip[x]] - rank[tip[y]] || x - y));
    const nl = layer.reduce((m, l) => Math.max(m, l + 1), 0);
    return { n, N: layer.length, layer, dummy, up, down, chainOf, nl };
  }

  /* A depth-first start, then median sweeps and adjacent swaps, keeping the
     order with the fewest crossings seen, since a sweep can make things worse.
     Four sweeps without progress, or a work budget counted in nodes rather than
     milliseconds so the result never depends on the machine, end the search. */
  function dagOrder(g, R) {
    const layers = Array.from({ length: g.nl }, () => []);
    const pos = new Int32Array(g.N), seen = new Uint8Array(g.N);
    for (const s of R.byRank.slice().sort((a, b) => g.layer[a] - g.layer[b])) {
      const stack = [s];
      while (stack.length) {
        const v = stack.pop();
        if (seen[v]) continue;
        seen[v] = 1; pos[v] = layers[g.layer[v]].length; layers[g.layer[v]].push(v);
        for (let j = g.down[v].length - 1; j >= 0; j--) if (!seen[g.down[v][j]]) stack.push(g.down[v][j]);
      }
    }
    const key = new Float64Array(g.N);
    let best = layers.map((a) => a.slice()), bestC = dagCrossings(layers, g, pos);
    for (let it = 0, stale = 0; it < 24 && stale < 4 && bestC > 0 && it * g.N < 120000; it++) {
      dagMedianSweep(layers, g, pos, key, it % 2 === 0);
      dagTranspose(layers, g, pos);
      const c = dagCrossings(layers, g, pos);
      if (c < bestC) { bestC = c; best = layers.map((a) => a.slice()); stale = 0; } else stale++;
    }
    best.forEach((a) => a.forEach((v, i) => { pos[v] = i; }));
    g.layers = best; g.pos = pos; g.crossings = bestC;
  }

  // Weighted towards the side where the neighbours sit closer together.
  function dagMedian(P) {
    const m = P.length >> 1;
    if (P.length % 2) return P[m];
    if (P.length === 2) return (P[0] + P[1]) / 2;
    const left = P[m - 1] - P[0], right = P[P.length - 1] - P[m];
    return left + right ? (P[m - 1] * right + P[m] * left) / (left + right) : (P[m - 1] + P[m]) / 2;
  }

  // A node with nothing on the swept side keeps its slot: it has no reason to move.
  function dagMedianSweep(layers, g, pos, key, downward) {
    const nb = downward ? g.up : g.down;
    const byKey = (a, b) => key[a] - key[b] || pos[a] - pos[b];
    for (let s = 1; s < layers.length; s++) {
      const layer = layers[downward ? s : layers.length - 1 - s], movers = [], slots = [];
      layer.forEach((v, i) => {
        const ns = nb[v];
        if (!ns.length) return;
        key[v] = ns.length === 1 ? pos[ns[0]] : dagMedian(ns.map((u) => pos[u]).sort((a, b) => a - b));
        movers.push(v); slots.push(i);
      });
      movers.sort(byKey);
      slots.forEach((slot, i) => { layer[slot] = movers[i]; });
      layer.forEach((v, i) => { pos[v] = i; });
    }
  }

  // Crossings with v above w, minus crossings with w above v, on one side.
  function dagSwapGain(A, B, pos) {
    let c = 0;
    for (const a of A) for (const b of B) c += pos[a] > pos[b] ? 1 : pos[a] < pos[b] ? -1 : 0;
    return c;
  }

  function dagTranspose(layers, g, pos) {
    for (let round = 0, better = true; better && round < 8; round++) {
      better = false;
      for (const layer of layers) {
        for (let j = 0; j + 1 < layer.length; j++) {
          const v = layer[j], w = layer[j + 1];
          if (dagSwapGain(g.up[v], g.up[w], pos) + dagSwapGain(g.down[v], g.down[w], pos) > 0) {
            layer[j] = w; layer[j + 1] = v; pos[w] = j; pos[v] = j + 1; better = true;
          }
        }
      }
    }
  }

  // Exact, between each pair of neighbouring columns, as inversions counted in a Fenwick tree.
  function dagCrossings(layers, g, pos) {
    let total = 0, tree = new Int32Array(1);
    const P = [];
    for (let l = 0; l + 1 < layers.length; l++) {
      const m = layers[l + 1].length;
      let seen = 0;
      if (tree.length < m + 1) tree = new Int32Array(m + 1); else tree.fill(0);
      for (const v of layers[l]) {
        P.length = 0;
        for (const w of g.down[v]) P.push(pos[w]);
        if (P.length > 1) P.sort((x, y) => x - y);
        for (const p of P) {
          let s = 0;
          for (let i = p + 1; i > 0; i -= i & -i) s += tree[i];
          total += seen - s;
          for (let i = p + 1; i <= m; i += i & -i) tree[i]++;
          seen++;
        }
      }
    }
    return total;
  }

  /* Brandes and Kopf (2001) on its side: their x is our y. Four alignments,
     towards parents or children, packed from the top or from the bottom; then
     each node takes the mean of its two middle values, because any single
     alignment leans every fan the same way. A lane that carries on keeps its
     line: a dummy follows the dummy next to it, and a box that a shared lane
     only hands a branch to bends towards it instead of dragging it off. A
     segment between two lanes comes out level unless another lane crosses it
     there, which the ordering does not rule out: of two crossing lanes, no
     alignment keeps both level. The isotonic step below keeps fewer level. */
  function bkCoords(g, dim) {
    const N = g.N, marked = bkConflicts(g), runs = [];
    for (const upward of [true, false]) {
      for (const flip of [false, true]) {
        let L = upward ? g.layers : g.layers.slice().reverse();
        if (flip) L = L.map((a) => a.slice().reverse());
        const p = new Int32Array(N);
        L.forEach((a) => a.forEach((v, i) => { p[v] = i; }));
        const beyond = upward ? g.down : g.up;
        const nb = (upward ? g.up : g.down).map((a, v) => {
          if (g.dummy[v]) {
            const lane = a.find((u) => g.dummy[u]);
            if (lane !== undefined) return [lane];
          } else a = a.filter((u) => !(g.dummy[u] && beyond[u].some((w) => g.dummy[w])));
          return a.length < 2 ? a : a.slice().sort((x, y) => p[x] - p[y]);
        });
        const y = bkCompact(L, bkAlign(L, nb, p, marked, upward), g, dim);
        if (flip) for (let i = 0; i < N; i++) y[i] = -y[i];
        runs.push(y);
      }
    }
    const ext = runs.map((y) => {
      let lo = Infinity, hi = -Infinity;
      for (let i = 0; i < N; i++) {
        const half = dagHalf(g, i, dim);
        lo = Math.min(lo, y[i] - half); hi = Math.max(hi, y[i] + half);
      }
      return [lo, hi];
    });
    let small = 0;
    ext.forEach((e, i) => { if (e[1] - e[0] < ext[small][1] - ext[small][0]) small = i; });
    const out = new Float64Array(N), four = [0, 0, 0, 0];
    for (let v = 0; v < N; v++) {
      // Each run is lined up on the narrowest one, by the side it was packed against.
      for (let r = 0; r < 4; r++) four[r] = runs[r][v] + (r % 2 ? ext[small][1] - ext[r][1] : ext[small][0] - ext[r][0]);
      four.sort((a, b) => a - b);
      out[v] = (four[1] + four[2]) / 2;
    }
    return out;
  }

  /* A segment between two dummies is the middle of a long edge. Any other
     segment crossing it gives up its right to be aligned, so long edges stay
     straight and the short ones bend around them. */
  function bkConflicts(g) {
    const marked = new Set(), { N, pos } = g;
    for (let l = 1; l < g.layers.length; l++) {
      const prev = g.layers[l - 1], layer = g.layers[l];
      let k0 = 0, scan = 0;
      for (let i = 0; i < layer.length; i++) {
        const v = layer[i];
        let w = -1;
        if (g.dummy[v]) for (const u of g.up[v]) if (g.dummy[u]) w = u;
        if (w < 0 && i < layer.length - 1) continue;
        const k1 = w >= 0 ? pos[w] : prev.length;
        for (; scan <= i; scan++) {
          const s = layer[scan];
          for (const u of g.up[s]) {
            if ((pos[u] < k0 || pos[u] > k1) && !(g.dummy[u] && g.dummy[s])) marked.add(u * N + s);
          }
        }
        k0 = k1;
      }
    }
    return marked;
  }

  // Each node joins the block of a median neighbour, unless that would cross a block already made.
  function bkAlign(L, nb, p, marked, upward) {
    const N = nb.length, root = new Int32Array(N), align = new Int32Array(N);
    for (let v = 0; v < N; v++) { root[v] = v; align[v] = v; }
    for (const layer of L) {
      let r = -1;
      for (const v of layer) {
        const ns = nb[v], k = ns.length, ms = [(k - 1) >> 1, k >> 1];
        for (let j = 0; k && j < 2 && align[v] === v; j++) {
          const u = ns[ms[j]];
          if (r < p[u] && !marked.has(upward ? u * N + v : v * N + u)) {
            align[u] = v; root[v] = root[u]; align[v] = root[v]; r = p[u];
          }
        }
      }
    }
    return root;
  }

  // Each block goes as high as the blocks above it let it.
  function bkCompact(L, root, g, dim) {
    const N = g.N, next = Array.from({ length: N }, () => []);
    const indeg = new Int32Array(N), top = new Float64Array(N);
    for (const layer of L) {
      for (let j = 1; j < layer.length; j++) {
        const a = layer[j - 1], b = layer[j];
        next[root[a]].push(root[b], dagGap(g, a, b, dim)); indeg[root[b]]++;
      }
    }
    const topo = [];
    for (let i = 0; i < N; i++) if (root[i] === i && !indeg[i]) topo.push(i);
    for (let h = 0; h < topo.length; h++) {
      const a = topo[h], nx = next[a];
      for (let k = 0; k < nx.length; k += 2) {
        if (top[nx[k]] < top[a] + nx[k + 1]) top[nx[k]] = top[a] + nx[k + 1];
        if (!--indeg[nx[k]]) topo.push(nx[k]);
      }
    }
    const y = new Float64Array(N);
    for (let i = 0; i < N; i++) y[i] = top[root[i]];
    return y;
  }

  /* The compact heights. Every column starts stacked and centred, then each
     node moves towards what it is wired to, as far as order and gaps allow. A
     lane follows only the side a sweep comes from, so a long edge runs level
     and bends at one end rather than sloping across every column, and the last
     sweep runs downstream, so an edge leaves its source level. Lanes pull
     harder than boxes, 8 between two lanes, 2 between a lane and a box, 1
     between two boxes, which keeps most long edges level. Whole pixels at the
     end: the gaps are whole, so rounding keeps every one of them. */
  function isoCoords(g, dim) {
    const N = g.N, y = new Float64Array(N), T = new Float64Array(N), Wt = new Float64Array(N);
    const bm = new Float64Array(N), bw = new Float64Array(N), bn = new Int32Array(N);
    const offs = g.layers.map((arr) => {
      const off = new Float64Array(arr.length);
      for (let i = 1; i < arr.length; i++) off[i] = off[i - 1] + dagGap(g, arr[i - 1], arr[i], dim);
      return off;
    });
    g.layers.forEach((arr, l) => {
      const mid = arr.length ? offs[l][arr.length - 1] / 2 : 0;
      arr.forEach((v, i) => { y[v] = offs[l][i] - mid; });
    });
    const pull = (i, v, list) => {
      for (const u of list) {
        const w = [1, 2, 8][g.dummy[v] + g.dummy[u]];
        T[i] += w * y[u]; Wt[i] += w;
      }
    };
    const settle = (l, downstream) => {
      const arr = g.layers[l];
      arr.forEach((v, i) => {
        T[i] = 0; Wt[i] = 0;
        if (g.dummy[v]) pull(i, v, downstream ? g.up[v] : g.down[v]);
        if (!Wt[i]) { pull(i, v, g.up[v]); pull(i, v, g.down[v]); }
        if (Wt[i]) T[i] /= Wt[i]; else { T[i] = y[v]; Wt[i] = 1e-3; }
      });
      isoFit(arr, offs[l], T, Wt, y, bm, bw, bn);
    };
    for (let it = 0; it < 8; it++) {
      if (it % 2) for (let l = 0; l < g.layers.length; l++) settle(l, true);
      else for (let l = g.layers.length - 1; l >= 0; l--) settle(l, false);
    }
    for (let v = 0; v < N; v++) y[v] = Math.round(y[v]);
    return y;
  }

  /* The placement of one column closest to its targets that keeps its order
     and gaps: adjacent violators pooled, each block at the weighted mean of
     what its members want. */
  function isoFit(arr, off, target, weight, y, bm, bw, bn) {
    let top = 0;
    for (let i = 0; i < arr.length; i++) {
      bm[top] = target[i] - off[i]; bw[top] = weight[i]; bn[top] = 1; top++;
      while (top > 1 && bm[top - 2] >= bm[top - 1]) {
        top--;
        const k = top - 1;
        bm[k] = (bm[k] * bw[k] + bm[top] * bw[top]) / (bw[k] + bw[top]); bw[k] += bw[top]; bn[k] += bn[top];
      }
    }
    for (let i = 0, k = 0; k < top; k++) for (let j = 0; j < bn[k]; j++, i++) y[arr[i]] = bm[k] + off[i];
  }

  /* One path per edge, through its lanes: level across a column it only
     passes, the canvas's usual curve between two columns. `back` marks an edge
     drawn against its direction, one turned round for a cycle or a self-loop,
     so the renderer can dash it: without arrowheads, left to right is the only
     direction a reader has. The back half of a two-node loop runs beside its
     forward half rather than on it. A self-loop is an ear on the right of its
     box, above the +N badge and right of the role tag. `bbox` counts the lanes,
     which can run above or below every box. */
  function dagDraw(d, g, lay, y, dim) {
    const step = dim.w + dim.hgap, n = d.nodes.length, bb = [Infinity, Infinity, -Infinity, -Infinity];
    const r = (v) => Math.round(v * 10) / 10;
    const grow = (x, yy) => {
      bb[0] = Math.min(bb[0], x); bb[1] = Math.min(bb[1], yy);
      bb[2] = Math.max(bb[2], x); bb[3] = Math.max(bb[3], yy);
    };
    const boxes = d.nodes.map((_, v) => {
      const b = { x: g.layer[v] * step, y: r(y[v] - dagHalf(g, v, dim)), w: dim.w, h: dim.h };
      grow(b.x, b.y); grow(b.x + b.w, b.y + b.h);
      return b;
    });
    for (let v = g.n; v < g.N; v++) grow(g.layer[v] * step, y[v]);
    const forward = new Set();
    lay.pairs.forEach((p, k) => { if (!lay.flip[k]) forward.add(p[0] * n + p[1]); });
    const back = [];
    const paths = d.edges.map(([a], i) => {
      const k = lay.edgePair[i];
      back.push(k < 0 || lay.flip[k] ? 1 : 0);
      if (k < 0) {
        const b = boxes[a], x0 = b.x + dim.w;
        grow(x0 + 23, b.y - 4);
        return `M${x0},${b.y + 8} C${x0 + 30},${b.y + 8} ${x0 + 30},${b.y - 12} ${x0},${b.y + 1}`;
      }
      const chain = g.chainOf[k], last = chain.length - 1;
      const shift = lay.flip[k] && last === 1 && forward.has(chain[0] * n + chain[1]) ? 8 : 0;
      let x = g.layer[chain[0]] * step + dim.w, yy = r(dagAnchor(g, y, chain[0], dim)) + shift, s = `M${x},${yy}`;
      for (let j = 1; j <= last; j++) {
        const w = chain[j], x2 = g.layer[w] * step, y2 = r(dagAnchor(g, y, w, dim)) + (j === last ? shift : 0);
        const dx = Math.max(34, (x2 - x) * 0.45);
        s += ` C${x + dx},${yy} ${x2 - dx},${y2} ${x2},${y2}`;
        x = x2; yy = y2;
        if (g.dummy[w]) { x += dim.w; s += ` L${x},${yy}`; }
      }
      return s;
    });
    return { boxes, paths, back, bbox: n ? { x0: bb[0], y0: bb[1], x1: bb[2], y1: bb[3] } : null };
  }

  let svg, root, handlers = {}, data = null, place = [], groups = [], bbox = null;
  let view = { k: 1, x: 0, y: 0 }, selected = null;
  // The folder bands drawn, and the layer over the canvas their names sit in.
  let bands = [], heads = null, drawn = null;
  // The models whose folded tests are shown, by id, kept across redraws, and
  // the options the last render() had, to draw again after a chip's click.
  const unfolded = new Set();
  let lastOpts = {};
  // Module scope, not init's: render() reads it to keep a hover card from
  // opening under a pointer that is in the middle of a pan.
  let drag = null;

  const el = (name, attrs = {}) => {
    const n = document.createElementNS(NS, name);
    for (const [k, v] of Object.entries(attrs)) n.setAttribute(k, v);
    return n;
  };
  const clip = (s, max) => (s.length > max ? s.slice(0, max - 1) + '…' : s);

  /* Second line of a node box. Column mode ships a ready-made `sub`; model mode
     builds one from the materialization, schema and test count.
     `tests` is a count in a lineage node and a list in an /api/node payload, and
     the hover card feeds it the second: counting here rather than at each call
     site keeps "[object Object]" out of the line. */
  function subtitle(n) {
    if (n.sub) return n.sub;
    const bits = [n.disabled ? 'disabled' : n.kind === 'source' ? 'source' : (n.materialized || n.kind)];
    if (n.schema) bits.push(n.schema);
    const tests = Array.isArray(n.tests) ? n.tests.length : n.tests;
    if (tests) bits.push(`${tests} test${tests > 1 ? 's' : ''}`);
    return bits.join('  ·  ');
  }

  function init(svgEl, h) {
    svg = svgEl; handlers = h || {};
    root = el('g');
    svg.appendChild(root);
    heads = document.createElement('div');
    heads.className = 'folder-heads';
    svg.parentNode.insertBefore(heads, svg.nextSibling);

    svg.addEventListener('wheel', (e) => {
      e.preventDefault();
      const r = svg.getBoundingClientRect();
      const mx = e.clientX - r.left, my = e.clientY - r.top;
      const f = Math.exp(-e.deltaY * 0.0015);
      const k = Math.min(3, Math.max(0.08, view.k * f));
      view.x = mx - (mx - view.x) * (k / view.k);
      view.y = my - (my - view.y) * (k / view.k);
      view.k = k;
      apply();
    }, { passive: false });

    svg.addEventListener('mousedown', (e) => {
      if (e.button !== 0) return;
      drag = { x: e.clientX, y: e.clientY, vx: view.x, vy: view.y, moved: false };
      svg.classList.add('panning');
    });
    window.addEventListener('mousemove', (e) => {
      if (!drag) return;
      const dx = e.clientX - drag.x, dy = e.clientY - drag.y;
      if (Math.abs(dx) + Math.abs(dy) > 3) drag.moved = true;
      view.x = drag.vx + dx; view.y = drag.vy + dy;
      apply();
    });
    window.addEventListener('mouseup', () => { drag = null; svg.classList.remove('panning'); });
    // A band covers the background, and a double click on it is still one on
    // the background.
    svg.addEventListener('dblclick', (e) => { if (e.target === svg || e.target.classList.contains('band')) fit(); });
  }

  /* The one place a pan, a wheel zoom and fit() all pass through, so closing the
     hover card here covers every way the boxes can move out from under it.
     onHoverClose, not onHoverOut: a pan fires this on every mousemove, and the
     grace period onHoverOut grants would just keep being restarted. */
  const apply = () => {
    root.setAttribute('transform', `translate(${view.x},${view.y}) scale(${view.k})`);
    placeHeads();
    handlers.onHoverClose && handlers.onHoverClose();
  };

  // Below whatever the page floats over the top of the canvas.
  const headTop = () => (handlers.headTop ? handlers.headTop() : 0) + 6;

  /* The band names, pinned to the top of the window over their bands the way
     a table's header is: a graph fitted to the window is drawn too small to
     read a name at its own scale, and one zoomed into has scrolled its top
     away. A name starts inside its band and may run on over the gutter, up to
     where the next name starts; it gives up when that leaves too little room
     to show even the number a numbered folder starts with. */
  function placeHeads() {
    if (!heads || !bands.length) return;
    const r = svg.getBoundingClientRect(), p = heads.getBoundingClientRect();
    const top = headTop();
    bands.forEach((b, i) => {
      const s = heads.children[i], next = bands[i + 1];
      const x0 = view.x + b.x0 * view.k, x1 = next ? view.x + next.x0 * view.k : view.x + b.x1 * view.k;
      const left = Math.max(x0, 0) + 4, room = Math.min(x1, r.width) - left - 4;
      s.style.display = room < 40 ? 'none' : '';
      s.style.left = `${r.left - p.left + left}px`;
      s.style.top = `${r.top - p.top + top}px`;
      s.style.maxWidth = `${room}px`;
    });
  }

  /* What a band says on hover: where the folder is, and how much of it is drawn. */
  function bandTitle(b, columnMode) {
    const noun = columnMode ? 'column' : 'node';
    return `${b.package ? `package ${b.name}` : `${b.key || '.'}/`}  ·  ${b.size} ${noun}${b.size === 1 ? '' : 's'} drawn`;
  }

  /* Columns from the drawn edges, a lane in every column a long edge skips,
     and Brandes and Kopf for the heights where they cost little (0027). */
  function layout(d, folders) {
    const L = dagLayout(d, {
      w: W, h: H, hgap: HGAP, vgap: VGAP, lane: LANE, bundle: BUNDLE,
      badge: d.mode === 'column' ? TAG_ROOM : 0,
    }, folders, unfolded);
    bbox = L.bbox;
    return L;
  }

  /* `opts.folders` draws the canvas by folder: the project's package, '' when
     it is not known, or null for the usual canvas (0029). What comes back says
     what the bands added, for the line under the canvas; null without them. */
  function render(d, opts = {}) {
    data = d;
    lastOpts = opts;
    const columnMode = d.mode === 'column';
    W = columnMode ? 180 : 200;
    H = columnMode ? 40 : 48;
    // Selection mode sends no focus, and this is why it does not need to: an
    // absent index reads as undefined here and marks no box, rather than
    // picking the first one by accident.
    selected = d.nodes[d.focus] ? d.nodes[d.focus].id : null;
    // Column mode only: in model mode the box colour already carries the answer.
    const roles = columnMode ? nodeRoles(d) : [];
    const L = layout(d, opts.folders);
    place = L.boxes;
    groups = L.groups || [];
    bands = L.bands || [];
    drawn = L.bands || L.folded ? { folders: bands.length, against: L.against || 0, folded: L.folded || 0 } : null;
    root.textContent = '';

    const bandLayer = el('g', { class: 'bands' }), edgeLayer = el('g'), nodeLayer = el('g');
    root.appendChild(bandLayer); root.appendChild(edgeLayer); root.appendChild(nodeLayer);

    // Far taller than the graph, so wherever a pan leaves the window, the name
    // pinned at its top still sits on its band. snapshot() cuts them to the
    // picture.
    heads.textContent = '';
    bands.forEach((b) => {
      const r = el('rect', {
        class: 'band', x: b.x0, y: L.bbox.y0 - 1e5, width: b.x1 - b.x0, height: L.bbox.y1 - L.bbox.y0 + 2e5,
      });
      const tip = el('title');
      tip.textContent = bandTitle(b, columnMode);
      r.appendChild(tip);
      bandLayer.appendChild(r);
      const s = document.createElement('span');
      s.className = 'folder-head';
      s.textContent = b.name;
      heads.appendChild(s);
    });

    // One path per edge even when it runs through lanes, so select() and the
    // exported page still find it by its two ends.
    d.edges.forEach(([a, b], i) => {
      if (!L.paths[i]) return;                // into a folded test
      const path = el('path', {
        // A line into a test links a check to what it checks, not data to
        // where it goes, so it is drawn apart: see `.edge.test`.
        class: (L.back[i] ? 'edge back' : 'edge') + (d.nodes[b].kind === 'test' ? ' test' : ''),
        'data-a': d.nodes[a].id, 'data-b': d.nodes[b].id, d: L.paths[i],
      });
      // A custom property, not `stroke`: setting the property leaves `.edge.hi`
      // free to override the stroke outright, so selecting an edge still turns
      // it accent-coloured instead of keeping its role hue.
      const kind = d.edge_kinds && d.edge_kinds[i];
      if (kind) {
        path.dataset.kind = kind;
        path.style.setProperty('--edge-col', roleColor(kind));
      }
      edgeLayer.appendChild(path);
    });
    // A group's row hangs from its host like a test does, so selecting the
    // host lights its stem too.
    groups.forEach((c) => {
      edgeLayer.appendChild(el('path', { class: 'edge test', 'data-a': d.nodes[c.host].id, d: c.stem }));
    });

    d.nodes.forEach((n, i) => {
      const b = place[i];
      if (!b) return;                         // a folded test, or one in a closed group
      const g = el('g', {
        class: 'nd' + (i === d.focus ? ' focus' : '') + (n.disabled ? ' off' : '') + (n.context ? ' ctx' : '')
          + (b.rider ? ` rider ${b.test}` + (b.member ? ' member' : '') + (b.warn ? ' warn' : '') : ''),
        transform: `translate(${b.x},${b.y})`,
      });
      g.dataset.id = n.id;
      g.appendChild(el('rect', { class: 'box', width: b.w, height: b.h }));
      // Inline style, not a fill attribute: a CSS rule such as `.nd rect` would
      // outrank the attribute and repaint this bar in the box colour. A
      // hanging test's bar has the shape of its kind, which riderBars draws.
      if (!b.rider) g.appendChild(el('rect', { class: 'kindbar', width: 6, height: b.h, style: `fill:${nodeColor(n)}` }));
      // Ephemeral models are inlined into their children, nothing exists in the
      // warehouse, so they are drawn like the disabled ones: dashed.
      if (matLabel(n) === 'ephemeral') g.classList.add('off');

      // A test hanging under its model is one line, saying what it checks:
      // the generic and its column, or the test's own name (0040). That it
      // is a test, the stem already says.
      if (b.rider) {
        const x = riderBars(g, b.test, b.h, TEST[b.test]);
        const t1 = el('text', { class: 't1', x, y: b.h / 2 + 4 });
        t1.textContent = rowClip(b.label, b.keep, riderMax(b.w, x, b.warn ? warnRoom(0) : 0));
        t1.dataset.full = b.label;
        g.appendChild(t1);
        if (b.warn) g.appendChild(warnMark(b.w, b.h, 0));
      } else {
        const t1 = el('text', { class: 't1', x: 12, y: 20 });
        t1.textContent = clip(n.name, columnMode ? 24 : 27);
        g.appendChild(t1);
      }

      if (!b.rider) {
        const t2 = el('text', { class: 't2', x: 12, y: 35 });
        t2.textContent = clip(subtitle(n), columnMode ? 30 : 34);
        g.appendChild(t2);
      }

      if (roles[i]) g.appendChild(roleBadge(roles[i]));

      // A hanging test's other inputs are counted on its right: its left is
      // where the stem from its model runs.
      if (n.hidden_up) g.appendChild(badge(b.rider ? b.w + 16 : -16, b.h / 2, `+${n.hidden_up}`, 'up', n));
      if (n.hidden_down) g.appendChild(badge(b.w + 16, b.h / 2, `+${n.hidden_down}`, 'down', n));

      // An aria-label rather than a <title>: the native tooltip a <title> draws
      // would arrive a second after the hover card and sit on top of it. The
      // group still needs an accessible name, and role="img" is what gets one
      // exposed on a bare <g>.
      g.setAttribute('role', 'img');
      g.setAttribute('aria-label', (b.rider ? `${b.label}, ${matLabel(n)}${n.warn ? ', severity warn' : ''}, ` : '')
        + `${n.id} ${n.file}${n.context ? ' (context, not selected)' : ''}`);

      g.addEventListener('click', (e) => { e.stopPropagation(); select(n.id); handlers.onSelect && handlers.onSelect(n); });
      g.addEventListener('dblclick', (e) => { e.stopPropagation(); handlers.onOpen && handlers.onOpen(n); });
      // enter/leave, not over/out: the group has four children, and crossing
      // between them would fire over/out as if the box had been left.
      g.addEventListener('mouseenter', () => { if (!drag) handlers.onHover && handlers.onHover(n, g); });
      g.addEventListener('mouseleave', () => handlers.onHoverOut && handlers.onHoverOut());
      nodeLayer.appendChild(g);
    });

    // A chip or a group's row turns what `key` names, draws again where the
    // view stands, and hands the keyboard back to the same row, which the
    // redraw replaced.
    const turn = (g, key) => {
      const toggle = (e) => {
        e.stopPropagation();
        if (unfolded.has(key)) unfolded.delete(key); else unfolded.add(key);
        const got = render(data, Object.assign({}, lastOpts, { keepView: true }));
        handlers.onRefold && handlers.onRefold(data, got);
        const again = [...root.querySelectorAll('[data-key]')].find((x) => x.dataset.key === key);
        if (again) again.focus({ preventScroll: true });
      };
      g.dataset.key = key;
      g.addEventListener('click', toggle);
      g.addEventListener('keydown', (e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); toggle(e); } });
    };

    // A row standing for a model's tests of one generic: it opens them, or
    // closes them again. No hover card, since it is no node; its tests have
    // theirs once open.
    groups.forEach((c) => {
      const host = d.nodes[c.host].name;
      const what = c.level === 'column' ? `on ${c.count} ${c.cols === c.count ? 'columns' : 'tests'}` : `${c.count} times`;
      const g = el('g', {
        class: `nd rider group ${c.level}` + (c.warn ? ' warn' : ''), transform: `translate(${c.x},${c.y})`,
        role: 'button', tabindex: '0', 'aria-expanded': String(c.open),
      });
      g.setAttribute('aria-label', c.open ? `Close the ${c.generic} tests of ${host}`
        : `${c.generic} ${what} of ${host}${c.warn ? `, ${c.warn} at severity warn` : ''}. Show them`);
      g.appendChild(el('rect', { class: 'box', width: c.w, height: c.h }));
      const x = riderBars(g, c.level, c.h, TEST[c.level]);
      const t = el('text', { class: 't1', x, y: c.h / 2 + 4 });
      t.textContent = rowClip(c.label, c.keep, riderMax(c.w, x, c.warn ? warnRoom(c.warn) : 0));
      t.dataset.full = c.label;
      g.appendChild(t);
      if (c.warn) g.appendChild(warnMark(c.w, c.h, c.warn));
      turn(g, c.key);
      nodeLayer.appendChild(g);
    });

    // A chip under a model with more rows than show: it opens them, or
    // folds them again. A button, so a keyboard reaches it too.
    (L.chips || []).forEach((c) => {
      const g = el('g', { class: 'nd rider fold', transform: `translate(${c.x},${c.y})`, role: 'button', tabindex: '0' });
      g.setAttribute('aria-label', c.open ? `Fold the tests of ${d.nodes[c.host].name}` : `Show ${c.count} more tests of ${d.nodes[c.host].name}`);
      g.appendChild(el('rect', { class: 'box', width: c.w, height: c.h }));
      const t = el('text', { class: 't1', x: 12, y: c.h / 2 + 4 });
      t.textContent = foldLabel(c);
      g.appendChild(t);
      turn(g, c.id);
      nodeLayer.appendChild(g);
    });

    if (!opts.keepView) fit(); else apply();
    select(selected);
    return drawn;
  }

  /* The role, as a tag above the box rather than inside it: both lines of a
     column box are already full, and a badge that overlaps a column name is
     worse than no badge. VGAP leaves room, and fit() pads beyond it.

     Dark fill with a coloured outline, not a coloured fill: the role ramp runs
     from a light slate to a dark grey, so one text colour could never stay
     legible across all of them. */
  function roleBadge(role) {
    const label = role.toUpperCase();
    const w = label.length * 5.8 + 12;
    const colour = roleColor(role);
    const g = el('g', { class: 'rolebadge' });
    g.appendChild(el('rect', {
      x: W - w - 4, y: -14, width: w, height: 14, rx: 3,
      style: `fill:#131922;stroke:${colour}`,
    }));
    const t = el('text', {
      class: 'rolelabel', x: W - w / 2 - 4, y: -4, 'text-anchor': 'middle',
      style: `fill:${colour}`,
    });
    t.textContent = label;
    g.appendChild(t);
    return g;
  }

  /* The node travels with the direction: model mode only needs to know which
     way to deepen, selection mode has to name the box in the expression. */
  /* A test row's bar, in the shape of its kind as well as its grey, since
     three greys alone are hard to tell apart at a glance: whole for a generic
     on a column, doubled by a thin one for a generic on the whole model, and
     in three pieces for a singular test, as its outline is dashed. Returns
     where the text starts. */
  function riderBars(g, level, h, colour) {
    const bar = (attrs) => g.appendChild(el('rect', Object.assign({ class: 'kindbar', width: 6, height: h, style: `fill:${colour}` }, attrs)));
    if (level === 'singular') {
      const piece = (h - 4) / 3;
      [0, 1, 2].forEach((i) => bar({ class: 'kindbar seg', y: i * (piece + 2), height: piece }));
      return 12;
    }
    bar({});
    if (level !== 'model') return 12;
    bar({ x: 8, width: 2 });
    return 14;
  }

  /* Characters a row has room for, after the text's start and a warn mark.
     An identifier at 10.5 px measured 5.1 to 5.4 px a character in Chrome on
     a Mac; 5.6 leaves a little for a wider system font. */
  function riderMax(w, x, reserve) {
    return Math.floor((w - x - 8 - reserve) / 5.6);
  }

  // The room a warn mark takes at a row's right end: the triangle, and the
  // count beside it on a group's row.
  function warnRoom(count) {
    return 17 + (count ? 6 * String(count).length + 2 : 0);
  }

  /* A test configured `severity: warn`: a triangle at the row's right end,
     its shape the meaning, since its amber is an incremental model's too. On
     a group's row it counts the tests at warn. */
  function warnMark(w, h, count) {
    const g = el('g', { class: 'warnmark' });
    const right = w - 5, mid = h / 2;
    g.appendChild(el('path', { d: `M${right - 10},${mid + 4} L${right - 5},${mid - 5} L${right},${mid + 4} Z` }));
    if (count) {
      const t = el('text', { class: 'warncount', x: right - 13, y: mid + 4, 'text-anchor': 'end' });
      t.textContent = String(count);
      g.appendChild(t);
    }
    return g;
  }

  function badge(x, y, label, dir, node) {
    const g = el('g', { class: 'more-badge', style: 'cursor:pointer', 'data-dir': dir });
    g.appendChild(el('circle', { cx: x, cy: y, r: 11, fill: '#1d2430', stroke: '#2a3340' }));
    const t = el('text', { class: 'more', x, y: y + 3, 'text-anchor': 'middle' });
    t.textContent = label;
    g.appendChild(t);
    g.addEventListener('click', (e) => { e.stopPropagation(); handlers.onExpand && handlers.onExpand(dir, node); });
    return g;
  }

  function select(id) {
    selected = id;
    root.querySelectorAll('.nd').forEach((g) => g.classList.toggle('sel', g.dataset.id === id));
    root.querySelectorAll('.edge').forEach((p) => {
      const hit = p.dataset.a === id || p.dataset.b === id;
      p.classList.toggle('hi', hit);
      // Last in its layer, so drawn on top: a shared lane carries several edges
      // on one line, and a sibling drawn after it would hide the highlight.
      if (hit) p.parentNode.appendChild(p);
    });
  }

  function fit() {
    if (!bbox || !data || !data.nodes.length) return;
    const r = svg.getBoundingClientRect();
    const pad = 30;
    // The band names are pinned over the top of the window, so the graph is
    // fitted below them rather than under them.
    const top = bands.length ? headTop() + 20 : 0;
    const k = Math.min(1.1, Math.max(0.08,
      Math.min((r.width - pad * 2) / (bbox.x1 - bbox.x0 || 1), (r.height - top - pad * 2) / (bbox.y1 - bbox.y0 || 1))));
    view.k = k;
    view.x = r.width / 2 - ((bbox.x0 + bbox.x1) / 2) * k;
    view.y = top + (r.height - top) / 2 - ((bbox.y0 + bbox.y1) / 2) * k;
    apply();
  }

  /* The canvas as one standalone SVG, for a file read where neither this page
     nor its stylesheet exists: a ticket attachment, a PDF, an image (0026).

     Only what a reader without this page would miss is changed. A clipped line
     is written out whole, smaller if it has to be, since a PDF has no hover
     card to spell a name and Ctrl+F has to find it. The accessible name becomes
     a <title>, the one tooltip a file can carry. The passing selection goes and
     the focus stays: the focus is what model mode is about.

     `css` and `tight` are for the image path. Inlined into a page, a <style>
     inside the SVG would style the whole page, so the page brings its own. */
  function snapshot({ css = '', scale = 1, tight = false } = {}) {
    if (!data || !bbox || !data.nodes.length) return null;
    const frame = tight ? frameOf(bbox, 0, 0) : frameOf(bbox);
    const byId = new Map(data.nodes.map((n) => [String(n.id), n]));
    const g = root.cloneNode(true);
    g.removeAttribute('transform');
    g.querySelectorAll('.sel').forEach((x) => x.classList.remove('sel'));
    g.querySelectorAll('.hi').forEach((x) => x.classList.remove('hi'));

    // Measured on the canvas itself, inside a box so the same rules apply: a
    // detached clone has no layout to measure. A clip counts characters, so
    // plenty of clipped lines fit whole once they are measured. A hanging
    // test's line is smaller than a model's, so it is measured in one.
    const host = root.querySelector('.nd'), rider = root.querySelector('.nd.rider') || host;
    const probes = {};
    const measure = (cls, text, size, small) => {
      const key = small ? `${cls} rider` : cls;
      if (!probes[key]) {
        probes[key] = el('text', { class: cls, visibility: 'hidden' });
        (small ? rider : host).appendChild(probes[key]);
      }
      const probe = probes[key];
      probe.style.fontSize = size ? `${size}px` : '';
      probe.textContent = text;
      return { wide: probe.getComputedTextLength(), size: parseFloat(getComputedStyle(probe).fontSize) };
    };
    // Shrunk to fit, down to a floor that keeps the glyphs' shape: the file is
    // zoomed into, not read at arm's length. Measured again once shrunk, since
    // small sizes do not scale in proportion, then pinned to that length, so
    // a reader whose fonts run wider sees the same line rather than one that
    // spills out of its box. Past the floor the pin squeezes it.
    // A row's line starts further in past a second bar, and stops short of a
    // warn mark, so its room is its own.
    const spell = (t, full) => {
      if (!t || !full || t.textContent === full) return;
      const cls = t.getAttribute('class'), nd = t.parentNode, small = nd.classList.contains('rider');
      const mark = nd.querySelector('.warnmark'), count = mark && mark.querySelector('.warncount');
      const room = (+nd.querySelector('rect.box').getAttribute('width') || W) - 8 - (+t.getAttribute('x') || 12)
        - (mark ? warnRoom(count ? count.textContent : 0) : 0);
      const natural = measure(cls, full, 0, small);
      t.textContent = full;
      if (!natural.wide) return;
      const shrink = Math.min(1, Math.max(0.6, room / natural.wide));
      let wide = natural.wide;
      if (shrink < 1) {
        const size = +(natural.size * shrink).toFixed(2);
        t.style.fontSize = `${size}px`;
        wide = measure(cls, full, size, small).wide;
      }
      t.setAttribute('textLength', Math.min(room, wide).toFixed(1));
      t.setAttribute('lengthAdjust', 'spacingAndGlyphs');
    };
    // A file is not interactive: a chip or a group's row stays as drawn, its
    // name moved into a tooltip, which is all a reader there can use.
    const byKey = new Map(groups.map((c) => [c.key, c]));
    g.querySelectorAll('[data-key]').forEach((nd) => {
      const c = byKey.get(nd.dataset.key), t1 = nd.querySelector('.t1');
      const title = el('title');
      if (c) {
        const names = c.members.map((v) => {
          const m = data.nodes[v];
          return (c.level === 'column' && c.cols === c.count ? m.column : m.name) + (m.warn ? ', severity warn' : '');
        });
        title.textContent = [`${c.generic}${c.ns ? ` from ${c.ns}` : ''}, ${c.count} ${c.level} tests`]
          .concat(names.slice(0, 20), names.length > 20 ? [`and ${names.length - 20} more`] : []).join('\n');
        spell(t1, t1.dataset.full);
      } else {
        title.textContent = t1.textContent;
      }
      nd.insertBefore(title, nd.firstChild);
      ['role', 'tabindex', 'aria-expanded', 'aria-label', 'data-key'].forEach((a) => nd.removeAttribute(a));
      t1.removeAttribute('data-full');
    });
    g.querySelectorAll('.nd').forEach((nd) => {
      const n = byId.get(nd.dataset.id);
      if (!n) return;
      nd.removeAttribute('aria-label');
      const title = el('title');
      title.textContent = [n.name, subtitle(n), n.file, n.warn && 'severity warn', n.context && 'context, not selected'].filter(Boolean).join('\n');
      nd.insertBefore(title, nd.firstChild);
      const t1 = nd.querySelector('.t1');
      spell(t1, t1 && t1.dataset.full ? t1.dataset.full : n.name);
      if (t1) t1.removeAttribute('data-full');
      spell(nd.querySelector('.t2'), subtitle(n));
      nd.querySelectorAll('.more-badge').forEach((b) => {
        const up = b.dataset.dir === 'up';
        b.removeAttribute('style');
        const tip = el('title');
        tip.textContent = `${up ? n.hidden_up : n.hidden_down} more ${up ? 'upstream' : 'downstream'}, not drawn`;
        b.insertBefore(tip, b.firstChild);
      });
    });
    // A file has no window to pin the band names to, so each is written at the
    // top of its band, and every band runs the height of the picture. A name
    // wider than its band is squeezed into it, as a box's name is.
    const layer = g.querySelector('.bands');
    g.querySelectorAll('rect.band').forEach((r) => {
      r.setAttribute('y', frame.y);
      r.setAttribute('height', frame.h);
    });
    bands.forEach((b) => {
      const t = el('text', { class: 'band-name', x: b.x0 + 10, y: frame.y + 22 });
      t.textContent = b.name;
      const room = b.x1 - b.x0 - 20, wide = measure('band-name', b.name).wide;
      if (wide > room) {
        t.setAttribute('textLength', room.toFixed(1));
        t.setAttribute('lengthAdjust', 'spacingAndGlyphs');
      }
      layer.appendChild(t);
    });
    Object.values(probes).forEach((p) => p.remove());

    const out = el('svg', {
      id: 'graph',
      viewBox: `${frame.x} ${frame.y} ${frame.w} ${frame.h}`,
      width: Math.round(frame.w * scale),
      height: Math.round(frame.h * scale),
      preserveAspectRatio: 'xMidYMid meet',
    });
    if (css) {
      const style = el('style');
      style.textContent = css;
      out.appendChild(style);
    }
    out.appendChild(g);
    return { svg: new XMLSerializer().serializeToString(out), frame, data, folders: drawn };
  }

  // clear() does not go through apply(), so it closes the card itself.
  const clear = () => {
    root && (root.textContent = '');
    heads && (heads.textContent = '');
    data = null; bbox = null; bands = []; drawn = null;
    handlers.onHoverClose && handlers.onHoverClose();
  };

  return { init, render, fit, select, clear, snapshot, subtitle, nodeColor, matLabel, testLevel, roleColor, nodeRoles };
})();
