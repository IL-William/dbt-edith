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
compared. dbt-edith is built here first and run from where cargo put it, or
is whatever DBT_EDITH names.

Fetching Shopify needs git and the network, for GitHub and dbt's hub. Without
them that project is skipped, and the run ends with exit 2 even when Jaffle
Shop agreed, so CI, which treats a skip as a failure, never passes without it.

On Jaffle Shop, the invented jobs under tests/fixtures/jobs/ are read too,
from outside the project copy, and each step's selection is compared with the
`dbt ls` the step's button types (0063).

On each project, the config paths of dbt_project.yml that apply to no
resource, which dbt warns of as it parses, are compared with what the
Warnings report works out without a run (0103). A third copy, of Jaffle Shop
with such paths added, makes that comparison say something.

On each project, the four lists of the Checks report (0116) are worked out
again in Python from the manifest dbt wrote, by the rules dbt-core's own code
gives them, and compared with what the report lists. A fourth copy, of Jaffle
Shop with sources, models and tests of every shape those rules tell apart,
makes that comparison say something.

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
import shlex
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
JOBS = ROOT / "tests" / "fixtures" / "jobs"

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
    # On the two incremental models JAFFLE_INCREMENTAL adds: dbt matches what
    # is configured, so a model on the adapter's default is not a merge here,
    # and a key given as a list is matched by membership.
    "config.materialized:incremental",
    "config.incremental_strategy:merge",
    "config.incremental_strategy:delete+insert",
    "config.unique_key:order_id",
    "config.unique_key:customer_id+",
    # On the versioned model JAFFLE_VERSIONS adds (0104): its name takes every
    # version, a version is named with a dot or an underscore, and a ref()
    # naming none reaches the latest.
    "customer_totals",
    "customer_totals.v1",
    "customer_totals_v3",
    "jaffle_shop.customer_totals.v2",
    "customer_totals.v1+",
    "+totals_latest",
    "version:latest",
    "version:prerelease",
    "version:old",
    "version:none",
    "customer_totals --exclude version:old",
]

# Added to the copy of Jaffle Shop, never to the fixture, whose screenshots and
# browser tests would move: one incremental model on the adapter's default and
# one keyed on a list. Shopify's four configure delete+insert, each on one key.
JAFFLE_INCREMENTAL = {
    "models/orders_appended.sql": "{{ config(materialized='incremental') }}\n"
                                  "select * from {{ ref('orders') }}\n",
    "models/order_payments.sql": "{{ config(materialized='incremental', incremental_strategy='delete+insert',\n"
                                 "          unique_key=['order_id', 'customer_id']) }}\n"
                                 "select * from {{ ref('orders') }}\n",
}

# Config paths that apply to nothing, and some that apply, written below the
# fixture's last block, so the first lines continue `jaffle_shop:` under
# `models:` (0103). Each is a shape dbt reads its own way: a folder under its
# config's name, a quoted number, an anchor merged, an empty mapping, a key
# holding nothing, a package not installed, a block with no resource of its
# kind, and `tests:`, which dbt reads as `data_tests:`. The disabled model
# keeps `snapshots:` used, as any disabled node does every block; the
# exposure and the unit test alone keep their own paths used, whose fqns are
# not a folder's. No config is written without its `+`: Fusion refuses one.
JAFFLE_UNUSED = """    stagign: &shared
      +materialized: view
    legacy:
      +enabled: false
    '2023':
      +tags: ['y']
    marts:
      core:
        +tags: ['deep']
    meta:
      +tags: ['m']
    empty: {}
    merged:
      <<: *shared
  other_package:
    +tags: ['pkg']

snapshots:
  +tags: ['snap']

sources:
  jaffle_shop:
    shop:
      +tags: ['src']
    shopp:
      +tags: ['typo']

exposures:
  jaffle_shop:
    reports:
      +tags: ['x']
    weekly:
      +tags: ['x']

unit_tests:
  +enabled: true
  jaffle_shop:
    staging:
      +tags: ['x']

tests:
  jaffle_shop:
    staging:
      +tags: ['t']
    nowhere:
      +tags: ['t']
"""
JAFFLE_UNUSED_FILES = {
    "models/legacy/old_orders.sql": "select 1 as id\n",
    "models/2023/orders_2023.sql": "select 1 as id\n",
    "models/shop.yml": "version: 2\nsources:\n  - name: shop\n    tables:\n      - name: raw_things\n"
                       "exposures:\n  - name: weekly\n    type: dashboard\n    owner:\n      name: someone\n"
                       "      email: someone@example.com\n    depends_on:\n      - ref('orders')\n",
    "models/staging/units.yml": "unit_tests:\n  - name: orders_keep_their_id\n    model: stg_orders\n    given:\n"
                                "      - input: ref('raw_orders')\n        rows:\n          - {id: 1, user_id: 1}\n"
                                "    expect:\n      rows:\n        - {order_id: 1}\n",
}

