#!/usr/bin/env python3
"""The Custom selection box against dbt itself, on the Jaffle Shop copy in tests/fixtures/.

Every named selector of that project, and a list of typed expressions below,
is resolved twice: by dbt-edith, over the manifest dbt just wrote, and by
`dbt ls`. The two lists of names have to be the same (0033). This is the only
check of the selector engine that does not rest on reading dbt's code right.

dbt runs in this process, through `dbtRunner`, so it needs dbt-core and
dbt-duckdb importable by the Python running the script. Without them it skips
itself, exit 2, because check.sh must need nothing installed (0013); CI
installs them and runs it for two versions of dbt-core. DBT names a dbt
executable to run instead, one process per answer, which is how dbt Fusion is
compared. dbt-edith is target/debug/dbt-edith, built here first, or whatever
DBT_EDITH names.

Nothing is written inside the repository: dbt works on a temporary copy of the
project, and dbt-edith keeps its settings in a temporary directory.

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
FIXTURE = ROOT / "tests" / "fixtures" / "jaffle_shop"

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

# Selectors the box has to refuse, with dbt left out of it.
REFUSED = {"changed"}


def skip(reason):
    print(f"SKIPPED: {reason}")
    sys.exit(2)


EXECUTABLE = os.environ.get("DBT")
if not EXECUTABLE:
    try:
        from dbt.cli.main import dbtRunner
        import dbt.adapters.duckdb  # noqa: F401  the fixture's profile names it
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
        deadline = time.time() + 60
        while True:
            try:
                self.get("/api/meta")
                return
            except (urllib.error.URLError, ConnectionError):
                if time.time() > deadline or self.proc.poll() is not None:
                    self.stop()
                    raise RuntimeError("dbt-edith did not start: " + (home / "server.log").read_text()[-2000:])
                time.sleep(0.2)

    def get(self, path):
        url = f"http://127.0.0.1:{self.port}{path}"
        try:
            with urllib.request.urlopen(url, timeout=30) as r:
                return json.load(r)
        except urllib.error.HTTPError as e:
            return json.load(e)

    def names(self, line):
        body = self.get("/api/select?" + urllib.parse.urlencode({"q": line, "tests": 1}))
        if "error" in body:
            return None, body["error"]
        return sorted(body.get("names", [])), None

    def stop(self):
        self.proc.terminate()
        try:
            self.proc.wait(timeout=10)
        except subprocess.TimeoutExpired:
            self.proc.kill()
        self.log.close()


class Dbt:
    """`dbt ls` and `dbt parse` against the project copy: in this process, or
    through the executable DBT names."""

    def __init__(self, project, work):
        self.runner = None if EXECUTABLE else dbtRunner()
        self.common = [
            "--project-dir", str(project), "--profiles-dir", str(project),
            "--target-path", str(work / "target"), "--log-path", str(work / "logs"),
        ]

    def version(self):
        if EXECUTABLE:
            out = subprocess.run([EXECUTABLE, "--version"], capture_output=True, text=True).stdout
            # dbt-core says `installed: 1.11.15` under a `Core:` heading, Fusion `dbt 2.0.6`.
            core = re.search(r"installed:\s*(\S+)", out)
            first = next((line.strip() for line in out.splitlines() if line.strip()), EXECUTABLE)
            return f"dbt-core {core.group(1)}" if core else first
        import dbt.version
        return f"dbt-core {dbt.version.__version__}"

    def invoke(self, args):
        if EXECUTABLE:
            # Quiet, both dbt-core and Fusion print one name per line and nothing
            # else, and nothing at all for an empty selection.
            run = subprocess.run([EXECUTABLE] + args[:1] + self.common + ["--quiet"] + args[1:],
                                 capture_output=True, text=True)
            if run.returncode != 0:
                raise RuntimeError(f"dbt {' '.join(args)} failed: {(run.stdout + run.stderr)[-2000:]}")
            return [line.strip() for line in run.stdout.splitlines() if line.strip()]
        res = self.runner.invoke(args[:1] + self.common + ["--log-level", "none"] + args[1:])
        if not res.success:
            raise RuntimeError(f"dbt {' '.join(args)} failed: {res.exception}")
        return res.result

    def ls(self, args):
        return sorted(self.invoke(["ls", "--output", "name"] + args) or [])



def typed_args(line):
    select, _, exclude = line.partition(" --exclude ")
    return ["--select", select] + (["--exclude", exclude] if exclude else [])


def main():
    os.environ.setdefault("DBT_SEND_ANONYMOUS_USAGE_STATS", "False")
    os.environ.setdefault("DO_NOT_TRACK", "1")
    binary = os.environ.get("DBT_EDITH")
    if not binary:
        subprocess.run(["cargo", "build", "--quiet"], cwd=ROOT, check=True)
        binary = ROOT / "target" / "debug" / ("dbt-edith.exe" if os.name == "nt" else "dbt-edith")

    failed = 0
    compared = 0

    def check(label, ours, theirs):
        nonlocal failed, compared
        compared += 1
        if ours == theirs:
            print(f"PASS  {label} ({len(theirs)})")
            return
        failed += 1
        print(f"FAIL  {label}")
        print(f"        only dbt-edith: {sorted(set(ours) - set(theirs))}")
        print(f"        only dbt:       {sorted(set(theirs) - set(ours))}")

    with tempfile.TemporaryDirectory(prefix="dbt-edith-compare-") as tmp:
        work = Path(tmp)
        project = work / "project"
        shutil.copytree(FIXTURE, project)
        oracle = Dbt(project, work)
        oracle.invoke(["parse"])
        manifest = work / "target" / "manifest.json"
        recorded = sorted(json.loads(manifest.read_text())["selectors"])
        print(f"{oracle.version()}, {len(recorded)} selectors in the manifest")

        edith = Edith(binary, project, manifest, work)
        try:
            listed = edith.get("/api/selectors")["selectors"]
            if sorted(s["name"] for s in listed) != recorded:
                failed += 1
                print("FAIL  /api/selectors lists what the manifest records")
            for s in listed:
                name = s["name"]
                if name in REFUSED:
                    ok = bool(s.get("unsupported"))
                    failed += not ok
                    print(f"{'PASS' if ok else 'FAIL'}  --selector {name} is refused")
                    continue
                if s.get("unsupported"):
                    failed += 1
                    print(f"FAIL  --selector {name} was refused: {s['unsupported']}")
                    continue
                ours, error = edith.names(f"--selector {name}")
                check(f"--selector {name}", ours if error is None else [f"error: {error}"], oracle.ls(["--selector", name]))
            for line in EXPRESSIONS:
                ours, error = edith.names(line)
                check(line, ours if error is None else [f"error: {error}"], oracle.ls(typed_args(line)))
        finally:
            edith.stop()

    # Fewer than this means the fixture or the listing broke, not that all is well.
    if compared < len(EXPRESSIONS) + 10:
        failed += 1
        print(f"FAIL  only {compared} answers compared")
    print(f"{compared - failed} of {compared} agree" if not failed else f"{failed} disagreement(s)")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
