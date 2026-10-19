#!/usr/bin/env python3
"""The README's pictures, taken in headless Chrome on Jaffle Shop.

Jaffle Shop is copied from tests/fixtures/ and built by dbt on duckdb, so the
Catalog has its column types and the Run tab its SQL, then committed to a git
repository of its own. Its jobs are the invented dbt Cloud ones of
tests/fixtures/jobs/ and an invented Airflow DAG written here: nothing in a
picture comes from a real project (0014). dbt-edith serves it with a
configuration directory and a HOME of its own, collin writes its column
lineage once picked, and Chrome is driven through the DevTools client of
walk_tour.py.

Each picture is 1440 by 900. All six are taken first and only then copied
into docs/shots/, over the ones there, so a run that stops halfway leaves the
last set whole.
Retake them when a change alters what one shows: they are committed, so every
retake stays in the history, and a picture is worth retaking only when it no
longer looks like the tool.

Needs what walk_tour.py needs, and collin on PATH or COLLIN naming one;
without any of them it skips itself, exit 2. Run it from the Python that has
dbt, and the status bar names that Python's environment.
"""

import os
import shutil
import subprocess
import sys
import tempfile
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import walk_tour as wt  # noqa: E402  skips itself without dbt or Chrome

SHOTS = wt.ROOT / "docs" / "shots"
WIDTH, HEIGHT = 1440, 900

COLLIN = os.environ.get("COLLIN") or shutil.which("collin")
if not COLLIN:
    wt.skip("no collin on PATH. Install it, or set COLLIN to one.")

# An invented DAG, in the shape the Jobs tab proposes for Airflow.
DAG = """\
dag_id: jaffle_daily
schedule: "0 5 * * *"
tasks:
  - task_id: seed
    command: dbt seed
  - task_id: staging
    command: dbt run --select staging
    depends_on: [seed]
  - task_id: marts
    command: dbt build --select customers orders
    depends_on: [staging]
"""


def dbt(work, project, *args):
    """Runs dbt from `work`, so the duckdb file and the logs land beside the
    project rather than in its file tree."""
    args = list(args) + ["--project-dir", str(project), "--profiles-dir", str(project),
                         "--log-path", str(work / "logs")]
    if wt.EXECUTABLE:
        run = subprocess.run([wt.EXECUTABLE] + args + ["--quiet"], cwd=work, capture_output=True, text=True)
        if run.returncode != 0:
            raise RuntimeError(f"dbt {args[0]} failed: {(run.stdout + run.stderr)[-2000:]}")
        return
    here = os.getcwd()
    os.chdir(work)
    try:
        res = wt.dbtRunner().invoke(args + ["--log-level", "none"])
    finally:
        os.chdir(here)
    if not res.success:
        raise RuntimeError(f"dbt {args[0]} failed: {res.exception}")


def prepare(work):
    """The project as someone would have it after a day's work: committed,
    built, its catalog written, and its jobs in a folder beside it."""
    project = work / "jaffle_shop"
    shutil.copytree(wt.JAFFLE, project)
    (project / ".gitignore").write_text("target/\nlogs/\ndbt_packages/\n")
    env = dict(os.environ, GIT_CONFIG_GLOBAL=os.devnull, GIT_CONFIG_NOSYSTEM="1",
               GIT_AUTHOR_NAME="Jaffle Shop", GIT_AUTHOR_EMAIL="jaffle@example.com",
               GIT_COMMITTER_NAME="Jaffle Shop", GIT_COMMITTER_EMAIL="jaffle@example.com")
    for args in (["init", "-q", "-b", "main"], ["add", "-A"], ["commit", "-q", "-m", "Jaffle Shop"]):
        subprocess.run(["git", *args], cwd=project, env=env, check=True)
    # Built, then the catalog, then built again, so the last run Reports reads
    # is a build rather than the catalog's generation.
    dbt(work, project, "build")
    dbt(work, project, "docs", "generate")
    dbt(work, project, "build")
    jobs = work / "orchestration"
    shutil.copytree(wt.ROOT / "tests" / "fixtures" / "jobs" / "dbt_cloud", jobs / "dbt_cloud")
    (jobs / "dags").mkdir()
    (jobs / "dags" / "jaffle_daily.yml").write_text(DAG)
    (work / "home").mkdir()
    return project