# Added to the copy too, for the same reason: a model in three versions, the
# second the latest and the third a prerelease, each with a test, a model
# reading it by its bare name, which is the latest, and one reading v1.
JAFFLE_VERSIONS = {
    "models/customer_totals_v1.sql": "select customer_id, number_of_orders from {{ ref('customers') }}\n",
    "models/customer_totals_v2.sql": "select customer_id, number_of_orders, total_order_amount from {{ ref('customers') }}\n",
    "models/customer_totals_v3.sql": "select customer_id, total_order_amount from {{ ref('customers') }}\n",
    "models/totals_latest.sql": "select * from {{ ref('customer_totals') }}\n",
    "models/totals_old.sql": "select * from {{ ref('customer_totals', v=1) }}\n",
    "models/versions.yml": "version: 2\n\nmodels:\n"
                           "  - name: customer_totals\n"
                           "    latest_version: 2\n"
                           "    columns:\n"
                           "      - name: customer_id\n"
                           "        data_tests:\n"
                           "          - not_null\n"
                           "    versions:\n"
                           "      - v: 1\n"
                           "      - v: 2\n"
                           "      - v: 3\n",
}

# Added to a copy of Jaffle Shop for the Checks report (0116): sources whose
# freshness comes from the source, from the table, from a `config:` block, from
# nothing, from `null`, from `{}` and from half a threshold, one a model reads,
# one tested and read by none, a test of it pointing at a model of the same
# file, and one disabled; and models that are described
# or not, tested by a test they declare, only pointed at by one, selected from
# by a singular test, targeted by a unit test, or by nothing, and one disabled.
JAFFLE_CHECKS = {
    "models/probe/sources_probe.yml": """version: 2
sources:
  - name: s_top
    loaded_at_field: ts
    freshness:
      warn_after: {count: 1, period: day}
    tables:
      - name: inherits
      - name: overrides_null
        freshness: null
      - name: own
        freshness:
          error_after: {count: 2, period: hour}
      - name: empty_own
        freshness: {}
  - name: s_none
    tables:
      - name: bare
      - name: cfg_style
        config:
          freshness:
            warn_after: {count: 3, period: day}
          loaded_at_field: ts
      - name: retired
        config:
          enabled: false
  - name: s_partial
    freshness:
      warn_after: {count: 1}
    tables:
      - name: half
        columns:
          - name: id
            data_tests:
              - not_null
              # Declared by a source, and pointing at a model of the same file,
              # which it does not test.
              - relationships:
                  arguments:
                    to: ref('probe_reader')
                    field: id

models:
  - name: probe_pointer
    description: Declares a relationships test that points at probe_target.
    columns:
      - name: id
        data_tests:
          - relationships:
              arguments:
                to: ref('probe_target')
                field: id
  - name: probe_target
    description: Only that relationships test points at it.
  - name: probe_singular
    description: Only a singular test selects from it.
  - name: probe_unit
    description: Only a unit test targets it.
  - name: probe_reader
    description: Reads a source, and nothing tests it.

unit_tests:
  - name: probe_unit_keeps_its_id
    model: probe_unit
    given: []
    expect:
      rows:
        - {id: 1}
""",
    "models/probe/probe_pointer.sql": "select 1 as id\n",
    "models/probe/probe_target.sql": "select 1 as id\n",
    "models/probe/probe_singular.sql": "select 1 as id\n",
    "models/probe/probe_unit.sql": "select 1 as id\n",
    "models/probe/probe_reader.sql": "select * from {{ source('s_top', 'inherits') }}\n",
    "models/probe/probe_undescribed.sql": "select 1 as id\n",
    "models/probe/probe_off.sql": "{{ config(enabled=false) }}\nselect 1 as id\n",
    "tests/probe_singular_check.sql": "select * from {{ ref('probe_singular') }} where false\n",
}

