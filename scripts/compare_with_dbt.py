#!/usr/bin/env python3
"""The Custom selection box and the lineage against dbt itself, on two public projects.

Jaffle Shop is copied from tests/fixtures/. Fivetran's Shopify package is
fetched at the tag and commit tests/fixtures/shopify/ names (0056). On each,
every named selector and a list of typed expressions is resolved twice: by
dbt-edith, over the manifest dbt just wrote, and by `dbt ls`. Then the lineage
of the models, the nodes the canvas draws around one at a given depth each way,
is compared with `dbt ls -s N+model+M`. The two lists of names have to be the
same (0033). This is the only check of the selector engine and of the lineage
that does not rest on reading dbt's code right.

dbt runs in this process, through `dbtRunner`, so it needs dbt-core and
dbt-duckdb importable by the Python running the script. Without them it skips
itself, exit 2, because check.sh must need nothing installed (0013); CI
installs them and runs it for two versions of dbt-core. DBT names a dbt
executable to run instead, one process per answer, which is how dbt Fusion is
compared. dbt-edith is target/debug/dbt-edith, built here first, or whatever
DBT_EDITH names.

Fetching Shopify needs git and the network, for GitHub and dbt's hub. Without
them that project is skipped, and the run ends with exit 2 even when Jaffle
Shop agreed, so CI, which treats a skip as a failure, never passes without it.

The lineage of every model is compared with --every-model, which CI passes.
Without it, a project with more than 40 models has every eighth compared,
which keeps this step of check.sh to about twenty seconds here.

Nothing is written inside the repository: dbt works on temporary copies, and
dbt-edith keeps its settings in a temporary directory.

Exit 0 when every answer agrees, 1 when one does not, 2 when skipped.
"""

import json
import os
import re
import shutil
import socket
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.parse
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
JAFFLE = ROOT / "tests" / "fixtures" / "jaffle_shop"
SHOPIFY = ROOT / "tests" / "fixtures" / "shopify"

# The tag dbt-collin pins too, so both repositories read the same project, and
# its commit, so a tag moved upstream fails the run instead of changing it.
SHOPIFY_REPO = "https://github.com/fivetran/dbt_shopify"
SHOPIFY_TAG = "v1.10.0"
SHOPIFY_COMMIT = "03e91d7aeb83151f968105f8580d13110f5375e4"

# Typed lines, each with the tests checkbox on, which is dbt's own default.
# Between them: every method the fixture can answer, every graph operator, the
# implicit fqn forms, both set operations, and an exclude half.
EXPRESSIONS = [
    "customers",
    "+customers",
    "customers+",
    "@stg_orders",
    "1+orders",
    "stg_orders+1",
    "tag:staging",
    "tag:staging+",
    "tag:recon",
    "path:models/staging",
    "file:orders.sql",
    "resource_type:seed",
    "resource_type:test",
    "config.materialized:view",
    "package:jaffle_shop",
    "jaffle_shop.staging.*",
    "staging.stg_orders",
    "*",
    "+orders,+customers",
    "stg_customers,stg_orders+",
    "resource_type:test,+orders",
    "raw_customers raw_orders+",
    "tag:staging --exclude stg_payments",
    "+customers --exclude resource_type:seed",
    "orders --exclude tag:recon",
    "customers --exclude customers",
]

# What Shopify has and Jaffle Shop does not: a package under a root project,
# sources, ephemeral and incremental models, folders three deep, and disabled
# models and sources. `*` and the fqn prefixes are what showed that the fqn
# method must never reach a source. `path:` of a folder that is the package's
# alone is left out: dbt globs the root project's disk for it, and dbt-edith
# reads the path the manifest recorded (0024).
SHOPIFY_EXPRESSIONS = [
    "shopify__customers",
    "+shopify__customers",
    "shopify__orders+",
    "@stg_shopify__order",
    "2+shopify__order_lines",
    "int_shopify__daily_orders+1",
    "package:shopify",
    "package:shopify_integration_tests",
    "shopify.graphql.staging.*",
    "shopify.rest",
    "graphql.intermediate",
    "shopify.utils.shopify__calendar",
    "path:seeds",
    "file:shopify__customers.sql",
    "file:shopify.yml",
    "source:shopify",
    "source:shopify_graphql.order",
    "source:shopify+",
    "source:shopify_graphql+1",
    "resource_type:source",
    "config.materialized:ephemeral",
    "config.materialized:incremental",
    "config.materialized:table+",
    "shopify.graphql.*,config.materialized:view",
    "+shopify__customers,+shopify__orders",
    "shopify__orders+ --exclude config.materialized:ephemeral",
    "package:shopify --exclude shopify.rest",
    "*",
    "* --exclude resource_type:test",
]

