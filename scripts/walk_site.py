#!/usr/bin/env python3
"""The static site (0177), written by `dbt-edith --site`, opened in headless Chrome beside the server.

Two projects, each in a git repository with an invented github.com origin and a
.env that holds a canary: Jaffle Shop, built, with its catalog, and Fivetran's
Shopify package at the pinned tag (0056), parsed only. Each project is in a
sub-folder of its repository, which a link to a file must say. On each:

  - the site is written, and nothing in it is a path of this machine or the
    canary; its header facts are the manifest's and git's;
  - the answers EdithSite gives in the browser, from the folder, are the
    server's, node by node: /api/lineage at 1+1, 2+2 and 20+20, with the tests
    on and off, and at 2+2 under a cap of 5; /api/node but for the location
    resolved against a .env, which a site never holds; and a set of searches;
  - the SQL (0180): for every node with a compiled text, the site's is what the
    server reads from target/compiled, and the source is the node's file, cut of
    its ends; on the project that was only parsed, no node is compiled and the
    page says so; a link to a file is the host's pattern at the commit, with
    the project's place in the repository, and a package's file has none;
  - the page reads: the header names the environment, the commit as a link
    that opens elsewhere and gives nothing away, when the manifest was written
    and dbt's version; the Models list, a click, a click on a box, the
    Catalog and its Columns, Cmd+K and an address that names a node; what the
    site has no use for is not on the page, and a click on what is hidden does
    nothing and says nothing;
  - the column lineage (0182): an invented cache and the report beside it, written before the
    site and before the server starts, over the project's own columns (the parent of a model's
    column of the same name, renames and casts, filters and join keys into every column of a
    model, roots, expressions, gaps, rules, a cycle, a self-edge, a table no node owns). For every
    column of every node, /api/collineage at 2+2 with the indirect edges hidden and drawn is the
    server's, and a sample of them at 1+1, at 20+20, with a role cut and under a cap; /api/colreach
    is the server's for every node, and a card carries collin's note; in the page, a column
    opens, the indirect button and the stop at menu work, there is no menu to pick another
    cache and no Regenerate, and data/cll.js is asked for once; a cache older than the manifest
    says so on the status line and in the Columns tab; and `--site --run-collin` runs a stand-in
    collin first and carries what it wrote;
  - the page asks for nothing beyond its own folder, throws nothing, writes no
    error to the console, and the policy it carries is never violated.

It is opened from file:// and again from a static host under a sub-path, which
is a Python http.server here, where every request it gets is counted.

It runs as scripts/walk_tour.py does, whose server and DevTools client it
borrows, and as scripts/compare_with_dbt.py does for Shopify: it needs dbt and
Chrome, and skips itself without either, exit 2. Without git or the network
Shopify alone is skipped, and the run ends with exit 2 even when Jaffle Shop
agreed.

Exit 0 when the site is the server's, 1 when it is not, 2 when skipped.
"""

import functools
import http.server
import json
import os
import random
import re
import shutil
import stat
import subprocess
import sys
import tempfile
import threading
import time
import urllib.parse
import urllib.request
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import compare_with_dbt as C  # skips, exit 2, without dbt
import walk_tour as W  # skips, exit 2, without Chrome

CANARY = "canary-7f3a9c1e-never-in-a-site"
ENV = "qa"
# What the browser answers per request, in one trip to the page: far past one
# answer's size is the DevTools message that carries the batch back.
BATCH = 25
# The page is read for these, from a click on the first row of the Models list.
GROUPS = [(1, 1, 400), (2, 2, 400), (20, 20, 400), (2, 2, 5)]

# What the site hides, by the selector that finds it, and what stays.
HIDDEN = ["#spaces", "#editor-pane", "#settings-btn", "#manifest-pill", "#github-btn", "#keys-btn",
          "#open-editors", "#reveal-btn", "[data-side=search]", "[data-side=git]",
          "[data-dock=terminal]", "[data-dock=jobs]", "[data-dock=compiled]", "[data-dock=run]",
          "#status-venv", "#status-env", "#status-branch", "#status-shell", "#status-update"]
SHOWN = ["#site-stamp", "#project", "[data-side=files]", "#tree", "[data-side=models]", "[data-dock=lineage]", "[data-dock=catalog]", "#palette-btn", "#dock"]

# Installed before the page's own scripts run, so nothing it does is missed: the
# policy's violations, every exception and rejection, what it writes to the
# console as an error, and every toast that says an error.
WATCH = """
window.__bad = [];
document.addEventListener('securitypolicyviolation', (e) => window.__bad.push('csp: ' + e.violatedDirective + ' ' + e.blockedURI));
window.addEventListener('error', (e) => window.__bad.push('error: ' + e.message));
window.addEventListener('unhandledrejection', (e) => window.__bad.push('rejection: ' + (e.reason && (e.reason.stack || e.reason.message) || e.reason)));
const ce = console.error;
console.error = function (...a) { window.__bad.push('console.error: ' + a.join(' ')); return ce.apply(console, a); };
document.addEventListener('DOMContentLoaded', () => {
  new MutationObserver((records) => {
    for (const r of records) for (const n of r.addedNodes) if (n.classList && n.classList.contains('err')) window.__bad.push('toast: ' + n.textContent);
  }).observe(document.querySelector('#toast'), { childList: true });
});
"""


class Page(W.Page):
    """The DevTools client, with what it is told as well as what it is answered:
    the requests the page makes, the console's errors and the log's."""

    def __init__(self, work, scale):
        self.events = []
        super().__init__(work, scale)

    def call(self, method, **params):
        self.id += 1
        self._send({"id": self.id, "method": method, "params": params})
        while True:
            m = self._recv()
            if "method" in m:
                self.events.append(m)
            if m.get("method") == "Runtime.exceptionThrown":
                self.errors.append(m["params"]["exceptionDetails"].get("exception", {}).get("description", "?"))
            if m.get("id") == self.id:
                if "error" in m:
                    raise RuntimeError(f"{method}: {m['error']}")
                return m.get("result", {})

    def requests(self):
        return [e["params"]["request"]["url"] for e in self.events if e["method"] == "Network.requestWillBeSent"]

    def console_errors(self):
        out = []
        for e in self.events:
            if e["method"] == "Log.entryAdded" and e["params"]["entry"]["level"] == "error":
                out.append("log: " + e["params"]["entry"]["text"][:200])
            if e["method"] == "Runtime.consoleAPICalled" and e["params"]["type"] == "error":
                out.append("console: " + " ".join(str(a.get("value") or a.get("description")) for a in e["params"]["args"])[:200])
        return out


class Tally:
    def __init__(self):
        self.failed = 0

    def check(self, label, ok, detail=""):
        print(("PASS  " if ok else "FAIL  ") + label + ("" if ok else f"\n        {detail}"))
        self.failed += not ok


def canon(value):
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False)


class Server:
    """What the server answers, over HTTP with the launch key it printed."""

    def __init__(self, edith):
        m = re.match(r"(http://127\.0\.0\.1:\d+)/\?key=(\w+)", edith.url)
        self.base, self.key = m.group(1), m.group(2)

    def get(self, path):
        req = urllib.request.Request(self.base + path, headers={"authorization": "Bearer " + self.key})
        with urllib.request.urlopen(req, timeout=60) as r:
            return json.load(r)


def git(project, *args):
    env = dict(os.environ, GIT_CONFIG_GLOBAL="/dev/null", GIT_CONFIG_NOSYSTEM="1", GIT_AUTHOR_NAME="Ada",
               GIT_AUTHOR_EMAIL="ada@example.com", GIT_COMMITTER_NAME="Ada", GIT_COMMITTER_EMAIL="ada@example.com")
    run = subprocess.run(["git", "-C", str(project), *args], capture_output=True, text=True, env=env)
    if run.returncode != 0:
        raise RuntimeError(f"git {' '.join(args)}: {run.stderr.strip()[-300:]}")
    return run.stdout.strip()


def dbt(project, *args):
    """dbt in a project copy, from inside it: a relative DuckDB path is written there and nowhere else."""
    where = ["--project-dir", str(project), "--profiles-dir", str(project)]
    if C.EXECUTABLE:
        run = subprocess.run([C.EXECUTABLE, *args, *where, "--quiet"], capture_output=True, text=True, cwd=project)
        if run.returncode != 0:
            raise RuntimeError(f"dbt {' '.join(args)} failed: {(run.stdout + run.stderr)[-2000:]}")
        return
    here = os.getcwd()
    os.chdir(project)
    try:
        res = C.dbtRunner().invoke([*args, *where, "--log-level", "none"])
    finally:
        os.chdir(here)
    if not res.success:
        raise RuntimeError(f"dbt {' '.join(args)} failed: {res.exception}")


