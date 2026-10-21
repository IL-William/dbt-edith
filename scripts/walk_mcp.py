#!/usr/bin/env python3
"""The MCP server against the command line, on Jaffle Shop (0110).

Jaffle Shop is copied from tests/fixtures/ and parsed by dbt. `dbt-edith --mcp`
is started over the copy and driven through its stdin and stdout, the way a
client does: `initialize`, `notifications/initialized`, `tools/list`, then the
questions as `tools/call`. Each question is also asked of the command line, on
the same copy and with the same HOME, and the two have to agree:

  - the first text block is the command line's stdout byte for byte, the
    second its stderr, an empty one left out;
  - `isError` is the exit code not being 0;
  - the ages the answers print, `(N s ago)` and `"age_s"`, and the time collin
    took, are the one thing the clock moves, so they are masked on both sides.

The questions are the five modes with their flags, a refusal, an empty answer,
a partial one, the cache moved away, and a manifest that is not there, whose
text legitimately differs: the command line says to pass --manifest and a
line cannot. `--column` is answered from an invented cache of three edges
written beside the manifest; `--run-collin` from a stand-in collin placed on
the PATH of both sides, where the server must change exactly the two files
collin writes. Besides that, the session must leave the project and the
folder of the manifest as they were, the configuration folder empty, every
line the server wrote one JSON-RPC message, and nothing on its stderr.

dbt runs in this process, through `dbtRunner`, so it needs dbt-core and
dbt-duckdb importable by the Python running the script, or DBT naming a dbt
executable. Without them it skips itself, exit 2, like the other checks that
need something installed (0013). dbt-edith is built here first and run from
where cargo put it, or is whatever DBT_EDITH names.

Exit 0 when the server and the command line agree, 1 when they do not, 2 when
skipped.
"""

import json
import os
import queue
import re
import shlex
import shutil
import stat
import subprocess
import sys
import tempfile
import threading
from pathlib import Path

# Importing it skips this script, exit 2, wherever dbt cannot be had.
import compare_with_dbt as C

WAIT = 120

# What the clock moves between two answers.
AGES = re.compile(r"\(\d+ (?:s|min|h|days) ago\)")
AGE_S = re.compile(r'"age_s":\d+')
TOOK = re.compile(r"\bin \d+ ms\b")


# The second a cache was written, which a run of collin sets to the moment it
# ran, a second apart for the server and the command line.
WRITTEN = re.compile(r"written \d{4}-\d\d-\d\dT[\d:]+Z")


def mask(text, ran=False):
    text = TOOK.sub("in N ms", AGE_S.sub('"age_s":N', AGES.sub("(N ago)", text)))
    return WRITTEN.sub("written T", text) if ran else text


# Written beside the manifest, where the answers to --column read them. The
# edges are invented: three columns of Jaffle Shop, one fed by another.
CACHE = {
    "version": 1,
    "source": "collin",
    "target": "dev",
    "generated_at": "2026-01-01T00:00:00Z",
    "producer": "collin 0.10.0",
    "edges": [
        {"from": "model.jaffle_shop.stg_payments", "from_col": "amount", "to": "model.jaffle_shop.orders", "to_col": "amount", "kind": "rename"},
        {"from": "model.jaffle_shop.stg_payments", "from_col": "amount", "to": "model.jaffle_shop.customers", "to_col": "customer_lifetime_value", "kind": "transform"},
        {"from": "model.jaffle_shop.stg_orders", "from_col": "order_id", "to": "model.jaffle_shop.orders", "to_col": "order_id", "kind": "passthrough"},
    ],
}
CATALOG = {
    "nodes": {
        "model.jaffle_shop.orders": {"columns": {"order_id": {"name": "order_id", "type": "BIGINT", "index": 1}}},
    },
}

# What a stand-in for collin writes, the same as the cache above, and its
# report: collin 0.10.0 saying it read the SQL of every model.
STAND_IN = """#!/bin/sh
[ "$1" = --version ] && echo 'collin 0.10.0' && exit 0
out=''
report=''
while [ $# -gt 0 ]; do
  [ "$1" = --out ] && out="$2"
  [ "$1" = --report ] && report="$2"
  shift
done
cat > "$out" <<'JSON'
%s
JSON
echo '{"totals":{"models":5,"parsed":5},"models":[]}' > "$report"
""" % json.dumps(CACHE)

