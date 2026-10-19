#!/usr/bin/env python3
"""The guided tour, walked in headless Chrome on Jaffle Shop (0074).

Jaffle Shop is copied from tests/fixtures/ and parsed by dbt, then served by
dbt-edith with a configuration directory and a HOME of its own, and driven
through the DevTools protocol over a websocket written here, so nothing is
installed but Chrome. It checks what the harness in web/tests/tour.js
cannot see:

  - started before dbt has written a manifest, the tour is not offered and
    nothing is noted, the shortcut list's tour saying there is no manifest
    and ringing the ? button it is taken again from;
    the offer comes once dbt parse writes one, at the end of a tour on
    screen when it lands (0078);
  - the tour is offered once, beside the ? button, and not after a reload;
  - every step rings the element it is about, with its card on screen, and
    the page under it shows what the step says: the model's file, its
    lineage, the Custom selection, the Catalog, the Jobs tab;
  - Back sets a step up as Next does;
  - it runs nothing: the terminal never starts, and no setting is written,
    the configuration directory holding the offer's file and nothing else;
  - ended part way from a page in use, it puts the tab, the canvas, the dock,
    the selection box and the depth boxes back as they were, and a model's
    lineage at the depth it was drawn at, the tour's own drawn at 1 and 1;
  - it starts from the shortcut list too.

dbt runs in this process, through `dbtRunner`, so it needs dbt-core and
dbt-duckdb importable by the Python running the script, or DBT naming a dbt
executable. Without dbt or without Chrome it skips itself, exit 2, like the
other checks that need something installed (0013). CHROME names a Chrome to
run instead of the one in /Applications. `--shots DIR` saves a picture of
each step there.

Exit 0 when the walk went as it should, 1 when it did not, 2 when skipped.
"""

import base64
import json
import os
import re
import shutil
import socket
import struct
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
JAFFLE = ROOT / "tests" / "fixtures" / "jaffle_shop"
CHROME = os.environ.get("CHROME") or "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"
MODEL = "stg_orders"


def skip(reason):
    print(f"SKIPPED: {reason}")
    sys.exit(2)


def edith_binary():
    """The dbt-edith DBT_EDITH names, or the one built here, found where cargo
    says it put it: CARGO_TARGET_DIR or a cargo config moves it out of
    target/, and a path written here would run whatever older build was left
    there."""
    if os.environ.get("DBT_EDITH"):
        return os.environ["DBT_EDITH"]
    out = subprocess.run(["cargo", "build", "--quiet", "--message-format=json-render-diagnostics"],
                         cwd=ROOT, check=True, stdout=subprocess.PIPE, encoding="utf-8").stdout
    for line in out.splitlines():
        msg = json.loads(line)
        if msg.get("reason") == "compiler-artifact" and msg["target"]["name"] == "dbt-edith" and msg.get("executable"):
            return Path(msg["executable"])
    sys.exit("cargo build named no dbt-edith executable")


EXECUTABLE = os.environ.get("DBT")
if not EXECUTABLE:
    try:
        from dbt.cli.main import dbtRunner
        import dbt.adapters.duckdb  # noqa: F401  the fixture's profile names it
    except ImportError as e:
        skip(f"dbt-core with dbt-duckdb is not importable here ({e.name}). "
             "Install both, point this at a Python that has them, or set DBT to a dbt executable.")
if not os.access(CHROME, os.X_OK):
    skip(f"no Chrome at {CHROME}. Set CHROME to one to run this.")


def parse(project):
    """Writes target/manifest.json inside the copy, where dbt-edith looks."""
    args = ["parse", "--project-dir", str(project), "--profiles-dir", str(project)]
    if EXECUTABLE:
        run = subprocess.run([EXECUTABLE] + args + ["--quiet"], capture_output=True, text=True)
        if run.returncode != 0:
            raise RuntimeError(f"dbt parse failed: {(run.stdout + run.stderr)[-2000:]}")
        return
    res = dbtRunner().invoke(args + ["--log-level", "none"])
    if not res.success:
        raise RuntimeError(f"dbt parse failed: {res.exception}")


