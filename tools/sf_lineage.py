#!/usr/bin/env python3
"""Snowflake for dbt-edith: column-level lineage and the user's query history.

dbt-edith is a single static binary with no HTTP client, no TLS and no credential
handling, and it has to stay that way to keep cross-compiling to one dependency
free .exe. So this script owns the Snowflake connection. dbt-edith starts it in
`serve` mode once a Snowflake feature needs it, column lineage or the query
history, and talks to it over stdin and stdout (docs/decisions/0016, 0048).

It reads the dbt profile, so SSO, key-pair and password targets all work without
anything specific here.

    probe   check what this role is actually allowed to read
    serve   answer JSON Lines requests on stdin, one per line
    dump    warm the cache for a set of models

Column lineage comes from SNOWFLAKE.CORE.GET_LINEAGE with the COLUMN domain,
which takes one column per call. A whole project is hundreds of thousands of
calls, so `serve` is the normal mode and `dump` is always scoped. The query
history comes from INFORMATION_SCHEMA.QUERY_HISTORY_BY_USER, which any role can
read for its own user, live and for the last seven days.
"""

import argparse
import contextlib
import json
import os
import queue
import re
import sys
import threading
import time
from datetime import datetime, timezone
from pathlib import Path

CACHE_VERSION = 1

# GET_LINEAGE's range for its distance argument.
MAX_DEPTH = 5
DIRECTIONS = ("UPSTREAM", "DOWNSTREAM")

# Set on every session this script opens, so the history it reads can leave
# out the queries it ran itself.
OWN_TAG = "dbt-edith"
# Rows of history per request. QUERY_HISTORY_BY_USER keeps its limit before
# any WHERE, so it is asked for its most and the page is cut after the filter.
HISTORY_PAGE = 20
HISTORY_SCAN = 10000
# Enough of a statement to recognise it; Snowsight has the rest.
HISTORY_TEXT = 10000

# The target keys this script reads, and so the ones checked for Jinja.
PROFILE_KEYS = (
    "account", "user", "role", "warehouse", "database", "schema",
    "authenticator", "password", "private_key_path", "private_key_passphrase",
)


# --------------------------------------------------------------- profile ----
def profiles_path(profiles_dir: str | None) -> Path:
    """Where dbt itself looks: the flag, DBT_PROFILES_DIR, the project, ~/.dbt.

    Always absolute: dbt-edith is told this path and has to open that file, not
    something relative to wherever it happens to be running.
    """
    explicit = profiles_dir or os.environ.get("DBT_PROFILES_DIR")
    if explicit:
        found = Path(explicit) / "profiles.yml"
    elif Path("profiles.yml").exists():
        found = Path("profiles.yml")
    else:
        found = Path.home() / ".dbt" / "profiles.yml"
    return found.resolve()


def profile_outputs(profile: str, profiles_dir: str | None):
    """The profile's block of profiles.yml and the file it came from."""
    try:
        import yaml
    except ImportError:
        die("PyYAML is not installed in this interpreter: pip install pyyaml")

    path = profiles_path(profiles_dir)
    if not path.exists():
        die(f"no profiles.yml at {path}")

    doc = yaml.safe_load(path.read_text()) or {}
    if profile not in doc:
        die(f"profile {profile!r} not in {path} (found: {', '.join(k for k in doc if k != 'config')})")
    return doc[profile], path


def is_jinja(value) -> bool:
    return isinstance(value, str) and ("{{" in value or "{%" in value)