# What the rules give on that copy, written by hand from them and not read off
# the manifest, so a manifest the comparison read wrongly cannot agree with
# itself: the staging models of Jaffle Shop have no description.
JAFFLE_CHECKS_FOUND = {
    "sources_without_freshness": ["source.jaffle_shop.s_none.bare", "source.jaffle_shop.s_partial.half",
                                  "source.jaffle_shop.s_top.overrides_null"],
    "models_without_test": ["model.jaffle_shop.probe_reader", "model.jaffle_shop.probe_target",
                            "model.jaffle_shop.probe_undescribed"],
    "models_without_description": ["model.jaffle_shop.probe_undescribed", "model.jaffle_shop.stg_customers",
                                   "model.jaffle_shop.stg_orders", "model.jaffle_shop.stg_payments"],
    "sources_not_read": ["source.jaffle_shop.s_none.bare", "source.jaffle_shop.s_none.cfg_style",
                         "source.jaffle_shop.s_partial.half", "source.jaffle_shop.s_top.empty_own",
                         "source.jaffle_shop.s_top.overrides_null", "source.jaffle_shop.s_top.own"],
}

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

# The job fixtures, as a person would list them in the settings: the renamed
# folder with the keys it renames.
JOB_SOURCES = [
    {"kind": "airflow", "path": str(JOBS / "airflow"), "label": "airflow", "keys": {}},
    {"kind": "airflow", "path": str(JOBS / "airflow_renamed"), "label": "renamed", "keys": {
        "dag_id": "pipeline.name", "schedule": "pipeline.cron", "tasks": "steps", "task_id": "id",
        "command": "dbt", "sensors": "waits", "sensor_id": "id",
        "external_dag_id": "upstream_dag", "external_task_id": "upstream_task",
    }},
    {"kind": "dbt_cloud", "path": str(JOBS / "dbt_cloud"), "label": "cloud", "keys": {}},
]

# Steps dbt-edith has to leave unresolved, by the line it shows, and how many
# steps the fixtures hold in all, so a fixture read short cannot pass.
JOBS_UNRESOLVED = {"dbt build --select state:modified+"}
JOB_STEPS = 15


def skip(reason):
    print(f"SKIPPED: {reason}")
    sys.exit(2)


def edith_binary():
    """The dbt-edith DBT_EDITH names, or the one built here, found where cargo
    says it put it, as walk_tour.edith_binary does: importing that script
    would skip this one wherever Chrome is missing, CI included."""
    if os.environ.get("DBT_EDITH"):
        return os.environ["DBT_EDITH"]
    out = subprocess.run(["cargo", "build", "--quiet", "--message-format=json-render-diagnostics"],
                         cwd=ROOT, check=True, stdout=subprocess.PIPE, encoding="utf-8").stdout
    for line in out.splitlines():
        msg = json.loads(line)
        if msg.get("reason") == "compiler-artifact" and msg["target"]["name"] == "dbt-edith" and msg.get("executable"):
            return Path(msg["executable"])
    sys.exit("cargo build named no dbt-edith executable")


