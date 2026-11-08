#!/usr/bin/env python3
"""The pictures of docs/shots/, taken in headless Chrome on Jaffle Shop.

    take_shots.py [--scale N] [--theme T] [--out DIR] [NAME ...]

Every picture (NAMES below, one per feature) is taken on each run, in an order
that lets the state chain: the clean ones first, then the ones that stage
something. Only then are the NAMEs given, or all without any, copied over the
ones in docs/shots/ or in DIR, so a run that stops halfway leaves the last set
whole. --scale 2 takes the same 1440 by 900 CSS pixels at twice the device
pixels, and wants --out. --theme (night, graphite, light or halloween) takes
them in another theme, which look.json in the configuration directory says
before the server starts, the apparitions off; it wants --out too, the pictures
of docs/shots being Night's. An unknown NAME fails before anything starts.

Jaffle Shop is copied from tests/fixtures/ and built by dbt on duckdb, so the
Catalog has its column types and the Run tab its SQL, then committed to a git
repository of its own. Its jobs are the invented dbt Cloud ones of
tests/fixtures/jobs/ and invented Airflow DAGs written here: nothing in a
picture comes from a real project (0014). dbt-edith serves it with a
configuration directory and a HOME of its own, collin writes its column
lineage once picked, and Chrome is driven through the DevTools client of
walk_tour.py.

What is staged, and when, so that no picture shows what belongs to another:

  - .env.dev, .env.qa and .env.prod, for the two Catalog pictures of
    environments, and the page loaded again to find them; the Catalog goes back
    to the manifest's own environment after them and the files are removed;
  - a remote a commit ahead and files to commit, for the Git tab, undone after;
  - a start with the update check on, from a copy of the binary in a cargo-like
    `bin/` under /tmp, built from a clone kept in a folder of its own under
    TAKE_SHOTS_CACHE (default /tmp/dbt-edith-shots-cache; refused if not empty
    and not made by this script), so that the update menu offers its command
    and every path it shows is a temporary one; a start without the check
    follows. The copy is built only when `updates` is among the names to copy
    (or none are given), and compiled again only when a source changed;
  - a second DAG, jaffle_refunds, for the order between jobs, once the coverage
    is taken; it stays, and is a row of Builds by job on purpose;
  - Snowflake: a stub of the dbt_artifacts package, a Snowflake target in the
    profile (dbt is told to build on duckdb, so it keeps working), .env.dev and
    .env.prod again, and the features switched on, with `snowflake.connector`
    replaced by the fake of fake_snowflake_connector.py, put first on the
    server's PYTHONPATH, so no warehouse is reached; all of it undone after;
  - an edit to a model dbt has not parsed, last, for the manifest's pill.

Nothing personal may show: the shell runs with a HOME of its own and a neutral
prompt, the work is done under /tmp, the Python environment is named by a link
there, and the update list is written here, with
a repository of local tags standing in for GitHub in that HOME's git
configuration, so the run never reaches it. A picture does show the clock of
the run (the manifest's time, a query's).
Retake them when a change alters what one shows: they are committed, so every
retake stays in the history, and a picture is worth retaking only when it no
longer looks like the tool.

Needs what walk_tour.py needs, and collin on PATH or COLLIN naming one;
without any of them it skips itself, exit 2. Run it from the Python that has
dbt, and the status bar names that Python's environment. Its pictures of
failures: TAKE_SHOTS_FAILED=/path/failed.png saves the page when a step gives
up, and what was taken, beside it.
"""

import argparse
import fcntl
import json
import os
import re
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

# In the order they are taken, which is the order the page's state allows.
NAMES = [
    "editor", "editor-project-keys", "lineage", "lineage-export", "jobs", "catalog", "column-lineage", "collin-report",
    "search-palette", "search-files", "lineage-node", "selection-card",
    "reports-coverage", "reports-job-order",
    "catalog-location", "catalog-compiled", "terminal", "git",
    "updates", "shortcuts", "themes",
    "snowflake-history", "reports-builds", "catalog-snowflake", "catalog-clone",
    "manifest-pill",
]

COLLIN = os.environ.get("COLLIN") or shutil.which("collin")
if not COLLIN:
    wt.skip("no collin on PATH. Install it, or set COLLIN to one.")

# An invented DAG, in the shape the Jobs tab proposes for Airflow.
DAG = """\
dag_id: jaffle_daily
schedule: "0 5 * * *"
tasks:
  - task_id: seed
    command: dbt seed --select raw_orders raw_payments
  - task_id: staging
    command: dbt run --select stg_orders stg_payments
    depends_on: [seed]
  - task_id: marts
    command: dbt build --select customers orders
    depends_on: [staging]
"""

# A DAG that rebuilds orders half an hour after the others, with nothing
# making it wait for the staging models its parents are.
DAG_LATE = """\
dag_id: jaffle_refunds
schedule: "30 6 * * *"
tasks:
  - task_id: rebuild_orders
    command: dbt run --select orders
"""


def dbt(work, project, *args):
    """Runs dbt from `work`, so the duckdb file and the logs land beside the
    project rather than in its file tree."""
    # Always the duckdb target: once the profile also names a Snowflake one,
    # it is the one the server sees by default (stage_snowflake).
    args = list(args) + ["--project-dir", str(project), "--profiles-dir", str(project),
                         "--log-path", str(work / "logs"), "--target", "dev"]
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


def git(cwd, *args):
    """git with no configuration but the author, so no machine's own leaks
    into a repository made here, and the author is an invented one."""
    env = dict(os.environ, GIT_CONFIG_GLOBAL=os.devnull, GIT_CONFIG_NOSYSTEM="1",
               GIT_AUTHOR_NAME="Jaffle Shop", GIT_AUTHOR_EMAIL="jaffle@example.com",
               GIT_COMMITTER_NAME="Jaffle Shop", GIT_COMMITTER_EMAIL="jaffle@example.com")
    return subprocess.run(["git", *args], cwd=cwd, env=env, check=True, capture_output=True, text=True).stdout


def next_releases():
    """The two releases after the one this checkout is, as the update pill is
    to show them: Cargo.toml's minor plus one and plus two, newest first, each
    with the invented title its pull request would have."""
    major, minor, _ = re.search(r'^version = "(\d+)\.(\d+)\.(\d+)"', (wt.ROOT / "Cargo.toml").read_text(), re.M).groups()
    titles = ["Keep the rule that built a column on its card", "Name the folders a job step leaves out"]
    return [(f"{major}.{int(minor) + 2}.0", titles[1]), (f"{major}.{int(minor) + 1}.0", titles[0])]


