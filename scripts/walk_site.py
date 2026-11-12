#!/usr/bin/env python3
"""The static site (0177), written by `dbt-edith --site`, opened in headless Chrome beside the server.

Two projects, each a git repository with an invented github.com origin and a
.env that holds a canary: Jaffle Shop, built, with its catalog, and Fivetran's
Shopify package at the pinned tag (0056), parsed only. On each:

  - the site is written, and nothing in it is a path of this machine or the
    canary; its header facts are the manifest's and git's;
  - the answers EdithSite gives in the browser, from the folder, are the
    server's, node by node: /api/lineage at 1+1, 2+2 and 20+20, with the tests
    on and off, and at 2+2 under a cap of 5; /api/node but for the location
    resolved against a .env, which a site never holds; and a set of searches;
  - the page reads: the header names the environment, the commit as a link
    that opens elsewhere and gives nothing away, when the manifest was written
    and dbt's version; the Models list, a click, a click on a box, the
    Catalog and its Columns, Cmd+K and an address that names a node; what the
    site has no use for is not on the page, and a click on what is hidden does
    nothing and says nothing;
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
import re
import shutil
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


def jaffle(work):
    """Jaffle Shop, built, with its catalog, a git repository with an invented origin."""
    project = work / "jaffle_shop"
    shutil.copytree(W.JAFFLE, project)
    (project / ".env").write_text(f"DBT_SCHEMA={CANARY}\nSECRET_TOKEN={CANARY}\n")
    git(project, "init", "-q", "-b", "main")
    git(project, "add", "-A")
    git(project, "commit", "-q", "-m", "jaffle shop")
    git(project, "remote", "add", "origin", "git@github.com:acme/jaffle-shop.git")
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


def generate(binary, project, tally, name):
    """The site, into its default folder, and what the header's facts are."""
    run = subprocess.run([str(binary), str(project), "--site", "--environment", ENV], capture_output=True, text=True,
                         env=dict(os.environ, HOME=str(project.parent / "home")))
    tally.check(f"{name}: --site writes the folder and exits 0", run.returncode == 0, run.stderr[-500:])
    site = project / "target" / "edith-site"
    tally.check(f"{name}: it prints the folder it wrote", run.stdout.splitlines()[:1] == [str(site.resolve())], run.stdout[:300])
    index = (site / "data" / "index.js").read_text()
    data = json.loads(index.removeprefix("window.EDITH_SITE=").rstrip().rstrip(";"))
    head = git(project, "rev-parse", "HEAD")
    meta = data["meta"]
    repo = "https://github.com/acme/" + ("jaffle-shop" if "jaffle" in name else "dbt-shopify")
    tally.check(f"{name}: the commit is HEAD, and its link the origin's host's", meta["commit"] == head and meta["commit_url"] == f"{repo}/commit/{head}", canon(meta))
    tally.check(f"{name}: the header says the environment, dbt's version and when", meta["environment"] == ENV and meta["dbt_version"] and meta["generated_at"], canon(meta))
    tally.check(f"{name}: the origin that was set is never in the site", "git@github.com" not in index and "acme/" not in index.replace(repo, ""), "")
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


def over_http(tally, name, work, site, server, ids, model):
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
        outside = end(tally, f"{name} over http", page, [f"{base}/docs/v1/"])
        stray = [p for p in ASKED if not p.startswith("/docs/v1/")]
        tally.check(f"{name} over http: the host was asked for nothing outside the sub-path", stray == [] and outside == 0, str(stray[:5]))
    finally:
        page.close()
        host.shutdown()


def walk_project(tally, binary, name, project, model, work):
    print(f"\n--- {name} ---")
    site, data = generate(binary, project, tally, name)
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
            smoke(tally, name, page, ids, model)
            # The same address, written into a fresh page: the site opens on its node.
            first = next(i for i in ids if i.startswith("model."))
            page.call("Page.navigate", url=url + "#" + urllib.parse.quote(first, safe=""))
            page.until(f"!!document.querySelector('#graph .nd.focus[data-id={json.dumps(first)}]')", "the node the address names", timeout=15)
            tally.check(f"{name}: a link to a node opens on it", True)
            end(tally, name, page, [url.rsplit("/", 1)[0] + "/"])
        finally:
            page.close()
        over_http(tally, name, work, site, server, ids, model)
        print(f"{name}: {len(ids)} nodes, {compared} answers compared")
    finally:
        edith.stop()


def main():
    os.environ.setdefault("DBT_SEND_ANONYMOUS_USAGE_STATS", "False")
    os.environ.setdefault("DO_NOT_TRACK", "1")
    binary = W.edith_binary()
    tally = Tally()
    skipped = ""
    with tempfile.TemporaryDirectory(prefix="dbt-edith-site-") as tmp:
        work = Path(tmp)
        (work / "home").mkdir()
        walk_project(tally, binary, "jaffle_shop", jaffle(work), "orders", work)
        try:
            project = shopify(work)
        except C.Unavailable as e:
            skipped = f"Shopify: {e}"
        else:
            walk_project(tally, binary, "shopify", project, "shopify__orders", work)
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