def unused_in_output(text):
    """The paths dbt lists under its UnusedResourceConfigPath warning, as
    dbt-core and Fusion both print it: one `- path` line each."""
    lines = re.sub(r"\x1b\[[0-9;]*m", "", text).splitlines()
    for i, line in enumerate(lines):
        if "unused configuration paths:" in line:
            found = []
            for item in lines[i + 1:]:
                m = re.match(r"\s*- (\S+)\s*$", item)
                if not m:
                    break
                found.append(m.group(1))
            return found
    return []


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
        self.binary, self.project, self.manifest = binary, project, manifest
        # A directory of its own, which the one-shot mode must never create:
        # it reads no settings and writes none (0057).
        self.command_line_config = home / "command-line-config"
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

    def send(self, path, method, body):
        url = f"http://127.0.0.1:{self.port}{path}"
        request = urllib.request.Request(
            url, data=json.dumps(body).encode(), method=method,
            headers={"Authorization": f"Bearer {self.key}", "Content-Type": "application/json"},
        )
        try:
            with urllib.request.urlopen(request, timeout=60) as r:
                return json.load(r)
        except urllib.error.HTTPError as e:
            return {"error": e.read().decode()}

    def names(self, line):
        body = self.get("/api/select?" + urllib.parse.urlencode({"q": line, "tests": 1}))
        if "error" in body:
            return None, body["error"]
        return sorted(body.get("names", [])), None

    def unused_paths(self):
        """The Warnings report's paths, as dbt writes them in its warning."""
        body = self.get("/api/reports/warnings")
        if "unused" not in body:
            return [f"error: {body}"]
        return sorted(".".join([u["block"]] + u["chain"]) for u in body["unused"])

    def checks(self):
        """The Checks report's four lists, as sorted unique ids, or a reason it cannot say."""
        body = self.get("/api/reports/checks")
        if "manifest_at" not in body:
            return {"error": [f"error: {body}"]}
        return {k: sorted(r["id"] for r in rows) for k, rows in body.items() if k != "manifest_at"}

    def lineage(self, unique_id, up, down, tests):
        """The names the canvas draws around a model, or a reason it cannot say."""
        query = {"id": unique_id, "up": up, "down": down, "tests": int(tests), "max": 3000}
        body = self.get("/api/lineage?" + urllib.parse.urlencode(query))
        if "nodes" not in body:
            return [f"error: {body}"]
        if body.get("truncated"):
            return ["error: capped at 3000 nodes"]
        return sorted(n["name"] for n in body["nodes"])

    def command_line(self, args):
        """The same answer from `--select` or `--selector`, which resolves and
        exits without serving (0057). Returns the names, or None and why."""
        env = dict(os.environ, DBT_EDITH_CONFIG_DIR=str(self.command_line_config))
        run = subprocess.run(
            [str(self.binary), str(self.project), "--manifest", str(self.manifest)] + args,
            capture_output=True, text=True, env=env, timeout=120,
        )
        if run.returncode != 0:
            return None, f"exit {run.returncode}: {run.stderr.strip()}"
        return sorted(line for line in run.stdout.splitlines() if line), None

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
        # The paths dbt's parse warned of, heard through its events, which
        # reach a callback whatever the log level.
        self.unused = []
        self.runner = None if EXECUTABLE else dbtRunner(callbacks=[self.heard])
        self.where = ["--project-dir", str(project), "--profiles-dir", str(project)]
        self.common = self.where + ["--target-path", str(work / "target"), "--log-path", str(work / "logs")]

    def heard(self, event):
        if event.info.name == "UnusedResourceConfigPath":
            self.unused = list(event.data.unused_config_paths)

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
        tenth of one, and the lineage asks a thousand times. An executable
        parses unquiet, since its warning is read off what it prints."""
        if EXECUTABLE:
            run = subprocess.run([EXECUTABLE, "parse"] + self.common, capture_output=True, text=True)
            if run.returncode != 0:
                raise RuntimeError(f"dbt parse failed: {(run.stdout + run.stderr)[-2000:]}")
            self.unused = unused_in_output(run.stdout + run.stderr)
            return
        manifest = self.invoke(["parse"])
        self.runner = dbtRunner(manifest=manifest)

    def ls(self, args):
        return sorted(self.invoke(["ls", "--output", "name"] + args) or [])


def search_name(node):
    """The name `dbt ls` prints for a model, and selects it alone by:
    `name.v2` for a versioned one, whose bare name takes every version."""
    if node.get("version") is None:
        return node["name"]
    return f"{node['name']}.v{node['version']}"


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


def compare_jobs(tally, edith, oracle):
    """Each step of the job fixtures against the `dbt ls` its button types."""
    # Kept in the settings and not in the project: the fixture is never written into.
    summary = edith.send("/api/jobs/sources", "PUT", {"sources": JOB_SOURCES, "place": "settings"})
    if "error" in summary:
        tally.fail(f"the job sources were refused: {summary['error']}")
        return
    problems = [s.get("error") for s in summary["sources"] if s.get("error")]
    problems += [f"{r['file']}:{r.get('line', 0)}: {r['message']}" for s in summary["sources"] for r in s.get("refused", [])]
    if problems or summary["steps"] != JOB_STEPS:
        tally.fail(f"the job fixtures read {summary['steps']} steps of {JOB_STEPS}: {problems}")
    agreed = asked = 0
    for n in range(summary["steps"]):
        step = edith.get(f"/api/jobs/step?n={n}")
        label = f"job step {step['shown']}"
        if step.get("unresolved"):
            if step["shown"] in JOBS_UNRESOLVED:
                tally.compared += 1
                print(f"PASS  {label} is unresolved: {step['unresolved']}")
            else:
                tally.fail(f"{label} was not resolved: {step['unresolved']}")
            continue
        if not step.get("ls"):
            continue
        args = shlex.split(step["ls"])
        # `dbt ls` and `--output name` are what Dbt.ls adds itself.
        if args[:2] != ["dbt", "ls"] or args[-2:] != ["--output", "name"]:
            tally.fail(f"{label}: an ls line of another shape, {step['ls']}")
            continue
        theirs = oracle.ls(args[2:-2])
        asked += 1
        agreed += tally.check(label, step["names"], theirs)
    verdict = "PASS" if agreed == asked and asked else "FAIL"
    print(f"{verdict}  the job fixtures: {agreed} of {asked} steps select what dbt ls lists")


def compare_project(tally, binary, project, work, expressions, every_model, jobs=False):
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
    asked = agreed = 0
    try:
        tally.check("config paths dbt_project.yml applies to nothing", edith.unused_paths(), sorted(oracle.unused))
        compare_checks_lists(tally, edith, parsed)
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
                _, error = edith.command_line(["--selector", name])
                if error and error.startswith("exit 1:"):
                    tally.compared += 1
                    print(f"PASS  --selector {name} is refused on the command line")
                else:
                    tally.fail(f"--selector {name} is refused on the command line, exit 1")
                continue
            if s.get("unsupported"):
                tally.fail(f"--selector {name} was refused: {s['unsupported']}")
                continue
            theirs = oracle.ls(["--selector", name])
            ours, error = edith.names(f"--selector {name}")
            tally.check(f"--selector {name}", ours if error is None else [f"error: {error}"], theirs)
            ours, error = edith.command_line(["--selector", name])
            asked += 1
            agreed += tally.check(f"--selector {name}, command line", ours if error is None else [f"error: {error}"], theirs, quiet=True)
        # The command line is given dbt's own arguments, so `--exclude` arrives
        # as a flag there where the box reads it inside the line.
        for line in expressions:
            theirs = oracle.ls(typed_args(line))
            ours, error = edith.names(line)
            tally.check(line, ours if error is None else [f"error: {error}"], theirs)
            ours, error = edith.command_line(typed_args(line))
            asked += 1
            agreed += tally.check(f"{line}, command line", ours if error is None else [f"error: {error}"], theirs, quiet=True)
        verdict = "PASS" if agreed == asked else "FAIL"
        print(f"{verdict}  the same {asked} selections on the command line: {agreed} agree with dbt")
        if edith.command_line_config.exists():
            tally.fail("the command line wrote settings")

        # dbt's `N+model+M` brings the tests of what it reaches, as the canvas
        # does with its tests box on; with the box off they are dropped by name.
        for up, down, with_tests in LINEAGE:
            box = "with tests" if with_tests else "without tests"
            agreed = 0
            for unique_id in sampled:
                name = search_name(parsed["nodes"][unique_id])
                theirs = oracle.ls(["--select", f"{up}+{name}+{down}"])
                if not with_tests:
                    theirs = [n for n in theirs if n not in tests]
                ours = edith.lineage(unique_id, up, down, with_tests)
                agreed += tally.check(f"lineage of {name}, {up} up and {down} down, {box}", ours, theirs, quiet=True)
            verdict = "PASS" if agreed == len(sampled) else "FAIL"
            print(f"{verdict}  lineage {up} up and {down} down, {box}: {agreed} of {len(sampled)} models")
        if jobs:
            compare_jobs(tally, edith, oracle)
    finally:
        edith.stop()
    return tally.compared - before


def has_freshness(threshold):
    """dbt-core's `bool(FreshnessThreshold)`: `warn_after` or `error_after`
    holds both a count and a period. `null`, `{}` and half a threshold are none."""
    if not isinstance(threshold, dict):
        return False
    return any(isinstance(threshold.get(k), dict) and threshold[k].get("count") is not None
               and threshold[k].get("period") is not None for k in ("warn_after", "error_after"))


def checks_in_manifest(manifest):
    """The Checks report's four lists from the manifest dbt wrote, by the
    rules of 0116 read off dbt's own fields: the project's own enabled models
    and sources, a test counting for the node it is attached to, a singular
    test for the nodes it selects from, a unit test for the model it targets."""
    project = manifest["metadata"]["project_name"]

    def own(node):
        return node["package_name"] == project and node.get("config", {}).get("enabled", True) is not False

    nodes = manifest["nodes"]
    tested = set()
    for n in nodes.values():
        if n["resource_type"] != "test":
            continue
        if n.get("test_metadata"):
            if n.get("attached_node"):
                tested.add(n["attached_node"])
        else:
            tested.update(n["depends_on"]["nodes"])
    for u in manifest.get("unit_tests", {}).values():
        tested.update(u["depends_on"]["nodes"])
    models = {k: n for k, n in nodes.items() if n["resource_type"] == "model" and own(n)}
    sources = {k: n for k, n in manifest["sources"].items() if own(n)}

    def read(source_id):
        return any(not c.startswith(("test.", "unit_test.")) for c in manifest["child_map"].get(source_id, []))

    return {
        "sources_without_freshness": sorted(k for k, n in sources.items() if not has_freshness(n.get("freshness"))),
        "models_without_test": sorted(k for k in models if k not in tested),
        "models_without_description": sorted(k for k, n in models.items() if not n["description"].strip()),
        "sources_not_read": sorted(k for k in sources if not read(k)),
    }


def compare_checks_lists(tally, edith, parsed, label=""):
    """The report's lists against the ones worked out from the manifest alone.
    Returns the latter, for a fixture that knows what it should hold."""
    theirs = checks_in_manifest(parsed)
    ours = edith.checks()
    for key, ids in theirs.items():
        tally.check(f"checks{label}: {key.replace('_', ' ')}", ours.get(key, ours.get("error")), ids)
    return theirs


def compare_unused(tally, binary, work):
    """Jaffle Shop with JAFFLE_UNUSED added: dbt's warning, path for path."""
    project = work / "project"
    shutil.copytree(JAFFLE, project)
    for name, text in JAFFLE_UNUSED_FILES.items():
        (project / name).parent.mkdir(parents=True, exist_ok=True)
        (project / name).write_text(text)
    written = (project / "dbt_project.yml").read_text()
    tops = [line.split(":")[0] for line in written.splitlines() if re.match(r"[A-Za-z]", line)]
    if tops[-1] != "models" or not written.endswith("\n"):
        tally.fail("the fixture's dbt_project.yml no longer ends with its models: block")
        return
    (project / "dbt_project.yml").write_text(written + JAFFLE_UNUSED)
    oracle = Dbt(project, work)
    oracle.parse()
    # Fewer than this means dbt read the fixture otherwise, not that all agrees.
    if len(oracle.unused) < 8:
        tally.fail(f"dbt warned of {len(oracle.unused)} unused config paths, not the 8 or more JAFFLE_UNUSED has")
    edith = Edith(binary, project, work / "target" / "manifest.json", work)
    try:
        tally.check("config paths dbt_project.yml applies to nothing", edith.unused_paths(), sorted(oracle.unused))
    finally:
        edith.stop()


