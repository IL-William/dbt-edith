#!/usr/bin/env python3
"""Tests for tools/sf_lineage.py, with nothing installed and no warehouse.

PyYAML and the Snowflake connector are replaced by fakes in sys.modules before
the script asks for them, so this runs on a bare interpreter. The fake connector
prints while connecting, the way single sign-on does: that is the failure serve
most needs to survive, since its stdout is the protocol dbt-edith reads.

Run from the repository root: python3 tools/test_sf_lineage.py
"""

import contextlib
import importlib.util
import io
import json
import os
import re
import sys
import tempfile
import types
import unittest
from datetime import datetime, timedelta, timezone
from pathlib import Path
from unittest import mock

spec = importlib.util.spec_from_file_location("sf_lineage", Path(__file__).resolve().parent / "sf_lineage.py")
sf = importlib.util.module_from_spec(spec)
spec.loader.exec_module(sf)

# JSON is YAML, so profiles written as JSON need no real parser.
FAKE_YAML = types.ModuleType("yaml")
FAKE_YAML.safe_load = lambda text: json.loads(text) if text.strip() else None

# GET_LINEAGE's column order: distance, source db, schema, object, column,
# target db, schema, object, column, source domain, target domain.
ROWS = [
    (1, "RAW", "CRM", "CUSTOMERS", "ID", "ANALYTICS", "STAGING", "STG_CUSTOMERS", "CUSTOMER_ID", "TABLE", "VIEW"),
    # Object level, no columns: dropped.
    (1, "ANALYTICS", "STAGING", "STG_CUSTOMERS", None, "ANALYTICS", "MARTS", "DIM_CUSTOMERS", None, "VIEW", "TABLE"),
    # A column pointing at itself: dropped.
    (2, "ANALYTICS", "MARTS", "DIM_CUSTOMERS", "CUSTOMER_ID", "ANALYTICS", "MARTS", "DIM_CUSTOMERS", "customer_id", "TABLE", "TABLE"),
]