def prepare(work):
    """The project as someone would have it after a day's work: committed,
    built, its catalog written, and its jobs in a folder beside it."""
    project = work / "jaffle_shop"
    shutil.copytree(wt.JAFFLE, project)
    (project / ".gitignore").write_text("target/\nlogs/\ndbt_packages/\n.env.*\n")
    # Every model's database comes from the environment, so that the Catalog
    # has places to tell apart once the .env files are there. Unset, it is
    # the database duckdb builds in.
    config = project / "dbt_project.yml"
    text = config.read_text()
    old = "    +materialized: table\n    staging:"
    if old not in text:
        raise AssertionError("dbt_project.yml of the fixture has changed")
    config.write_text(text.replace(old, "    +materialized: table\n"
                                        "    +database: \"{{ env_var('JAFFLE_DB', 'jaffle_shop') }}\"\n"
                                        "    +schema: main\n    staging:", 1))
    # A schema written out takes the macro every project that does so has,
    # or dbt builds it as main_main; and the relations of the compiled SQL
    # can then be written for an environment, which they cannot with a schema
    # left to the target.
    (project / "macros").mkdir()
    (project / "macros" / "generate_schema_name.sql").write_text(
        "{% macro generate_schema_name(custom_schema_name, node) -%}\n"
        "    {{ custom_schema_name | trim if custom_schema_name is not none else target.schema }}\n"
        "{%- endmacro %}\n")
    # The fixture's own README talks of this repository's scripts: this
    # project has one of its own, with short lines.
    (project / "README.md").write_text("# Jaffle Shop\n\nCustomers, orders and payments of a small shop.\n")
    git(project, "init", "-q", "-b", "main")
    git(project, "add", "-A")
    git(project, "commit", "-q", "-m", "Jaffle Shop")
    readme = project / "README.md"
    for more, message in (("\n## Building\n\nRun `dbt build` once the seeds are loaded.\n", "Say how to build the project"),
                          ("\n## Tests\n\nEvery model has its keys tested in schema.yml.\n", "Describe the tests")):
        readme.write_text(readme.read_text() + more)
        git(project, "add", "README.md")
        git(project, "commit", "-q", "-m", message)
    # Built, then the catalog, then built again, so the last run Reports reads
    # is a build rather than the catalog's generation.
    dbt(work, project, "build")
    dbt(work, project, "docs", "generate")
    dbt(work, project, "build")
    jobs = work / "orchestration"
    shutil.copytree(wt.ROOT / "tests" / "fixtures" / "jobs" / "dbt_cloud", jobs / "dbt_cloud")
    (jobs / "dags").mkdir()
    (jobs / "dags" / "jaffle_daily.yml").write_text(DAG)

    # The HOME the server and its shell run with: a prompt that names the
    # folder alone, git without a pager, and an author. The release tags of
    # dbt-edith are served from a repository here, so that a look the page
    # makes at GitHub, on a turn of the hour for instance, ends up on this
    # disk.
    home = work / "home"
    home.mkdir()
    (home / ".zshrc").write_text("PROMPT='%1~ %# '\nRPROMPT=''\nunsetopt prompt_sp\nexport GIT_PAGER=cat\n")
    for name in (".bash_profile", ".bashrc"):
        (home / name).write_text("PS1='\\W \\$ '\nexport GIT_PAGER=cat\n")
    fake = work / "fake-edith.git"
    src = work / "fake-edith"
    src.mkdir()
    git(src, "init", "-q", "-b", "main")
    for version, title in reversed(next_releases()):
        (src / "VERSION").write_text(version + "\n")
        git(src, "add", "-A")
        git(src, "commit", "-q", "-m", title)
        git(src, "tag", f"v{version}")
    git(work, "clone", "-q", "--bare", str(src), str(fake))
    (home / ".gitconfig").write_text(
        f'[user]\n\tname = Jaffle Shop\n\temail = jaffle@example.com\n'
        f'[url "file://{fake}"]\n\tinsteadOf = https://github.com/IL-William/dbt-edith\n')
    return project


ENVS = {"dev": "jaffle_shop", "qa": "jaffle_shop_qa", "prod": "jaffle_shop_prod"}


def stage_envs(project, names=tuple(ENVS)):
    """The .env files someone keeps beside dbt_project.yml, one per
    environment, written when the page is to show them: gitignored, so the Git
    tab stays about the work. Dev is the database the project was built in."""
    for name in names:
        (project / f".env.{name}").write_text(f"DBT_TARGET={name}\nJAFFLE_DB={ENVS[name]}\n")


def remove_envs(project):
    for env in project.glob(".env.*"):
        env.unlink()


def stage_git(work, project):
    """A remote with a commit the project lacks, and a working tree with
    something in it: a note edited, another not yet added. The remote is
    named as GitHub would be, and a url.insteadOf in the project's own
    configuration sends it to a bare repository here (0121 allows the key)."""
    # Notes in a folder of their own, which the Git tab names by its folder
    # where a file at the root is named twice, and clipped. Made now, and not
    # with the project, so that no earlier picture has the folder in its tree.
    (project / "docs").mkdir()
    (project / "docs" / "building.md").write_text("# Building\n\nRun `dbt build` once the seeds are loaded.\n")
    git(project, "add", "docs")
    git(project, "commit", "-q", "-m", "Add the notes")
    bare = work / "origin.git"
    git(work, "init", "-q", "--bare", "-b", "main", str(bare))
    git(project, "remote", "add", "origin", "https://github.com/example/jaffle_shop.git")
    git(project, "config", f"url.{bare}.insteadOf", "https://github.com/example/jaffle_shop.git")
    git(project, "push", "-q", "-u", "origin", "main")
    other = work / "colleague"
    git(work, "clone", "-q", str(bare), str(other))
    (other / "README.md").write_text((other / "README.md").read_text() + "\n## Contributing\n\nOpen a pull request.\n")
    git(other, "commit", "-q", "-am", "Say how to contribute")
    git(other, "push", "-q", "origin", "main")
    git(project, "fetch", "-q", "origin")
    building = project / "docs" / "building.md"
    building.write_text(building.read_text() + "\nThe orders model counts a refund as a negative payment.\n")
    (project / "docs" / "todo.md").write_text("- add a test on the payment methods\n")


def stage_update(config):
    """What GitHub would have answered, written as the page's own look would
    have, so that a start with the check on finds it fresh and asks nothing."""
    config.mkdir(exist_ok=True)
    releases = next_releases()
    seen = {"checked": int(time.time()), "latest": releases[0][0],
            "releases": [v for v, _ in releases], "titles": {v: t for v, t in releases}}
    (config / "update.json").write_text(json.dumps(seen, indent=2))