def free_port():
    with socket.socket() as s:
        s.bind(("127.0.0.1", 0))
        return s.getsockname()[1]


class Edith:
    def __init__(self, binary, project, home):
        self.port = free_port()
        self.log_path = home / "server.log"
        self.log = open(self.log_path, "w")
        env = dict(os.environ, DBT_EDITH_CONFIG_DIR=str(home / "config"), HOME=str(home / "home"))
        env.pop("DBT_PROFILES_DIR", None)
        self.proc = subprocess.Popen(
            [str(binary), str(project), "--port", str(self.port), "--no-open", "--no-update-check"],
            stdout=self.log, stderr=subprocess.STDOUT, env=env,
        )
        deadline = time.time() + 60
        while True:
            found = re.search(r"(http://127\.0\.0\.1:\d+/\?key=[0-9a-f]{64})", self.log_path.read_text())
            if found:
                self.url = found.group(1)
                return
            if time.time() > deadline or self.proc.poll() is not None:
                self.stop()
                raise RuntimeError("dbt-edith did not start: " + self.log_path.read_text()[-2000:])
            time.sleep(0.2)

    def stop(self):
        self.proc.terminate()
        try:
            self.proc.wait(timeout=10)
        except subprocess.TimeoutExpired:
            self.proc.kill()
        self.log.close()


class Page:
    """One Chrome tab, through the DevTools protocol over a websocket written
    here: the standard library has no websocket client."""

    def __init__(self, work, scale):
        # Chrome is stopped if anything after its start fails, or every run
        # that fails here would leave one running.
        self.chrome = None
        try:
            self._start(work, scale)
        except BaseException:
            self.close()
            raise

    def _start(self, work, scale):
        profile = work / "chrome"
        profile.mkdir()
        # Port 0: Chrome picks a free one and writes it down, so two checks
        # running at once never drive each other's browser.
        self.chrome = subprocess.Popen(
            [CHROME, "--headless=new", "--remote-debugging-port=0", f"--user-data-dir={profile}",
             "--window-size=1400,900", f"--force-device-scale-factor={scale}", "--hide-scrollbars",
             "--no-first-run", "--no-default-browser-check", "about:blank"],
            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
        )
        deadline = time.time() + 30
        while True:
            try:
                port = int((profile / "DevToolsActivePort").read_text().split()[0])
                tabs = json.load(urllib.request.urlopen(f"http://127.0.0.1:{port}/json", timeout=5))
                url = next(t["webSocketDebuggerUrl"] for t in tabs if t["type"] == "page")
                break
            except (OSError, ValueError, StopIteration, IndexError):
                if time.time() > deadline:
                    raise RuntimeError("Chrome did not start")
                time.sleep(0.2)
        hostport, path = url.split("://", 1)[1].split("/", 1)
        host, port = hostport.split(":")
        self.sock = socket.create_connection((host, int(port)), timeout=60)
        key = base64.b64encode(os.urandom(16)).decode()
        self.sock.sendall((f"GET /{path} HTTP/1.1\r\nHost: {hostport}\r\nUpgrade: websocket\r\n"
                           f"Connection: Upgrade\r\nSec-WebSocket-Key: {key}\r\nSec-WebSocket-Version: 13\r\n\r\n").encode())
        buf = b""
        while b"\r\n\r\n" not in buf:
            buf += self.sock.recv(4096)
        self.buf = buf.split(b"\r\n\r\n", 1)[1]
        self.id = 0
        self.errors = []
        self.call("Page.enable")
        self.call("Runtime.enable")

    def _exact(self, n):
        while len(self.buf) < n:
            chunk = self.sock.recv(65536)
            if not chunk:
                raise RuntimeError("Chrome closed the connection")
            self.buf += chunk
        out, self.buf = self.buf[:n], self.buf[n:]
        return out

    def _send(self, obj):
        data = json.dumps(obj).encode()
        head = bytearray([0x81])
        n = len(data)
        if n < 126:
            head.append(0x80 | n)
        elif n < 65536:
            head.append(0x80 | 126)
            head += struct.pack(">H", n)
        else:
            head.append(0x80 | 127)
            head += struct.pack(">Q", n)
        mask = os.urandom(4)
        self.sock.sendall(bytes(head) + mask + bytes(b ^ mask[i % 4] for i, b in enumerate(data)))

    def _recv(self):
        head = self._exact(2)
        n = head[1] & 0x7F
        if n == 126:
            n = struct.unpack(">H", self._exact(2))[0]
        elif n == 127:
            n = struct.unpack(">Q", self._exact(8))[0]
        return json.loads(self._exact(n))

    def call(self, method, **params):
        self.id += 1
        self._send({"id": self.id, "method": method, "params": params})
        while True:
            m = self._recv()
            if m.get("method") == "Runtime.exceptionThrown":
                self.errors.append(m["params"]["exceptionDetails"].get("exception", {}).get("description", "?"))
            if m.get("id") == self.id:
                if "error" in m:
                    raise RuntimeError(f"{method}: {m['error']}")
                return m.get("result", {})

    def js(self, expr):
        r = self.call("Runtime.evaluate", expression=expr, returnByValue=True, awaitPromise=True)
        if "exceptionDetails" in r:
            raise RuntimeError(f"{expr[:80]}: {r['exceptionDetails'].get('exception', {}).get('description')}")
        return r["result"].get("value")

    def until(self, expr, what, timeout=15):
        deadline = time.time() + timeout
        while True:
            value = self.js(expr)
            if value:
                return value
            if time.time() > deadline:
                raise AssertionError(f"timed out waiting for {what}")
            time.sleep(0.1)

    def key(self, key, code, vk, text=None):
        base = dict(key=key, code=code, windowsVirtualKeyCode=vk, nativeVirtualKeyCode=vk)
        self.call("Input.dispatchKeyEvent", type="keyDown" if text else "rawKeyDown", text=text or "", **base)
        self.call("Input.dispatchKeyEvent", type="keyUp", **base)

    def shot(self, path):
        data = self.call("Page.captureScreenshot", format="png")["data"]
        path.write_bytes(base64.b64decode(data))

    def close(self):
        if not self.chrome:
            return
        self.chrome.terminate()
        try:
            self.chrome.wait(timeout=10)
        except subprocess.TimeoutExpired:
            self.chrome.kill()