def load_profile(profile: str, target: str | None, profiles_dir: str | None):
    """Returns the connection kwargs for a dbt target, secrets included but never logged."""
    block, path = profile_outputs(profile, profiles_dir)
    # dbt reads DBT_TARGET the same way, so both pick the same connection.
    target = target or os.environ.get("DBT_TARGET") or block.get("target")
    outputs = block.get("outputs", {})
    if target not in outputs:
        die(f"target {target!r} not in profile {profile!r} (found: {', '.join(outputs)})")

    cfg = dict(outputs[target])
    if cfg.get("type") != "snowflake":
        die(f"target {target!r} is type {cfg.get('type')!r}, not snowflake")
    for key in PROFILE_KEYS:
        if is_jinja(cfg.get(key)):
            # Named, never shown: the expression may be where a secret comes from.
            die(f"{key} of target {target!r} in {path} is a Jinja expression, "
                "and this script reads literal values only")

    kwargs = {
        "account": cfg["account"],
        "user": cfg.get("user"),
        "role": cfg.get("role"),
        "warehouse": cfg.get("warehouse"),
        "database": cfg.get("database"),
        "schema": cfg.get("schema"),
        # One browser prompt per machine rather than one per process.
        "client_store_temporary_credential": True,
        "application": "dbt_edith",
        "session_parameters": {"QUERY_TAG": OWN_TAG},
    }
    if cfg.get("authenticator"):
        kwargs["authenticator"] = cfg["authenticator"]
    if cfg.get("password"):
        kwargs["password"] = cfg["password"]
    if cfg.get("private_key_path"):
        kwargs["private_key_file"] = cfg["private_key_path"]
        if cfg.get("private_key_passphrase"):
            kwargs["private_key_file_pwd"] = cfg["private_key_passphrase"]
    return {k: v for k, v in kwargs.items() if v is not None}, target


def target_roles(outputs: dict, connected: str):
    """The profile's Snowflake targets, each with its role and whether it signs in as the connected one.

    A query history belongs to one user, so a target that signs in as someone
    else needs a connection of its own to show its queries, which this script
    does not open. Only names and roles leave: whether two targets share a
    login is said, never who that login is.
    """
    mine = outputs.get(connected) or {}
    found = []
    for name, cfg in outputs.items():
        if not isinstance(cfg, dict) or cfg.get("type") != "snowflake":
            continue
        entry = {"name": name}
        role = cfg.get("role")
        if is_jinja(role):
            entry["why"] = "its role is a Jinja expression"
        elif isinstance(role, str) and role:
            entry["role"] = role
        else:
            entry["why"] = "it names no role, so it signs in with the user's default one"
        login = [cfg.get(k) for k in ("account", "user")]
        if name == connected:
            entry["same_login"] = True
        elif any(is_jinja(v) for v in login):
            entry["same_login"] = False
            entry["why"] = "its account or user is a Jinja expression"
        else:
            theirs = [str(v or "").lower() for v in login]
            entry["same_login"] = theirs == [str(mine.get(k) or "").lower() for k in ("account", "user")]
            if not entry["same_login"]:
                entry["why"] = "it signs in as another user or account"
        found.append(entry)
    return found


def connector():
    """The Snowflake connector module, or a readable exit."""
    try:
        import snowflake.connector
    except ImportError:
        die("snowflake-connector-python is not installed in this interpreter")
    return snowflake.connector


def connect(kwargs):
    sf = connector()
    import logging

    # The connector is chatty in some paths, and single sign-on prints its
    # instructions: all of it goes to stderr, because for serve stdout is the
    # protocol.
    logging.getLogger("snowflake").setLevel(logging.ERROR)
    with contextlib.redirect_stdout(sys.stderr):
        return sf.connect(**kwargs)


def profile_from_project():
    """The profile named by the dbt project in the working directory, if any."""
    path = Path("dbt_project.yml")
    if not path.exists():
        return None
    try:
        import yaml
        return (yaml.safe_load(path.read_text()) or {}).get("profile")
    except Exception:  # noqa: BLE001 - an unreadable project just means no default
        return None


def die(msg: str, code: int = 2):
    print(f"sf_lineage: {msg}", file=sys.stderr)
    sys.exit(code)


def first_line(error: BaseException, limit: int) -> str:
    lines = str(error).strip().splitlines()
    return (lines[0] if lines else type(error).__name__)[:limit]


# --------------------------------------------------------------- queries ----
def column_lineage(cur, relation: str, column: str, direction: str, distance: int):
    """One GET_LINEAGE call for one column. Returns raw rows."""
    sql = (
        "select distance, "
        "source_object_database, source_object_schema, source_object_name, source_column_name, "
        "target_object_database, target_object_schema, target_object_name, target_column_name, "
        "source_object_domain, target_object_domain "
        "from table(snowflake.core.get_lineage(%s, 'COLUMN', %s, %s))"
    )
    cur.execute(sql, (f"{relation}.{column}", direction, distance))
    return cur.fetchall()