JAFFLE_FOLDER = "analytics"


def jaffle(work):
    """Jaffle Shop, built, with its catalog, in a folder of a git repository with an invented origin: its files
    are not at the repository's root, which a link to one must say."""
    repo = work / "jaffle_repo"
    project = repo / JAFFLE_FOLDER
    shutil.copytree(W.JAFFLE, project)
    (project / ".env").write_text(f"DBT_SCHEMA={CANARY}\nSECRET_TOKEN={CANARY}\n")
    git(repo, "init", "-q", "-b", "main")
    git(repo, "add", "-A")
    git(repo, "commit", "-q", "-m", "jaffle shop")
    git(repo, "remote", "add", "origin", "git@github.com:acme/jaffle-shop.git")
    dbt(project, "build")
    dbt(project, "docs", "generate")
    return project


def shopify(work):
    """Shopify at the pinned tag, parsed only, a clone whose origin is invented: the real one would be a link to Fivetran's."""
    project = C.fetch_shopify(work / "shopify")
    git(project, "remote", "set-url", "origin", "git@github.com:acme/dbt-shopify.git")
    (project / ".env").write_text(f"DBT_SCHEMA={CANARY}\nSECRET_TOKEN={CANARY}\n")
    dbt(project, "deps")
    # `dbt deps` links a `local:` package, a path out of the project that the server does not follow (0094), so its
    # models would be listed by the site alone. Installed as a copy, as a package from the hub is, the server finds
    # them under dbt_packages and the two trees can be compared over them.
    link = project / "dbt_packages" / "shopify"
    if link.is_symlink():
        source = link.resolve()
        link.unlink()
        shutil.copytree(source, link, ignore=shutil.ignore_patterns("integration_tests", ".git"))
    W.parse(project)
    return project


KINDS = ["passthrough", "rename", "cast", "transform", "aggregate", "window", "inferred", ""]
ROWS = ["filter", "join_key", "dedup_key", "FILTER"]
OWNED = ("model", "seed", "snapshot", "source")


def invent_cache(project, seed):
    """collin's cache and report beside the manifest, invented over the project's own columns: from the YAML and
    the catalog, so every shape the walks read has columns to land on. Edges between a model and its parents
    by column name, with every role, a filter or a join key from one parent column into every column of a
    model, a cycle, a column reading itself, a column from a table no node owns, one edge that names a node the
    manifest lacks, roots (`[]` among them, a column spelled in two cases), expressions at 32 steps and cut,
    gaps of every reason, rules, and a report with notes. Written after the manifest, so it is not behind it.
    Returns {"edges": [...], "kinds": [...]} for what the walk compares and the page opens."""
    rng = random.Random(seed)
    target = project / "target"
    manifest = json.loads((target / "manifest.json").read_text())
    pool = {**manifest["nodes"], **manifest.get("sources", {})}
    cols = {uid: list(n.get("columns", {})) for uid, n in pool.items() if n["resource_type"] in OWNED and (n.get("config") or {}).get("enabled", True)}
    catalog = target / "catalog.json"
    if catalog.is_file():
        found = json.loads(catalog.read_text())
        for uid, entry in {**found.get("nodes", {}), **found.get("sources", {})}.items():
            if uid in cols:
                known = {c.lower() for c in cols[uid]}
                cols[uid] += [c for c in entry.get("columns", {}) if c.lower() not in known]
    cols = {uid: names for uid, names in cols.items() if names}
    parents = {uid: [p for p in ps if p in cols] for uid, ps in manifest["parent_map"].items() if uid in cols}
    edges, seen, into = [], set(), {}

    def edge(f, fc, t, tc, kind):
        key = (f, fc.lower(), t, tc.lower())
        if key not in seen:
            seen.add(key)
            into.setdefault(t, set()).add(tc.lower())
            edges.append({"from": f, "from_col": fc, "to": t, "to_col": tc, "kind": kind})

    for child, ps in sorted(parents.items()):
        for p in ps:
            shared = [c for c in cols[child] if c.lower() in {x.lower() for x in cols[p]}]
            for c in shared[:60]:
                edge(p, c, child, c, rng.choice(KINDS))
        loose = [c for c in cols[child] if c.lower() not in into.get(child, set())]
        for c in loose[:20]:
            if ps and rng.random() < 0.4:
                p = rng.choice(ps)
                edge(p, rng.choice(cols[p]), child, c, rng.choice(["rename", "transform", "aggregate"]))
        # A column that decides which rows a model keeps reaches every column of it, as collin writes it.
        if ps and rng.random() < 0.3:
            p = rng.choice(ps)
            pc = rng.choice(cols[p])
            role = rng.choice(ROWS)
            for c in cols[child][:25]:
                edge(p, pc, child, c, role)
    plain = [e for e in edges if e["kind"] not in ROWS]
    for e in rng.sample(plain, min(3, len(plain))):
        edge(e["to"], e["to_col"], e["from"], e["from_col"], "transform")              # a cycle
    for uid in rng.sample(sorted(cols), min(2, len(cols))):
        edge(uid, cols[uid][0], uid, cols[uid][0], "passthrough")                      # a column reading itself
    sources = [u for u in sorted(cols) if u.startswith("source.")]
    for i, uid in enumerate(rng.sample(sources, min(3, len(sources)))):
        edge(f"rel:analytics.raw.table_{i}", cols[uid][0], uid, cols[uid][0], "passthrough")
    some = sorted(cols)[0]
    edge("model.nobody.ghost", "id", some, cols[some][0], "passthrough")
    edge(some, cols[some][0], "model.nobody.ghost", "id", "passthrough")

    models = [u for u in sorted(cols) if u.startswith("model.")]

    def pick():
        u = rng.choice(models)
        return u, rng.choice(cols[u])

    roots, expressions, partial, rules = [], [], [], []
    for _ in range(12):
        u, c = pick()
        roots.append({"node": u, "column": c, "expressions": rng.choice([[], ["'a'"], ["'a'", "'b'"]])})
    u, c = pick()
    roots += [{"node": u, "column": c.upper(), "expressions": ["'x'"]}, {"node": u, "column": c.lower(), "expressions": ["'y'"]}]
    for _ in range(25):
        u, c = pick()
        expressions.append({"node": u, "column": c, "kind": rng.choice(KINDS), "text": f"f({c})", "steps": rng.choice([1, 2, 3, 5, 32]), "cut": rng.random() < 0.2})
    for _ in range(20):
        u, c = pick()
        partial.append({"node": u, "column": c, "reason": rng.choice(["name_unresolved", "relation_unknown", "steps_capped", "cte_lacks_name"]), "name": rng.choice(["", "a", "b"])})
    for u in rng.sample(models, min(12, len(models))):
        for k in range(rng.choice([1, 2, 3])):
            reads = [{"node": u, "column": rng.choice(cols[u])}]
            if parents.get(u):
                p = rng.choice(parents[u])
                reads.append({"node": p, "column": rng.choice(cols[p]).upper()})
            rules.append({"node": u, "clause": rng.choice(["WHERE", "JOIN", "QUALIFY", "HAVING"]), "text": f"c{k} > 1\tand x", "cut": rng.random() < 0.2, "columns": reads})
    cache = {"version": 1, "source": "collin", "target": "qa", "generated_at": "2030-01-02T03:04:05Z", "producer": "collin 0.12.0",
             "edges": edges, "roots": roots, "rules": rules, "expressions": expressions, "partial": partial}
    (target / "column_lineage.collin.json").write_text(json.dumps(cache))
    noted = rng.sample(models, min(8, len(models)))
    report = {"totals": {"models": len(models), "parsed": len(models)}, "models": [
        {"unique_id": u, "provenance": rng.choice(["per_column", "parsed", "inferred"]), "parse_error": rng.choice(["", "a name collin could not read"]),
         "sql_file": f"compiled/{u.split('.')[1]}/{u.split('.')[2]}.sql", "lost_columns": [{"column": cols[u][0].lower(), "ends": [{"reason": "x", "names": ["y"]}]}]}
        for u in noted]}
    (target / "collin.report.json").write_text(json.dumps(report))
    return {"edges": edges, "kinds": sorted({e["kind"] for e in edges}), "cols": cols}