class Shots:
    def __init__(self, page, out):
        self.page = page
        self.out = out
        self.taken = []

    def js(self, expr):
        return self.page.js(expr)

    def until(self, expr, what, timeout=15):
        return self.page.until(expr, what, timeout)

    def click(self, selector):
        self.js(f"document.querySelector({selector!r}).click()")

    def mouse(self, kind, x, y, **more):
        self.page.call("Input.dispatchMouseEvent", type=kind, x=x, y=y, **more)

    def rest(self):
        """The pointer on the status bar, where it opens no card."""
        self.mouse("mouseMoved", 1, HEIGHT - 1)

    def take(self, name):
        time.sleep(0.6)
        # A toast says what was just done, and leaves within a few seconds.
        self.until("!document.querySelector('#toast .toast')", "the toasts to go", 10)
        self.page.shot(self.out / f"{name}.png")
        self.taken.append(name)
        print(f"took {name}.png")

    def open_model(self, path, name):
        """Opens a model's file, pinned, from the tree, which moves the lineage
        and the Catalog onto it."""
        parts = path.split("/")[:-1]
        for i in range(len(parts)):
            sub = "/".join(parts[: i + 1])
            inside = f"!!document.querySelector('#tree .row[data-path^=\"{sub}/\"]')"
            if not self.js(inside):
                self.js(f"document.querySelector('#tree .row[data-path=\"{sub}\"]').click()")
                self.until(inside, sub)
        self.until(f"!!document.querySelector('#tree .row[data-path=\"{path}\"]')", path)
        self.js(f"document.querySelector('#tree .row[data-path=\"{path}\"]')"
                ".dispatchEvent(new MouseEvent('dblclick', { bubbles: true }))")
        self.until(f"document.querySelector('.cat-title')?.textContent.includes('{name}')", f"{name} in the Catalog")
        time.sleep(1)

    def dock_to(self, y):
        """Drags the line between the editor and the dock to `y`."""
        x, at = self.js("(b => [b.left + b.width / 2, b.top + b.height / 2])"
                        "(document.querySelector('#gutter-dock').getBoundingClientRect())")
        self.mouse("mousePressed", x, at, button="left", clickCount=1)
        step = -40 if y < at else 40
        for yy in range(int(at), y, step):
            self.mouse("mouseMoved", x, yy, button="left", buttons=1)
        self.mouse("mouseReleased", x, y, button="left", clickCount=1)

    def dock(self, tab):
        self.click(f"[data-dock={tab}]")
        time.sleep(0.5)

    def run(self, url):
        page = self.page
        page.call("Emulation.setDeviceMetricsOverride", width=WIDTH, height=HEIGHT, deviceScaleFactor=1, mobile=False)
        page.call("Page.navigate", url=url)
        self.until("document.querySelectorAll('#counts .chip').length > 0", "the page to load", 30)
        self.until("[...document.querySelectorAll('.tour-offer button')].some((b) => b.textContent === 'Not now')", "the tour's offer")
        self.js("[...document.querySelectorAll('.tour-offer button')].find((b) => b.textContent === 'Not now').click()")
        # Named through the route the sources form saves with.
        answer = self.js("""fetch('/api/jobs/sources', { method: 'PUT', headers: { 'content-type': 'application/json' },
          body: JSON.stringify({ sources: [
            { kind: 'airflow', path: '../orchestration/dags', label: 'airflow', keys: {} },
            { kind: 'dbt_cloud', path: '../orchestration/dbt_cloud', label: 'dbt cloud', keys: {} }] }) })
          .then((r) => r.json()).then((r) => r.jobs)""")
        if answer != 4:
            raise AssertionError(f"the job sources read {answer} jobs, not 4")

        # The editor: a ref() under the pointer, its card, and the lineage of
        # the file below.
        self.open_model("models/customers.sql", "customers")
        x, y = self.js("""(() => {
          const ref = [...document.querySelectorAll('#editor-host .cm-reflink')].find((e) => e.textContent.includes('stg_orders'));
          const b = ref.getBoundingClientRect();
          return [b.left + b.width / 2, b.top + b.height / 2];
        })()""")
        self.mouse("mouseMoved", x - 4, y)
        self.mouse("mouseMoved", x, y)
        self.until("!!document.querySelector('.hovercard')", "the hover card")
        self.take("editor")
        self.rest()

        # The lineage: the whole project as a Custom selection, by folder, its
        # tests hanging under their models.
        self.dock_to(170)
        self.click("[data-mode=select]")
        self.js("document.querySelector('#select-input').value = '+customers +orders'")
        self.js("document.querySelector('#select-input').dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter' }))")
        self.until("document.querySelector('#select-sum').textContent !== ''", "the selection")
        self.click("#by-folder")
        self.click("#with-tests")
        # By folder opens its levels box, which closes a moment after the
        # pointer leaves the button, as it does for a hand on the mouse.
        x, y = self.js("(b => [b.left + b.width / 2, b.top + b.height / 2])"
                       "(document.querySelector('#by-folder').getBoundingClientRect())")
        self.mouse("mouseMoved", x, y)
        self.rest()
        self.until("!document.querySelector('#folder-levels-box').classList.contains('open')", "the levels box to close", 10)
        self.click("#fit-btn")
        self.take("lineage")
        self.click("#by-folder")
        self.click("#with-tests")
        self.click("#selpanel-close")

        # Jobs: which of them run customers, and the one that cannot be told.
        self.dock_to(380)
        self.dock("jobs")
        self.until("document.querySelector('#jobs-head .jobs-about')?.textContent === 'customers'", "the Jobs tab on customers")
        self.until("document.querySelectorAll('#jobs-body .jobcard').length >= 2", "its jobs")
        self.js("document.querySelector('#jobs-body details > summary').click()")
        self.take("jobs")

        # Column lineage, from collin, picked in the top bar.
        self.dock_to(170)
        self.click("#cll-pick-host .cllpick")
        self.until("[...document.querySelectorAll('button')].some((b) => b.textContent.startsWith('Collin'))", "the menu")
        self.js("[...document.querySelectorAll('button')].find((b) => b.textContent.startsWith('Collin')).click()")
        self.until("/\\d+ col edges/.test(document.querySelector('#cll-pick-host').textContent)", "collin's run", 120)

        # The Catalog, its Columns tab with what collin found.
        self.dock_to(400)
        self.dock("catalog")
        self.js("[...document.querySelectorAll('.cat-tabs .ctab')].find((t) => t.textContent.startsWith('Columns')).click()")
        self.until("document.querySelectorAll('#catalog tr.c-linked').length > 0", "the columns")
        self.take("catalog")

        # One column's lineage on the canvas.
        self.dock_to(170)
        self.open_model("models/staging/stg_payments.sql", "stg_payments")
        self.dock("catalog")
        self.js("[...document.querySelectorAll('.cat-tabs .ctab')].find((t) => t.textContent.startsWith('Columns')).click()")
        self.until("[...document.querySelectorAll('#catalog tr.c-linked')].some((t) => t.querySelector('.c-name').textContent === 'amount')", "amount")
        self.js("[...document.querySelectorAll('#catalog tr.c-linked')].find((t) => t.querySelector('.c-name').textContent === 'amount').click()")
        self.dock("lineage")
        self.until("document.querySelector('#graph-mode .active')?.dataset.mode === 'column'", "the columns canvas")
        time.sleep(1)
        self.click("#fit-btn")
        self.rest()
        self.take("column-lineage")

        # collin's report, in the Reports panel.
        self.click("#reports-btn")
        self.until("document.querySelector('#reports').classList.contains('open')", "the Reports panel")
        self.js("[...document.querySelectorAll('#reports-nav button, #reports-nav a')].find((b) => b.textContent === \"collin's report\")?.click()")
        self.until("document.querySelector('#reports-view')?.textContent.includes(\"collin's report\")", "collin's report")
        self.take("collin-report")

        if page.errors:
            raise AssertionError("errors in the page: " + "; ".join(page.errors))