RIGHT = ("ArrowRight", "ArrowRight", 39)
LEFT = ("ArrowLeft", "ArrowLeft", 37)
ESCAPE = ("Escape", "Escape", 27)

# What each step rings, by its title, and what the page under it must show.
STEPS = [
    ("A tour of dbt-edith", None, None),
    ("manifest.json, and how fresh it is", "#fresh-badge",
     "document.querySelector('#fresh-badge .lbl')?.textContent.startsWith('manifest ')"),
    (f"{MODEL}, in the editor", "#editor-pane",
     f"document.querySelector('#tabbar .ftab.active')?.textContent.includes('{MODEL}.sql')"),
    ("Where dbt_project.yml configures it", ["#crumbs .cfg", ".crumbmenu.config"],
     "[...document.querySelectorAll('.crumbmenu.config button .lbl')].some((l) => l.textContent === 'staging')"),
    ("Lineage", "#dock-lineage",
     "!document.querySelector('#dock-lineage').classList.contains('hidden')"
     f" && [...document.querySelectorAll('#graph text')].some((t) => t.textContent === '{MODEL}')"
     " && document.querySelector('#graph-mode .active').dataset.mode === 'model'"),
    ("Custom selection", "#select-bar",
     f"document.querySelector('#select-input').value === '+{MODEL}+'"
     " && document.querySelector('#graph-mode .active').dataset.mode === 'select'"
     " && document.querySelector('#select-sum').textContent !== ''"
     " && !document.querySelector('.crumbmenu')"),
    ("Export, or copy as an image", ["#export-btn", "#image-btn", "#select-copy"],
     "document.querySelector('#graph-mode .active').dataset.mode === 'select'"),
    ("Catalog", "#dock-catalog",
     f"!document.querySelector('#dock-catalog').classList.contains('hidden')"
     f" && document.querySelector('.cat-title')?.textContent.includes('{MODEL}')"
     " && document.querySelector('#graph-mode .active').dataset.mode === 'model'"),
    ("Column lineage", "#cll-pick-host .cllpick",
     "document.querySelector('.cat-tabs .ctab.active')?.textContent.startsWith('Columns')"),
    ("Jobs", "#dock-jobs",
     "!document.querySelector('#dock-jobs').classList.contains('hidden') && !!document.querySelector('#jobs-body .jobs-none')"),
    ("Reports", "#reports-btn",
     "(b => !b.classList.contains('hidden') && b.classList.contains('dim'))(document.querySelector('#reports-btn'))"),
    ("Terminal", "#dock [data-dock=terminal]", None),
    ("Search", "#palette-btn", None),
    ("Every shortcut, and this tour", "#keys-btn", None),
]