# A week of one user's queries, in QUERY_HISTORY's terms plus the tag. Two
# queries start at each instant, so a page boundary can fall between them,
# and every fifth is one this script ran itself.
HISTORY = [
    {
        "query_id": f"01b7a2c4-0000-{i:04x}",
        "at": 1_700_000_000_000_000_000 + (i // 2) * 1_000_000_000,
        "ms": 1200 + i,
        "status": "FAILED_WITH_ERROR" if i % 7 == 0 else "SUCCESS",
        "error": "SQL compilation error" if i % 7 == 0 else None,
        "role": "TRANSFORMER" if i % 3 else "REPORTER",
        "warehouse": "TRANSFORMING" if i % 4 else None,
        "database": "ANALYTICS",
        "schema": "DBT_SOMEONE",
        "type": "SELECT",
        "text": f"select {i} from orders",
        "length": 21,
        "rows": i,
        "tag": "dbt-edith" if i % 5 == 0 else "",
    }
    for i in range(60)
]
HISTORY_COLS = ("query_id", "at", "ms", "status", "error", "role", "warehouse",
                "database", "schema", "type", "text", "length", "rows")


def history_rows(params):
    """What the history query answers, done here in Python: the fake runs no SQL."""
    rows = [h for h in HISTORY if h["tag"] != params["tag"]]
    if "role" in params:
        rows = [h for h in rows if h["role"].upper() == params["role"].upper()]
    if "at" in params:
        rows = [h for h in rows if (h["at"], h["query_id"]) < (params["at"], params["qid"])]
    rows.sort(key=lambda h: (h["at"], h["query_id"]), reverse=True)
    return [tuple(h[c] for c in HISTORY_COLS) for h in rows[: params["limit"]]]


# What the last-builds query answers, in its column order: node, target, status,
# started, completed, compiled, invocation, was_full_refresh, rows, message, command,
# cloud job, cloud run, last success. The fake runs no SQL, so these are the
# rows after the window and the QUALIFY, and they hold no secret column: the
# real tables also keep env_vars and adapter_response, which are never asked.
BUILD_ROWS = [
    ("model.jaffle.orders", "prod", "SUCCESS", datetime(2026, 10, 20, 3, 0, 5), datetime(2026, 10, 20, 3, 1, 9),
     datetime(2026, 10, 20, 3, 0, 8), "6f1c0c2e-0000-4000-8000-000000000001", "false", 120, None, "build", "4321", "98765",
     datetime(2026, 10, 20, 3, 1, 9)),
    ("model.jaffle.customers", "prod", "ERROR", datetime(2026, 10, 21, 3, 0, 5), None,
     datetime(2026, 10, 21, 3, 0, 40), "6f1c0c2e-0000-4000-8000-000000000002", "TRUE", None, "Database Error: object does not exist", "run", None, None,
     datetime(2026, 10, 14, 3, 1, 9)),
    ("model.jaffle.customers", "dev", "success", datetime(2026, 10, 21, 9, 30, 0, tzinfo=timezone(timedelta(hours=2))),
     datetime(2026, 10, 21, 9, 31, 0, tzinfo=timezone(timedelta(hours=2))), None,
     "6f1c0c2e-0000-4000-8000-000000000003",
     False, 7, "", "run", None, None, None),
]


def profiles(**dev_overrides):
    dev = {
        "type": "snowflake",
        "account": "xy12345",
        "user": "someone@example.com",
        "role": "transformer",
        "authenticator": "externalbrowser",
        "warehouse": "transforming",
        "database": "analytics",
        "schema": "dbt_someone",
    }
    dev.update(dev_overrides)
    dev = {k: v for k, v in dev.items() if v is not None}
    return {"shop": {"target": "dev", "outputs": {"dev": dev, "prod": dict(dev, role="reporter")}}}


class FakeCursor:
    def __init__(self, conn):
        self.conn = conn
        self.result = []

    def execute(self, sql, params=None):
        if "get_lineage" in sql:
            self.conn.executed.append(params)
            if params[0].startswith("X.Y.BROKEN"):
                raise RuntimeError("SQL compilation error:\nObject 'X.Y.BROKEN' does not exist")
            self.result = ROWS
        elif "current_organization_name" in sql:
            self.conn.asked_who += 1
            if self.conn.no_org:
                raise RuntimeError("SQL compilation error:\nUnknown function CURRENT_ORGANIZATION_NAME")
            self.result = [("MYORG", "MY_ACCOUNT", "SOMEONE", "TRANSFORMER")]
        elif "current_account_name" in sql:
            self.result = [("MY_ACCOUNT", "SOMEONE", "TRANSFORMER")]
        elif "identifier(%(exe)s)" in sql:
            self.conn.builds.append((sql, params))
            self.result = list(self.conn.build_rows)
        elif "query_history_by_user" in sql:
            self.conn.history.append((sql, params))
            self.result = history_rows(params)
        else:
            raise AssertionError(f"the fake does not know this query: {sql}")

    def fetchall(self):
        return self.result

    def close(self):
        pass


class FakeConnection:
    def __init__(self, no_org=False):
        self.executed = []
        self.history = []
        self.builds = []
        self.build_rows = BUILD_ROWS
        self.asked_who = 0
        self.no_org = no_org
        self.closed = False

    def cursor(self):
        return FakeCursor(self)

    def is_closed(self):
        return self.closed

    def close(self):
        self.closed = True


class ScriptTest(unittest.TestCase):
    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.project = Path(tmp.name) / "project"
        self.home = Path(tmp.name) / "home"
        self.project.mkdir()
        (self.home / ".dbt").mkdir(parents=True)
        (self.project / "dbt_project.yml").write_text(json.dumps({"name": "shop", "profile": "shop"}))
        (self.project / "profiles.yml").write_text(json.dumps(profiles()))

        cwd = os.getcwd()
        os.chdir(self.project)
        self.addCleanup(os.chdir, cwd)
        for patcher in (
            mock.patch.dict(os.environ),
            mock.patch.dict(sys.modules, {"yaml": FAKE_YAML, **self.fake_connector()}),
            mock.patch.object(Path, "home", return_value=self.home),
        ):
            patcher.start()
            self.addCleanup(patcher.stop)
        os.environ.pop("DBT_PROFILES_DIR", None)
        os.environ.pop("DBT_TARGET", None)

    def fake_connector(self):
        self.connections = []
        self.refuse_connection = None
        self.no_org = False
        module = types.ModuleType("snowflake.connector")

        def connect(**kwargs):
            print("Initiating login request with your identity provider. A browser window should have opened.")
            if self.refuse_connection:
                raise RuntimeError(self.refuse_connection)
            conn = FakeConnection(self.no_org)
            self.connections.append((kwargs, conn))
            return conn

        module.connect = connect
        package = types.ModuleType("snowflake")
        package.connector = module
        return {"snowflake": package, "snowflake.connector": module}

    def serve(self, *requests, target=None):
        """Runs serve in process. Returns the exit code, the stdout lines, and stderr."""
        stdin = io.StringIO("".join((r if isinstance(r, str) else json.dumps(r)) + "\n" for r in requests))
        out, err = io.StringIO(), io.StringIO()
        args = types.SimpleNamespace(profile="shop", target=target, profiles_dir=None, manifest=None)
        code = 0
        with mock.patch.object(sys, "stdin", stdin), contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
            try:
                sf.cmd_serve(args)
            except SystemExit as e:
                code = e.code
        return code, out.getvalue().splitlines(), err.getvalue()

    def replies(self, lines):
        """Every stdout line parsed, failing on the first one that is not JSON."""
        parsed = []
        for line in lines:
            try:
                parsed.append(json.loads(line))
            except json.JSONDecodeError:
                self.fail(f"stdout carries a line that is not JSON: {line!r}")
        return parsed

    def events(self, lines):
        return [r["event"] for r in self.replies(lines) if "event" in r]

    def answers(self, lines):
        return [r for r in self.replies(lines) if "id" in r]


class Serve(ScriptTest):
    def test_replies_stay_json_lines_when_sign_on_prints(self):
        code, lines, err = self.serve(
            {"id": 1, "relation": "ANALYTICS.MARTS.DIM_CUSTOMERS", "column": "CUSTOMER_ID", "direction": "UPSTREAM", "depth": 2},
            {"op": "quit"},
        )
        self.assertEqual(code, 0)
        named, ready = [r for r in self.replies(lines) if "event" in r]
        answer = self.answers(lines)[0]
        self.assertEqual(named, {"event": "profiles", "path": str((self.project / "profiles.yml").resolve())})
        self.assertEqual(
            ready,
            {
                "event": "ready",
                "profile": "shop",
                "target": "dev",
                "role": "transformer",
                "authenticator": "externalbrowser",
                "targets": [
                    {"name": "dev", "role": "transformer", "same_login": True},
                    {"name": "prod", "role": "reporter", "same_login": True},
                ],
            },
        )
        self.assertEqual(
            answer,
            {
                "id": 1,
                "rows": [
                    {
                        "from_rel": "RAW.CRM.CUSTOMERS",
                        "from_col": "id",
                        "to_rel": "ANALYTICS.STAGING.STG_CUSTOMERS",
                        "to_col": "customer_id",
                        "kind": "view",
                        "distance": 1,
                    }
                ],
            },
        )
        self.assertIn("Initiating login request", err, "the sign-on message should be on stderr")
        self.assertEqual(self.connections[0][1].executed, [("ANALYTICS.MARTS.DIM_CUSTOMERS.CUSTOMER_ID", "UPSTREAM", 2)])

    def test_the_connection_waits_for_the_first_request(self):
        code, lines, _ = self.serve({"op": "quit"})
        self.assertEqual(code, 0)
        self.assertEqual(self.events(lines), ["profiles", "ready"])
        self.assertEqual(self.connections, [], "switching on must not open a connection")

    def test_one_connection_serves_every_request(self):
        ask = {"relation": "ANALYTICS.MARTS.DIM_CUSTOMERS", "column": "CUSTOMER_ID"}
        code, lines, _ = self.serve({"id": 1, **ask}, {"id": 2, **ask, "direction": "downstream"})
        self.assertEqual(code, 0)
        self.assertEqual([r["id"] for r in self.answers(lines)], [1, 2])
        self.assertEqual(len(self.connections), 1)
        self.assertEqual([p[1] for p in self.connections[0][1].executed], ["UPSTREAM", "DOWNSTREAM"])

    def test_direction_is_checked_and_depth_kept_in_range(self):
        ask = {"relation": "ANALYTICS.MARTS.DIM_CUSTOMERS", "column": "CUSTOMER_ID"}
        code, lines, _ = self.serve(
            {"id": 1, **ask, "direction": "SIDEWAYS"},
            {"id": 2, **ask, "depth": 9},
            {"id": 3, **ask, "depth": 0},
            {"id": 4, **ask, "depth": "deep"},
            {"id": 5, "column": "CUSTOMER_ID"},
        )
        self.assertEqual(code, 0)
        by_id = {r["id"]: r for r in self.answers(lines)}
        self.assertIn("UPSTREAM or DOWNSTREAM", by_id[1]["error"])
        self.assertIn("rows", by_id[2])
        self.assertIn("rows", by_id[3])
        self.assertIn("whole number", by_id[4]["error"])
        self.assertIn("relation and a column", by_id[5]["error"])
        self.assertEqual([p[2] for p in self.connections[0][1].executed], [5, 1])

    def test_a_failing_query_is_a_reply_and_the_next_one_still_runs(self):
        code, lines, _ = self.serve(
            {"id": 1, "relation": "X.Y.BROKEN", "column": "C"},
            {"id": 2, "relation": "ANALYTICS.MARTS.DIM_CUSTOMERS", "column": "CUSTOMER_ID"},
        )
        self.assertEqual(code, 0)
        first, second = self.answers(lines)
        self.assertEqual(first, {"id": 1, "error": "SQL compilation error:", "phase": "query"})
        self.assertEqual(len(second["rows"]), 1)

    def test_noise_on_stdin_is_ignored(self):
        code, lines, _ = self.serve("not json", "[1, 2]", "", {"op": "quit"})
        self.assertEqual(code, 0)
        self.assertEqual(self.answers(lines), [])

    def test_a_refused_connection_blames_the_profile_and_a_bad_query_does_not(self):
        ask = {"relation": "ANALYTICS.MARTS.DIM_CUSTOMERS", "column": "CUSTOMER_ID"}
        self.refuse_connection = "251005: User is empty, but it must be provided"
        code, lines, _ = self.serve({"id": 1, **ask}, {"id": 2, **ask, "direction": "SIDEWAYS"})
        self.assertEqual(code, 0)
        by_id = {r["id"]: r for r in self.answers(lines)}
        self.assertEqual(by_id[1]["phase"], "connect")
        self.assertTrue(by_id[1]["error"].startswith("251005"))
        # A request this script refuses on its own is nobody's profile problem.
        self.assertEqual(by_id[2]["phase"], "request")

    def test_the_file_is_named_before_it_is_read(self):
        elsewhere = self.home / "elsewhere"
        elsewhere.mkdir()
        (elsewhere / "profiles.yml").write_text(json.dumps(profiles()))
        os.environ["DBT_PROFILES_DIR"] = str(elsewhere)
        _, lines, _ = self.serve({"op": "quit"})
        named = self.replies(lines)[0]
        self.assertEqual(named, {"event": "profiles", "path": str((elsewhere / "profiles.yml").resolve())})

    def test_a_missing_connector_fails_before_ready(self):
        sys.modules["snowflake"] = None
        sys.modules["snowflake.connector"] = None
        code, lines, err = self.serve({"op": "quit"})
        self.assertEqual(code, 2)
        self.assertEqual(self.events(lines), ["profiles"], "ready must not follow a setup that cannot work")
        self.assertIn("snowflake-connector-python is not installed", err)


class History(ScriptTest):
    def ask(self, *requests, **kwargs):
        code, lines, _ = self.serve(*requests, **kwargs)
        self.assertEqual(code, 0)
        return {r["id"]: r for r in self.answers(lines)}

    def test_a_page_is_the_newest_twenty_with_who_and_where(self):
        answer = self.ask({"id": 1, "op": "history", "role": "*"})[1]
        self.assertEqual(answer["session"], {"org": "MYORG", "account": "MY_ACCOUNT", "user": "SOMEONE", "role": "TRANSFORMER"})
        self.assertEqual(len(answer["rows"]), 20)
        self.assertTrue(answer["more"])
        newest = answer["rows"][0]
        self.assertEqual(
            newest,
            {
                "query_id": "01b7a2c4-0000-003b",
                "at": str(HISTORY[59]["at"]),
                "start": "2023-11-14T22:13:49.000Z",
                "ms": 1259,
                "status": "success",
                "role": "TRANSFORMER",
                "warehouse": "TRANSFORMING",
                "database": "ANALYTICS",
                "schema": "DBT_SOMEONE",
                "type": "SELECT",
                "text": "select 59 from orders",
                "length": 21,
                "rows": 59,
            },
        )
        self.assertNotIn("error", newest, "an empty column is left out")
        sql, params = self.connections[0][1].history[0]
        self.assertIn("result_limit => %(scan)s", sql)
        self.assertEqual((params["scan"], params["limit"]), (10000, 21), "the most rows scanned, one past the page")
        self.assertNotIn("role", params, "* reads every role")

    def test_the_scripts_own_queries_are_tagged_and_left_out(self):
        self.ask({"id": 1, "op": "history", "role": "*"})
        kwargs, conn = self.connections[0]
        self.assertEqual(kwargs["session_parameters"], {"QUERY_TAG": "dbt-edith"})
        self.assertEqual(conn.history[0][1]["tag"], "dbt-edith")

    def test_the_role_is_the_targets_unless_another_is_named(self):
        answers = self.ask({"id": 1, "op": "history"}, {"id": 2, "op": "history", "role": "reporter"})
        roles = [params.get("role") for _, params in self.connections[0][1].history]
        self.assertEqual(roles, ["transformer", "reporter"])
        self.assertEqual({r["role"] for r in answers[2]["rows"]}, {"REPORTER"}, "roles compare without case")

    def test_a_target_with_no_role_reads_the_sessions(self):
        (self.project / "profiles.yml").write_text(json.dumps(profiles(role=None)))
        self.ask({"id": 1, "op": "history"}, {"id": 2, "op": "history"})
        conn = self.connections[0][1]
        self.assertEqual(conn.history[0][1]["role"], "TRANSFORMER")
        self.assertEqual(conn.asked_who, 1, "who the session is is asked once per connection")

    def test_an_account_with_no_organization_still_has_a_history(self):
        self.no_org = True
        answer = self.ask({"id": 1, "op": "history", "role": "*"})[1]
        self.assertEqual(answer["session"], {"account": "MY_ACCOUNT", "user": "SOMEONE", "role": "TRANSFORMER"})
        self.assertEqual(len(answer["rows"]), 20)

    def test_the_session_alone_is_who_and_where_asked_once(self):
        answers = self.ask({"id": 1, "op": "session"}, {"id": 2, "op": "history", "role": "*"}, {"id": 3, "op": "session"})
        self.assertEqual(answers[1], {"id": 1, "session": {"org": "MYORG", "account": "MY_ACCOUNT", "user": "SOMEONE", "role": "TRANSFORMER"}})
        self.assertEqual(answers[3]["session"], answers[1]["session"])
        conn = self.connections[0][1]
        self.assertEqual(conn.asked_who, 1, "the history reuses what the session op asked")
        self.assertEqual(len(conn.history), 1, "the session op reads no history")

    def test_bad_requests_are_refused_before_connecting(self):
        answers = self.ask(
            {"id": 1, "op": "history", "role": ""},
            {"id": 2, "op": "history", "role": "x" * 256},
            {"id": 3, "op": "history", "role": "a\nb"},
            {"id": 4, "op": "history", "before": {"at": "-1", "query_id": "01b7"}},
            {"id": 5, "op": "history", "before": {"at": "1", "query_id": "x' or 1=1"}},
            {"id": 6, "op": "history", "before": "yesterday"},
            {"id": 7, "op": "drop"},
        )
        self.assertEqual({a["phase"] for a in answers.values()}, {"request"})
        self.assertIn("role name", answers[1]["error"])
        self.assertIn("epoch nanoseconds", answers[4]["error"])
        self.assertIn("query id", answers[5]["error"])
        self.assertIn("unknown op", answers[7]["error"])
        self.assertEqual(self.connections, [])

    def test_lineage_named_by_its_op_is_unchanged(self):
        answer = self.ask({"id": 1, "op": "lineage", "relation": "ANALYTICS.MARTS.DIM_CUSTOMERS", "column": "CUSTOMER_ID"})[1]
        self.assertEqual(len(answer["rows"]), 1)


class Builds(ScriptTest):
    EXE = "ANALYTICS_META.DBT_ARTIFACTS.MODEL_EXECUTIONS"
    INV = '"ANALYTICS_META"."DBT_ARTIFACTS"."INVOCATIONS"'

    def ask(self, *requests):
        code, lines, _ = self.serve(*requests)
        self.assertEqual(code, 0)
        return {r["id"]: r for r in self.answers(lines)}

    def request(self, **kwargs):
        return {"id": 1, "op": "builds", "executions": self.EXE, "invocations": self.INV, **kwargs}

    def test_a_run_is_what_the_page_reads_and_nothing_else(self):
        answer = self.ask(self.request())[1]
        self.assertFalse(answer["cut"])
        orders, failed, dev = answer["rows"]
        self.assertEqual(
            orders,
            {
                "node": "model.jaffle.orders",
                "target": "prod",
                "status": "success",
                "started": "2026-10-20T03:00:05Z",
                "completed": "2026-10-20T03:01:09Z",
                "compiled": "2026-10-20T03:00:08Z",
                "invocation": "6f1c0c2e-0000-4000-8000-000000000001",
                "rows": 120,
                "command": "build",
                "cloud_job": "4321",
                "cloud_run": "98765",
                "last_success": "2026-10-20T03:01:09Z",
            },
        )
        self.assertNotIn("full_refresh", orders, "false is left out")
        self.assertEqual(failed["status"], "error")
        self.assertTrue(failed["full_refresh"])
        self.assertNotIn("completed", failed, "a run that never completed has no end")
        self.assertEqual(failed["compiled"], "2026-10-21T03:00:40Z", "but it says when the model began")
        self.assertNotIn("compiled", dev, "an empty time is left out")
        self.assertEqual(failed["last_success"], "2026-10-14T03:01:09Z", "the last success survives a later failure")
        self.assertEqual(failed["message"], "Database Error: object does not exist")
        self.assertEqual(dev["started"], "2026-10-21T07:30:00Z", "a zoned time is written in UTC")
        self.assertNotIn("message", dev, "an empty message is left out")
        self.assertNotIn("last_success", dev)

    def test_the_tables_reach_the_sql_only_as_binds(self):
        self.ask(self.request())
        sql, params = self.connections[0][1].builds[0]
        self.assertEqual((params["exe"], params["inv"]), (self.EXE, self.INV))
        self.assertIn("from identifier(%(exe)s) e", sql)
        self.assertIn("join identifier(%(inv)s) i", sql)
        for relation in ("ANALYTICS_META", "MODEL_EXECUTIONS", "INVOCATIONS"):
            self.assertNotIn(relation, sql, "a name is never formatted into the statement")
        self.assertEqual((params["days"], params["limit"], params["text"]), (90, 50001, 600), "the window, one row past the cap, the message cut")

    def test_the_secret_columns_are_never_selected(self):
        self.ask(self.request())
        sql = self.connections[0][1].builds[0][0].lower()
        for column in ("env_vars", "dbt_vars", "dbt_custom_envs", "invocation_args", "adapter_response", "target_schema", "dbt_cloud_run_reason"):
            self.assertNotIn(column, sql)
        self.assertIn("partition by e.node_id, i.target_name", sql, "one row per model and target")
        self.assertIn("max(iff(lower(e.status) = 'success', coalesce(e.query_completed_at, e.compile_started_at, e.run_started_at)", sql,
                      "the last success is dated by the same clock as the last attempt")
        self.assertIn("coalesce(e.query_completed_at, e.compile_started_at, e.run_started_at) desc, e.command_invocation_id desc", sql,
                      "the last attempt is when the model ran, not when its invocation started, and two runs ending together still pick one")

    def test_a_success_is_matched_whatever_its_case(self):
        self.ask(self.request())
        sql = self.connections[0][1].builds[0][0]
        self.assertIn("lower(e.status) = 'success'", sql, "the engine spells the status as it likes, and Snowflake compares case-sensitively")

    def test_the_page_names_the_same_window_as_the_script(self):
        # "No run in N days" is said by the page about the window the script reads.
        app = (Path(__file__).resolve().parent.parent / "web" / "app.js").read_text(encoding="utf-8")
        found = re.search(r"function buildsWindowDays\(\) \{\s*return (\d+);", app)
        self.assertIsNotNone(found, "buildsWindowDays is a constant in web/app.js")
        self.assertEqual(int(found.group(1)), sf.BUILDS_DAYS, "the page and the script read the same number of days")

    def test_a_table_past_the_cap_says_it_was_cut(self):
        many = [("model.jaffle.m%d" % i, "prod", "success", datetime(2026, 10, 20), None, None, "x", None, None, None, None, None, None, None)
                for i in range(sf.BUILDS_MAX + 1)]
        cur = FakeCursor(FakeConnection())
        cur.conn.build_rows = many
        rows, cut = sf.last_builds(cur, self.EXE, self.INV)
        self.assertEqual((len(rows), cut), (sf.BUILDS_MAX, True))
        cur.conn.build_rows = many[:3]
        rows, cut = sf.last_builds(cur, self.EXE, self.INV)
        self.assertEqual((len(rows), cut), (3, False))

    def test_a_relation_that_is_not_three_parts_is_refused_before_connecting(self):
        answers = self.ask(
            self.request(id=1, executions="MODEL_EXECUTIONS"),
            self.request(id=2, executions="a.b"),
            self.request(id=3, executions="a.b.c.d"),
            self.request(id=4, executions="a.b.c; drop table x"),
            self.request(id=5, invocations="a.b.c' or 1=1 --"),
            self.request(id=6, invocations=None),
            self.request(id=7, executions="a.b." + "c" * 401),
            self.request(id=8, invocations='a."b\nc".d'),
            self.request(id=9, executions=""),
        )
        self.assertEqual({a["phase"] for a in answers.values()}, {"request"})
        self.assertIn("executions must be a relation", answers[1]["error"])
        self.assertIn("invocations must be a relation", answers[5]["error"])
        self.assertEqual(self.connections, [])

    def test_a_quoted_part_may_hold_a_dot_and_a_quote(self):
        self.assertEqual(sf.builds_request(self.request(executions='"my.db"."sch""ema".events', invocations="a.b.c")),
                         ('"my.db"."sch""ema".events', "a.b.c"))

    def test_a_failing_read_is_a_reply_with_its_phase(self):
        self.refuse_connection = "Incorrect username or password was specified."
        answer = self.ask(self.request())[1]
        self.assertEqual(answer["phase"], "connect")
        self.assertIn("Incorrect username", answer["error"])


class HistoryPages(unittest.TestCase):
    def pages(self, role):
        cur = FakeCursor(FakeConnection())
        before, seen, sizes = None, [], []
        while True:
            rows, more = sf.query_history(cur, role, before)
            sizes.append(len(rows))
            seen.extend(r["query_id"] for r in rows)
            if not more:
                return seen, sizes
            last = rows[-1]
            before = sf.history_request({"before": {"at": last["at"], "query_id": last["query_id"]}})[1]

    def test_paging_reaches_every_query_once_across_shared_starts(self):
        seen, sizes = self.pages(None)
        mine = sorted((h for h in HISTORY if not h["tag"]), key=lambda h: (h["at"], h["query_id"]), reverse=True)
        self.assertEqual(seen, [h["query_id"] for h in mine])
        self.assertEqual(sizes, [20, 20, 8])

    def test_a_role_pages_on_its_own_rows(self):
        seen, sizes = self.pages("reporter")
        self.assertEqual(len(seen), len([h for h in HISTORY if not h["tag"] and h["role"] == "REPORTER"]))
        self.assertEqual(len(seen), len(set(seen)))
        self.assertEqual(sizes, [16])

    def test_the_start_is_utc_to_the_millisecond(self):
        row = sf.history_row(("q", 1_700_000_000_123_456_789, None, "RUNNING", None, "R", None, None, None, "SELECT", "", 0, None))
        self.assertEqual(row, {"query_id": "q", "at": "1700000000123456789", "start": "2023-11-14T22:13:20.123Z",
                               "status": "running", "role": "R", "type": "SELECT", "length": 0})


class Targets(unittest.TestCase):
    def test_each_snowflake_target_says_whether_it_shares_the_login(self):
        base = {"type": "snowflake", "account": "XY12345", "user": "Someone@Example.com", "role": "transformer"}
        outputs = {
            "dev": base,
            "prod": dict(base, account="xy12345", user="someone@example.com", role="reporter"),
            "ci": dict(base, user="ci_service", role="ci_runner"),
            "dyn": dict(base, role="{{ env_var('SHOP_ROLE') }}"),
            "envuser": dict(base, user="{{ env_var('SHOP_USER') }}"),
            "plain": {k: v for k, v in base.items() if k != "role"},
            "local": {"type": "duckdb", "path": "shop.duckdb"},
        }
        self.assertEqual(
            sf.target_roles(outputs, "dev"),
            [
                {"name": "dev", "role": "transformer", "same_login": True},
                {"name": "prod", "role": "reporter", "same_login": True},
                {"name": "ci", "role": "ci_runner", "same_login": False, "why": "it signs in as another user or account"},
                {"name": "dyn", "why": "its role is a Jinja expression", "same_login": True},
                {"name": "envuser", "role": "transformer", "same_login": False, "why": "its account or user is a Jinja expression"},
                {"name": "plain", "why": "it names no role, so it signs in with the user's default one", "same_login": True},
            ],
        )
        self.assertNotIn("SHOP_ROLE", json.dumps(sf.target_roles(outputs, "dev")))


class Profile(ScriptTest):
    def write(self, directory, doc):
        (directory / "profiles.yml").write_text(json.dumps(doc))

    def test_jinja_is_refused_without_showing_it(self):
        self.write(self.project, profiles(user="{{ env_var('SHOP_SNOWFLAKE_USER') }}"))
        code, lines, err = self.serve({"op": "quit"})
        self.assertEqual(code, 2)
        self.assertEqual(self.events(lines), ["profiles"], "the file is named even when reading it fails")
        self.assertIn("user of target 'dev'", err)
        self.assertNotIn("SHOP_SNOWFLAKE_USER", err)

    def test_the_project_comes_before_home_and_the_variable_before_both(self):
        self.write(self.project, profiles(role="from_project"))
        self.write(self.home / ".dbt", profiles(role="from_home"))
        elsewhere = self.home / "elsewhere"
        elsewhere.mkdir()
        self.write(elsewhere, profiles(role="from_variable"))

        role = lambda: sf.load_profile("shop", None, None)[0]["role"]  # noqa: E731
        self.assertEqual(role(), "from_project")
        os.environ["DBT_PROFILES_DIR"] = str(elsewhere)
        self.assertEqual(role(), "from_variable")
        del os.environ["DBT_PROFILES_DIR"]
        (self.project / "profiles.yml").unlink()
        self.assertEqual(role(), "from_home")

    @unittest.skipIf(os.name == "nt", "a link needs a privilege on Windows")
    def test_a_project_profile_linking_out_is_not_read(self):
        # The project's profiles.yml names a file of the user's, here a key
        # that parses as a string: on main its characters ended up listed in
        # the error the page shows (0094).
        key = self.home / "id_ed25519"
        key.write_text('"-----BEGIN OPENSSH PRIVATE KEY-----"')
        (self.project / "profiles.yml").unlink()
        (self.project / "profiles.yml").symlink_to(key)
        code, lines, err = self.serve({"op": "quit"})
        self.assertEqual(code, 2)
        self.assertIn("links to a file outside it", err)
        self.assertNotIn("PRIVATE", err)
        self.assertNotIn("O, P, E, N", err)

        # One the user keeps outside the project is followed, as dbt does.
        self.write(self.home, profiles(role="from_dotfiles"))
        (self.project / "profiles.yml").unlink()
        (self.home / ".dbt" / "profiles.yml").symlink_to(self.home / "profiles.yml")
        self.assertEqual(sf.load_profile("shop", None, None)[0]["role"], "from_dotfiles")

        # And one that stays in the project is a file of it.
        (self.project / "config").mkdir()
        self.write(self.project / "config", profiles(role="from_config"))
        (self.project / "profiles.yml").symlink_to("config/profiles.yml")
        self.assertEqual(sf.load_profile("shop", None, None)[0]["role"], "from_config")

    @unittest.skipIf(os.name == "nt", "a link needs a privilege on Windows")
    def test_a_project_profile_is_judged_as_dbt_edith_judges_it(self):
        # The same cases as profiles::tests in dbt-edith, which must agree
        # with this script on every profile (0049, 0094).
        os.environ["DBT_PROFILES_DIR"] = "."
        (self.project / "config").mkdir()
        self.write(self.project / "config", profiles())
        local = self.project / "profiles.yml"

        def refused():
            local_err = io.StringIO()
            with contextlib.redirect_stderr(local_err):
                try:
                    sf.profiles_path(None)
                except SystemExit:
                    return "links to a file outside it" in local_err.getvalue()
            return False

        def link(target):
            if local.is_symlink() or local.exists():
                local.unlink()
            local.symlink_to(target)

        link(self.home / "missing.yml")
        self.assertTrue(refused(), "a link to nothing outside")
        link("nodir/../config/profiles.yml")
        self.assertTrue(refused(), "a `..` after a folder that is not there")
        for i in range(2, 9):
            (self.project / f"c{i}").symlink_to(f"c{i + 1}" if i < 8 else "config/none.yml")
        link("c2")
        self.assertFalse(refused(), "eight links to a file not there yet")
        (self.project / "c1").symlink_to("c2")
        link("c1")
        self.assertTrue(refused(), "nine")
        (self.project / "hop").symlink_to("config/profiles.yml")
        link("hop")
        self.assertFalse(refused(), "two links that stay in the project")
        self.assertEqual(sf.load_profile("shop", None, None)[1], "dev")

    def test_a_profile_that_is_no_mapping_is_not_listed_in_the_error(self):
        (self.home / ".dbt" / "profiles.yml").write_text('"hunter2"')
        (self.project / "profiles.yml").unlink()
        code, lines, err = self.serve({"op": "quit"})
        self.assertEqual(code, 2)
        self.assertIn("is not a dbt profile", err)
        self.assertNotIn("h, u, n", err)

    def test_dbt_target_is_read_the_way_dbt_reads_it(self):
        self.assertEqual(sf.load_profile("shop", None, None)[1], "dev")
        os.environ["DBT_TARGET"] = "prod"
        kwargs, target = sf.load_profile("shop", None, None)
        self.assertEqual((target, kwargs["role"]), ("prod", "reporter"))
        self.assertEqual(sf.load_profile("shop", "dev", None)[1], "dev", "an explicit --target wins")


class Rows(unittest.TestCase):
    def test_dump_still_names_dbt_nodes(self):
        known = {"RAW.CRM.CUSTOMERS": "source.shop.crm.customers"}
        edges = sf.rows_to_edges(ROWS, lambda rel: known.get(rel, "rel:" + rel.lower()))
        self.assertEqual(
            edges,
            [
                {
                    "from": "source.shop.crm.customers",
                    "from_col": "id",
                    "to": "rel:analytics.staging.stg_customers",
                    "to_col": "customer_id",
                    "kind": "view",
                }
            ],
        )


if __name__ == "__main__":
    unittest.main(verbosity=1)
