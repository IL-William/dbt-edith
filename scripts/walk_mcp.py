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

The questions are the six modes with their flags, a refusal, an empty answer,
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

`--measure` does not compare: it times the same questions through the command
line and through one server process, on Jaffle Shop and on Fivetran's Shopify,
and prints tables (0110). It builds the release binary. Shopify needs git and
the network; SHOPIFY_FIXTURE names a folder holding `target/manifest.json`,
`target/catalog.json` and `expected/cache.json` (dbt-collin keeps one), whose
files stand in for `dbt parse` and for a cache, else dbt parses it and the
`--column` questions are left out. `--work DIR` keeps the projects it builds in
DIR. Nothing of it is run by check.sh.
"""

import json
import os
import queue
import statistics
import time
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
echo "$@" > "$(dirname "$0")/args"
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
    ("--describe orders --compiled-sql", 0),
    ("--describe orders,customers --sql --compiled-sql --output json", 0),
    ("--describe stg_payments --compiled-sql", 0),
    ("--describe raw_orders --compiled-sql", 0),
    ("--describe orders,nope", 1),
    ("--describe nope", 1),
    ("--describe ,", 1),
    ("-s 'stg_orders+1' --exclude resource_type:test --output compact", 0),
    ("-s '+customers'", 0),
    ("-s 'path:models/staging/*.sql' --output json", 0),
    ("--selector orders_eager", 0),
    ("-s '+customers' --exclude resource_type:test --count-by folder --output json", 0),
    ("-s 'resource_type:model' --count-by materialized", 0),
    ("-s 'tag:nothing_has_this_tag'", 0),
    ("-s 'nosuchmethod:orders'", 1),
    ("--column stg_payments.amount --down all --output compact", 0),
    ("--column orders.amount --up all --output json", 0),
    ("--column stg_payments.amount --up 1 --down 1", 0),
    ("--column orders.nope", 1),
    # The jobs of the project's own .dbt-edith/jobs.yml, over the job fixtures:
    # a node's card says which steps run it, and a job its steps.
    ("--describe orders", 0),
    ("--describe orders --output json", 0),
    ("--job jaffle_daily", 0),
    ("--job jaffle_daily --output json", 0),
    ("--job 'Nightly build'", 0),
    ("--job nightly", 0),
    ("--job jaffle", 1),
    ("--job no_such_job", 1),
    ("--job-order", 0),
    ("--job-order --output json", 0),
    ("--job-coverage", 0),
    ("--job-coverage --output json", 0),
    ("--job-coverage --output compact", 0),
]


def sources_file(sources):
    """The project's .dbt-edith/jobs.yml for these sources. A JSON string is a
    double-quoted YAML scalar, so it needs no quoting of its own."""
    lines = ["jobs:"]
    for s in sources:
        lines += [f"  - kind: {json.dumps(s['kind'])}", f"    path: {json.dumps(s['path'])}", f"    label: {json.dumps(s['label'])}"]
        if s["keys"]:
            lines.append("    keys:")
            lines += [f"      {k}: {json.dumps(v)}" for k, v in s["keys"].items()]
    return "\n".join(lines) + "\n"


class Server:
    """dbt-edith --mcp, spoken to the way a client speaks to it."""

    def __init__(self, binary, project, manifest, env, work, name):
        self.stderr_path = work / f"{name}-stderr.log"
        self.log = open(self.stderr_path, "w")
        self.proc = subprocess.Popen(
            [str(binary), "--mcp", str(project)] + (["--manifest", str(manifest)] if manifest else []),
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


# ---------------------------------------------------------------- --measure

# The first plan's four tools, kept for the comparison of what sits in the
# context. Never served.
FOUR_TOOLS = [
    {"name": "overview", "description": "What the dbt project holds: kinds, materializations, tags, selectors, folders. Ask first.",
     "inputSchema": {"type": "object", "properties": {"depth": {"type": "integer"}, "format": {"type": "string", "enum": ["text", "json"]}}, "additionalProperties": False},
     "annotations": {"readOnlyHint": True, "openWorldHint": False}},
    {"name": "describe", "description": "A node's card: columns with types and tests, parents, children, tests, reach; sql adds its file.",
     "inputSchema": {"type": "object", "properties": {"names": {"type": "array", "items": {"type": "string"}}, "sql": {"type": "boolean"}, "format": {"type": "string", "enum": ["text", "json"]}}, "required": ["names"], "additionalProperties": False},
     "annotations": {"readOnlyHint": True, "openWorldHint": False}},
    {"name": "select", "description": "What a dbt selection matches, as dbt ls, tests included unless excluded.",
     "inputSchema": {"type": "object", "properties": {"select": {"type": "string"}, "exclude": {"type": "string"}, "selector": {"type": "string"}, "format": {"type": "string", "enum": ["name", "json", "compact"]}}, "additionalProperties": False},
     "annotations": {"readOnlyHint": True, "openWorldHint": False}},
    {"name": "column", "description": "A column's lineage over the cache beside the manifest; run_collin writes that cache first.",
     "inputSchema": {"type": "object", "properties": {"column": {"type": "string"}, "up": {"type": ["integer", "string"]}, "down": {"type": ["integer", "string"]}, "stop_at": {"type": "array", "items": {"type": "string"}}, "indirect": {"type": "boolean"}, "tool": {"type": "string", "enum": ["fusion", "collin", "snowflake"]}, "run_collin": {"type": "boolean"}, "format": {"type": "string", "enum": ["name", "json", "compact"]}}, "required": ["column"], "additionalProperties": False},
     "annotations": {"readOnlyHint": False, "destructiveHint": False, "openWorldHint": False}},
]
# What a Claude Code user would put in a skill's description for the recipes:
# the one-line skill 0110 weighed. The skill dbt-edith ships (0114) has a longer
# description, about 190 tokens, which this figure does not include.
SKILL_LINE = "dbt-edith: answers what a dbt project, node, selection or column is from the manifest (dbt-edith . --overview / --describe / -s / --column); read docs/features/command-line.md#for-an-agent first."

PASSES = 3
N_CALLS = 20


def release_binary():
    if os.environ.get("DBT_EDITH"):
        return Path(os.environ["DBT_EDITH"])
    out = subprocess.run(["cargo", "build", "--release", "--locked", "--quiet", "--message-format=json-render-diagnostics"],
                         cwd=C.ROOT, check=True, stdout=subprocess.PIPE, encoding="utf-8").stdout
    for line in out.splitlines():
        msg = json.loads(line)
        if msg.get("reason") == "compiler-artifact" and msg["target"]["name"] == "dbt-edith" and msg.get("executable"):
            return Path(msg["executable"])
    sys.exit("cargo build named no dbt-edith executable")


def rss_kib(pid):
    out = subprocess.run(["ps", "-o", "rss=", "-p", str(pid)], capture_output=True, text=True).stdout.strip()
    return int(out) if out else 0


def ms_since(t):
    return (time.perf_counter() - t) * 1000.0


def med(values):
    return statistics.median(values)


def text_bytes(response):
    return sum(len(b["text"].encode()) for b in response["result"]["content"] if b["type"] == "text")


def questions_of(models, columns):
    """Twenty lines spread over the modes, by the first models by name."""
    m = lambda i: models[i % len(models)]
    overview = ["--overview", "--overview --depth 1 --output json"]
    describe = [f"--describe {m(0)}", f"--describe {m(1)} --sql", f"--describe {m(2)},{m(3)}", f"--describe {m(4)} --output json", f"--describe {m(0)}"]
    select = [f"-s '+{m(0)}'", f"-s '{m(1)}+'", f"-s '+{m(2)}+' --exclude resource_type:test", f"-s '{m(3)}+2' --output compact",
              "-s resource_type:seed", "-s config.materialized:table", f"-s 'resource_type:test,+{m(4)}'", f"-s '1+{m(5)}+1'"]
    column = [f"--column {c} --down all --output compact" for c in columns[:5]]
    order = [("o", 0), ("d", 0), ("s", 0), ("s", 1), ("c", 0), ("d", 1), ("s", 2), ("c", 1), ("d", 2), ("s", 3),
             ("c", 2), ("d", 3), ("s", 4), ("s", 5), ("c", 3), ("d", 4), ("s", 6), ("c", 4), ("o", 1), ("s", 7)]
    pools = {"o": overview, "d": describe, "s": select, "c": column}
    out = []
    for kind, i in order:
        if i < len(pools[kind]):
            out.append((kind, pools[kind][i]))
    return out


MODE = {"o": "--overview", "d": "--describe", "s": "-s", "c": "--column"}


def cli_pass(binary, project, manifest, questions, env, work):
    """Each line through its own process: wall ms and answer bytes."""
    rows = []
    for _, line in questions:
        t = time.perf_counter()
        run = subprocess.run([str(binary), str(project), "--manifest", str(manifest)] + shlex.split(line),
                             capture_output=True, env=env, cwd=work, timeout=WAIT)
        rows.append((ms_since(t), len(run.stdout) + len(run.stderr), run.returncode))
    return rows


def server_pass(binary, project, manifest, questions, env, work, reload_lines=None):
    """One server process: the handshake, then each line from the write of
    the request to the read of its response."""
    s = Server(binary, project, manifest, env, work, "measure")
    t = time.perf_counter()
    s.request("initialize", {"protocolVersion": "2025-11-25", "capabilities": {}, "clientInfo": {"name": "measure", "version": "0"}})
    init_ms = ms_since(t)
    s.notify("notifications/initialized")
    s.request("tools/list")
    rss_idle = rss_kib(s.proc.pid)
    rows = []
    for _, line in questions:
        t = time.perf_counter()
        said = s.call(line)
        took = ms_since(t)
        rows.append((took, text_bytes(said), len(s.written[-1]), said["result"]["isError"]))
    rss_after = rss_kib(s.proc.pid)
    out = {"init_ms": init_ms, "rss_idle": rss_idle, "rss_after": rss_after, "rows": rows, "reload": {}}
    # The reload: the manifest moves on, the next call reads it again.
    for label, line in (reload_lines or []):
        warm = []
        for _ in range(3):
            t = time.perf_counter()
            s.call(line)
            warm.append(ms_since(t))
        os.utime(manifest, None)
        t = time.perf_counter()
        s.call(line)
        out["reload"][label] = (med(warm), ms_since(t))
    out["rss_reload"] = rss_kib(s.proc.pid)
    s.close()
    return out


def probe_pass(binary, project, manifest, env, work):
    """What Claude Code pays for a probe: a process, a modern server/discover
    answered -32601, and the exit when stdin closes."""
    log = open(work / "probe-stderr.log", "w")
    t = time.perf_counter()
    proc = subprocess.Popen([str(binary), "--mcp", str(project), "--manifest", str(manifest)],
                            stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=log, env=env, cwd=work)
    proc.stdin.write((json.dumps({"jsonrpc": "2.0", "id": 1, "method": "server/discover", "params": {}}) + "\n").encode())
    proc.stdin.flush()
    answer = json.loads(proc.stdout.readline())
    first = ms_since(t)
    proc.stdin.close()
    proc.wait(timeout=30)
    exited = ms_since(t)
    log.close()
    assert answer["error"]["code"] == -32601, answer
    return first, exited


def peak_rss_mib(binary, project, manifest, line, env, work):
    """The command line's peak resident memory for one line, from /usr/bin/time."""
    flag = "-l" if sys.platform == "darwin" else "-v"
    if not Path("/usr/bin/time").exists() or os.name == "nt":
        return None
    run = subprocess.run(["/usr/bin/time", flag, str(binary), str(project), "--manifest", str(manifest)] + shlex.split(line),
                         capture_output=True, text=True, env=env, cwd=work)
    for row in run.stderr.splitlines():
        if "maximum resident set size" in row.lower():
            n = int(re.search(r"\d+", row.replace("Maximum resident set size (kbytes):", "").strip() or row).group())
            return (n / (1024 * 1024) if sys.platform == "darwin" else n / 1024)
    return None