def leaks(site, project, work):
    """What a folder must not hold: a path of this machine, or what the .env holds. The vendored
    libraries are checked for the canary alone: their own text names paths of their authors."""
    needles = {str(project), str(project.resolve()), str(work), str(work.resolve()), str(Path.home()), tempfile.gettempdir()}
    needles.discard("/")
    found = []
    for path in sorted(site.rglob("*")):
        if not path.is_file():
            continue
        text = path.read_bytes().decode("utf-8", "replace")
        rel = path.relative_to(site).as_posix()
        if CANARY in text:
            found.append(f"{rel} holds the canary")
        if not rel.startswith("vendor/"):
            found += [f"{rel} holds {n}" for n in needles if n and n in text]
    return found


def generate(binary, project, tally, name, prefix, compiled, home):
    """The site, into its default folder, and what the header's facts are. `prefix` is the project's place in its
    repository, `compiled` whether its manifest was written by a command that compiles."""
    run = subprocess.run([str(binary), str(project), "--site", "--environment", ENV], capture_output=True, text=True,
                         env=dict(os.environ, HOME=str(home)))
    tally.check(f"{name}: --site writes the folder and exits 0", run.returncode == 0, run.stderr[-500:])
    edges = len(json.loads((project / "target" / "column_lineage.collin.json").read_text())["edges"])
    tally.check(f"{name}: the run says the column lineage it carries, and is quiet about its age", "column lineage: " in run.stdout and "edges from collin" in run.stdout
                and "older than the manifest" not in run.stderr and "no column lineage" not in run.stderr, run.stdout[-300:] + run.stderr[-300:])
    site = project / "target" / "edith-site"
    tally.check(f"{name}: it prints the folder it wrote", run.stdout.splitlines()[:1] == [str(site.resolve())], run.stdout[:300])
    index = (site / "data" / "index.js").read_text()
    data = json.loads(index.removeprefix("window.EDITH_SITE=").rstrip().rstrip(";"))
    head = git(project, "rev-parse", "HEAD")
    meta = data["meta"]
    repo = "https://github.com/acme/" + ("jaffle-shop" if "jaffle" in name else "dbt-shopify")
    tally.check(f"{name}: the commit is HEAD, and its link the origin's host's", meta["commit"] == head and meta["commit_url"] == f"{repo}/commit/{head}", canon(meta))
    tally.check(f"{name}: a file is linked at the commit, below the project's place in its repository", meta["file_base"] == f"{repo}/blob/{head}/{prefix}", canon(meta))
    said = "no compiled SQL in the manifest" in run.stderr
    tally.check(f"{name}: the run says when no node was compiled, and only then", said == (not compiled), run.stderr[-500:])
    tally.check(f"{name}: the site holds the SQL in buckets of its own", (site / "data" / "sql" / "0000.js").is_file() and bool(data["sql_cuts"]), canon(data["sql_cuts"][:3]))
    tally.check(f"{name}: the header says the environment, dbt's version and when", meta["environment"] == ENV and meta["dbt_version"] and meta["generated_at"], canon(meta))
    tally.check(f"{name}: the origin that was set is never in the site", "git@github.com" not in index and "acme/" not in index.replace(repo, ""), "")
    tally.check(f"{name}: the column lineage is carried, named bare, current, and its file is written", meta["cll"] is True and meta["cll_file"] == "column_lineage.collin.json" and meta["cll_tool"] == "collin"
                and meta["cll_stale"] is False and (site / "data" / "cll.js").is_file() and meta["cll_edges"] > 0 and meta["cll_edges"] <= edges, canon(meta))
    return site, data


def new_page(work, name):
    (work / name).mkdir(parents=True, exist_ok=True)
    page = Page(work / name, 1)
    page.call("Page.enable")
    page.call("Network.enable")
    page.call("Log.enable")
    page.call("Page.addScriptToEvaluateOnNewDocument", source=WATCH)
    return page


def open_site(page, url):
    page.call("Page.navigate", url=url)
    page.until("typeof EdithSite !== 'undefined' && !!EdithSite.get && !!document.querySelector('#tree .row')", "the tree", timeout=30)


def batch_in_page(page, paths, node=False):
    """EdithSite's answer to each path, as canonical JSON text, in chunks."""
    out = []
    for i in range(0, len(paths), BATCH):
        chunk = json.dumps(paths[i:i + BATCH])
        out += page.js(f"""(async () => {{
          const canon = (v) => JSON.stringify(v, (k, x) => x && typeof x === 'object' && !Array.isArray(x)
            ? Object.fromEntries(Object.keys(x).sort().map((key) => [key, x[key]])) : x);
          return Promise.all({chunk}.map((p) => EdithSite.get(p).then(canon, (e) => 'ERR ' + e.message)));
        }})()""")
    return out


def lineage_path(node_id, up, down, tests, cap):
    return f"/api/lineage?id={urllib.parse.quote(node_id, safe='')}&up={up}&down={down}&tests={tests}&max={cap}"


def compare(tally, name, page, server, ids, searches):
    """Each node's lineage, card and the searches, the server's answers and the page's."""
    bad = []
    lineage = [lineage_path(i, u, d, t, m) for i in ids for (u, d, m) in GROUPS for t in (0, 1)]
    got = batch_in_page(page, lineage)
    for path, text in zip(lineage, got, strict=True):
        if text != canon(server.get(path)):
            bad.append(path)
    tally.check(f"{name}: {len(lineage)} lineages of {len(ids)} nodes are the server's", not bad, "\n        ".join(bad[:5]))

    bad = []
    cards = [f"/api/node?id={urllib.parse.quote(i, safe='')}" for i in ids]
    got = batch_in_page(page, cards)
    for path, text in zip(cards, got, strict=True):
        want = server.get(path)
        # Resolved against a .env, which a site never holds (0177).
        want.get("location", {}).pop("envs", None)
        if text != canon(want):
            bad.append(path)
    tally.check(f"{name}: {len(cards)} cards are the server's, but for the .env's locations", not bad, "\n        ".join(bad[:5]))

    bad = []
    paths = [f"/api/search?q={urllib.parse.quote(q, safe='')}&kind={kind}" + (f"&limit={limit}" if limit is not None else "") for q, kind, limit in searches]
    got = batch_in_page(page, paths)
    for path, text in zip(paths, got, strict=True):
        if text != canon(server.get(path)):
            bad.append(path)
    tally.check(f"{name}: {len(paths)} searches are the server's", not bad, "\n        ".join(bad[:5]))
    return len(lineage) + len(cards) + len(paths)


def collineage_path(node, column, up, down, indirect, cap=None, cut=""):
    path = (f"/api/collineage?id={urllib.parse.quote(node, safe='')}&column={urllib.parse.quote(column, safe='')}"
            f"&up={up}&down={down}&indirect={indirect}")
    if cap is not None:
        path += f"&max={cap}"
    if cut:
        path += f"&cut={urllib.parse.quote(cut, safe='')}"
    return path


def same_as_server(page, server, paths, workers=8):
    """The paths the page answers differently from the server, in chunks: the page's answers in one trip, the server's
    over a few connections. Returns the paths that differ."""
    from concurrent.futures import ThreadPoolExecutor
    bad = []
    with ThreadPoolExecutor(workers) as pool:
        for i in range(0, len(paths), 200):
            chunk = paths[i:i + 200]
            got = batch_in_page(page, chunk)
            want = list(pool.map(lambda p: canon(server.get(p)), chunk))
            bad += [p for p, g, w in zip(chunk, got, want, strict=True) if g != w]
    return bad