def rows_to_raw(rows):
    """GET_LINEAGE rows as column pairs between Snowflake objects, dropping self and empty pairs."""
    pairs = []
    for r in rows:
        (dist, sdb, ssc, snm, scol, tdb, tsc, tnm, tcol, sdom, tdom) = r[:11]
        if not scol or not tcol:
            continue  # object-level row, no column information
        src = f"{sdb}.{ssc}.{snm}"
        dst = f"{tdb}.{tsc}.{tnm}"
        if src.upper() == dst.upper() and scol.lower() == tcol.lower():
            continue
        pairs.append(
            {
                "from_rel": src,
                "from_col": scol.lower(),
                "to_rel": dst,
                "to_col": tcol.lower(),
                "kind": (tdom or sdom or "").lower(),
                "distance": int(dist or 0),
            }
        )
    return pairs


def rows_to_edges(rows, resolve):
    """GET_LINEAGE rows as cache edges between dbt nodes."""
    return [
        {
            "from": resolve(p["from_rel"]),
            "from_col": p["from_col"],
            "to": resolve(p["to_rel"]),
            "to_col": p["to_col"],
            "kind": p["kind"],
        }
        for p in rows_to_raw(rows)
    ]


def session_info(cur):
    """Who and where the session is, for the Snowsight link and the role a request names by default."""
    try:
        cur.execute("select current_organization_name(), current_account_name(), current_user(), current_role()")
        org, account, user, role = cur.fetchall()[0][:4]
    except Exception:  # noqa: BLE001 - an account with no organization still has a history
        cur.execute("select current_account_name(), current_user(), current_role()")
        account, user, role = cur.fetchall()[0][:3]
        org = None
    return {k: v for k, v in {"org": org, "account": account, "user": user, "role": role}.items() if v}


def history_request(req):
    """(role, before) out of one history request, or ValueError.

    role None means the connected target's own, "*" every role. before is the
    oldest row already shown, as its start in epoch nanoseconds and its query
    id, so the next page starts after it whatever ran since.
    """
    role = req.get("role")
    if role is not None:
        if not isinstance(role, str) or not role.strip() or len(role) > 255 or not role.isprintable():
            raise ValueError("role must be a role name, or * for every role")
    before = req.get("before")
    if before is not None:
        at = before.get("at") if isinstance(before, dict) else None
        qid = before.get("query_id") if isinstance(before, dict) else None
        if not isinstance(at, str) or not re.fullmatch(r"[0-9]{1,20}", at):
            raise ValueError("before.at must be a start time in epoch nanoseconds")
        if not isinstance(qid, str) or not re.fullmatch(r"[0-9A-Fa-f-]{1,64}", qid):
            raise ValueError("before.query_id must be a query id")
        before = (int(at), qid)
    return role, before


HISTORY_COLUMNS = (
    "query_id", "at", "ms", "status", "error", "role", "warehouse",
    "database", "schema", "type", "text", "length", "rows",
)


def query_history(cur, role: str | None, before, page: int = HISTORY_PAGE):
    """One page of the user's own queries, newest first, and whether there are older ones.

    role None reads every role. Ordered by start and then query id, because two
    queries can start in the same nanosecond and the page boundary must not
    drop either.
    """
    where = ["coalesce(query_tag, '') <> %(tag)s"]
    params = {"tag": OWN_TAG, "scan": HISTORY_SCAN, "text": HISTORY_TEXT, "limit": page + 1}
    if role is not None:
        where.append("upper(role_name) = upper(%(role)s)")
        params["role"] = role
    if before is not None:
        where.append(
            "(date_part(epoch_nanosecond, start_time) < %(at)s"
            " or (date_part(epoch_nanosecond, start_time) = %(at)s and query_id < %(qid)s))"
        )
        params["at"], params["qid"] = before
    sql = (
        "select query_id, date_part(epoch_nanosecond, start_time), total_elapsed_time, "
        "execution_status, error_message, role_name, warehouse_name, database_name, schema_name, "
        "query_type, left(query_text, %(text)s), length(query_text), rows_produced "
        "from table(information_schema.query_history_by_user(result_limit => %(scan)s)) "
        f"where {' and '.join(where)} "
        "order by start_time desc, query_id desc "
        "limit %(limit)s"
    )
    cur.execute(sql, params)
    rows = [history_row(r) for r in cur.fetchall()]
    return rows[:page], len(rows) > page


