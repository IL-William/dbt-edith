"""A stand-in for snowflake-connector-python, for scripts/take_shots.py.

It is copied there as snowflake/connector/__init__.py in a folder put first on
the server's PYTHONPATH, so that tools/sf_lineage.py imports it and reaches no
warehouse (0014, 0144). It knows the few statements that script sends, by a
substring of each, and answers them with invented rows: a session, a query
history, the runs dbt_artifacts would hold. Any other statement fails, as it
does in tools/test_sf_lineage.py.

The recent times fall within the run (from TAKE_SHOTS_STARTED, the epoch
second take_shots.py began at, to now), and the older ones sit the night
before, around 03:00 UTC, where a nightly job's would.
"""

import hashlib
import os
from datetime import datetime, timedelta, timezone


# tools/sf_lineage.py refuses a connector older than the advisories' fixes.
__version__ = "4.7.3"


def connect(**kwargs):
    return Connection()


class Connection:
    closed = False

    def cursor(self):
        return Cursor()

    def is_closed(self):
        return self.closed

    def close(self):
        self.closed = True


def uuid(seed):
    h = hashlib.sha1(seed.encode()).hexdigest()
    return f"01b{h[:5]}-0{h[5:8]}-{h[8:12]}-{h[12:16]}-{h[16:28]}"


def last_at(now, hour):
    """The last `hour`:00 that is at least an hour before `now`, so that what
    is dated within the hour after it is not in the future."""
    ref = now - timedelta(hours=1)
    t = ref.replace(hour=hour, minute=0, second=0, microsecond=0)
    return t if t <= ref else t - timedelta(days=1)


def night_start():
    """The last 03:00 UTC, when the nightly builds have run."""
    return last_at(datetime.now(timezone.utc), 3)


def within(share, now):
    """A moment within the run, `share` of the way from its start to `now`."""
    began = os.environ.get("TAKE_SHOTS_STARTED")
    start = datetime.fromtimestamp(float(began), timezone.utc) if began else now - timedelta(minutes=5)
    return start + (now - start) * share


def history():
    now = datetime.now(timezone.utc)
    night = night_start()
    # when, ms, status, error, type, text, rows: a number is how far through
    # the run, and a time is the night's.
    spec = [
        (0.95, 184, "SUCCESS", "", "SELECT",
         "select customer_id, number_of_orders\nfrom analytics.dbt_jaffle.customers\nwhere number_of_orders > 3\norder by 2 desc", 17),
        (0.85, 96, "FAILED_WITH_ERROR", "SQL compilation error:\nObject 'ANALYTICS.DBT_JAFFLE.PAYMENTS' does not exist or not authorized.", "SELECT",
         "select * from analytics.dbt_jaffle.payments limit 10", None),
        (0.6, 2290, "SUCCESS", "", "CREATE_TABLE_AS_SELECT",
         '/* {"app": "dbt", "dbt_version": "1.12.6", "profile_name": "jaffle_shop", "target_name": "snowflake", "node_id": "model.jaffle_shop.customers"} */\ncreate or replace transient table analytics.dbt_jaffle.customers as (\n  select * from analytics.dbt_jaffle.stg_customers\n)', 1),
        (0.6, 413, "SUCCESS", "", "CREATE_VIEW",
         '/* {"app": "dbt", "dbt_version": "1.12.6", "profile_name": "jaffle_shop", "target_name": "snowflake", "node_id": "model.jaffle_shop.stg_orders"} */\ncreate or replace view analytics.dbt_jaffle.stg_orders as (\n  select * from analytics.dbt_jaffle.raw_orders\n)', 1),
        (0.5, 3410, "FAILED_WITH_ERROR", "Numeric value 'N/A' is not recognized", "CREATE_TABLE_AS_SELECT",
         '/* {"app": "dbt", "dbt_version": "1.12.6", "profile_name": "jaffle_shop", "target_name": "snowflake", "node_id": "model.jaffle_shop.orders"} */\ncreate or replace transient table analytics.dbt_jaffle.orders as (\n  select order_id, sum(amount::number) as amount\n  from analytics.dbt_jaffle.stg_payments group by 1\n)', None),
        (night + timedelta(minutes=50), 94110, "SUCCESS", "", "SELECT",
         "select date_trunc('month', order_date) as month, count(*) as orders\nfrom analytics.dbt_jaffle.orders\ngroup by 1 order by 1", 11),
        (night + timedelta(minutes=40), 12040, "FAILED_WITH_ERROR", "SQL execution canceled", "SELECT",
         "select * from analytics.dbt_jaffle.orders o\njoin analytics.dbt_jaffle.customers c using (customer_id)", None),
        (night + timedelta(minutes=30), 121, "SUCCESS", "", "SHOW", "show tables in schema analytics.dbt_jaffle", 8),
        (night + timedelta(minutes=29), 88, "SUCCESS", "", "DESCRIBE", "describe table analytics.dbt_jaffle.stg_orders", 6),
        (night + timedelta(minutes=20), 342, "SUCCESS", "", "DROP", "drop view if exists analytics.dbt_jaffle.stg_orders_old", 1),
        (night + timedelta(minutes=10), 61, "SUCCESS", "", "ALTER_WAREHOUSE", "alter warehouse transforming resume if suspended", 1),
        (night + timedelta(minutes=5), 640, "SUCCESS", "", "SELECT", "select count(*) from analytics.dbt_jaffle.stg_payments where amount < 0", 1),
    ]
    rows = []
    for i, (when, ms, status, error, kind, text, count) in enumerate(spec):
        start = min(when, now) if isinstance(when, datetime) else within(when, now)
        at = int(start.timestamp() * 1_000_000_000)
        rows.append((uuid(f"{i} {text}"), at, ms, status, error or None, "TRANSFORMER", "TRANSFORMING",
                     "ANALYTICS", "DBT_JAFFLE", kind, text, len(text), count))
    return sorted(rows, key=lambda r: (r[1], r[0]), reverse=True)