# The canvas around one model, as (up, down, tests box): one level, two, and
# twenty, which is the most the lineage route takes and deeper than either
# project goes; then two with the tests a model's lineage hangs under it.
LINEAGE = [(1, 1, False), (2, 2, False), (20, 20, False), (2, 2, True)]

# Selectors the box has to refuse, with dbt left out of it.
REFUSED = {"changed"}


def skip(reason):
    print(f"SKIPPED: {reason}")
    sys.exit(2)


class Unavailable(Exception):
    """A project that cannot be had here: no git, or no network."""


EXECUTABLE = os.environ.get("DBT")
if not EXECUTABLE:
    try:
        from dbt.cli.main import dbtRunner
        import dbt.adapters.duckdb  # noqa: F401  both profiles name it
    except ImportError as e:
        skip(f"dbt-core with dbt-duckdb is not importable here ({e.name}). "
             "Install both, point this at a Python that has them, or set DBT to a dbt executable.")


def free_port():
    with socket.socket() as s:
        s.bind(("127.0.0.1", 0))
        return s.getsockname()[1]


class Edith:
    """dbt-edith over the project copy and the manifest dbt wrote for it."""

    def __init__(self, binary, project, manifest, home):
        self.port = free_port()
        self.log = open(home / "server.log", "w")
        env = dict(os.environ, DBT_EDITH_CONFIG_DIR=str(home / "config"))
        self.proc = subprocess.Popen(
            [str(binary), str(project), "--manifest", str(manifest), "--port", str(self.port), "--no-open"],
            stdout=self.log, stderr=subprocess.STDOUT, env=env,
        )
        # The API wants the key the launch prints in its `open` line (0052),
        # so it is read there, as a person would read it, before anything is asked.
        self.key = None
        deadline = time.time() + 60
        while True:
            try:
                if self.key is None:
                    found = re.search(r"\?key=([0-9a-f]{64})", (home / "server.log").read_text())
                    if not found:
                        raise ConnectionError("no key printed yet")
                    self.key = found.group(1)
                self.get("/api/meta")
                return
            except (urllib.error.URLError, ConnectionError):
                if time.time() > deadline or self.proc.poll() is not None:
                    self.stop()
                    raise RuntimeError("dbt-edith did not start: " + (home / "server.log").read_text()[-2000:])
                time.sleep(0.2)

    def get(self, path):
        url = f"http://127.0.0.1:{self.port}{path}"
        request = urllib.request.Request(url, headers={"Authorization": f"Bearer {self.key}"})
        try:
            with urllib.request.urlopen(request, timeout=30) as r:
                return json.load(r)
        except urllib.error.HTTPError as e:
            return json.load(e)

    def names(self, line):
        body = self.get("/api/select?" + urllib.parse.urlencode({"q": line, "tests": 1}))
        if "error" in body:
            return None, body["error"]
        return sorted(body.get("names", [])), None

    def lineage(self, unique_id, up, down, tests):
        """The names the canvas draws around a model, or a reason it cannot say."""
        query = {"id": unique_id, "up": up, "down": down, "tests": int(tests), "max": 3000}
        body = self.get("/api/lineage?" + urllib.parse.urlencode(query))
        if "nodes" not in body:
            return [f"error: {body}"]
        if body.get("truncated"):
            return ["error: capped at 3000 nodes"]
        return sorted(n["name"] for n in body["nodes"])

    def stop(self):
        self.proc.terminate()
        try:
            self.proc.wait(timeout=10)
        except subprocess.TimeoutExpired:
            self.proc.kill()
        self.log.close()