def history_row(r):
    """One QUERY_HISTORY row as the page reads it, without the columns Snowflake left empty."""
    row = dict(zip(HISTORY_COLUMNS, r))
    at = int(row["at"])
    row["at"] = str(at)  # past what a JavaScript number holds exactly
    row["start"] = datetime.fromtimestamp(at // 1_000_000_000, timezone.utc).strftime("%Y-%m-%dT%H:%M:%S") + (
        f".{at % 1_000_000_000 // 1_000_000:03d}Z"
    )
    if row["status"]:
        row["status"] = str(row["status"]).lower()
    for key in ("ms", "length", "rows"):
        if row[key] is not None:
            row[key] = int(row[key])
    return {k: v for k, v in row.items() if v is not None and v != ""}


# ----------------------------------------------------------------- probe ----
PROBE_SQL = [
    (
        "edition",
        "current account edition",
        "select current_version() as v, current_account() as a, current_role() as r",
    ),
]


def cmd_probe(args):
    kwargs, target = load_profile(args.profile, args.target, args.profiles_dir)
    print(f"connecting to profile {args.profile!r} target {target!r} "
          f"as role {kwargs.get('role')} ({kwargs.get('authenticator', 'password')})")
    conn = connect(kwargs)
    cur = conn.cursor()

    def attempt(label, sql, params=None):
        t0 = time.time()
        try:
            cur.execute(sql, params) if params else cur.execute(sql)
            rows = cur.fetchall()
            print(f"  OK    {label}: {len(rows)} row(s) in {time.time() - t0:.1f}s")
            return rows
        except Exception as e:  # noqa: BLE001 - the whole point is to report any failure
            print(f"  FAIL  {label}: {first_line(e, 160)}")
            return None

    print("\n[1] session")
    attempt("current_role / version", "select current_version(), current_account(), current_role()")

    relation = args.relation
    print(f"\n[2] GET_LINEAGE, TABLE domain on {relation}")
    tbl = attempt(
        "table lineage",
        "select distance, source_object_name, target_object_name, source_object_domain "
        "from table(snowflake.core.get_lineage(%s, 'TABLE', 'UPSTREAM', 2))",
        (relation,),
    )

    print(f"\n[3] GET_LINEAGE, COLUMN domain on {relation}.{args.column}")
    col = attempt(
        "column lineage",
        "select distance, source_object_name, source_column_name, target_column_name "
        "from table(snowflake.core.get_lineage(%s, 'COLUMN', 'UPSTREAM', 2))",
        (f"{relation}.{args.column}",),
    )
    if col:
        for row in col[:5]:
            print(f"        d={row[0]}  {row[1]}.{row[2]} -> {row[3]}")

    print("\n[4] ACCOUNT_USAGE.ACCESS_HISTORY (bulk path, needs IMPORTED PRIVILEGES)")
    acc = attempt(
        "access_history",
        "select count(*) from snowflake.account_usage.access_history "
        "where query_start_time > dateadd(day, -1, current_timestamp())",
    )

    print("\nverdict")
    if col:
        print("  column lineage works: on-demand mode is viable")
    elif tbl is not None:
        print("  GET_LINEAGE works but returned no column rows for this object.")
        print("  Either the object was not built by a query Snowflake could analyse,")
        print("  or this dev object has no lineage yet. Try a model you rebuilt recently.")
    else:
        print("  GET_LINEAGE is unavailable to this role. Ask for VIEW LINEAGE,")
        print("  and check the account is Enterprise Edition or higher.")
    if acc:
        print("  ACCESS_HISTORY is readable: a bulk load of the whole project is possible,")
        print("  which is much cheaper than one call per column. Tell dbt-edith about it.")
    else:
        print("  ACCESS_HISTORY is not readable: stay with per-column calls.")
    cur.close()
    conn.close()


# ----------------------------------------------------------------- serve ----
def lineage_request(req):
    """(relation, column, direction, depth) out of one serve request, or ValueError."""
    relation, column = req.get("relation"), req.get("column")
    if not isinstance(relation, str) or not relation or not isinstance(column, str) or not column:
        raise ValueError("a request needs a relation and a column")
    direction = str(req.get("direction", "UPSTREAM")).upper()
    if direction not in DIRECTIONS:
        raise ValueError(f"direction must be UPSTREAM or DOWNSTREAM, not {direction!r}")
    try:
        depth = int(req.get("depth", 1))
    except (TypeError, ValueError):
        raise ValueError("depth must be a whole number") from None
    return relation, column, direction, min(max(depth, 1), MAX_DEPTH)