def write_fake_snowflake(work):
    """snowflake/connector on a path of its own, for the server's PYTHONPATH:
    the invented one of scripts/fake_snowflake_connector.py, which no warehouse
    is behind."""
    package = work / "fake_sf" / "snowflake"
    (package / "connector").mkdir(parents=True)
    (package / "__init__.py").write_text("")
    shutil.copyfile(Path(__file__).with_name("fake_snowflake_connector.py"), package / "connector" / "__init__.py")
    return work / "fake_sf"


def stage_snowflake(work, project):
    """The project as a Snowflake one would be: a stub of the dbt_artifacts
    package (its two tables are all dbt-edith asks for), placed in a database
    each environment names, and a Snowflake target in the profile, which is
    the one the server sees while dbt is told to build on duckdb. Parsed
    again so the manifest holds the package, and committed, so the Git tab has
    nothing of it. Written when the shots reach Snowflake, which the earlier
    ones would show in their trees and counts."""
    stub = work / "dbt_artifacts_stub"
    (stub / "models").mkdir(parents=True)
    (stub / "dbt_project.yml").write_text(
        "name: dbt_artifacts\nversion: '2.0.0'\nconfig-version: 2\nmodel-paths: ['models']\n")
    for table in ("model_executions", "invocations"):
        (stub / "models" / f"{table}.sql").write_text("select 1 as x\n")
    (project / "packages.yml").write_text("packages:\n  - local: ../dbt_artifacts_stub\n")
    config = project / "dbt_project.yml"
    config.write_text(config.read_text().rstrip("\n") + "\n"
                      "  dbt_artifacts:\n"
                      "    +database: \"{{ env_var('JAFFLE_DB', 'jaffle_shop') }}\"\n"
                      "    +schema: dbt_artifacts\n")
    (project / "profiles.yml").write_text(
        "jaffle_shop:\n"
        "  target: snowflake\n"
        "  outputs:\n"
        "    dev:\n"
        "      type: duckdb\n"
        "      path: 'jaffle_shop.duckdb'\n"
        "      threads: 24\n"
        "    snowflake:\n"
        "      type: snowflake\n"
        "      account: xy12345\n"
        "      user: analyst@example.com\n"
        "      role: TRANSFORMER\n"
        "      warehouse: TRANSFORMING\n"
        "      database: ANALYTICS\n"
        "      schema: DBT_JAFFLE\n"
        "      authenticator: externalbrowser\n")
    dbt(work, project, "deps")
    dbt(work, project, "parse")
    git(project, "add", "-A")
    git(project, "commit", "-q", "-m", "Track the builds with dbt_artifacts")
    # Two environments to read the builds of: a third would push the last
    # column of Builds by job off the pane.
    stage_envs(project, ("dev", "prod"))


def unstage_snowflake(work, project):
    """The project as it was before stage_snowflake: its commit dropped, the
    package's folder removed, and parsed again, so that the last picture has
    the manifest's counts and the tree of the others."""
    git(project, "reset", "-q", "--hard", "HEAD~1")
    shutil.rmtree(project / "dbt_packages", ignore_errors=True)
    remove_envs(project)
    dbt(work, project, "parse")


MARKER = ".take-shots-clone"


def own(path):
    """Refuses a link, or a folder someone else owns, and makes it this user's
    alone (0700): the copy is made with rsync --delete, a built binary comes
    out of it, and the copy of the checkout is no one else's to read."""
    st = path.lstat()
    if path.is_symlink() or st.st_uid != os.getuid():
        raise RuntimeError(f"{path} is a link or is not owned by this user: not used")
    os.chmod(path, 0o700)


def changed_for_build(line):
    """Whether one `rsync --itemize-changes` line is a change cargo must see.
    GNU rsync 3 prints 11 flag characters and openrsync 9, so the name is what
    follows the first space. A build.rs with only its time different is not one:
    touching it is what makes the time differ, and every run would compile."""
    if line.startswith("*deleting"):
        return True
    if not line.startswith((">f", "cf")) or " " not in line:
        return False
    flags, name = line.split(" ", 1)
    return not (name.lstrip() == "build.rs" and "+" not in flags and flags[2:4] == "..")


def temp_install(work, cache):
    """A copy of this checkout's dbt-edith, built from a clone under `cache`
    and put in a cargo-like `bin/` under `work`, so that the update menu finds
    what it needs to offer its command (src/update.rs `how`): a binary cargo
    installed, built from a clone still on the machine. The clone is where
    the binary says it was built, so it is a temporary one too, not this
    checkout, and the page shows no path of the machine's own. It is kept
    between runs with its build, in a folder of its own that carries a marker
    this script wrote: one that is not empty and has none is refused, since the
    copy is made with rsync --delete. A lock keeps two runs from building in
    it at once. The build is told its own target directory, so no
    CARGO_TARGET_DIR or build.target-dir can send it to, or over, the build of
    the main binary. Built only when the update picture is to be copied, and
    compiled again only when rsync changed a file. It fails the run when it
    cannot be made: the picture would say there is no command."""
    clone = cache / "take-shots-dbt-edith"
    try:
        cache.mkdir(mode=0o700, parents=True, exist_ok=True)
        own(cache)
        with open(cache / "take-shots.lock", "w") as lock:
            fcntl.flock(lock, fcntl.LOCK_EX)
            if clone.exists() and any(clone.iterdir()) and not (clone / MARKER).exists():
                raise RuntimeError(f"{clone} is not empty and was not made by this script: not touched")
            clone.mkdir(mode=0o700, exist_ok=True)
            own(clone)
            (clone / MARKER).write_text("A copy of dbt-edith for scripts/take_shots.py; emptied and refilled by it.\n")
            copied = subprocess.run(
                ["rsync", "-a", "--delete", "--itemize-changes", "--exclude=/.git", "--exclude=/target",
                 "--exclude=/docs/shots", f"--exclude=/{MARKER}", f"{wt.ROOT}/", f"{clone}/"],
                check=True, capture_output=True, text=True).stdout
            # rsync copies the checkout's folder mode to the clone's, and openrsync
            # (macOS) ignores --chmod: own() is what keeps the clone private.
            own(clone)
            if not (clone / ".git").exists():
                git(clone, "init", "-q")
            # The build stamp is made by build.rs, which cargo runs again only
            # when it changes: touched when a file did.
            # (Not build.rs with only its time put back: touching it is what
            # makes that time differ, and every run would then compile.)
            changed = [line for line in copied.splitlines() if changed_for_build(line)]
            if changed:
                (clone / "build.rs").touch()
            print(f"building the copy of dbt-edith the update menu is shown from, in {clone} "
                  "(a few minutes the first time)", flush=True)
            subprocess.run(["cargo", "build", "--quiet", "--target-dir", str(clone / "target")],
                           cwd=clone, check=True, capture_output=True)
            root = work / "cargo"
            (root / "bin").mkdir(parents=True)
            (root / ".crates.toml").write_text("")
            shutil.copy2(clone / "target" / "debug" / "dbt-edith", root / "bin" / "dbt-edith")
        return root / "bin" / "dbt-edith"
    except (OSError, RuntimeError, subprocess.CalledProcessError) as e:
        if isinstance(e, subprocess.CalledProcessError):
            why = (e.stderr if isinstance(e.stderr, str) else (e.stderr or b"").decode(errors="replace"))[-1500:]
        else:
            why = str(e)
        raise RuntimeError(f"no installed copy for the update menu: {why}") from e