def builds():
    clock = datetime.now(timezone.utc)
    now = clock.replace(tzinfo=None)
    # The last 02:00 and 05:00 UTC, when the invented jobs start.
    nightly, daily = last_at(now, 2), last_at(now, 5)
    rows = []

    def add(node, target, status, start, took, command, message=None, job=None, run=None, last_success=None, rows_affected=None):
        end = start + timedelta(seconds=took)
        rows.append((f"model.jaffle_shop.{node}", target, status, start, end, start + timedelta(seconds=1),
                     uuid(node + target + start.isoformat()), "false", rows_affected, message, command,
                     job, run, last_success or (end if status == "success" else None)))

    # prod: what the nightly build did, and a model that did not follow.
    for node, n in (("stg_orders", 99), ("stg_payments", 113), ("stg_customers", 100)):
        add(node, "prod", "success", nightly + timedelta(minutes=2), 6, "dbt build", job=1001, run=90210, rows_affected=n)
    add("orders", "prod", "error", nightly + timedelta(minutes=4), 9, "dbt build",
        "Database Error in model orders (models/orders.sql)\n  100038 (22018): Numeric value 'N/A' is not recognized",
        job=1001, run=90210, last_success=nightly - timedelta(days=2, minutes=-4))
    add("customers", "prod", "success", daily - timedelta(days=10), 12, "dbt run", job=1002, run=90001, rows_affected=100)
    # dev: a developer's own builds, more recent, one of them failed.
    for node, ago, n in (("stg_customers", 0.7, 100), ("stg_orders", 0.7, 99), ("orders", 0.75, 99), ("customers", 0.75, 100)):
        add(node, "dev", "success", within(ago, clock).replace(tzinfo=None), 5, "dbt run", rows_affected=n)
    add("stg_payments", "dev", "error", within(0.65, clock).replace(tzinfo=None), 3, "dbt run",
        "Compilation Error in model stg_payments (models/staging/stg_payments.sql)\n  'amount_cents' is undefined")
    return rows


class Cursor:
    result = []

    def execute(self, sql, params=None):
        low = sql.lower()
        if "current_organization_name" in low:
            self.result = [("JAFFLEORG", "XY12345", "ANALYST", "TRANSFORMER")]
        elif "current_account_name" in low:
            self.result = [("XY12345", "ANALYST", "TRANSFORMER")]
        elif "query_history_by_user" in low:
            rows = history()
            before = (params or {}).get("at")
            if before is not None:
                rows = [r for r in rows if r[1] < before or (r[1] == before and r[0] < params["qid"])]
            self.result = rows[: (params or {}).get("limit", 21)]
        elif "identifier(%(exe)s)" in low:
            self.result = builds()
        elif "get_lineage" in low:
            self.result = []
        else:
            raise RuntimeError("the fake Snowflake does not know this statement: " + sql[:80])

    def fetchall(self):
        return self.result

    def close(self):
        pass