# (line, whether the command line is expected to exit 0, 1 or 2). The command
# line parses the words as a shell would; the tool is given the line as it is.
QUESTIONS = [
    ("--overview", 0),
    ("--overview --depth 1 --output json", 0),
    ("--describe stg_payments", 0),
    ("--describe orders,raw_orders --output json", 0),
    ("--describe orders --sql", 0),
    ("--describe orders,nope", 1),
    ("--describe nope", 1),
    ("--describe ,", 1),
    ("-s 'stg_orders+1' --exclude resource_type:test --output compact", 0),
    ("-s '+customers'", 0),
    ("-s 'path:models/staging/*.sql' --output json", 0),
    ("--selector orders_eager", 0),
    ("-s 'tag:nothing_has_this_tag'", 0),
    ("-s 'nosuchmethod:orders'", 1),
    ("--column stg_payments.amount --down all --output compact", 0),
    ("--column orders.amount --up all --output json", 0),
    ("--column stg_payments.amount --up 1 --down 1", 0),
    ("--column orders.nope", 1),
]


class Server:
    """dbt-edith --mcp, spoken to the way a client speaks to it."""

    def __init__(self, binary, project, manifest, env, work, name):
        self.stderr_path = work / f"{name}-stderr.log"
        self.log = open(self.stderr_path, "w")
        self.proc = subprocess.Popen(
            [str(binary), "--mcp", str(project), "--manifest", str(manifest)],
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=self.log, env=env, cwd=work,
        )
        self.lines = queue.Queue()
        self.written = []
        self.next_id = 0
        threading.Thread(target=self._read, daemon=True).start()

    def _read(self):
        for raw in self.proc.stdout:
            self.lines.put(raw)
        self.lines.put(None)

    def _send(self, message):
        self.proc.stdin.write((json.dumps(message) + "\n").encode())
        self.proc.stdin.flush()

    def notify(self, method, params=None):
        self._send({"jsonrpc": "2.0", "method": method, **({"params": params} if params is not None else {})})

    def request(self, method, params=None):
        self.next_id += 1
        self._send({"jsonrpc": "2.0", "id": self.next_id, "method": method, **({"params": params} if params is not None else {})})
        raw = self.lines.get(timeout=WAIT)
        if raw is None:
            raise RuntimeError("the server ended while a request was waiting")
        self.written.append(raw)
        message = json.loads(raw)
        if message.get("id") != self.next_id:
            raise RuntimeError(f"answered another id: {message}")
        return message

    def call(self, line):
        return self.request("tools/call", {"name": "edith", "arguments": {"line": line}})

    def close(self):
        """Closes stdin and waits for the exit: the way a client ends a session."""
        self.proc.stdin.close()
        try:
            code = self.proc.wait(timeout=30)
        except subprocess.TimeoutExpired:
            self.proc.kill()
            code = None
        while True:
            raw = self.lines.get(timeout=WAIT)
            if raw is None:
                break
            self.written.append(raw)
        self.log.close()
        return code


def tree(*roots):
    """Every file under the folders, with its size and modification time."""
    found = {}
    for root in roots:
        for path in sorted(root.rglob("*")):
            if path.is_file():
                st = path.stat()
                found[str(path)] = (st.st_size, st.st_mtime_ns)
    return found


def command_line(binary, project, manifest, line, env, work):
    """What the command line says to the same question, and how it ends."""
    run = subprocess.run(
        [str(binary), str(project), "--manifest", str(manifest)] + shlex.split(line),
        capture_output=True, text=True, env=env, cwd=work, timeout=WAIT,
    )
    return run.stdout, run.stderr, run.returncode


def blocks_of(response):
    result = response.get("result")
    if not result:
        return None, None
    return [b["text"] for b in result["content"] if b["type"] == "text"], result["isError"]