def table(head, rows):
    print("| " + " | ".join(head) + " |")
    print("| " + " | ".join("---" for _ in head) + " |")
    for row in rows:
        print("| " + " | ".join(str(c) for c in row) + " |")
    print()


def tokens_of(text):
    try:
        import tiktoken
        return len(tiktoken.get_encoding("o200k_base").encode(text))
    except Exception:
        return None


def context_sizes(binary, project, manifest, env, work):
    s = Server(binary, project, manifest, env, work, "sizes")
    init = s.request("initialize", {"protocolVersion": "2025-11-25"})["result"]
    s.notify("notifications/initialized")
    tools = s.request("tools/list")["result"]["tools"]
    helped = text_bytes(s.call("help"))
    helptext = s.call("help")["result"]["content"][0]["text"]
    s.close()
    compact = lambda v: json.dumps(v, separators=(",", ":"))
    rows = [
        ("one tool: tools/list result", compact({"tools": tools})),
        ("one tool: INSTRUCTIONS", init["instructions"]),
        ("one tool: both (always in context)", compact({"tools": tools}) + init["instructions"]),
        ("`help` answer (only when asked)", helptext),
        ("four tools: tools/list result", compact({"tools": FOUR_TOOLS})),
        ("skill line", SKILL_LINE),
    ]
    out = []
    for label, text in rows:
        n = len(text.encode())
        tok = tokens_of(text)
        out.append((label, n, n // 4, tok if tok is not None else "n/a"))
    print("### What sits in the context\n")
    table(["Candidate", "Bytes", "Tokens at bytes/4", "Tokens, o200k"], out)
    return helped


def measure_project(label, binary, project, manifest, models, columns, env, work, alone=None):
    print(f"## {label}\n")
    questions = questions_of(models, columns)
    n_by_mode = {k: sum(1 for q in questions if q[0] == k) for k in MODE}
    print(f"{len(questions)} lines: " + ", ".join(f"{n} of {MODE[k]}" for k, n in n_by_mode.items()) + f"; {PASSES} passes, medians.\n")
    cli = [cli_pass(binary, project, manifest, questions, env, work) for _ in range(PASSES)]
    reload_lines = [("-s after touching the manifest", questions[2][1]), ("--describe after touching the manifest", questions[1][1])]
    srv = [server_pass(binary, project, manifest, questions, env, work, reload_lines) for _ in range(PASSES)]
    per = lambda passes, i: med([p[i][0] for p in passes])
    cli_ms = [per(cli, i) for i in range(len(questions))]
    srv_ms = [med([p["rows"][i][0] for p in srv]) for i in range(len(questions))]
    srv_first = med([p["rows"][0][0] for p in srv])
    rows = []
    for i, (kind, line) in enumerate(questions):
        rows.append((i + 1, line, f"{cli_ms[i]:.1f}", f"{srv_ms[i]:.1f}", cli[0][i][1], srv[0]["rows"][i][1], srv[0]["rows"][i][2]))
    table(["#", "line", "CLI ms", "server ms", "answer bytes (CLI)", "answer bytes (server text)", "response line bytes"], rows)

    def mode_row(name, idx):
        c = [cli_ms[i] for i in idx]
        sv = [srv_ms[i] for i in idx]
        return (name, len(idx), f"{med(c):.1f}", f"{med(sv):.1f}", f"{med(c) / med(sv):.1f}x")

    modes = [mode_row(MODE[k], [i for i, q in enumerate(questions) if q[0] == k]) for k in MODE]
    warm_idx = list(range(1, len(questions)))
    modes.append(mode_row("all, first call left out", warm_idx))
    print("### Median per call, by mode\n")
    table(["mode", "calls", "CLI ms", "server ms (warm)", "CLI / server"], modes)
    total_cli = med([sum(r[0] for r in p) for p in cli])
    total_srv = med([sum(r[0] for r in p["rows"]) for p in srv])
    print("### Total for the lines\n")
    table(["", "ms"], [("CLI, 20 processes", f"{total_cli:.0f}"), ("server, 20 calls on one process", f"{total_srv:.0f}"),
                       ("server's first call (the load)", f"{srv_first:.1f}"),
                       ("server's handshake, start to `initialize` answered", f"{med([p['init_ms'] for p in srv]):.1f}"),
                       ("per call saved, total / N", f"{(total_cli - total_srv) / len(questions):.1f}")])
    print("### The reload\n")
    table(["call", "warm ms", "after touch ms"],
          [(k, f"{med([p['reload'][k][0] for p in srv]):.1f}", f"{med([p['reload'][k][1] for p in srv]):.1f}") for k in srv[0]["reload"]])
    print("### Memory\n")
    peak_sel = peak_rss_mib(binary, project, manifest, next(l for k, l in questions if k == "s"), env, work)
    peak_col = peak_rss_mib(binary, project, manifest, next((l for k, l in questions if k == "c"), questions[0][1]), env, work)
    table(["", "MiB"], [("server, after the handshake", f"{med([p['rss_idle'] for p in srv]) / 1024:.1f}"),
                        ("server, after the lines", f"{med([p['rss_after'] for p in srv]) / 1024:.1f}"),
                        ("server, after the reloads", f"{med([p['rss_reload'] for p in srv]) / 1024:.1f}"),
                        ("one CLI -s call, peak", "n/a" if peak_sel is None else f"{peak_sel:.1f}"),
                        ("one CLI --column call, peak", "n/a" if peak_col is None else f"{peak_col:.1f}")])
    probes = [probe_pass(binary, project, manifest, env, work) for _ in range(PASSES)]
    print("### Claude Code's probe\n")
    table(["", "ms"], [("process start to the -32601 answer", f"{med([p[0] for p in probes]):.1f}"),
                       ("process start to exit once stdin closes", f"{med([p[1] for p in probes]):.1f}")])
    # --column alone, twenty times: does cloning the graph per call matter.
    cols = [("c", f"--column {c} --down all --output compact") for c in (columns * 4)[:N_CALLS]] if columns else []
    if cols:
        ccli = [cli_pass(binary, project, manifest, cols, env, work) for _ in range(PASSES)]
        csrv = [server_pass(binary, project, manifest, cols, env, work) for _ in range(PASSES)]
        c_c = med([sum(r[0] for r in p) for p in ccli])
        c_s = med([sum(r[0] for r in p["rows"]) for p in csrv])
        c_first = med([p["rows"][0][0] for p in csrv])
        c_warm = med([r[0] for p in csrv for r in p["rows"][1:]])
        print("### `--column` alone, 20 calls\n")
        table(["", "ms"], [("CLI, 20 processes", f"{c_c:.0f}"), ("server, 20 calls", f"{c_s:.0f}"), ("server, first call", f"{c_first:.1f}"),
                           ("server, median of the others", f"{c_warm:.1f}"), ("CLI, median", f"{med([r[0] for p in ccli for r in p]):.1f}")])
    if alone is not None:
        # The same project without dbt_project.yml: the freshness line is not
        # computed ("not checked"), so the difference is what it costs.
        aproject, amanifest = alone
        acli = [cli_pass(binary, aproject, amanifest, questions, env, work) for _ in range(PASSES)]
        asrv = [server_pass(binary, aproject, amanifest, questions, env, work) for _ in range(PASSES)]
        acli_ms = [per(acli, i) for i in range(len(questions))]
        asrv_ms = [med([p["rows"][i][0] for p in asrv]) for i in range(len(questions))]
        print("### Without the freshness walk (no dbt_project.yml beside the manifest)\n")
        table(["", "with freshness ms", "without ms", "freshness costs ms"],
              [("CLI, median per call", f"{med(cli_ms[1:]):.1f}", f"{med(acli_ms[1:]):.1f}", f"{med(cli_ms[1:]) - med(acli_ms[1:]):.1f}"),
               ("server, median per call, first left out", f"{med(srv_ms[1:]):.1f}", f"{med(asrv_ms[1:]):.1f}", f"{med(srv_ms[1:]) - med(asrv_ms[1:]):.1f}")])


def measure(argv):
    binary = release_binary()
    keep = Path(argv[argv.index("--work") + 1]) if "--work" in argv else None
    os.environ.setdefault("DBT_SEND_ANONYMOUS_USAGE_STATS", "False")
    os.environ.setdefault("DO_NOT_TRACK", "1")
    ctx = tempfile.TemporaryDirectory(prefix="dbt-edith-measure-") if keep is None else None
    work = Path(ctx.name) if ctx else keep
    work.mkdir(parents=True, exist_ok=True)
    print(f"# dbt-edith {binary.name} --mcp against the command line, {time.strftime('%Y-%m-%d')}\n")
    print(f"{C.Dbt(work, work).version()}; {sys.platform}; release build; each line is a whole process for the command line.\n")
    home = work / "home"
    home.mkdir(exist_ok=True)
    env = {k: v for k, v in os.environ.items() if k != "DBT_EDITH_CONFIG_DIR"}
    env.update(HOME=str(home), XDG_CONFIG_HOME=str(home), APPDATA=str(home), LOCALAPPDATA=str(home), USERPROFILE=str(home))

    # Jaffle Shop.
    project = work / "jaffle" / "project"
    if not project.exists():
        shutil.copytree(C.JAFFLE, project)
        C.Dbt(project, work / "jaffle").parse()
    target = work / "jaffle" / "target"
    (target / "column_lineage.collin.json").write_text(json.dumps(CACHE))
    (target / "catalog.json").write_text(json.dumps(CATALOG))
    manifest = target / "manifest.json"
    models = sorted(n["name"] for n in json.loads(manifest.read_text())["nodes"].values() if n["resource_type"] == "model")
    columns = ["stg_payments.amount", "orders.amount", "stg_orders.order_id", "stg_payments.amount", "orders.order_id"]
    context_sizes(binary, project, manifest, env, work)
    alone = work / "jaffle-alone"
    (alone / "target").mkdir(parents=True, exist_ok=True)
    for name in ("manifest.json", "catalog.json", "column_lineage.collin.json"):
        shutil.copy(target / name, alone / "target" / name)
    measure_project("Jaffle Shop", binary, project, manifest, models, columns, env, work, (alone, alone / "target" / "manifest.json"))

    # Shopify.
    fixture = Path(os.environ["SHOPIFY_FIXTURE"]) if os.environ.get("SHOPIFY_FIXTURE") else None
    try:
        shopify = work / "shopify" / "dbt_shopify" / "integration_tests"
        if not shopify.exists():
            shopify = C.fetch_shopify(work / "shopify")
            C.Dbt(shopify, work / "shopify").deps()
    except (C.Unavailable, RuntimeError, subprocess.CalledProcessError) as e:
        print(f"## Shopify\n\nSKIPPED: {e}\n")
        return 0
    starget = work / "shopify" / "target"
    starget.mkdir(parents=True, exist_ok=True)
    smanifest = starget / "manifest.json"
    scolumns = []
    if fixture:
        shutil.copy(fixture / "target" / "manifest.json", smanifest)
        shutil.copy(fixture / "target" / "catalog.json", starget / "catalog.json")
        shutil.copy(fixture / "expected" / "cache.json", starget / "column_lineage.collin.json")
        graph = json.loads(smanifest.read_text())["nodes"]
        edges = json.loads((starget / "column_lineage.collin.json").read_text())["edges"]
        pairs = sorted({(graph[e["from"]]["name"], e["from_col"]) for e in edges if e["from"] in graph and graph[e["from"]]["resource_type"] == "model"})
        by_model = {}
        for model, col in pairs:
            by_model.setdefault(model, col)
        scolumns = [f"{m}.{c}" for m, c in list(by_model.items())[:5]]
    elif not smanifest.exists():
        C.Dbt(shopify, work / "shopify").parse()
    nodes = json.loads(smanifest.read_text())["nodes"]
    smodels = sorted(n["name"] for n in nodes.values() if n["resource_type"] == "model")
    salone = work / "shopify-alone"
    (salone / "target").mkdir(parents=True, exist_ok=True)
    for f in starget.iterdir():
        shutil.copy(f, salone / "target" / f.name)
    print(f"Shopify v{C.SHOPIFY_TAG.lstrip('v')} ({C.SHOPIFY_COMMIT[:7]}): {len(smodels)} models, {len(nodes)} nodes; "
          f"{'manifest, catalog and cache from SHOPIFY_FIXTURE' if fixture else 'manifest from dbt parse, no cache'}.\n")
    measure_project("Fivetran's Shopify v1.10.0", binary, shopify, smanifest, smodels, scolumns, env, work, (salone, salone / "target" / "manifest.json"))
    print(f"Projects kept in {work}" if keep else "")
    return 0


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
        dbt = C.Dbt(project, work / "jaffle")
        dbt.parse()
        # Compiled, but for one model: --compiled-sql reads the files dbt wrote
        # under the target folder, and says so for the one it did not write.
        # Compiling opens the profile's relative database file, so it is
        # created in the work folder and not wherever this ran from.
        here = os.getcwd()
        os.chdir(work)
        try:
            dbt.invoke(["compile", "--exclude", "stg_payments"])
        finally:
            os.chdir(here)
        target = work / "jaffle" / "target"
        manifest = target / "manifest.json"
        (target / "column_lineage.collin.json").write_text(json.dumps(CACHE))
        (target / "catalog.json").write_text(json.dumps(CATALOG))
        # The project names where its jobs are, in the file the Jobs settings
        # write: a copy of the invented job fixtures, named from the project
        # (the file takes no other kind of path), read by the command line and
        # the tool alike (0115).
        jobs = work / "jobs"
        shutil.copytree(C.JOBS, jobs)
        (project / ".dbt-edith").mkdir()
        (project / ".dbt-edith" / "jobs.yml").write_text(
            sources_file([dict(src, path=os.path.relpath(jobs / Path(src["path"]).name, project).replace(os.sep, "/")) for src in C.JOB_SOURCES]))

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
        flags = ["--overview", "--describe", "--select", "--selector", "--job", "--job-order", "--job-coverage", "--column", "--run-collin", "--output"]
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

        # The same for --compiled-sql: the compiled SQL is what answers, with
        # the Jinja gone, and the model dbt did not compile is only said.
        blocks, _ = blocks_of(server.call("--describe orders --compiled-sql"))
        walk.check(
            blocks is not None and "\n  compiled sql\n" in blocks[0] and "{{" not in blocks[0].split("\n  compiled sql\n")[1],
            "--compiled-sql prints the compiled SQL, not the model's own", str(blocks),
        )
        blocks, _ = blocks_of(server.call("--describe stg_payments --compiled-sql"))
        walk.check(
            blocks is not None and "compiled sql" not in blocks[0] and len(blocks) == 2 and "run dbt compile -s stg_payments" in blocks[1],
            "--compiled-sql says what to run for a model dbt did not compile", str(blocks),
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
            given = (bin_dir / "args").read_text()
            walk.check("--project" not in given.split(), "collin is not given --project beside a manifest outside the project", given)
            changed = sorted(Path(p).name for p in after if before.get(p) != after[p])
            walk.check(
                changed == ["collin.report.json", "column_lineage.collin.json"] and not any(p not in after for p in before),
                "--run-collin changes exactly the two files collin writes, beside the manifest", str(changed),
            )
            walk.same(f"the tool answers as the command line does: {line}", served, command_line(binary, project, manifest, line, env, work), ran=True)
            code = late.close()
            walk.check(code == 0, "a second server ends with exit 0 when stdin closes", str(code))
            walk.check(late.stderr_path.read_text() == "", "nothing was written to its stderr")

            # A copy laid out as documented: `--mcp <project>` alone, the
            # manifest at <project>/target/manifest.json inside it. That is
            # when collin is given --project (0109).
            documented = work / "documented"
            shutil.copytree(C.JAFFLE, documented)
            (documented / "target").mkdir()
            shutil.copy(manifest, documented / "target" / "manifest.json")
            alone = Server(binary, documented, None, env, work, "documented")
            alone.request("initialize", {"protocolVersion": "2025-11-25"})
            blocks, is_error = blocks_of(alone.call("--describe orders"))
            walk.check(blocks is not None and not is_error and "orders" in blocks[0], "the documented layout answers with no --manifest", str(blocks))
            blocks, is_error = blocks_of(alone.call("--column stg_payments.amount --down all --run-collin"))
            args = (bin_dir / "args").read_text().split()
            walk.check(
                blocks is not None and not is_error and "--project" in args and Path(args[args.index("--project") + 1]).resolve() == documented.resolve(),
                "--run-collin on the documented layout gives collin --project <project>", f"{args}\n{blocks}",
            )
            code = alone.close()
            walk.check(code == 0 and alone.stderr_path.read_text() == "", "the documented-layout server ends with exit 0 and a silent stderr", str(code))

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
    sys.exit(measure(sys.argv[1:]) if "--measure" in sys.argv[1:] else main())