STATE = """(() => ({
  active: document.querySelector('#tabbar .ftab.active')?.textContent || '',
  tabs: [...document.querySelectorAll('#tabbar .ftab')].map((t) => t.textContent + (t.classList.contains('preview') ? ' (preview)' : '')),
  dock: document.querySelector('#dock .tabs .tab.active')?.dataset.dock,
  mode: document.querySelector('#graph-mode .active')?.dataset.mode,
  select: document.querySelector('#select-input').value,
  depth: document.querySelector('#up').value + ' up, ' + document.querySelector('#down').value + ' down',
  catalog: document.querySelector('.cat-title')?.textContent || document.querySelector('#catalog').textContent,
  jobs: document.querySelector('#jobs-head .jobs-about')?.textContent || '',
  reports: document.querySelector('#reports-btn').className,
  panel: document.querySelector('#reports').classList.contains('open'),
}))()"""

# The model and the depth of the last lineage drawn, as the page asked for it.
LINEAGE = """(() => { const l = performance.getEntriesByType('resource').map((e) => e.name)
  .filter((n) => n.includes('/api/lineage?')).pop(); if (!l) return ''; const u = new URL(l);
  return u.searchParams.get('id') + ' ' + u.searchParams.get('up') + '/' + u.searchParams.get('down'); })()"""

# The requests that look at the job files, so far: the tour makes none, and
# puts back a Reports panel without reading its section again.
LOOKS = """(() => performance.getEntriesByType('resource')
  .filter((e) => /look=true|\\/api\\/jobs\\/coverage/.test(e.name)).length)()"""

# A key the app answers from anywhere, sent where the tour has the focus:
# whether the tour kept the browser from acting on it too.
HELD = """((key, code, mod) => {
  const e = new KeyboardEvent('keydown', { key, code, metaKey: mod, ctrlKey: mod, bubbles: true, cancelable: true });
  document.querySelector('#tour .tour-next').dispatchEvent(e);
  return e.defaultPrevented;
})"""