def cmd_serve(args):
    """JSON Lines for dbt-edith, one request per line on stdin, one reply per line on stdout.

    Requests: {"id": 1, "op": "lineage", "relation": "DB.SCHEMA.OBJECT",
               "column": "C", "direction": "UPSTREAM", "depth": 2}, where no
              op means lineage too;
              {"id": 2, "op": "history", "role": "R" | "*" | null,
               "before": {"at": "<epoch ns>", "query_id": "..."} | null};
              {"id": 3, "op": "session"}, for the account a Snowsight link
               names; or {"op": "quit"}.
    Replies:  {"event": "profiles", "path": ...} and {"event": "ready", ...}
              once each, then {"id": 1, "rows": [...]} for lineage,
              {"id": 2, "session": {...}, "rows": [...], "more": bool} for
              history, {"id": 3, "session": {...}} for session, or
              {"id": 1, "error": "...", "phase": "connect" | "query"}.

    The phase says whether the profile is to blame or not: a connection that
    Snowflake refuses points at profiles.yml, a query that fails does not.

    Rows name Snowflake objects, not dbt nodes: dbt-edith maps them itself,
    because it knows the current manifest and the environment the user picked.
    The connection opens on the first request, never before, so a sign-in tab
    can only follow a click.
    """
    out = sys.stdout
    conn = None
    session = None

    def reply(obj):
        out.write(json.dumps(obj, separators=(",", ":")) + "\n")
        out.flush()

    # Named before it is read, so dbt-edith can point at the file even when
    # reading it is what fails.
    reply({"event": "profiles", "path": str(profiles_path(args.profiles_dir))})
    kwargs, target = load_profile(args.profile, args.target, args.profiles_dir)
    outputs = profile_outputs(args.profile, args.profiles_dir)[0].get("outputs", {})
    # Everything that needs no network fails here, before ready, rather than on
    # the first click.
    connector()

    reply(
        {
            "event": "ready",
            "profile": args.profile,
            "target": target,
            "role": kwargs.get("role") or "",
            "authenticator": kwargs.get("authenticator", "password"),
            "targets": target_roles(outputs, target),
        }
    )

    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue
        try:
            req = json.loads(line)
        except json.JSONDecodeError:
            continue
        if not isinstance(req, dict):
            continue
        op = req.get("op", "lineage")
        if op == "quit":
            break
        rid = req.get("id")
        phase = "request"
        try:
            if op == "lineage":
                asked = lineage_request(req)
            elif op == "history":
                asked = history_request(req)
            elif op == "session":
                asked = None
            else:
                raise ValueError(f"unknown op {op!r}")
            # Whatever the connector prints mid-session, a renewed sign-in
            # included, must not land between two replies.
            with contextlib.redirect_stdout(sys.stderr):
                if conn is None:
                    phase = "connect"
                    conn = connect(kwargs)
                    session = None
                phase = "query"
                cur = conn.cursor()
                try:
                    if op == "lineage":
                        answer = {"rows": rows_to_raw(column_lineage(cur, *asked))}
                    else:
                        if session is None:
                            session = session_info(cur)
                        answer = {"session": session}
                    if op == "history":
                        role, before = asked
                        if role is None:
                            role = kwargs.get("role") or session.get("role")
                        rows, more = query_history(cur, None if role == "*" else role, before)
                        answer.update(rows=rows, more=more)
                finally:
                    cur.close()
            reply({"id": rid, **answer})
        except Exception as e:  # noqa: BLE001 - every failure becomes a reply
            reply({"id": rid, "error": first_line(e, 300), "phase": phase})
            # A dead session would fail every later request the same way.
            if conn is not None and getattr(conn, "is_closed", lambda: False)():
                conn = None

    if conn is not None:
        conn.close()