class Dbt:
    """`dbt deps`, `dbt parse` and `dbt ls` against a project copy: in this
    process, or through the executable DBT names."""

    def __init__(self, project, work):
        self.runner = None if EXECUTABLE else dbtRunner()
        self.where = ["--project-dir", str(project), "--profiles-dir", str(project)]
        self.common = self.where + ["--target-path", str(work / "target"), "--log-path", str(work / "logs")]

    def version(self):
        if EXECUTABLE:
            out = subprocess.run([EXECUTABLE, "--version"], capture_output=True, text=True).stdout
            # dbt-core says `installed: 1.11.15` under a `Core:` heading, Fusion `dbt 2.0.6`.
            core = re.search(r"installed:\s*(\S+)", out)
            first = next((line.strip() for line in out.splitlines() if line.strip()), EXECUTABLE)
            return f"dbt-core {core.group(1)}" if core else first
        import dbt.version
        return f"dbt-core {dbt.version.__version__}"

    def invoke(self, args, common=None):
        common = self.common if common is None else common
        if EXECUTABLE:
            # Quiet, both dbt-core and Fusion print one name per line and nothing
            # else, and nothing at all for an empty selection.
            run = subprocess.run([EXECUTABLE] + args[:1] + common + ["--quiet"] + args[1:],
                                 capture_output=True, text=True)
            if run.returncode != 0:
                raise RuntimeError(f"dbt {' '.join(args)} failed: {(run.stdout + run.stderr)[-2000:]}")
            return [line.strip() for line in run.stdout.splitlines() if line.strip()]
        res = self.runner.invoke(args[:1] + common + ["--log-level", "none"] + args[1:])
        if not res.success:
            raise RuntimeError(f"dbt {' '.join(args)} failed: {res.exception}")
        return res.result

    def deps(self):
        # `deps` takes no target or log path.
        self.invoke(["deps"], common=self.where)

    def parse(self):
        """Parses, then hands the manifest to every later call in this process.
        Each `ls` would parse again otherwise: a second for Shopify, against a
        tenth of one, and the lineage asks a thousand times."""
        manifest = self.invoke(["parse"])
        if not EXECUTABLE:
            self.runner = dbtRunner(manifest=manifest)

    def ls(self, args):
        return sorted(self.invoke(["ls", "--output", "name"] + args) or [])


def typed_args(line):
    select, _, exclude = line.partition(" --exclude ")
    return ["--select", select] + (["--exclude", exclude] if exclude else [])


def fetch_shopify(work):
    """Fivetran's package at the pinned tag, with the profile and the lock
    dbt-edith keeps beside it. Returns the project dbt runs in."""
    if not shutil.which("git"):
        raise Unavailable("git is not installed")
    dest = work / "dbt_shopify"
    run = subprocess.run(
        ["git", "-c", "advice.detachedHead=false", "clone", "--quiet", "--depth", "1",
         "--branch", SHOPIFY_TAG, SHOPIFY_REPO, str(dest)],
        capture_output=True, text=True,
    )
    if run.returncode != 0:
        raise Unavailable(f"cannot fetch {SHOPIFY_REPO} at {SHOPIFY_TAG}: {run.stderr.strip()[-300:]}")
    head = subprocess.run(["git", "-C", str(dest), "rev-parse", "HEAD"], capture_output=True, text=True).stdout.strip()
    if head != SHOPIFY_COMMIT:
        raise RuntimeError(f"{SHOPIFY_TAG} of {SHOPIFY_REPO} is now {head}, not {SHOPIFY_COMMIT}: the tag moved")
    project = dest / "integration_tests"
    for name in ("profiles.yml", "package-lock.yml"):
        shutil.copy(SHOPIFY / name, project / name)
    return project


class Tally:
    def __init__(self):
        self.compared = 0
        self.failed = 0

    def check(self, label, ours, theirs, quiet=False):
        self.compared += 1
        if ours == theirs:
            if not quiet:
                print(f"PASS  {label} ({len(theirs)})")
            return True
        self.failed += 1
        print(f"FAIL  {label}")
        print(f"        only dbt-edith: {sorted(set(ours) - set(theirs))}")
        print(f"        only dbt:       {sorted(set(theirs) - set(ours))}")
        return False

    def fail(self, label):
        self.compared += 1
        self.failed += 1
        print(f"FAIL  {label}")