def compare_columns(tally, name, page, server, ids, cache):
    """The column lineage (0182): every column of every node, at 2+2 with the indirect edges hidden and drawn, and a sample
    at 1+1, 20+20, with a role cut and under a cap; the chains of every node; a card with collin's note. Returns the number
    of /api/collineage answers compared."""
    cards = dict(zip(ids, (json.loads(t) for t in batch_in_page(page, [f"/api/node?id={urllib.parse.quote(i, safe='')}" for i in ids])), strict=True))
    columns = [(i, c["name"]) for i in ids for c in cards[i]["columns"]]
    tally.check(f"{name}: the cards carry collin's note, and the columns the cache named", any("collin" in c for c in cards.values())
                and any(c.get("up") or c.get("down") for card in cards.values() for c in card["columns"]), "")
    every = [collineage_path(i, c, 2, 2, ind) for i, c in columns for ind in (0, 1)]
    bad = same_as_server(page, server, every)
    tally.check(f"{name}: /api/collineage of {len(columns)} columns at 2+2, the indirect edges hidden and drawn ({len(every)} answers), is the server's", not bad, "\n        ".join(bad[:5]))

    named = {(e["from"], e["from_col"].lower()) for e in cache["edges"]} | {(e["to"], e["to_col"].lower()) for e in cache["edges"]}
    with_edges = [(i, c) for i, c in columns if (i, c.lower()) in named]
    step = max(1, len(with_edges) // 80)
    sample = with_edges[::step]
    roles = [k for k in cache["kinds"] if k and k not in ROWS]
    cut = ",".join(roles[:2])
    more = [collineage_path(i, c, up, down, ind, cap, cuts) for i, c in sample
            for up, down, ind, cap, cuts in ((1, 1, 0, None, ""), (20, 20, 0, None, ""), (20, 20, 1, None, ""), (3, 3, 0, None, cut),
                                             (20, 20, 1, 5, ""), (2, 2, 1, 3, cut), (0, 4, 0, 6, "window,nonsense"))]
    bad = same_as_server(page, server, more)
    tally.check(f"{name}: {len(more)} answers for {len(sample)} columns at 1+1, 20+20, with a role cut and under a cap are the server's", not bad, "\n        ".join(bad[:5]))

    reach = [f"/api/colreach?id={urllib.parse.quote(i, safe='')}" for i in ids]
    bad = same_as_server(page, server, reach)
    tally.check(f"{name}: /api/colreach of all {len(ids)} nodes is the server's", not bad, "\n        ".join(bad[:5]))
    return len(every) + len(more) + len(reach), len(columns)


def encode_component(text):
    """encodeURIComponent, as the page writes a segment of a path."""
    return urllib.parse.quote(text, safe="!~*'()")


def file_url(meta, file):
    return meta["file_base"] + "/".join(encode_component(p) for p in file.split("/"))


def compare_code(tally, name, page, server, project, data, compiled):
    """The SQL the site gives for every node: compiled, what the server reads from target/compiled; source, the
    node's file cut of its ends. A project that was only parsed has no compiled text, and a node says so.
    Returns (nodes with code, compiled compared, source compared)."""
    base = data["base"]
    ids = [b["id"] for b in base]
    paths = [f"/api/site/code?id={urllib.parse.quote(i, safe='')}" for i in ids]
    got = batch_in_page(page, paths)
    failed = [i for i, t in zip(ids, got, strict=True) if t.startswith("ERR ")]
    tally.check(f"{name}: the site answers the code of all {len(ids)} nodes", not failed, ", ".join(failed[:5]))
    answers = {i: json.loads(t) for i, t in zip(ids, got, strict=True) if not t.startswith("ERR ")}
    meta = data["meta"]

    with_code = [i for i, a in answers.items() if a["raw"] or a["compiled"] is not None]
    # Only the kinds that have a Code tab have a text.
    kinds = {b["id"]: b["kind"] for b in base}
    stray = [i for i in with_code if kinds[i] in ("seed", "source", "exposure")]
    tally.check(f"{name}: {len(with_code)} nodes have code, and no seed, source or exposure is one", len(with_code) > 0 and not stray, ", ".join(stray[:5]))

    made = [i for i in with_code if answers[i]["compiled"] is not None]
    if compiled:
        bad, found = [], 0
        for i in made:
            # The answer's own fields, `info` being what the page calls them once read.
            info = server.get(f"/api/compiled?kind=compiled&id={urllib.parse.quote(i, safe='')}")
            if not info["found"] or info["truncated"]:
                continue
            found += 1
            if info["content"] != answers[i]["compiled"]:
                bad.append(i)
        tally.check(f"{name}: the compiled SQL of {found} nodes is what the server reads from target/compiled", found > 0 and not bad,
                    ", ".join(bad[:5]) or "no node was compared")
    else:
        found = 0
        tally.check(f"{name}: no node is compiled, in a manifest only parsed", not made, ", ".join(made[:5]))

    # The raw code of a model, a snapshot, an analysis or a singular test is its file, as dbt reads it: cut of its ends.
    bad, same = [], 0
    for i in with_code:
        file = answers[i]["file"]
        path = project / file
        if kinds[i] in ("operation",) or not file.endswith(".sql") or not path.is_file():
            continue
        same += 1
        if path.read_text(encoding="utf-8").strip() != answers[i]["raw"]:
            bad.append(i)
    tally.check(f"{name}: the source of {same} nodes is their file, trimmed", same > 0 and not bad, ", ".join(bad[:5]) or "no node was compared")

    # A link is the host's pattern at the commit below the project's place, for the project's own files only.
    root = meta["root"]
    wrong = []
    for i, a in answers.items():
        mine = a["file"] and (i.split(".")[1] == root or kinds[i] == "operation")
        want = file_url(meta, a["file"]) if mine and not a["file"].startswith("dbt_packages/") else None
        if a["url"] != want:
            wrong.append(f"{i}: {a['url']} for {a['file']}")
    tally.check(f"{name}: a node's file is linked below the project's place in the repository, and a package's has no link", not wrong, "\n        ".join(wrong[:5]))
    return len(with_code), found, same


def server_tree(server, project, path="", seen=None):
    """What the server lists of the project's folders, cut to what a site holds: the files that carry a
    kind (a model, a seed or a snapshot is built from them), and the folders that hold one at any depth.
    Walked from the root over the disk, a link that loops left alone. Returns {folder: entries}."""
    seen = set() if seen is None else seen
    real = (project / path).resolve()
    if real in seen:
        return {}
    seen.add(real)
    out, kept = {}, []
    for e in server.get("/api/dir?path=" + urllib.parse.quote(path, safe="")):
        if e["dir"]:
            below = server_tree(server, project, e["path"], seen)
            if below:
                out.update(below)
                kept.append(e)
        elif "kind" in e:
            kept.append(e)
    if kept:
        out[path] = kept
    return out


def compare_tree(tally, name, page, server, project, data):
    """The site's /api/dir for every folder it has, the server's, name by name, and the folders themselves the same set.
    A package model whose package the server could not find under the project (a `local:` package is a link out of
    it, which the server does not follow, 0094) is listed by the site where `dbt deps` would put it and cannot be
    listed by the server: those files, and the folders that hold nothing else, are left out of the comparison."""
    want = server_tree(server, project)
    folders = sorted(data["tree"])
    answers = dict(zip(folders, (json.loads(t) for t in batch_in_page(page, ["/api/dir?path=" + urllib.parse.quote(d, safe="") for d in folders])), strict=True))
    moved = set(data["placed"].values())

    def kept(folder):
        out = []
        for e in answers.get(folder, []):
            if e["dir"]:
                if kept(e["path"]):
                    out.append(e)
            elif e["path"] not in moved:
                out.append(e)
        return out

    have = {f: k for f in folders if (k := kept(f))}
    tally.check(f"{name}: the site's folders are the ones the server's tree holds files of models in", sorted(want) == sorted(have),
                f"only server {sorted(set(want) - set(have))[:5]}, only site {sorted(set(have) - set(want))[:5]}")
    bad = []
    fields = ("name", "path", "dir", "kind", "materialized")
    for folder, got in have.items():
        ours = [tuple(e.get(f) for f in fields) for e in got]
        theirs = [tuple(e.get(f) for f in fields) for e in want.get(folder, [])]
        # The server's order of two names that lower-case alike is the disk's: the order of what lower-cases is compared, and the entries.
        order = [(not e["dir"], e["name"].lower()) for e in got] == [(not e["dir"], e["name"].lower()) for e in want.get(folder, [])]
        if sorted(ours) != sorted(theirs) or not order:
            bad.append(folder or "(the root)")
    entries = sum(len(v) for v in want.values())
    tally.check(f"{name}: {len(have)} folders and {entries} entries of the tree are the server's ({len(moved)} files of packages not found on disk are the site's alone)", not bad, ", ".join(bad[:5]))


def searches_of(names):
    """Names, parts of names and the shapes a query takes, for every kind and several limits."""
    out = [("", "", None), ("", "model", 5), ("", "source", None), ("", "test", 10), ("", "seed,snapshot", None), ("", "exposure", None),
           ("", "nonsense", 7), ("  ", "model", 3), ("\u0085" + names[0][:3], "", 5), ("﻿" + names[0][:3], "", 5), ("order", "", None),
           ("ORDER", "model", 20), ("models/", "", 10), ("staging", "model", 10), (".sql", "", 5), ("zz-nothing-matches", "", None),
           ("_", "model,source", 50), ("a", "", 0)]
    step = max(1, len(names) // 25)
    for name in names[::step]:
        out += [(name, "", 8), (name[:4], "model", 6), (name[-5:], "", 4)]
    return out


def box_of(page, expr):
    return page.js(f"(() => {{ const e = {expr}; if (!e) return null; const r = e.getBoundingClientRect(); return [r.x, r.y, r.width, r.height]; }})()")


def smoke(tally, name, page, ids, model):
    """The page, read as a person reads it."""
    def check(label, ok, detail=""):
        tally.check(f"{name}: {label}", ok, detail)

    stamp = page.js("(() => { const s = document.querySelector('#site-stamp'); return s ? { text: s.textContent, hidden: s.classList.contains('hidden'),"
                    " a: (s.querySelector('a') ? [s.querySelector('a').textContent, s.querySelector('a').getAttribute('href'), s.querySelector('a').getAttribute('rel'),"
                    " s.querySelector('a').getAttribute('target')] : null) } : null; })()")
    meta = page.js("EdithSite.meta")
    check("the header names the environment, the commit as a link, the time and dbt's version",
          stamp and not stamp["hidden"] and ENV in stamp["text"] and f"dbt {meta['dbt_version']}" in stamp["text"] and bool(stamp["a"])
          and stamp["a"][0] == meta["commit"][:7] and stamp["a"][1] == meta["commit_url"], json.dumps(stamp))
    check("the link opens elsewhere and gives the opener nothing", stamp and stamp["a"] and stamp["a"][2] == "noopener noreferrer" and stamp["a"][3] == "_blank")
    check("the time is written in the reader's own clock, not as dbt wrote it", stamp and meta["generated_at"] not in stamp["text"] and re.search(r"\d", stamp["text"]), json.dumps(stamp))
    title = page.js("document.title")
    check("the tab's title is the project, the environment and the tool", title == f"{meta['project']} · {ENV} · dbt-edith", title)

    shown = page.js(f"{json.dumps(HIDDEN)}.filter((s) => {{ const e = document.querySelector(s); return e && getComputedStyle(e).display !== 'none'; }})")
    check("what a site has no use for is not on the page", shown == [], str(shown))
    missing = page.js(f"{json.dumps(SHOWN)}.filter((s) => {{ const e = document.querySelector(s); return !e || getComputedStyle(e).display === 'none'; }})")
    check("what it has is", missing == [], str(missing))
    dock, main = (page.js(f"document.querySelector('{s}').getBoundingClientRect().height") for s in ("#dock", "#main"))
    check("the dock fills the window the editor left", abs(dock - main) < 2, f"{dock} of {main}")

    # A click on what is hidden does nothing, and says nothing (the page's own watch catches a throw).
    for sel in ["[data-dock=terminal]", "[data-dock=jobs]", "[data-dock=compiled]", "[data-dock=run]", "[data-side=git]", "[data-side=search]",
                "#space-reports", "#space-lineage", "#settings-btn", "#github-btn"]:
        page.js(f"document.querySelector('{sel}').click()")
    time.sleep(0.4)
    state = page.js("({ dock: document.querySelector('#dock .tab.active').dataset.dock, side: document.querySelector('[data-side].active').dataset.side,"
                    " panels: [...document.querySelectorAll('.panel')].filter((p) => p.getAttribute('aria-hidden') === 'false').length,"
                    " terminal: !!document.querySelector('#term .xterm') })")
    check("a click on what is hidden changes nothing", state == {"dock": "lineage", "side": "files", "panels": 0, "terminal": False}, json.dumps(state))

    # The Project tab is the one in front, and says so.
    check("the Files tab is the Project, and in front", page.js("document.querySelector('[data-side=files]').textContent") == "Project"
          and page.js("document.querySelector('[data-side=files]').classList.contains('active')"))

    # The Models list, and a click.
    page.js("document.querySelector('[data-side=models]').click()")
    page.until("!!document.querySelector('#model-list .row')", "the Models list")
    rows = page.js("document.querySelectorAll('#model-list .row').length")
    check("the Models list has its rows", rows > 0, str(rows))
    # Filtered by name: the list shows the first few hundred, and Shopify has more.
    page.js(f"(i => {{ i.value = {json.dumps(model)}; i.dispatchEvent(new Event('input')); }})(document.querySelector('#model-filter'))")
    page.until(f"[...document.querySelectorAll('#model-list .row .nm')].some((n) => n.textContent === {json.dumps(model)})", "the model in the list")
    page.js(f"[...document.querySelectorAll('#model-list .row')].find((r) => r.querySelector('.nm').textContent === {json.dumps(model)}).click()")
    focus = next(i for i in ids if i.endswith("." + model) and i.startswith("model."))
    page.until(f"document.querySelectorAll('#graph .nd').length > 1 && !!document.querySelector('#graph .nd.focus[data-id={json.dumps(focus)}]')", "the lineage drawn")
    check("a click draws the model's lineage and writes it into the address", page.js("decodeURIComponent(location.hash)") == "#" + focus, page.js("location.hash"))
    check("the status line says what was drawn", "node" in page.js("document.querySelector('#lineage-status').textContent"))

    # A click on a box: the Catalog of that node, with its Columns.
    other = page.js(f"""(async () => {{
      for (const g of document.querySelectorAll('#graph .nd')) {{
        const id = g.dataset.id;
        if (id === {json.dumps(focus)} || !id.startsWith('model.')) continue;
        if ((await EdithSite.get('/api/node?id=' + encodeURIComponent(id))).columns.length) return id;
      }}
      return null;
    }})()""")
    page.js(f"document.querySelector('#graph .nd[data-id={json.dumps(other)}]').dispatchEvent(new MouseEvent('click', {{ bubbles: true }}))")
    page.js("document.querySelector('[data-dock=catalog]').click()")
    card = page.js(f"EdithSite.get('/api/node?id=' + encodeURIComponent({json.dumps(other)}))")
    page.until(f"document.querySelector('#catalog .nm') && document.querySelector('#catalog .nm').textContent === {json.dumps(card['name'])}", "the Catalog of the box")
    check("a click on a box shows that node's Catalog", True)
    check("the Catalog has no Open file", not page.js("[...document.querySelectorAll('#catalog button')].some((b) => b.textContent === 'Open file')"))
    page.js("[...document.querySelectorAll('#catalog .ctab')].find((t) => t.textContent.startsWith('Columns')).click()")
    columns = card["columns"]
    page.until(f"document.querySelector('#catalog .cat-body').textContent.includes({json.dumps(columns[0]['name'])})", "the Columns", timeout=10)
    text = page.js("document.querySelector('#catalog .cat-body').textContent")
    check("the Columns list the node's columns", all(c["name"] in text for c in columns[:5]), text[:200])
    typed = [c["data_type"] for c in columns if c["data_type"]]
    if meta["catalog"] and typed:
        check("and the types the catalog gave", typed[0].lower() in text.lower(), text[:200])
    hint = page.js("(() => { const h = document.querySelector('#catalog .coltools'); return h ? h.textContent : ''; })()")
    if "no types" in hint:
        check("the Columns hint names the site's step, not Snowflake", "dbt-edith --site" in hint and "Snowflake" not in hint, hint)

    # The Code tab of that node: its source, its compiled text or why there is none, and the link to its file.
    code = page.js(f"EdithSite.get('/api/site/code?id=' + encodeURIComponent({json.dumps(other)}))")
    page.js("[...document.querySelectorAll('#catalog .ctab')].find((t) => t.textContent === 'Code').click()")
    page.until("!!document.querySelector('#catalog .cat-code-cm .CodeMirror')", "the Code tab's editor")
    editor = "document.querySelector('#catalog .CodeMirror').CodeMirror"
    check("the Code tab opens on the source, in a read-only editor", page.js(f"{editor}.getValue()") == code["raw"] and page.js(f"{editor}.getOption('readOnly')") is True)
    page.js("[...document.querySelectorAll('#catalog .segbtn')].find((b) => b.textContent === 'Compiled').click()")
    if code["compiled"] is None:
        page.until("!!document.querySelector('#catalog .compiled-empty')", "the note")
        note = page.js("document.querySelector('#catalog .compiled-empty').textContent")
        check("a node dbt did not compile says so and names the remedy", "did not compile this node" in note and "dbt-edith --site" in note and "dbt docs generate" in note, note)
        check("and keeps no editor for it", page.js("!document.querySelector('#catalog .CodeMirror')"))
    else:
        page.until(f"!!document.querySelector('#catalog .CodeMirror') && {editor}.getValue() === {json.dumps(code['compiled'])}", "the compiled SQL")
        check("Compiled shows the compiled SQL", True)
    link = page.js("(() => { const a = document.querySelector('#catalog .cat-code a.repolink'); return a ? [a.getAttribute('href'), a.getAttribute('target'), a.getAttribute('rel'), a.textContent] : null; })()")
    crumb = page.js("(() => { const a = document.querySelector('#catalog .crumb a.repolink'); return a ? a.getAttribute('href') : null; })()")
    if code["url"]:
        want = file_url(meta, code["file"])
        check("the file is a link to the repository at the commit, with the project's place", link is not None and link[0] == want == code["url"], f"{link} for {want}")
        check("the link opens elsewhere and gives the opener nothing", link is not None and link[1] == "_blank" and link[2] == "noopener noreferrer")
        check("the crumb's file is the same link", crumb == want, str(crumb))
    else:
        # A model of a package: its file is not in the repository.
        check("a package's file is no link, in the tab or in the crumb", link is None and crumb is None, f"{link} {crumb}")
    page.js("[...document.querySelectorAll('#catalog .segbtn')].find((b) => b.textContent === 'Source').click()")
    page.until(f"!!document.querySelector('#catalog .CodeMirror') && {editor}.getValue() === {json.dumps(code['raw'])}", "the source again")
    check("the toggle goes back to the source", True)

    # A package's file is not in the repository.
    outside = next((i for i in ids if i.startswith("model.") and not i.startswith(f"model.{meta['root']}.")), None)
    if outside:
        page.js(f"location.hash = {json.dumps(urllib.parse.quote(outside, safe=''))}")
        page.until(f"!!document.querySelector('#graph .nd.focus[data-id={json.dumps(outside)}]')", "the package's model")
        page.js("document.querySelector('[data-dock=catalog]').click()")
        page.until("!!document.querySelector('#catalog .cat-code .code-path')", "the package model's Code tab")
        check("a package's file has no link, in the tab or in the crumb", page.js("!document.querySelector('#catalog a.repolink')")
              and page.js("document.querySelector('#catalog .code-path').textContent") != "")
    page.js("[...document.querySelectorAll('#catalog .ctab')].find((t) => t.textContent === 'Preview').click()")
    page.js("document.querySelector('[data-dock=lineage]').click()")

    # Cmd+K.
    page.js("document.dispatchEvent(new KeyboardEvent('keydown', { key: 'k', metaKey: true, ctrlKey: true, bubbles: true }))")
    page.until("!document.querySelector('#palette').classList.contains('hidden')", "the palette")
    check("Cmd+K opens a palette that says what it searches", "macros" not in page.js("document.querySelector('#palette-input').placeholder"))
    page.js(f"(i => {{ i.value = {json.dumps(model[:8])}; i.dispatchEvent(new Event('input')); }})(document.querySelector('#palette-input'))")
    page.until(f"[...document.querySelectorAll('#palette-results .pal-row .pal-nm')].some((n) => n.textContent === {json.dumps(model)})", "the model in the palette")
    kinds = page.js("[...document.querySelectorAll('#palette-results .pal-row')].map((r) => r.className)")
    check("the palette lists nodes only, no macro, no file and no search in the files", "t-node" in kinds[0] and not any(re.search(r"t-(macro|file|grep)", k) for k in kinds), str(kinds[:8]))
    page.js(f"(i => {{ i.value = {json.dumps(card['name'])}; i.dispatchEvent(new Event('input')); }})(document.querySelector('#palette-input'))")
    page.until(f"[...document.querySelectorAll('#palette-results .pal-row .pal-nm')].some((n) => n.textContent === {json.dumps(card['name'])})", "a node in the palette")
    page.js(f"[...document.querySelectorAll('#palette-results .pal-row')].find((r) => r.querySelector('.pal-nm').textContent === {json.dumps(card['name'])}).click()")
    page.until(f"decodeURIComponent(location.hash) === '#' + {json.dumps(other)}", "the palette's pick")
    check("a pick in the palette focuses it", True)
    page.js("document.querySelector('[data-side=files]').click()")
    sel = page.js("(() => { const r = document.querySelector('#tree .row.sel'); return r ? r.dataset.path : null; })()")
    want = page.js(f"EdithSite.treePath({json.dumps(other)})")
    check("the focused model's file is lit in the tree, opened up to it", sel == want, f"{sel} for {want}")
    # A folder opened and a file clicked: the model is focused and its row lit.
    target = next(i for i in ids if i.startswith("model.") and i != other)
    path = page.js(f"EdithSite.treePath({json.dumps(target)})")
    page.js("document.querySelector('#tree').querySelectorAll('.kids').forEach((k) => k.classList.add('hidden'))")
    parts = path.split("/")
    for k in range(1, len(parts)):
        folder = "/".join(parts[:k])
        page.until(f"!!document.querySelector('#tree .row.folder[data-path={json.dumps(folder)}]')", f"the folder {folder}")
        page.js(f"(() => {{ const r = document.querySelector('#tree .row.folder[data-path={json.dumps(folder)}]'); if (!r.classList.contains('open')) r.click(); }})()")
        page.until(f"document.querySelector('#tree .row.folder[data-path={json.dumps(folder)}]').classList.contains('open')", f"{folder} open")
    page.until(f"!!document.querySelector('#tree .row.file[data-path={json.dumps(path)}]')", "the file's row")
    page.js(f"document.querySelector('#tree .row.file[data-path={json.dumps(path)}]').click()")
    page.until(f"!!document.querySelector('#graph .nd.focus[data-id={json.dumps(target)}]')", "the lineage of the file's model")
    check("a click on a file's row focuses its model", page.js("decodeURIComponent(location.hash)") == "#" + target)
    check("and lights the row", page.js("document.querySelector('#tree .row.sel') && document.querySelector('#tree .row.sel').dataset.path") == path)
    check("the tree has no file a site cannot show: no test, no YAML", page.js("[...document.querySelectorAll('#tree .row.file .nm')].every((n) => /\\.(sql|py|csv)$/.test(n.textContent))"))

    # An address that names a node opens on it.
    last = next(i for i in reversed(ids) if i.startswith("model.") and i != other)
    page.js(f"location.hash = {json.dumps(urllib.parse.quote(last, safe=''))}")
    page.until(f"!!document.querySelector('#graph .nd.focus[data-id={json.dumps(last)}]')", "the node the address names")
    check("an address that changes names a node, and the page follows it", True)


def requests_of(page, suffix):
    return [u for u in page.requests() if u.endswith(suffix)]


def open_columns(tally, name, page, cache, ids, stale=False):
    """A column of the site, opened as a person opens it: from the Columns tab to its canvas, with the indirect button and the
    stop at menu, and none of what a site has no collin, Snowflake or menu for."""
    def check(label, ok, detail=""):
        tally.check(f"{name}: {label}", ok, detail)

    owned = set(ids)
    # A column that something reads and that reads something: a column reading itself is no lineage.
    outs = {(e["from"], e["from_col"].lower()) for e in cache["edges"] if e["kind"] not in ROWS and (e["from"], e["from_col"].lower()) != (e["to"], e["to_col"].lower())}
    chosen = next((e for e in cache["edges"] if e["kind"] not in ROWS and e["from"] != e["to"] and e["to"] in owned and e["to"].startswith("model.")
                   and (e["to"], e["to_col"].lower()) in outs), None)
    if not chosen:
        check("a column with an edge on each side exists to open", False, "the cache has none")
        return
    node, column = chosen["to"], chosen["to_col"]
    page.js(f"location.hash = {json.dumps(urllib.parse.quote(node, safe=''))}")
    page.until(f"!!document.querySelector('#graph .nd.focus[data-id={json.dumps(node)}]')", "the model of the column")
    page.js("document.querySelector('[data-dock=catalog]').click()")
    page.js("[...document.querySelectorAll('#catalog .ctab')].find((t) => t.textContent.startsWith('Columns')).click()")
    page.until("!!document.querySelector('#catalog table.cols')", "the Columns table")
    check("no menu to pick another cache, and no Regenerate", page.js("!document.querySelector('.cllpick')")
          and page.js("![...document.querySelectorAll('button')].some((b) => b.textContent === 'Regenerate')"))
    hint = page.js("(() => { const h = document.querySelector('#catalog .colhint:not(.modeltests)'); return h ? [h.textContent, h.dataset.tone, h.title] : null; })()")
    if stale:
        check("the Columns tab says the cache is older than the manifest, in its own tone", hint and hint[1] == "stale" and "older than the manifest" in hint[0]
              and "--run-collin" in hint[2], json.dumps(hint))
    else:
        check("the Columns tab says how far to trust collin's, and nothing of its age", hint and hint[1] == "alpha" and hint[0].startswith("alpha, can be wrong"), json.dumps(hint))
    reach = page.js(f"EdithSite.get('/api/colreach?id=' + encodeURIComponent({json.dumps(node)})).then((rows) => rows.find((r) => r.name.toLowerCase() === {json.dumps(column.lower())}))")
    page.until(f"[...document.querySelectorAll('#catalog tr.c-linked')].some((r) => r.querySelector('.c-name span').textContent.toLowerCase() === {json.dumps(column.lower())}"
               f" && r.querySelector('.c-lin').textContent.includes('\u2190{reach['up']}') && r.querySelector('.c-lin').textContent.includes('{reach['down']}\u2192'))",
               "the column's chain in its Lineage cell", timeout=20)
    check("the Lineage cell says the whole chain, as /api/colreach counts it", True)

    row = f"[...document.querySelectorAll('#catalog tr.c-linked')].find((r) => r.querySelector('.c-name span').textContent.toLowerCase() === {json.dumps(column.lower())})"
    page.js(f"{row}.querySelector('.c-type').click()")
    page.until("!!document.querySelector('#graph .nd[data-id*=\"::\"]')", "the column canvas", timeout=30)
    up, down = page.js("[+document.querySelector('#up').value, +document.querySelector('#down').value]")
    answer = page.js(f"EdithSite.get({json.dumps(collineage_path(node, column, up, down, 0))})")
    status = page.js("document.querySelector('#lineage-status').textContent")
    check("the status line names the column and what was drawn", status.startswith(answer["focus_column"] + " \u00b7 "), status)
    title = page.js("document.querySelector('#lineage-status').title")
    if stale:
        check("and says the cache is older than the manifest, the whole sentence in its title", status.endswith("column lineage older than the manifest") and "may not match this commit" in title, f"{status} / {title}")
    else:
        check("and says nothing of the cache's age", "older than the manifest" not in status and title == "", f"{status} / {title}")
    drawn = page.js("document.querySelectorAll('#graph .nd').length")
    check("the canvas draws the answer's boxes", drawn == len(answer["nodes"]), f"{drawn} of {len(answer['nodes'])}")

    # A second column asks for nothing more of the folder: the file is loaded once.
    page.js("document.querySelector('[data-dock=catalog]').click()")
    page.until("!!document.querySelector('#catalog tr.c-linked')", "the Columns table again")
    page.js("document.querySelectorAll('#catalog tr.c-linked')[document.querySelectorAll('#catalog tr.c-linked').length - 1].querySelector('.c-type').click()")
    page.until("document.querySelector('#lineage-status').textContent.includes(' columns') || document.querySelector('#lineage-status').textContent.includes(' column ')", "a second column")
    check("data/cll.js is asked for once, however many columns", len(requests_of(page, "/data/cll.js")) == 1, str(requests_of(page, "/data/cll.js")))

    has_rows = any(e["kind"] in ROWS for e in cache["edges"])
    page.js("document.querySelector('[data-dock=catalog]').click()")
    page.until("!!document.querySelector('#catalog tr.c-linked')", "the Columns table")
    page.js(f"{row}.querySelector('.c-type').click()")
    page.until(f"document.querySelector('#lineage-status').textContent.startsWith({json.dumps(answer['focus_column'])})", "the column again")
    shown = page.js("(() => { const b = document.querySelector('#indirect-btn'); return !!b && !b.classList.contains('hidden'); })()")
    check("the indirect button is in view when the cache holds indirect edges", shown == has_rows, str(shown))
    if has_rows:
        page.js("document.querySelector('#indirect-btn').click()")
        page.until("document.querySelector('#indirect-btn').getAttribute('aria-pressed') === 'true'", "the indirect button pressed")
        up, down = page.js("[+document.querySelector('#up').value, +document.querySelector('#down').value]")
        want = page.js(f"EdithSite.get({json.dumps(collineage_path(node, column, up, down, 1))})")
        page.until(f"document.querySelectorAll('#graph .nd').length === {len(want['nodes'])}", "the canvas with the indirect edges", timeout=20)
        check("the indirect button draws the answer with the indirect edges", True)
        page.js("document.querySelector('#indirect-btn').click()")
        page.until("document.querySelector('#indirect-btn').getAttribute('aria-pressed') === 'false'", "the indirect button back")

    page.js("document.querySelector('#stop-at').click()")
    page.until("!!document.querySelector('[aria-label=\"Roles the column walk stops at\"]')", "the stop at menu")
    roles = page.js("[...document.querySelectorAll('[aria-label=\"Roles the column walk stops at\"] [role=menuitemcheckbox] .lbl')].map((b) => b.textContent)")
    check("the stop at menu lists the cache's roles, never an indirect one", roles and set(roles) <= set(k for k in cache["kinds"] if k) and not set(roles) & {r.lower() for r in ROWS}, str(roles))
    role = roles[0]
    page.js(f"[...document.querySelectorAll('[aria-label=\"Roles the column walk stops at\"] [role=menuitemcheckbox]')].find((b) => b.querySelector('.lbl').textContent === {json.dumps(role)}).click()")
    up, down = page.js("[+document.querySelector('#up').value, +document.querySelector('#down').value]")
    want = page.js(f"EdithSite.get({json.dumps(collineage_path(node, column, up, down, 0, None, role))})")
    page.until(f"document.querySelectorAll('#graph .nd').length === {len(want['nodes'])}", "the canvas stopped at the role", timeout=20)
    check("a role ticked redraws the canvas with the walk stopped at it", True)
    page.js("document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }))")


def end(tally, name, page, prefixes):
    """What the page did: asked for nothing outside its folder, and said nothing wrong."""
    bad = page.js("window.__bad")
    tally.check(f"{name}: no exception, no rejection, no console error, no toast that says one, no violation of the policy", bad == [] and page.errors == [] and page.console_errors() == [],
                json.dumps([*bad, *page.errors, *page.console_errors()])[:600])
    outside = [u for u in page.requests() if not u.startswith("data:") and not any(u.startswith(p) for p in prefixes)]
    tally.check(f"{name}: {len(page.requests())} requests, none outside the folder", outside == [], "\n        ".join(outside[:5]))
    return len(outside)


ASKED = []


class Quiet(http.server.SimpleHTTPRequestHandler):
    """A static host that counts what it is asked for."""

    def log_message(self, fmt, *args):
        ASKED.append(self.path)


def stale_site(tally, binary, name, project, work, cache, ids, home):
    """A cache older than the manifest is carried with a warning: on stderr, on the status line and in the Columns tab."""
    target = project / "target"
    old = (target / "manifest.json").stat().st_mtime - 3600
    os.utime(target / "column_lineage.collin.json", (old, old))
    folder = target / "edith-site-stale"
    try:
        run = subprocess.run([str(binary), str(project), "--site", str(folder), "--environment", ENV], capture_output=True, text=True, env=dict(os.environ, HOME=str(home)))
        tally.check(f"{name}: a cache older than the manifest is carried, exit 0, and stderr says it is older", run.returncode == 0
                    and "warning: collin's column lineage is older than the manifest: it may not match this commit; --run-collin writes it again" in run.stderr, run.stderr[-400:])
        data = json.loads((folder / "data" / "index.js").read_text().removeprefix("window.EDITH_SITE=").rstrip().rstrip(";"))
        tally.check(f"{name}: and the site says so", data["meta"]["cll_stale"] is True and (folder / "data" / "cll.js").is_file(), canon(data["meta"]))
        page = new_page(work, f"chrome-stale-{name}")
        try:
            url = (folder / "index.html").resolve().as_uri()
            open_site(page, url)
            open_columns(tally, f"{name} (cache older than the manifest)", page, cache, ids, stale=True)
            end(tally, f"{name} (cache older than the manifest)", page, [url.rsplit("/", 1)[0] + "/"])
        finally:
            page.close()
    finally:
        shutil.rmtree(folder, ignore_errors=True)
        os.utime(target / "column_lineage.collin.json", None)


def run_collin(tally, binary, work, project):
    """`--site --run-collin` runs collin first, over a copy of the project, with a stand-in on the PATH, and carries what it wrote;
    `--site` alone, with that stand-in there, never starts it."""
    import walk_mcp as M  # its stand-in and the three edges it writes
    name = "jaffle_shop (--run-collin)"
    copy = work / "jaffle_run"
    shutil.copytree(project, copy, ignore=shutil.ignore_patterns("edith-site*"))
    target = copy / "target"
    (target / "column_lineage.collin.json").unlink()
    (target / "collin.report.json").unlink()
    bin_dir = work / "collin-bin"
    bin_dir.mkdir()
    stand_in = bin_dir / "collin"
    stand_in.write_text(M.STAND_IN)
    stand_in.chmod(stand_in.stat().st_mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)
    env = dict(os.environ, HOME=str(work / "home"), PATH=bin_dir.as_posix() + os.pathsep + os.environ.get("PATH", ""))
    site = target / "edith-site"

    def listing():
        return sorted(p.relative_to(copy).as_posix() for p in copy.rglob("*") if p.is_file() and "edith-site" not in p.parts)

    before = listing()
    run = subprocess.run([str(binary), str(copy), "--site"], capture_output=True, text=True, env=env)
    data = json.loads((site / "data" / "index.js").read_text().removeprefix("window.EDITH_SITE=").rstrip().rstrip(";"))
    tally.check(f"{name}: --site alone, with a cache missing, says so, carries none and never starts collin", run.returncode == 0 and "no column lineage" in run.stderr
                and data["meta"]["cll"] is False and not (site / "data" / "cll.js").exists() and not (bin_dir / "args").exists() and listing() == before, run.stderr[-300:])
    run = subprocess.run([str(binary), str(copy), "--site", "--run-collin"], capture_output=True, text=True, env=env)
    tally.check(f"{name}: --site --run-collin runs collin, writes the two files it writes, and says so", run.returncode == 0 and run.stderr.startswith("collin 0.12.0 wrote")
                and (bin_dir / "args").is_file() and sorted(set(listing()) - set(before)) == ["target/collin.report.json", "target/column_lineage.collin.json"], run.stderr[-400:])
    data = json.loads((site / "data" / "index.js").read_text().removeprefix("window.EDITH_SITE=").rstrip().rstrip(";"))
    tally.check(f"{name}: the site carries what it wrote, current", data["meta"]["cll"] is True and data["meta"]["cll_edges"] == len(M.CACHE["edges"]) and data["meta"]["cll_stale"] is False
                and f"column lineage: {len(M.CACHE['edges'])} edges from collin" in run.stdout, run.stdout[-300:] + canon(data["meta"]))
    page = new_page(work, "chrome-run-collin")
    try:
        url = (site / "index.html").resolve().as_uri()
        open_site(page, url)
        got = page.js("EdithSite.get('/api/collineage?id=model.jaffle_shop.orders&column=amount&up=2&down=2').then((a) => a.nodes.map((n) => n.id))")
        tally.check(f"{name}: a column of what collin wrote draws, from its upstream", "model.jaffle_shop.stg_payments::amount" in got and "model.jaffle_shop.orders::amount" in got, str(got))
        end(tally, name, page, [url.rsplit("/", 1)[0] + "/"])
    finally:
        page.close()


def over_http(tally, name, work, site, server, ids, model, cache):
    root = work / "served"
    shutil.rmtree(root, ignore_errors=True)
    shutil.copytree(site, root / "docs" / "v1")
    ASKED.clear()
    host = http.server.ThreadingHTTPServer(("127.0.0.1", 0), functools.partial(Quiet, directory=str(root)))
    threading.Thread(target=host.serve_forever, daemon=True).start()
    base = f"http://127.0.0.1:{host.server_address[1]}"
    page = new_page(work, f"chrome-http-{name}")
    try:
        open_site(page, f"{base}/docs/v1/index.html")
        sample = ids[:40]
        compare(tally, f"{name} over http", page, server, sample, [("", "model", 5), (model, "", 3), ("order", "", None)])
        smoke(tally, f"{name} over http", page, ids, model)
        # An edge into a node the manifest lacks names no column of the site.
        known = set(ids)
        edges = [e for e in cache["edges"] if e["to"] in known][:: max(1, len(cache["edges"]) // 60)]
        bad = same_as_server(page, server, [collineage_path(e["to"], e["to_col"], 2, 2, ind) for e in edges for ind in (0, 1)] + [f"/api/colreach?id={urllib.parse.quote(e['to'], safe='')}" for e in edges])
        tally.check(f"{name} over http: {len(edges) * 2} column answers and {len(edges)} chains are the server's", not bad, "\n        ".join(bad[:5]))
        open_columns(tally, f"{name} over http", page, cache, ids)
        asked = [p for p in ASKED if p.endswith("/data/cll.js")]
        tally.check(f"{name} over http: the host was asked for data/cll.js once", len(asked) == 1, str(asked))
        outside = end(tally, f"{name} over http", page, [f"{base}/docs/v1/"])
        stray = [p for p in ASKED if not p.startswith("/docs/v1/")]
        tally.check(f"{name} over http: the host was asked for nothing outside the sub-path", stray == [] and outside == 0, str(stray[:5]))
    finally:
        page.close()
        host.shutdown()


def walk_project(tally, binary, name, project, model, work, prefix, compiled, seed):
    print(f"\n--- {name} ---")
    # Written after the manifest and before the site and the server, which both read it.
    cache = invent_cache(project, seed)
    site, data = generate(binary, project, tally, name, prefix, compiled, work / "home")
    found = leaks(site, project, work)
    tally.check(f"{name}: the folder holds no path of this machine and no value of the .env", not found, "; ".join(found[:5]))
    ids = [b["id"] for b in data["base"]]
    names = [b["name"] for b in data["base"] if b["kind"] in ("model", "source", "seed")]
    home = work / f"home-{name}"
    (home / "config").mkdir(parents=True)
    (home / "home").mkdir()
    edith = W.Edith(binary, project, home)
    try:
        server = Server(edith)
        page = new_page(work, f"chrome-{name}")
        try:
            url = (site / "index.html").resolve().as_uri()
            open_site(page, url)
            compared = compare(tally, name, page, server, ids, searches_of(names))
            compare_tree(tally, name, page, server, project, data)
            with_code, compiled_same, source_same = compare_code(tally, name, page, server, project, data, compiled)
            col_compared, col_count = compare_columns(tally, name, page, server, ids, cache)
            smoke(tally, name, page, ids, model)
            # The same address, written into a fresh page: the site opens on its node.
            first = next(i for i in ids if i.startswith("model."))
            page.call("Page.navigate", url=url + "#" + urllib.parse.quote(first, safe=""))
            page.until(f"!!document.querySelector('#graph .nd.focus[data-id={json.dumps(first)}]')", "the node the address names", timeout=15)
            tally.check(f"{name}: a link to a node opens on it", True)
            open_columns(tally, name, page, cache, ids)
            end(tally, name, page, [url.rsplit("/", 1)[0] + "/"])
        finally:
            page.close()
        over_http(tally, name, work, site, server, ids, model, cache)
        print(f"{name}: {len(ids)} nodes, {compared} answers compared, the compiled SQL of {compiled_same} nodes and the source of {source_same} of {with_code} with code, "
              f"{col_compared} column lineage answers over {col_count} columns")
    finally:
        edith.stop()
    stale_site(tally, binary, name, project, work, cache, ids, work / "home")


def main():
    os.environ.setdefault("DBT_SEND_ANONYMOUS_USAGE_STATS", "False")
    os.environ.setdefault("DO_NOT_TRACK", "1")
    binary = W.edith_binary()
    tally = Tally()
    skipped = ""
    with tempfile.TemporaryDirectory(prefix="dbt-edith-site-") as tmp:
        work = Path(tmp)
        (work / "home").mkdir()
        jaffle_project = jaffle(work)
        walk_project(tally, binary, "jaffle_shop", jaffle_project, "orders", work, JAFFLE_FOLDER + "/", True, 1)
        run_collin(tally, binary, work, jaffle_project)
        try:
            project = shopify(work)
        except C.Unavailable as e:
            skipped = f"Shopify: {e}"
        else:
            walk_project(tally, binary, "shopify", project, "shopify__orders", work, "integration_tests/", False, 2)
    if tally.failed:
        print(f"{tally.failed} check(s) failed")
        return 1
    if skipped:
        print("the site is the server's on Jaffle Shop")
        print(f"SKIPPED: {skipped}")
        return 2
    print("the site is the server's, on Jaffle Shop and on Shopify")
    return 0


if __name__ == "__main__":
    sys.exit(main())