# ------------------------------------------------------------------ dump ----
def cmd_dump(args):
    kwargs, target = load_profile(args.profile, args.target, args.profiles_dir)
    resolve = make_resolver(args.manifest)
    manifest = json.loads(Path(args.manifest).read_text())

    wanted = [w.strip() for w in args.select.split(",") if w.strip()]
    targets = []
    for uid, node in manifest.get("nodes", {}).items():
        if node.get("resource_type") != "model" or not node.get("relation_name"):
            continue
        if wanted and node["name"] not in wanted and uid not in wanted:
            continue
        cols = sorted(node.get("columns", {}))
        if cols:
            targets.append((uid, node["relation_name"], cols))
    if not targets:
        die("no model matched --select, or the matched models declare no columns in YAML")

    total_cols = sum(len(c) for _, _, c in targets)
    print(f"{len(targets)} model(s), {total_cols} column(s), {args.threads} thread(s)", file=sys.stderr)

    work = queue.Queue()
    for uid, rel, cols in targets:
        for c in cols:
            work.put((rel, c))
    found, errors, done = [], [], [0]
    lock = threading.Lock()

    def worker():
        conn = connect(kwargs)
        cur = conn.cursor()
        while True:
            try:
                rel, col = work.get_nowait()
            except queue.Empty:
                break
            try:
                edges = rows_to_edges(column_lineage(cur, rel, col, "UPSTREAM", 1), resolve)
            except Exception as e:  # noqa: BLE001
                with lock:
                    errors.append(f"{rel}.{col}: {first_line(e, 120)}")
                edges = []
            with lock:
                found.extend(edges)
                done[0] += 1
                if done[0] % 25 == 0:
                    print(f"  {done[0]}/{total_cols} columns, {len(found)} edges", file=sys.stderr)
        cur.close()
        conn.close()

    threads = [threading.Thread(target=worker, daemon=True) for _ in range(args.threads)]
    [t.start() for t in threads]
    [t.join() for t in threads]

    write_cache(args.out, found, target, errors)
    print(f"wrote {args.out}: {len(found)} edges, {len(errors)} error(s)", file=sys.stderr)
    for e in errors[:5]:
        print(f"  {e}", file=sys.stderr)


def make_resolver(manifest_path: str | None):
    """Maps DB.SCHEMA.OBJECT back to a dbt unique_id, so the cache is environment neutral."""
    by_rel = {}
    if manifest_path and Path(manifest_path).exists():
        doc = json.loads(Path(manifest_path).read_text())
        for coll in ("nodes", "sources"):
            for uid, node in doc.get(coll, {}).items():
                rel = node.get("relation_name")
                if rel:
                    by_rel[rel.replace('"', "").upper()] = uid

    def resolve(relation: str) -> str:
        return by_rel.get(relation.replace('"', "").upper(), "rel:" + relation.lower())

    return resolve


def write_cache(path, edges, target, errors):
    seen, unique = set(), []
    for e in edges:
        key = (e["from"], e["from_col"], e["to"], e["to_col"])
        if key not in seen:
            seen.add(key)
            unique.append(e)
    Path(path).write_text(
        json.dumps(
            {
                "version": CACHE_VERSION,
                "source": "snowflake",
                "target": target,
                "generated_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
                "errors": errors[:50],
                "edges": unique,
            },
            indent=1,
        )
    )


# ------------------------------------------------------------------ main ----
def main():
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("command", choices=["probe", "serve", "dump"])
    p.add_argument("--profile", default=None, help="dbt profile (default: read from ./dbt_project.yml)")
    p.add_argument("--target", default=None)
    p.add_argument("--profiles-dir", default=None)
    p.add_argument("--manifest", default="target/manifest.json")
    p.add_argument("--out", default="target/column_lineage.json")
    p.add_argument("--select", default="", help="dump: comma separated model names")
    p.add_argument("--threads", type=int, default=6)
    p.add_argument("--relation", default=None, help="probe: fully qualified object")
    p.add_argument("--column", default=None, help="probe: column of that object")
    args = p.parse_args()
    args.profile = args.profile or profile_from_project()
    if not args.profile:
        die("no --profile given and no dbt_project.yml in the current directory to read it from")

    if args.command == "probe":
        if not args.relation or not args.column:
            die("probe needs --relation DB.SCHEMA.TABLE and --column NAME")
        cmd_probe(args)
    elif args.command == "serve":
        cmd_serve(args)
    else:
        cmd_dump(args)


if __name__ == "__main__":
    main()