def compare_project(tally, binary, project, work, expressions, every_model):
    """Every named selector, the typed lines, and the lineage of the models.
    Returns how many answers were compared, so a project that compared too few
    can be told from one that agreed."""
    oracle = Dbt(project, work)
    oracle.parse()
    manifest = work / "target" / "manifest.json"
    parsed = json.loads(manifest.read_text())
    recorded = sorted(parsed.get("selectors") or {})
    models = sorted(uid for uid, n in parsed["nodes"].items() if n["resource_type"] == "model")
    tests = {n["name"] for n in parsed["nodes"].values() if n["resource_type"] in ("test", "unit_test")}
    sampled = models if every_model or len(models) <= 40 else models[::8]
    print(f"   {len(recorded)} selectors in the manifest, {len(models)} models, the lineage of {len(sampled)} compared")
    before = tally.compared

    edith = Edith(binary, project, manifest, work)
    try:
        listed = edith.get("/api/selectors")["selectors"]
        if sorted(s["name"] for s in listed) != recorded:
            tally.fail("/api/selectors lists what the manifest records")
        for s in listed:
            name = s["name"]
            if name in REFUSED:
                if s.get("unsupported"):
                    tally.compared += 1
                    print(f"PASS  --selector {name} is refused")
                else:
                    tally.fail(f"--selector {name} is refused")
                continue
            if s.get("unsupported"):
                tally.fail(f"--selector {name} was refused: {s['unsupported']}")
                continue
            ours, error = edith.names(f"--selector {name}")
            tally.check(f"--selector {name}", ours if error is None else [f"error: {error}"], oracle.ls(["--selector", name]))
        for line in expressions:
            ours, error = edith.names(line)
            tally.check(line, ours if error is None else [f"error: {error}"], oracle.ls(typed_args(line)))

        # dbt's `N+model+M` brings the tests of what it reaches, as the canvas
        # does with its tests box on; with the box off they are dropped by name.
        for up, down, with_tests in LINEAGE:
            box = "with tests" if with_tests else "without tests"
            agreed = 0
            for unique_id in sampled:
                name = parsed["nodes"][unique_id]["name"]
                theirs = oracle.ls(["--select", f"{up}+{name}+{down}"])
                if not with_tests:
                    theirs = [n for n in theirs if n not in tests]
                ours = edith.lineage(unique_id, up, down, with_tests)
                agreed += tally.check(f"lineage of {name}, {up} up and {down} down, {box}", ours, theirs, quiet=True)
            verdict = "PASS" if agreed == len(sampled) else "FAIL"
            print(f"{verdict}  lineage {up} up and {down} down, {box}: {agreed} of {len(sampled)} models")
    finally:
        edith.stop()
    return tally.compared - before


def main():
    os.environ.setdefault("DBT_SEND_ANONYMOUS_USAGE_STATS", "False")
    os.environ.setdefault("DO_NOT_TRACK", "1")
    every_model = "--every-model" in sys.argv[1:]
    binary = os.environ.get("DBT_EDITH")
    if not binary:
        subprocess.run(["cargo", "build", "--quiet"], cwd=ROOT, check=True)
        binary = ROOT / "target" / "debug" / ("dbt-edith.exe" if os.name == "nt" else "dbt-edith")

    tally = Tally()
    skipped = None
    with tempfile.TemporaryDirectory(prefix="dbt-edith-compare-") as tmp:
        work = Path(tmp)
        print(f"{Dbt(work, work).version()}, on Jaffle Shop and Fivetran's Shopify {SHOPIFY_TAG}")

        print("== Jaffle Shop")
        jaffle = work / "jaffle" / "project"
        shutil.copytree(JAFFLE, jaffle)
        compared = compare_project(tally, binary, jaffle, jaffle.parent, EXPRESSIONS, every_model)
        # Fewer than this means the fixture or the listing broke, not that all is well.
        if compared < len(EXPRESSIONS) + 10:
            tally.fail(f"only {compared} answers compared on Jaffle Shop")

        print(f"== Shopify {SHOPIFY_TAG} ({SHOPIFY_COMMIT[:7]})")
        try:
            shopify = fetch_shopify(work / "shopify")
            Dbt(shopify, work / "shopify").deps()
        except (Unavailable, RuntimeError) as e:
            if isinstance(e, RuntimeError) and "tag moved" in str(e):
                tally.fail(str(e))
            else:
                skipped = f"Shopify: {e}"
                print(f"SKIPPED: {skipped}")
        else:
            compared = compare_project(tally, binary, shopify, work / "shopify", SHOPIFY_EXPRESSIONS, every_model)
            if compared < len(SHOPIFY_EXPRESSIONS):
                tally.fail(f"only {compared} answers compared on Shopify")

    agreed = tally.compared - tally.failed
    if tally.failed:
        print(f"{tally.failed} disagreement(s)")
        return 1
    if skipped:
        print(f"SKIPPED: {skipped}; the rest agrees, {agreed} of {tally.compared}")
        return 2
    print(f"{agreed} of {tally.compared} agree")
    return 0


if __name__ == "__main__":
    sys.exit(main())