class Shots:
    def __init__(self, page, out, work, project, binary, scale, installed=None):
        self.page = page
        self.out = out
        self.work, self.project, self.binary, self.scale = work, project, binary, scale
        self.installed = installed
        self.edith = None
        self.taken = []

    def js(self, expr):
        return self.page.js(expr)

    def until(self, expr, what, timeout=15):
        return self.page.until(expr, what, timeout)

    def click(self, selector):
        self.js(f"document.querySelector({selector!r}).click()")

    def mouse(self, kind, x, y, **more):
        self.page.call("Input.dispatchMouseEvent", type=kind, x=x, y=y, **more)

    def frame(self):
        """A frame made. Headless Chrome makes none until one is asked for, and
        what the page does on a frame (a dragged edge taking its place, the
        terminal drawing) waits for it: a cheap capture asks."""
        self.page.call("Page.captureScreenshot", format="jpeg", quality=1)

    def front(self):
        """Headless Chrome may call its tab hidden, and a hidden page stops
        asking for the git status and the freshness, and draws no terminal."""
        self.page.call("Page.bringToFront")

    def centre(self, selector):
        return self.js(f"(b => [b.left + b.width / 2, b.top + b.height / 2])(document.querySelector({selector!r}).getBoundingClientRect())")

    def hover(self, selector):
        self.until(f"!!document.querySelector({selector!r})", selector)
        x, y = self.centre(selector)
        self.mouse("mouseMoved", x - 4, y)
        self.mouse("mouseMoved", x, y)

    def press(self, selector):
        """A click with a pointer, for what listens for mouse events: the
        lineage's boxes."""
        self.until(f"!!document.querySelector({selector!r})", selector)
        time.sleep(0.8)
        x, y = self.centre(selector)
        self.mouse("mouseMoved", x, y)
        self.mouse("mousePressed", x, y, button="left", clickCount=1)
        self.mouse("mouseReleased", x, y, button="left", clickCount=1)

    def rest(self):
        """The pointer on the status bar, where it opens no card."""
        self.mouse("mouseMoved", 1, HEIGHT - 1)

    def take(self, name):
        time.sleep(0.6)
        # A toast says what was just done, and leaves within a few seconds.
        self.until("!document.querySelector('#toast .toast')", "the toasts to go", 10)
        # Nothing selected: a drag that missed its handle selects text.
        self.js("window.getSelection().removeAllRanges()")
        self.page.shot(self.out / f"{name}.png")
        self.taken.append(name)
        print(f"took {name}.png")

    def open_model(self, path, name):
        """Opens a model's file, pinned, from the tree, which moves the lineage
        and the Catalog onto it."""
        parts = path.split("/")[:-1]
        for i in range(len(parts)):
            sub = "/".join(parts[: i + 1])
            # Open, not merely listed: a folder folded again keeps its rows.
            inside = f"document.querySelector('#tree .row[data-path=\"{sub}\"]')?.classList.contains('open') === true"
            if not self.js(inside):
                self.js(f"document.querySelector('#tree .row[data-path=\"{sub}\"]').click()")
                self.until(inside, sub)
        self.until(f"!!document.querySelector('#tree .row[data-path=\"{path}\"]')", path)
        self.js(f"document.querySelector('#tree .row[data-path=\"{path}\"]')"
                ".dispatchEvent(new MouseEvent('dblclick', { bubbles: true }))")
        self.until(f"document.querySelector('.cat-title')?.textContent.includes('{name}')", f"{name} in the Catalog")
        time.sleep(1)

    def drag(self, selector, to=None, by=None, framed=False):
        """Drags what is at `selector` to a point, or by an offset, as a hand
        does: pressed, moved in steps and released, and a frame made after it
        if `framed`, for a page that acts on the release on one."""
        x, y = self.centre(selector)
        # A panel still rolling up covers the handle, and a drag started on it
        # selects its text instead of moving the line.
        self.until(f"(e => !!e && !!e.closest({selector!r}))(document.elementFromPoint({x}, {y}))", f"{selector} under the pointer")
        tx, ty = to if to else (x + by[0], y + by[1])
        steps = int(max(abs(tx - x), abs(ty - y)) // 30) + 1
        self.mouse("mousePressed", x, y, button="left", clickCount=1)
        for i in range(1, steps):
            self.mouse("mouseMoved", x + (tx - x) * i / steps, y + (ty - y) * i / steps, button="left", buttons=1)
        self.mouse("mouseReleased", tx, ty, button="left", clickCount=1)
        if framed:
            self.frame()

    def dock_to(self, y):
        """Drags the line between the editor and the dock to `y`."""
        self.drag("#gutter-dock", to=(self.centre("#gutter-dock")[0], y))

    def side_to(self, x):
        """Drags the line between the sidebar and the editor to `x`."""
        self.drag("#gutter-side", to=(x, self.centre("#gutter-side")[1]))

    def reload(self):
        """The page loaded again, on the model it starts from."""
        self.load(self.edith.url)
        self.open_model("models/customers.sql", "customers")

    def reports(self, key, text):
        """A section of Reports, once its panel is down and it says `text`."""
        self.click("#space-reports")
        self.until("document.querySelector('#reports').classList.contains('open')", "the Reports panel")
        self.js(f"document.querySelector('#reports-nav [data-key={key}]').click()")
        self.until(f"document.querySelector('#reports-view')?.textContent.includes({text!r})", f"{key} in Reports")

    def features(self, on):
        """Snowflake's features on or off for the project, as the switch in the
        settings does, and the page loaded again."""
        status = self.js(f"""fetch('/api/features', {{ method: 'POST', headers: {{ 'content-type': 'application/json',
          authorization: 'Bearer ' + localStorage.getItem('dbt-edith-key') }},
          body: JSON.stringify({{ snowflake: {str(on).lower()} }}) }}).then((r) => r.status)""")
        if status != 200:
            raise AssertionError(f"/api/features answered {status}")
        self.reload()
        if on:
            # The manifest just parsed and its files just committed: the pill
            # is red until the page has asked again, and the pictures wait.
            for _ in range(100):
                self.js("document.dispatchEvent(new Event('visibilitychange'))")
                if self.js("document.querySelector('#fresh-badge').dataset.tone === 'ok'"):
                    break
                time.sleep(0.3)
            else:
                raise AssertionError("the manifest's pill did not turn green")

    def env_to(self, name):
        """The environment the Catalog's Location table is resolved with, by
        its menu: a .env file's name, or `manifest`."""
        self.dock("catalog")
        self.catalog_tab("Preview")
        self.until("!!document.querySelector('#catalog table.loc .envpill')", "the environment switch")
        self.click("#catalog table.loc .envpill")
        self.until("!!document.querySelector('.envmenu')", "the environment menu")
        self.js(f"[...document.querySelectorAll('.envmenu button')].find((b) => b.querySelector('.lbl').textContent.toLowerCase() === {name!r}).click()")
        self.until(f"document.querySelector('#catalog table.loc .envpill').textContent.toLowerCase().startsWith({name!r})", f"{name} picked")

    def dock(self, tab):
        self.click(f"[data-dock={tab}]")
        time.sleep(0.5)

    def catalog_tab(self, name):
        self.js(f"[...document.querySelectorAll('.cat-tabs .ctab')].find((t) => t.textContent.startsWith({name!r})).click()")

    def code(self):
        """Back to the editor from a panel; a tab never toggles. Rolled up, not
        rolling: a panel still covers the dock's edge while it goes."""
        self.click("#space-code")
        self.until("['#reports', '#settings', '#select-panel'].every((s) => !document.querySelector(s)"
                   " || getComputedStyle(document.querySelector(s)).visibility === 'hidden')", "the editor", 20)
        time.sleep(0.4)

    def selection(self, text):
        self.click("[data-mode=select]")
        self.js(f"document.querySelector('#select-input').value = {text!r}")
        self.js("document.querySelector('#select-input').dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter' }))")
        self.until("document.querySelector('#select-sum').textContent !== ''", "the selection")

    def load(self, url):
        self.page.call("Page.navigate", url=url)
        self.front()
        self.until("document.querySelectorAll('#counts .chip').length > 0", "the page to load", 30)

    def run(self, edith):
        self.edith = edith
        page = self.page
        page.call("Emulation.setDeviceMetricsOverride", width=WIDTH, height=HEIGHT, deviceScaleFactor=self.scale, mobile=False)
        page.call("Page.navigate", url=edith.url)
        self.front()
        self.until("document.querySelectorAll('#counts .chip').length > 0", "the page to load", 30)
        self.until("[...document.querySelectorAll('.tour-offer button')].some((b) => b.textContent === 'Not now')", "the tour's offer")
        self.js("[...document.querySelectorAll('.tour-offer button')].find((b) => b.textContent === 'Not now').click()")
        # Named through the route the sources form saves with, in the settings
        # so that no file is written into the project.
        # The page keeps its key in this origin's storage once an answer
        # has come (0122), and the route wants it as a header.
        answer = self.js("""fetch('/api/jobs/sources', { method: 'PUT', headers: { 'content-type': 'application/json',
          authorization: 'Bearer ' + localStorage.getItem('dbt-edith-key') },
          body: JSON.stringify({ place: 'settings', sources: [
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

        # The violet button at the end of the breadcrumb, and the lines of
        # dbt_project.yml that reach a staging model: one the folder above
        # sets is struck where the staging folder sets it again.
        self.open_model("models/staging/stg_payments.sql", "stg_payments")
        self.click("#crumbs .cfg")
        self.until("document.querySelectorAll('.crumbmenu.config .yl').length > 0", "the keys dialog")
        self.take("editor-project-keys")
        self.page.key(*wt.ESCAPE)
        self.until("!document.querySelector('.crumbmenu')", "the dialog to close")
        # As a person leaves it: the tab closed and the folder folded again,
        # so the shots below are what a retake without this one would give,
        # with the model they follow, customers, in front.
        self.js("document.querySelector('#tabbar .ftab[data-key=\"models/staging/stg_payments.sql\"] .x').click()")
        self.js("document.querySelector('#tree .row[data-path=\"models/staging\"]').click()")
        self.until("!document.querySelector('#tree .row[data-path=\"models/staging\"]').classList.contains('open')", "staging to fold")
        self.open_model("models/customers.sql", "customers")

        # The lineage: the whole project as a selection in the Lineage space,
        # by folder, its tests hanging under their models.
        self.dock_to(170)
        self.selection("+customers +orders")
        self.click("#by-folder")
        self.click("#with-tests")
        # By folder opens its levels box, which closes a moment after the
        # pointer leaves the button, as it does for a hand on the mouse.
        x, y = self.centre("#by-folder")
        self.mouse("mouseMoved", x, y)
        self.rest()
        # The pointer's leaving is heard on a move: moved again until it is.
        for i in range(60):
            if not self.js("document.querySelector('#folder-levels-box').classList.contains('open')"):
                break
            self.mouse("mouseMoved", 1 + i % 5, HEIGHT - 1 - i % 3)
            time.sleep(0.4)
        else:
            raise AssertionError("timed out waiting for the levels box to close")
        self.click("#fit-btn")
        self.take("lineage")
        # The export menu over the same canvas, shut again for what follows.
        self.click("#export-btn")
        self.until("!!document.querySelector('.exportmenu')", "the export menu")
        self.take("lineage-export")
        self.page.key(*wt.ESCAPE)
        self.until("!document.querySelector('.exportmenu')", "the export menu to close")
        self.click("#by-folder")
        self.click("#with-tests")
        self.code()

        # Jobs: which of them run customers, and the one that cannot be told.
        self.dock_to(380)
        self.dock("jobs")
        self.until("document.querySelector('#jobs-head .jobs-about')?.textContent === 'customers'", "the Jobs tab on customers")
        self.until("document.querySelectorAll('#jobs-body .jobcard').length >= 2", "its jobs")
        self.js("document.querySelector('#jobs-body details > summary').click()")
        self.take("jobs")

        # Column lineage, from collin, picked in the settings.
        self.dock_to(170)
        self.click("#settings-btn")
        self.until("document.querySelector('#settings').classList.contains('open')", "the settings")
        self.js("[...document.querySelectorAll('#settings-nav button')].find((b) => b.textContent.startsWith('Column lineage')).click()")
        collin = "[...document.querySelectorAll('#settings-view [data-live=columns] button')].find((b) => b.textContent.startsWith('Collin'))"
        self.until(f"!!{collin}", "the Column lineage section")
        self.js(f"{collin}.click()")
        self.until("/\\d+ col edges/.test(document.querySelector('#settings-view .cllnow').textContent)", "collin's run", 120)
        self.code()

        # The Catalog, its Columns tab with what collin found.
        self.dock_to(400)
        self.dock("catalog")
        self.catalog_tab("Columns")
        self.until("document.querySelectorAll('#catalog tr.c-linked').length > 0", "the columns")
        self.take("catalog")

        # One column's lineage on the canvas.
        self.dock_to(170)
        self.open_model("models/staging/stg_payments.sql", "stg_payments")
        self.dock("catalog")
        self.catalog_tab("Columns")
        self.until("[...document.querySelectorAll('#catalog tr.c-linked')].some((t) => t.querySelector('.c-name').textContent === 'amount')", "amount")
        self.js("[...document.querySelectorAll('#catalog tr.c-linked')].find((t) => t.querySelector('.c-name').textContent === 'amount').click()")
        self.dock("lineage")
        self.until("document.querySelector('#graph-mode .active')?.dataset.mode === 'column'", "the columns canvas")
        time.sleep(1)
        self.click("#fit-btn")
        self.rest()
        self.take("column-lineage")

        # collin's report, in the Reports panel.
        self.reports("collin", "collin's report")
        self.take("collin-report")

        # Search by name, from anywhere: models, a seed and files. The editor
        # tall and the dock on a model, so the page behind is the usual one.
        self.code()
        self.dock_to(520)
        self.click("[data-mode=model]")
        time.sleep(0.5)
        self.click("#palette-btn")
        self.until("!document.querySelector('#palette').classList.contains('hidden')", "the palette")
        self.js("(i => { i.value = 's'; i.dispatchEvent(new Event('input', { bubbles: true })); })(document.querySelector('#palette-input'))")
        self.until("document.querySelectorAll('#palette-results .pal-row').length > 8", "the palette's answer")
        time.sleep(0.5)
        self.rest()
        self.take("search-palette")
        self.page.key(*wt.ESCAPE)
        self.until("document.querySelector('#palette').classList.contains('hidden')", "the palette to close")

        # Search inside the files, in the sidebar.
        self.click("[data-side=search]")
        self.js("(i => { i.value = 'amount'; i.dispatchEvent(new Event('input', { bubbles: true })); })(document.querySelector('#grep-input'))")
        self.until("document.querySelectorAll('#grep-results .ghit').length > 3", "the search's hits")
        self.rest()
        self.take("search-files")
        self.click("[data-side=files]")

        # A model's lineage two levels up and one down, a box selected, the
        # pointer at rest.
        self.dock_to(260)
        self.open_model("models/orders.sql", "orders")
        self.dock("lineage")
        self.click("[data-mode=model]")
        self.js("for (const [id, v] of [['up', '2'], ['down', '1']]) {"
                " const el = document.querySelector('#' + id); el.value = v; el.dispatchEvent(new Event('change')); }")
        self.until("document.querySelectorAll('#graph .nd').length >= 5", "the model's lineage")
        time.sleep(1)
        # One box selected by a click.
        self.press("#graph .nd[data-id$='.orders']")
        self.until("!!document.querySelector('#graph .nd.sel')", "the box selected")
        # The pointer put at rest, so its card goes and the toolbar is clear.
        self.rest()
        self.until("!document.querySelector('.hovercard')", "the card to go")
        self.take("lineage-node")

        # A named selector, three boxes in a column that leave the canvas
        # room beside the card, and one clicked: its card beside the canvas.
        self.selection("--selector staging_only")
        self.until("document.querySelectorAll('#select-panel #graph .nd').length === 3", "the selection drawn")
        self.press("#select-panel #graph .nd[data-id$='.stg_orders']")
        self.until("!document.querySelector('#selpanel-card').classList.contains('hidden')"
                   " && !!document.querySelector('#selpanel-card .cat-title')", "the card")
        # Its Preview, which fits the card as it opens: the Columns tab is
        # wider than the card.
        self.js("[...document.querySelectorAll('#selpanel-card .cat-tabs .ctab')].find((t) => t.textContent.startsWith('Preview')).click()")
        # Its left edge dragged out, for the room its Location table wants.
        self.drag("#selpanel-grip", by=(-450, 0), framed=True)
        # The card then moves the canvas to keep the box clicked in sight,
        # which it does a moment after: waited out, before the canvas is panned.
        time.sleep(1)
        self.frame()
        # The boxes slid to the middle of what the card leaves of the canvas,
        # by a drag on its background: Fit goes by the size it had before.
        measure = lambda: self.js("""(() => {
          const nd = [...document.querySelectorAll('#select-panel #graph .nd rect.box')].map((n) => n.getBoundingClientRect());
          const card = document.querySelector('#selpanel-card').getBoundingClientRect();
          return [(Math.min(...nd.map((r) => r.left)) + Math.max(...nd.map((r) => r.right))) / 2, card.left];
        })()""")
        for attempt in range(5):
            centre, room = measure()
            if abs(centre - room / 2) < 40:
                break
            if attempt == 4:
                raise AssertionError(f"the boxes are at {centre} in a canvas {room} wide")
            # From near the card's edge, so that the move stays on the page.
            start = room - 20
            end = start + (room / 2 - centre)
            self.mouse("mousePressed", start, 220, button="left", clickCount=1)
            self.mouse("mouseMoved", (start + end) / 2, 220, button="left", buttons=1)
            self.mouse("mouseMoved", end, 220, button="left", buttons=1)
            self.mouse("mouseReleased", end, 220, button="left", clickCount=1)
            self.frame()
            time.sleep(0.5)
        self.rest()
        self.take("selection-card")
        self.code()

        # Reports: which models the jobs run, and the order between them.
        self.reports("jobs", "Job coverage")
        time.sleep(1)
        self.take("reports-coverage")
        # A second DAG, which builds a model whose parents other jobs build,
        # on a clock of its own: the order has something to say. Written now
        # that the coverage is taken, which it would change.
        (self.work / "orchestration" / "dags" / "jaffle_refunds.yml").write_text(DAG_LATE)
        self.reports("joborder", "pair")
        time.sleep(1)
        self.take("reports-job-order")
        self.code()

        # Where a model lives in each environment: the .env files appear, and
        # the page is loaded again to find them.
        stage_envs(self.project)
        self.reload()
        self.dock_to(250)
        self.dock("catalog")
        self.catalog_tab("Preview")
        self.until("!!document.querySelector('#catalog table.loc .envpill')", "the environment switch")
        # PROD picked from the menu, which is closed again: its resolved
        # values beside what dbt built.
        self.env_to("prod")
        time.sleep(0.5)
        self.rest()
        self.take("catalog-location")

        # The compiled SQL, as it would read in that environment.
        self.dock("compiled")
        self.until("document.querySelector('#dock-compiled').textContent.includes('Shown for')", "the environment's SQL")
        self.rest()
        self.take("catalog-compiled")
        # Back on the manifest's own environment, which the pictures after
        # these show in the status bar.
        self.env_to("manifest")
        # And the .env files gone, which the pictures after these (but the
        # Snowflake ones, which write two again) show in the tree.
        remove_envs(self.project)
        self.reload()

        # The terminal, after a command.
        self.dock("terminal")
        self.until("document.querySelector('#term .xterm-rows')?.textContent.length && /[%$#]\\s*$/.test(document.querySelector('#term .xterm-rows').textContent)", "the prompt", 30)
        time.sleep(1)
        self.page.call("Input.insertText", text="git log --oneline -3")
        self.page.key("Enter", "Enter", 13, text="\r")
        # The terminal draws on a frame, and headless Chrome makes none until
        # one is asked for: a cheap capture each time round gets it drawn.
        for _ in range(100):
            self.frame()
            if self.js("/Say\\s+how\\s+to\\s+build/.test([...document.querySelectorAll('#term .xterm-rows')].map((r) => r.textContent).join(' '))"):
                break
            time.sleep(0.1)
        else:
            raise AssertionError("timed out waiting for git's answer")
        self.rest()
        self.take("terminal")

        # The Git tab: something to commit, and a commit to pull.
        stage_git(self.work, self.project)
        self.dock_to(560)
        self.side_to(330)
        self.dock("lineage")
        self.click("[data-side=git]")
        self.until("document.querySelector('#git-body')?.textContent.includes('building.md')"
                   " && document.querySelector('#git-body')?.textContent.includes('todo.md')", "the changes")
        # The diff of the edited note, side by side, in its own tab.
        self.js("[...document.querySelectorAll('#git-body *')].find((e) => e.children.length === 0 && e.textContent === 'building.md').click()")
        self.until("!document.querySelector('#diff-host').classList.contains('hidden')", "the diff")
        self.until("document.querySelector('#behind-n')?.textContent.trim() !== ''", "the commit to pull", 30)
        self.rest()
        self.take("git")
        self.side_to(271)
        self.click("[data-side=files]")
        # What was staged for it is undone, so that no later picture shows a
        # branch behind its origin or files to commit.
        git(self.project, "remote", "remove", "origin")
        git(self.project, "reset", "-q", "--hard", "HEAD~1")
        shutil.rmtree(self.project / "docs")

        # A start with the update check on: the pill, and the list. From a
        # cargo-like install under /tmp when there is one, which the update
        # menu then offers its command for.
        self.restart(update_check=True, binary=self.installed)
        self.dock("lineage")
        self.until("!document.querySelector('#status-update').classList.contains('hidden')", "the update pill", 30)
        self.click("#status-update")
        self.until("document.querySelector('#settings').classList.contains('open')"
                   " && document.querySelector('#settings-view')?.textContent.includes('" + next_releases()[0][1] + "')", "the releases listed")
        self.rest()
        self.take("updates")
        self.code()
        # Without the check again, so that no later picture holds the pill.
        self.restart()

        # Every shortcut, with the tour at its head.
        self.js("document.activeElement && document.activeElement.blur(); document.body.focus()")
        self.page.key("?", "Slash", 191, text="?")
        self.until("!document.querySelector('#shortcuts')?.classList.contains('hidden')", "the shortcut list")
        self.rest()
        self.take("shortcuts")
        self.page.key(*wt.ESCAPE)
        self.until("document.querySelector('#shortcuts').classList.contains('hidden')", "the list to close")

        # The Theme section of the settings: the four cards and Follow the system.
        self.click("#settings-btn")
        self.until("document.querySelector('#settings').classList.contains('open')", "the settings")
        self.js("[...document.querySelectorAll('#settings-nav button')].find((b) => b.textContent.startsWith('Theme')).click()")
        self.until("!!document.querySelector('#settings-view .tcards .tcard')", "the theme cards")
        self.rest()
        self.take("themes")
        self.code()

        # Snowflake, on a fake one: the project gets its Snowflake target and
        # the dbt_artifacts stub, and the features are switched on.
        stage_snowflake(self.work, self.project)
        self.features(True)
        self.dock_to(300)
        self.dock("history")
        self.until("document.querySelector('#dock-history').textContent.includes('number_of_orders')", "the query history", 30)
        self.rest()
        self.take("snowflake-history")

        # The last builds of each job, once Snowflake has been asked.
        self.reports("jobbuilds", "Builds by job")
        self.until("[...document.querySelectorAll('#reports-view button')].some((b) => b.textContent.includes('Read from Snowflake'))", "the read button")
        self.js("[...document.querySelectorAll('#reports-view button')].find((b) => b.textContent.includes('Read from Snowflake')).click()")
        self.until("[...document.querySelectorAll('#reports-view button')].some((b) => b.textContent.includes('Refresh'))", "the builds read", 60)
        time.sleep(1)
        self.rest()
        self.take("reports-builds")
        self.code()

        # The Catalog's Location with its Snowsight menus and Clone button, and
        # the last builds of the model, then the statements the button writes.
        self.dock_to(130)
        self.dock("catalog")
        self.catalog_tab("Preview")
        self.until("!!document.querySelector('#catalog .clonebtn')", "the clone button")
        self.until("!!document.querySelector('#catalog .cat-builds')?.textContent.includes('success')", "the last builds", 30)
        self.js("document.querySelector('#catalog table.loc').scrollIntoView({ block: 'start' })")
        time.sleep(0.6)
        self.rest()
        self.take("catalog-snowflake")
        self.click("#catalog .clonebtn")
        self.until("!document.querySelector('#modal').classList.contains('hidden') && document.querySelector('#modal').textContent.includes('clone')", "the clone statements")
        time.sleep(0.6)
        self.take("catalog-clone")
        self.page.key(*wt.ESCAPE)
        self.until("document.querySelector('#modal').classList.contains('hidden')", "the modal to close")
        unstage_snowflake(self.work, self.project)
        self.features(False)

        # The manifest, behind a model edited since dbt parsed it.
        model = self.project / "models" / "customers.sql"
        model.write_text(model.read_text() + "\n-- split the lifetime value by payment method\n")
        # The page asks again every five seconds: asked at once.
        self.front()
        for _ in range(100):
            self.js("document.dispatchEvent(new Event('visibilitychange'))")
            if self.js("document.querySelector('#fresh-badge').dataset.tone === 'warn'"):
                break
            time.sleep(0.3)
        else:
            raise AssertionError("timed out waiting for the amber pill")
        self.hover("#fresh-badge")
        self.until("!!document.querySelector('.mcard')", "the pill's card")
        self.take("manifest-pill")

        if page.errors:
            raise AssertionError("errors in the page: " + "; ".join(page.errors))

    def restart(self, update_check=False, binary=None):
        """The same project served again, with the update check on or not.
        On, the release list is already in the configuration directory, as the
        look at GitHub would have left it."""
        self.edith.stop()
        if update_check:
            stage_update(self.work / "config")
        self.edith = wt.Edith(binary or self.binary, self.project, self.work, update_check=update_check)
        self.reload()


def main():
    parser = argparse.ArgumentParser(description="The pictures of docs/shots/, taken in headless Chrome on Jaffle Shop.")
    parser.add_argument("--scale", type=int, default=1, help="device pixels per CSS pixel (default 1)")
    parser.add_argument("--theme", choices=["night", "graphite", "light", "halloween"], default="night",
                        help="the theme the pictures are taken in (default night, the only one docs/shots holds)")
    parser.add_argument("--out", type=Path, default=None, help="where the pictures are copied (default docs/shots)")
    parser.add_argument("names", nargs="*", metavar="NAME", help="copy only these: " + ", ".join(NAMES))
    args = parser.parse_args()
    unknown = [n for n in args.names if n not in NAMES]
    if unknown or args.scale < 1:
        sys.exit(f"unknown picture {', '.join(unknown)}; they are: {', '.join(NAMES)}" if unknown else "--scale wants 1 or more")
    if args.scale > 1 and (args.out is None or args.out.resolve() == SHOTS.resolve() or SHOTS.resolve() in args.out.resolve().parents):
        sys.exit("--scale above 1 wants --out elsewhere than docs/shots: a picture at more than 1440 by 900 never goes there (0144)")
    if args.theme != "night" and (args.out is None or args.out.resolve() == SHOTS.resolve() or SHOTS.resolve() in args.out.resolve().parents):
        sys.exit("--theme other than night wants --out elsewhere than docs/shots: the pictures of docs/shots are Night's (0150)")
    args.out = args.out or SHOTS

    os.environ.setdefault("DBT_SEND_ANONYMOUS_USAGE_STATS", "False")
    os.environ.setdefault("DO_NOT_TRACK", "1")
    # When the run began, for the fake Snowflake's recent times to fall within it.
    os.environ["TAKE_SHOTS_STARTED"] = str(time.time())
    # Nothing of the terminal this runs in reaches the shell the pictures show.
    for name in ("TERM_PROGRAM", "TERM_PROGRAM_VERSION", "TERM_SESSION_ID", "ZDOTDIR", "STARSHIP_SHELL"):
        os.environ.pop(name, None)
    os.environ["SHELL_SESSIONS_DISABLE"] = "1"
    binary = wt.edith_binary()
    # Under /tmp, so no path in a picture is a long one of the machine's.
    with tempfile.TemporaryDirectory(prefix="dbt-edith-shots-", dir="/tmp") as tmp:
        work = Path(tmp)
        out = work / "shots"
        out.mkdir()
        project = prepare(work)
        # The environment dbt runs from, active as it would be for whoever
        # starts dbt-edith, so the status bar names it, by a link in the work
        # folder: the terminal types the path of VIRTUAL_ENV, which then is a
        # temporary one wherever the venv is. And collin where it is looked for.
        path = [str(Path(COLLIN).parent)]
        if sys.prefix != sys.base_prefix:
            venv = work / "dbt"
            venv.symlink_to(sys.prefix)
            os.environ["VIRTUAL_ENV"] = str(venv)
            path.insert(0, str(venv / "bin"))
        os.environ["PATH"] = os.pathsep.join(path + [os.environ.get("PATH", "")])
        # The server's git reads the HOME's configuration alone, whatever the
        # shell this runs in exports: it holds the redirect that keeps the
        # update look off GitHub, and none of the user's own.
        os.environ["GIT_CONFIG_GLOBAL"] = str(work / "home" / ".gitconfig")
        os.environ.pop("XDG_CONFIG_HOME", None)
        # What the server's Python finds for `import snowflake.connector`.
        os.environ["PYTHONPATH"] = str(write_fake_snowflake(work))
        cache = Path(os.environ.get("TAKE_SHOTS_CACHE", "/tmp/dbt-edith-shots-cache"))
        # Only the update picture wants it, and a run for other names skips the build.
        installed = temp_install(work, cache) if not args.names or "updates" in args.names else None
        # The theme is kept per machine, so the server is told it as a person would have left it.
        (work / "config").mkdir(exist_ok=True)
        (work / "config" / "look.json").write_text(json.dumps(
            {"theme": args.theme, "follow": False, "apparitions": False, "boxes": "tombstone"}) + "\n")
        edith = wt.Edith(binary, project, work)
        page = None
        shots = None
        try:
            page = wt.Page(work, args.scale)
            shots = Shots(page, out, work, project, binary, args.scale, installed)
            shots.run(edith)
        except BaseException:
            # What the page looked like when a step gave up, for whoever fixes it.
            if page and os.environ.get("TAKE_SHOTS_FAILED"):
                failed = Path(os.environ["TAKE_SHOTS_FAILED"])
                page.shot(failed)
                for done in out.glob("*.png"):
                    shutil.copyfile(done, failed.parent / done.name)
            raise
        finally:
            if page:
                page.close()
            (shots.edith if shots and shots.edith else edith).stop()
        if shots.taken != NAMES:
            raise AssertionError(f"took {shots.taken}, not {NAMES}")
        args.out.mkdir(parents=True, exist_ok=True)
        for name in args.names or NAMES:
            shutil.copyfile(out / f"{name}.png", args.out / f"{name}.png")
            print(f"wrote {args.out / (name + '.png')}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