def main():
    os.environ.setdefault("DBT_SEND_ANONYMOUS_USAGE_STATS", "False")
    os.environ.setdefault("DO_NOT_TRACK", "1")
    # The environment dbt runs from, active as it would be for whoever starts
    # dbt-edith, so the status bar names it; and collin where it is looked for.
    path = [str(Path(COLLIN).parent)]
    if sys.prefix != sys.base_prefix:
        os.environ["VIRTUAL_ENV"] = sys.prefix
        path.insert(0, str(Path(sys.prefix) / "bin"))
    os.environ["PATH"] = os.pathsep.join(path + [os.environ.get("PATH", "")])
    binary = wt.edith_binary()
    with tempfile.TemporaryDirectory(prefix="dbt-edith-shots-") as tmp:
        work = Path(tmp)
        out = work / "shots"
        out.mkdir()
        project = prepare(work)
        edith = wt.Edith(binary, project, work)
        page = None
        try:
            page = wt.Page(work, 1)
            shots = Shots(page, out)
            shots.run(edith.url)
        finally:
            if page:
                page.close()
            edith.stop()
        SHOTS.mkdir(parents=True, exist_ok=True)
        for name in shots.taken:
            shutil.copyfile(out / f"{name}.png", SHOTS / f"{name}.png")
            print(f"wrote docs/shots/{name}.png")
    return 0


if __name__ == "__main__":
    sys.exit(main())