class Walk:
    def __init__(self):
        self.checked = 0
        self.failed = 0

    def check(self, ok, label, why=""):
        self.checked += 1
        if ok:
            print(f"PASS  {label}")
        else:
            self.failed += 1
            print(f"FAIL  {label}")
            if why:
                print("        " + why.replace("\n", "\n        ")[:3000])
        return ok

    def same(self, label, server, cli, ran=False):
        """The server's answer to a line against the command line's."""
        blocks, is_error = blocks_of(server)
        if blocks is None:
            return self.check(False, label, f"no result: {server}")
        stdout, stderr, code = cli
        want = [t for t in (mask(stdout, ran), mask(stderr, ran)) if t]
        got = [mask(t, ran) for t in blocks]
        ok = got == want and is_error == (code != 0)
        return self.check(ok, label, f"exit {code}, isError {is_error}\nserver: {got!r}\ncli:    {want!r}")


def main():
    os.environ.setdefault("DBT_SEND_ANONYMOUS_USAGE_STATS", "False")
    os.environ.setdefault("DO_NOT_TRACK", "1")
    binary = C.edith_binary()
    walk = Walk()
    with tempfile.TemporaryDirectory(prefix="dbt-edith-mcp-") as tmp:
        work = Path(tmp)
        print(f"{C.Dbt(work, work).version()}, on Jaffle Shop")
        project = work / "jaffle" / "project"
        shutil.copytree(C.JAFFLE, project)
        C.Dbt(project, work / "jaffle").parse()
        target = work / "jaffle" / "target"
        manifest = target / "manifest.json"
        (target / "column_lineage.collin.json").write_text(json.dumps(CACHE))
        (target / "catalog.json").write_text(json.dumps(CATALOG))

        # A home of its own, empty, for the server and the command line alike:
        # neither may write a setting, and the one-shot mode reads none.
        home = work / "home"
        home.mkdir()
        env = {k: v for k, v in os.environ.items() if k != "DBT_EDITH_CONFIG_DIR"}
        env.update(HOME=str(home), XDG_CONFIG_HOME=str(home), APPDATA=str(home), LOCALAPPDATA=str(home), USERPROFILE=str(home))

        before = tree(project, target)
        server = Server(binary, project, manifest, env, work, "first")

        # The handshake.
        said = server.request("initialize", {"protocolVersion": "2025-11-25", "capabilities": {}, "clientInfo": {"name": "walk_mcp", "version": "0"}})
        result = said.get("result", {})
        walk.check(
            result.get("protocolVersion") == "2025-11-25" and result.get("serverInfo", {}).get("name") == "dbt-edith"
            and result.get("capabilities") == {"tools": {}} and "instructions" in result,
            "initialize answers 2025-11-25 with the tools capability", str(said),
        )
        server.notify("notifications/initialized")
        said = server.request("tools/list")
        tools = said.get("result", {}).get("tools", [])
        walk.check(
            [t.get("name") for t in tools] == ["edith"] and tools[0].get("annotations") == {"openWorldHint": False}
            and tools[0]["inputSchema"]["type"] == "object" and len(json.dumps(tools)) <= 512,
            "tools/list is the one tool, under its byte ceiling", str(said),
        )
        said = server.request("server/discover", {})
        walk.check(said.get("error", {}).get("code") == -32601, "a modern server/discover is not found, and the client falls back", str(said))
        walk.check(server.request("ping").get("result") == {}, "ping answers an empty result")

        # The help has no command line to be compared with.
        blocks, is_error = blocks_of(server.call("help"))
        flags = ["--overview", "--describe", "--select", "--selector", "--column", "--run-collin", "--output"]
        walk.check(
            blocks is not None and len(blocks) == 1 and not is_error and blocks[0].startswith("dbt-edith ") and all(f in blocks[0] for f in flags),
            "help lists the flags in one block", str(blocks),
        )
        refused, is_error = blocks_of(server.call("--overview | head"))
        walk.check(is_error and len(refused) == 1 and "shell" in refused[0] and refused[0].endswith("`help` lists every flag."), "a pipe is refused with its reason", str(refused))
        refused, is_error = blocks_of(server.call("--overview --port 4321"))
        walk.check(is_error and "--port" in refused[0], "a flag that is not a question is refused", str(refused))

        # The questions, against the command line.
        for line, expected in QUESTIONS:
            cli = command_line(binary, project, manifest, line, env, work)
            walk.check(cli[2] == expected, f"the command line exits {expected} for {line}", f"exit {cli[2]}: {cli[1]}")
            walk.same(f"the tool answers as the command line does: {line}", server.call(line), cli)

        # An agreement on nothing would pass too: the invented edges are walked.
        blocks, _ = blocks_of(server.call("--column stg_payments.amount --down all --output compact"))
        walk.check(
            blocks is not None and "customer_lifetime_value" in blocks[0] and "orders" in blocks[0],
            "--column walks the edges of the cache down to the columns they feed", str(blocks),
        )

        # The cache moved away: --column cannot answer, and says what to do.
        cache = target / "column_lineage.collin.json"
        moved = work / "cache-moved-away.json"
        os.rename(cache, moved)
        line = "--column stg_payments.amount --down all"
        cli = command_line(binary, project, manifest, line, env, work)
        walk.check(cli[2] == 2, "the command line cannot answer --column without a cache", str(cli))
        walk.same(f"the tool answers as the command line does with no cache: {line}", server.call(line), cli)
        os.rename(moved, cache)

        # A manifest that is not there. The text differs on purpose: the command
        # line says to pass --manifest, which a line cannot.
        moved = work / "manifest-moved-away.json"
        os.rename(manifest, moved)
        cli = command_line(binary, project, manifest, "--overview", env, work)
        blocks, is_error = blocks_of(server.call("--overview"))
        walk.check(cli[2] == 2 and "pass --manifest" in cli[1], "the command line cannot answer without a manifest", str(cli))
        walk.check(
            is_error and len(blocks) == 1 and blocks[0].startswith("error: no manifest at ") and "--manifest" not in blocks[0]
            and "arguments the server was started with" in blocks[0],
            "the tool says there is no manifest, and does not tell a line to pass a flag", str(blocks),
        )
        os.rename(moved, manifest)
        walk.same("the tool answers again once the manifest is back", server.call("--describe orders"),
                  command_line(binary, project, manifest, "--describe orders", env, work))

        # So far nothing was written.
        walk.check(tree(project, target) == before, "the session left the project and the manifest's folder as they were")
        walk.check(not any(home.iterdir()), "the configuration folder stays empty")

        # --run-collin, with a stand-in for collin on the PATH of both sides.
        if os.name == "nt":
            print("NOTE  --run-collin is not walked on Windows: the stand-in collin is a shell script")
        else:
            bin_dir = work / "bin"
            bin_dir.mkdir()
            collin = bin_dir / "collin"
            collin.write_text(STAND_IN)
            collin.chmod(collin.stat().st_mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)
            env["PATH"] = str(bin_dir) + os.pathsep + env.get("PATH", "")
            # The server was started before the PATH had it: a second one,
            # which is what a client configured after installing collin does.
            late = Server(binary, project, manifest, env, work, "late")
            late.request("initialize", {"protocolVersion": "2025-11-25"})
            line = "--column stg_payments.amount --down all --run-collin"
            before = tree(project, target)
            served = late.call(line)
            after = tree(project, target)
            changed = sorted(Path(p).name for p in after if before.get(p) != after[p])
            walk.check(
                changed == ["collin.report.json", "column_lineage.collin.json"] and not any(p not in after for p in before),
                "--run-collin changes exactly the two files collin writes, beside the manifest", str(changed),
            )
            walk.same(f"the tool answers as the command line does: {line}", served, command_line(binary, project, manifest, line, env, work), ran=True)
            code = late.close()
            walk.check(code == 0, "a second server ends with exit 0 when stdin closes", str(code))
            walk.check(late.stderr_path.read_text() == "", "nothing was written to its stderr")

        code = server.close()
        walk.check(code == 0, "the server ends with exit 0 when stdin closes", str(code))
        walk.check(server.stderr_path.read_text() == "", "nothing was written to its stderr")
        bad = []
        for raw in server.written:
            try:
                message = json.loads(raw)
                if message.get("jsonrpc") != "2.0" or ("result" not in message) == ("error" not in message) or raw.count(b"\n") != 1:
                    bad.append(raw)
            except ValueError:
                bad.append(raw)
        walk.check(not bad and server.written, f"every line the server wrote is one JSON-RPC response ({len(server.written)})", str(bad[:3]))

    print(f"{walk.checked - walk.failed} of {walk.checked} agree")
    return 1 if walk.failed else 0


if __name__ == "__main__":
    sys.exit(main())