class Walk:
    def __init__(self, page, config, shots):
        self.page, self.config, self.shots = page, config, shots
        self.failed = 0

    def check(self, label, ok, detail=""):
        print(("PASS  " if ok else "FAIL  ") + label + ("" if ok else f"\n        {detail}"))
        self.failed += not ok

    def settle(self, want, timeout=10):
        """The page's state once it matches `want`, or as it stands after
        `timeout`: the tour's end reads a tab again and redraws a canvas."""
        deadline = time.time() + timeout
        while (got := self.page.js(STATE)) != want and time.time() < deadline:
            time.sleep(0.1)
        return got

    def step_shown(self, index, total):
        return self.page.until(
            f"document.querySelector('#tour .tour-count')?.textContent === '{index + 1} of {total}'"
            " && document.querySelector('#tour-title').textContent",
            f"step {index + 1}")

    def look(self, index, total):
        """The step on screen against STEPS: its title, its ring around its
        target, its card on screen, and the page under it."""
        title, target, shows = STEPS[index]
        got = self.step_shown(index, total)
        self.check(f"step {index + 1}: {title}", got == title, f"got {got!r}")
        # The ring slides from one target to the next in 0.2 s.
        time.sleep(0.35)
        geometry = self.page.js(f"""(() => {{
          const ring = document.querySelector('#tour .tour-ring');
          const card = document.querySelector('#tour .tour-card').getBoundingClientRect();
          const rs = [].concat({json.dumps(target)} || []).map((s) => document.querySelector(s))
            .filter(Boolean).map((e) => e.getBoundingClientRect());
          const r = rs.length ? {{ left: Math.min(...rs.map((x) => x.left)), top: Math.min(...rs.map((x) => x.top)),
            right: Math.max(...rs.map((x) => x.right)), bottom: Math.max(...rs.map((x) => x.bottom)) }} : null;
          const g = ring.classList.contains('hidden') ? null : ring.getBoundingClientRect();
          return {{ ring: g && [g.left, g.top, g.right, g.bottom], target: r && [r.left, r.top, r.right, r.bottom],
                   card: [card.left, card.top, card.right, card.bottom], view: [innerWidth, innerHeight] }};
        }})()""")
        view = geometry["view"]
        card = geometry["card"]
        self.check("  its card is on screen", card[0] >= 0 and card[1] >= 0 and card[2] <= view[0] and card[3] <= view[1],
                   f"card {card} in {view}")
        if target is None:
            self.check("  rings nothing, the card centred", geometry["ring"] is None, f"ring {geometry['ring']}")
        else:
            ring, box = geometry["ring"], geometry["target"]
            around = ring and box and all(abs(a - b) <= 6 for a, b in zip(ring, [
                max(2, box[0] - 4), max(2, box[1] - 4), min(view[0] - 2, box[2] + 4), min(view[1] - 2, box[3] + 4)]))
            self.check(f"  rings {' and '.join([target] if isinstance(target, str) else target)}", bool(around),
                       f"ring {ring} around {box}")
        if shows:
            self.check("  the page shows what it says", bool(self.page.until(shows, f"step {index + 1}'s page", 10)))
        if self.shots:
            self.page.call("Input.dispatchMouseEvent", type="mouseMoved", x=1, y=899)
            self.page.shot(self.shots / f"{index + 1:02d}-{re.sub(r'[^a-z]+', '-', title.lower()).strip('-')}.png")

    def run(self, url, parse):
        page = self.page
        page.call("Page.navigate", url=url)
        page.until("document.querySelectorAll('#tree .row').length > 0", "the page to load", 30)

        print("--- before a manifest ---")
        time.sleep(1.5)
        self.check("not offered with no model to show", not page.js("!!document.querySelector('.tour-offer')"))
        self.check("  and nothing noted", not (self.config / "tour.json").exists())
        page.key("?", "Slash", 191, text="?")
        page.until("!document.querySelector('#shortcuts')?.classList.contains('hidden')", "the shortcut list")
        page.js("[...document.querySelectorAll('#shortcuts button')].find((b) => b.textContent === 'Give me a tour').click()")
        total = int(page.until("document.querySelector('#tour .tour-count')?.textContent.split(' of ')[1]", "the tour"))
        welcome = page.until("document.querySelector('#tour .tour-text')?.textContent", "the welcome")
        self.check("the shortcut list's tour leaves out the steps about a model, and says there is no manifest",
                   total == 7 and welcome.startswith("There is no manifest.json to read yet"), f"{total} steps: {welcome}")
        time.sleep(0.35)
        rung = page.js("""(() => {
          const g = document.querySelector('#tour .tour-ring');
          if (g.classList.contains('hidden')) return false;
          const r = g.getBoundingClientRect(), k = document.querySelector('#keys-btn').getBoundingClientRect();
          return r.left <= k.left && r.top <= k.top && r.right >= k.right && r.bottom >= k.bottom && r.width < k.width + 20;
        })()""")
        self.check("  ringing the ? button it is taken again from", rung)
        if self.shots:
            page.shot(self.shots / "00-no-manifest.png")
        # dbt parse lands with the tour still up. The watcher reads the
        # manifest, and the page hears of it at its next poll.
        parse()
        page.until("document.querySelectorAll('#counts .chip').length > 0", "the manifest", 30)
        time.sleep(1)
        self.check("a manifest landing during the tour offers nothing under it, and notes nothing",
                   not page.js("!!document.querySelector('.tour-offer')") and not (self.config / "tour.json").exists())
        page.key(*ESCAPE)
        page.until("!document.querySelector('#tour')", "Escape to end it")

        print("--- the offer ---")
        offer = page.until("document.querySelector('.tour-offer')?.textContent", "the offer")
        self.check("offered once the tour ends, dbt parse having written a manifest", "New to dbt-edith?" in offer, offer)
        under = page.js("""(() => {
          const o = document.querySelector('.tour-offer').getBoundingClientRect();
          const k = document.querySelector('#keys-btn').getBoundingClientRect();
          return o.top >= k.bottom && o.left <= k.left && o.right >= k.right;
        })()""")
        self.check("  under the ? button", under)
        self.check("  and noted at once", (self.config / "tour.json").exists())
        self.check("the Reports button in view before any report, dimmed (0075)",
                   page.js("(b => !b.classList.contains('hidden') && b.classList.contains('dim'))(document.querySelector('#reports-btn'))"))
        if self.shots:
            page.shot(self.shots / "00-the-offer.png")
        before = page.js(STATE)

        print("--- the walk ---")
        page.js("performance.setResourceTimingBufferSize(100000)")
        looks = page.js(LOOKS)
        page.js("[...document.querySelectorAll('.tour-offer button')].find((b) => b.textContent === 'Give me a tour').click()")
        total = int(page.until("document.querySelector('#tour .tour-count')?.textContent.split(' of ')[1]", "the tour"))
        self.check("every step on a project with a model", total == len(STEPS), f"got {total} of {len(STEPS)}")
        self.check("the page under it takes no click or key",
                   page.js("['#topbar', '#body', '#statusbar'].every((s) => document.querySelector(s).inert)"))
        for i in range(total):
            self.look(i, total)
            if i == 7:
                # Back from the Catalog draws the selection again, and Next
                # leaves it for the model's lineage once more.
                page.key(*LEFT)
                self.look(6, total)
                page.key(*RIGHT)
                self.look(7, total)
            if i < total - 1:
                if i % 2:
                    page.key(*RIGHT)
                else:
                    page.js("document.querySelector('#tour .tour-next').click()")
        self.check("the last button says Done", page.js("document.querySelector('#tour .tour-next').textContent") == "Done")
        page.js("document.querySelector('#tour .tour-next').click()")
        page.until("!document.querySelector('#tour')", "the tour to end")
        after = page.js(STATE)

        print("--- what it left ---")
        self.check("the page takes clicks and keys again",
                   page.js("['#topbar', '#body', '#statusbar'].every((s) => !document.querySelector(s).inert)"))
        self.check("the terminal never started", page.js("document.querySelector('#term').childElementCount") == 0)
        written = sorted(str(p.relative_to(self.config)) for p in self.config.rglob("*"))
        self.check("no setting written: the offer's file alone", written == ["tour.json"], f"found {written}")
        kept = json.loads((self.config / "tour.json").read_text())
        self.check("  which keeps when it was offered and the version, and nothing else",
                   sorted(kept) == ["offered", "version"] and re.fullmatch(r"\d+\.\d+\.\d+", kept["version"]), f"{kept}")
        self.check("no menu left open", not page.js("!!document.querySelector('.crumbmenu')"))
        self.check("the Reports button as it was", after["reports"] == before["reports"], f"{before} then {after}")
        self.check("no job file looked at through every step, the Jobs step included",
                   page.js(LOOKS) == looks, f"{looks} requests looking at them before, {page.js(LOOKS)} after")
        self.check("with no tab open before, the model stays open as a place to start",
                   after["active"].startswith(f"{MODEL}.sql") and after["dock"] == before["dock"], f"{after}")

        print("--- a reload ---")
        # Marked first, so the wait below cannot be answered by the old page.
        page.js("window.beforeReload = true")
        page.call("Page.reload")
        page.until("!window.beforeReload && document.querySelectorAll('#tree .row').length > 0", "the page to load again", 30)
        time.sleep(1.5)
        self.check("offers it no more", not page.js("!!document.querySelector('.tour-offer')"))

        print("--- from the shortcut list, on a page in use ---")
        page.until("!!document.querySelector('#tree .row[data-path=\"models\"]')", "the tree")
        page.js("document.querySelector('#tree .row[data-path=\"models\"]').click()")
        page.until("!!document.querySelector('#tree .row[data-path=\"models/customers.sql\"]')", "the models folder")
        # orders.sql as the preview, behind customers.sql pinned in front.
        page.js("document.querySelector('#tree .row[data-path=\"models/orders.sql\"]').click()")
        page.until("document.querySelector('#tabbar .ftab.preview')?.textContent.startsWith('orders.sql')", "orders as the preview")
        page.js("document.querySelector('#tree .row[data-path=\"models/customers.sql\"]')"
                ".dispatchEvent(new MouseEvent('dblclick', { bubbles: true }))")
        page.until("document.querySelector('.cat-title')?.textContent.includes('customers')", "customers in the Catalog")
        # A depth of its own, which the tour's model, opened from the editor,
        # starts without.
        page.js("for (const [id, v] of [['up', '3'], ['down', '2']]) {"
                " const el = document.querySelector('#' + id); el.value = v; el.dispatchEvent(new Event('change')); }")
        page.js("document.querySelector('[data-dock=catalog]').click()")
        page.js("document.querySelector('[data-mode=select]').click()")
        page.js("document.querySelector('#select-input').value = 'tag:staging'")
        page.js("document.querySelector('#select-input').dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter' }))")
        page.until("document.querySelector('#select-sum').textContent !== ''", "the selection")
        page.js("document.querySelector('[data-dock=jobs]').click()")
        page.until("document.querySelector('#jobs-head .jobs-about')?.textContent === 'customers'", "the Jobs tab on customers")
        page.js("document.querySelector('#reports-btn').click()")
        page.until("document.querySelector('#reports').classList.contains('open')", "the Reports panel")
        in_use = page.js(STATE)
        page.js("performance.setResourceTimingBufferSize(100000)")
        page.until("document.querySelector('#reports-view')?.textContent.includes('Job coverage')", "the coverage drawn")
        looks = page.js(LOOKS)
        page.key("?", "Slash", 191, text="?")
        page.until("!document.querySelector('#shortcuts')?.classList.contains('hidden')", "the shortcut list")
        page.js("[...document.querySelectorAll('#shortcuts button')].find((b) => b.textContent === 'Give me a tour').click()")
        total = int(page.until("document.querySelector('#tour .tour-count')?.textContent.split(' of ')[1]", "the tour"))
        self.check("it starts from the list, over the Reports panel",
                   total == len(STEPS) and page.js("document.querySelector('#shortcuts').classList.contains('hidden')"))
        for i in range(6):
            self.step_shown(i, total)
            if i == 3:
                # The dbt_project.yml menu is shown, not used: Tab stays on the card.
                page.until("!!document.querySelector('.crumbmenu.config')", "the menu")
                for _ in range(4):
                    page.key("Tab", "Tab", 9)
                self.check("Tab stays on the card, the menu it shows being inert",
                           page.js("!!document.activeElement.closest('#tour') && document.querySelector('.crumbmenu.config').inert"))
            if i == 4:
                self.check("the app's keys wait, and the browser does not act on them either",
                           page.js(HELD + "('s', 'KeyS', true)") and page.js(HELD + "('F1', 'F1', false)")
                           and page.js("document.querySelector('#shortcuts').classList.contains('hidden')"))
            page.key(*RIGHT)
        self.step_shown(6, total)
        page.key(*ESCAPE)
        page.until("!document.querySelector('#tour')", "Escape to end it")
        back = self.settle(in_use)
        self.check("Escape ends it, and the tabs, the canvas, the dock, the Jobs tab's node and the panel are as they were",
                   back == in_use, f"before {in_use}\n        after  {back}")
        time.sleep(1)
        self.check("  with no job file looked at, the panel's section put back unread",
                   page.js(LOOKS) == looks, f"{looks} requests looking at them before, {page.js(LOOKS)} after")

        print("--- ended while a step is still setting the page up ---")
        page.js("document.querySelector('#reports-btn').click()")
        page.until("!document.querySelector('#reports').classList.contains('open')", "the panel rolled up")
        in_use = page.js(STATE)
        page.js("document.body.focus()")
        page.key("?", "Slash", 191, text="?")
        page.until("!document.querySelector('#shortcuts')?.classList.contains('hidden')", "the shortcut list")
        page.js("[...document.querySelectorAll('#shortcuts button')].find((b) => b.textContent === 'Give me a tour').click()")
        self.step_shown(0, total)
        page.key(*RIGHT)
        page.key(*RIGHT)
        page.key(*RIGHT)
        page.key(*ESCAPE)
        self.check("the overlay stays while the page is put back, and says so",
                   page.js("!document.querySelector('#tour') || "
                           "document.querySelector('#tour .tour-text').textContent === 'Putting the page back…'"))
        # A ? meanwhile neither reaches the page nor starts a second tour.
        page.key("?", "Slash", 191, text="?")
        page.until("!document.querySelector('#tour')", "Escape to end it")
        time.sleep(2)
        self.check("  and a key typed meanwhile opens nothing",
                   page.js("document.querySelector('#shortcuts').classList.contains('hidden')"))
        back = self.settle(in_use)
        self.check("no menu opens after the end, and the page is as it was",
                   back == in_use and not page.js("!!document.querySelector('.crumbmenu')"), f"before {in_use}\n        after  {back}")

        print("--- ended on a model's lineage ---")
        # Back on customers, at the 3 up and 2 down it was given above.
        page.js("document.querySelector('[data-mode=model]').click()")
        page.until(f"({LINEAGE}) === 'model.jaffle_shop.customers 3/2'", "customers at 3 up and 2 down")
        in_use = page.js(STATE)
        page.js("document.body.focus()")
        page.key("?", "Slash", 191, text="?")
        page.until("!document.querySelector('#shortcuts')?.classList.contains('hidden')", "the shortcut list")
        page.js("[...document.querySelectorAll('#shortcuts button')].find((b) => b.textContent === 'Give me a tour').click()")
        # On to the step opening its model, after the welcome and the badge.
        for i in range(2):
            self.step_shown(i, total)
            page.key(*RIGHT)
        self.step_shown(2, total)
        try:
            page.until(f"({LINEAGE}) === 'model.jaffle_shop.{MODEL} 1/1'", "the tour's model", timeout=10)
        except AssertionError:
            pass
        self.check(f"the tour draws {MODEL} at 1 up and 1 down",
                   page.js(LINEAGE) == f"model.jaffle_shop.{MODEL} 1/1", page.js(LINEAGE))
        page.key(*ESCAPE)
        page.until("!document.querySelector('#tour')", "Escape to end it")
        back = self.settle(in_use)
        try:
            page.until(f"({LINEAGE}) === 'model.jaffle_shop.customers 3/2'", "customers again", timeout=10)
        except AssertionError:
            pass
        self.check("its end draws customers again at 3 up and 2 down",
                   back == in_use and page.js(LINEAGE) == "model.jaffle_shop.customers 3/2",
                   f"before {in_use}\n        after  {back}, {page.js(LINEAGE)}")
        self.check("no error in the page", not page.errors, "; ".join(page.errors))
        return self.failed


def main():
    os.environ.setdefault("DBT_SEND_ANONYMOUS_USAGE_STATS", "False")
    os.environ.setdefault("DO_NOT_TRACK", "1")
    shots = None
    if "--shots" in sys.argv:
        shots = Path(sys.argv[sys.argv.index("--shots") + 1]).resolve()
        shots.mkdir(parents=True, exist_ok=True)
    binary = edith_binary()
    with tempfile.TemporaryDirectory(prefix="dbt-edith-tour-") as tmp:
        work = Path(tmp)
        # Named as the project is, so the tree's root reads jaffle_shop.
        project = work / "jaffle_shop"
        shutil.copytree(JAFFLE, project)
        (work / "home").mkdir()
        edith = Edith(binary, project, work)
        page = None
        try:
            page = Page(work, 2 if shots else 1)
            failed = Walk(page, work / "config", shots).run(edith.url, lambda: parse(project))
        finally:
            if page:
                page.close()
            edith.stop()
    if failed:
        print(f"{failed} check(s) failed")
        return 1
    print("the tour walks")
    return 0


if __name__ == "__main__":
    sys.exit(main())