def compare_checks(tally, binary, work):
    """Jaffle Shop with JAFFLE_CHECKS added: the four lists, id for id."""
    project = work / "project"
    shutil.copytree(JAFFLE, project)
    for name, text in JAFFLE_CHECKS.items():
        (project / name).parent.mkdir(parents=True, exist_ok=True)
        (project / name).write_text(text)
    oracle = Dbt(project, work)
    oracle.parse()
    manifest = work / "target" / "manifest.json"
    parsed = json.loads(manifest.read_text())
    edith = Edith(binary, project, manifest, work)
    try:
        found = compare_checks_lists(tally, edith, parsed)
    finally:
        edith.stop()
    # Written by hand from the rules: a project this reads wrongly would still
    # agree with itself.
    for key, ids in JAFFLE_CHECKS_FOUND.items():
        tally.check(f"the rules by hand: {key.replace('_', ' ')}", found[key], ids)


def main():
    os.environ.setdefault("DBT_SEND_ANONYMOUS_USAGE_STATS", "False")
    os.environ.setdefault("DO_NOT_TRACK", "1")
    every_model = "--every-model" in sys.argv[1:]
    binary = edith_binary()

    tally = Tally()
    skipped = None
    with tempfile.TemporaryDirectory(prefix="dbt-edith-compare-") as tmp:
        work = Path(tmp)
        print(f"{Dbt(work, work).version()}, on Jaffle Shop and Fivetran's Shopify {SHOPIFY_TAG}")

        print("== Jaffle Shop")
        jaffle = work / "jaffle" / "project"
        shutil.copytree(JAFFLE, jaffle)
        for name, sql in {**JAFFLE_INCREMENTAL, **JAFFLE_VERSIONS}.items():
            (jaffle / name).write_text(sql)
        compared = compare_project(tally, binary, jaffle, jaffle.parent, EXPRESSIONS, every_model, jobs=True)
        # Fewer than this means the fixture or the listing broke, not that all is well.
        if compared < len(EXPRESSIONS) + 10:
            tally.fail(f"only {compared} answers compared on Jaffle Shop")

        print("== Jaffle Shop, with config paths that apply to nothing")
        compare_unused(tally, binary, work / "unused")

        print("== Jaffle Shop, with sources, models and tests of every shape the checks tell apart")
        compare_checks(tally, binary, work / "checks")

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
