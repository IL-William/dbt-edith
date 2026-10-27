# 0117. The last build per environment is read from dbt_artifacts, on a click, and never kept

Date: 2026-10-26 · Status: accepted, amended by 0119 · Amends 0017, 0048 and 0065

**Trigger:** read before showing a model's runs read from the warehouse, before
changing what the builds read selects or sends, or before putting a status on a
lineage box.

## Context

Knowing when a model was last rebuilt in DEV, UAT and PROD meant opening each
environment's tables in Snowsight. A project that uses the `dbt_artifacts`
package already records every model run there, with the invocation's target,
command and ids. `run_results.json` (0065) says only what the last command on
this machine did.

## Decision

- **The package's own models, `model_executions` and `invocations`**, not the
  `fct_dbt__*` views over them, which a project may never build. Both are in
  the manifest like any model, so the page places them in each environment
  with the same relation the Location table gives (0113) and asks one query
  per distinct pair of tables.
- **`sf_lineage.py` gains a `builds` op** beside `history` (0048). The two
  relations are bind values of `identifier()`, never formatted into the
  statement, and a request not made of two three-part relations is refused
  before it connects. The server holds the same check (`valid_relation`).
- **What leaves the warehouse (amends 0017, 0048)**: per model and target, the
  last run's status, start (the invocation's), when the model began and ended,
  the last success, the invocation id, whether it was a full refresh, rows
  affected, dbt's message cut to 600 characters, the dbt command, the target's
  name and the dbt Cloud job and run ids. Other people's runs now reach the
  page, behind the same guard (0015). Never selected: `env_vars`, `dbt_vars`,
  `dbt_custom_envs`, `invocation_args`, `adapter_response` and the schema and
  reason columns, which can hold secrets. The Rust structs name every field, so
  a column the script starts to send reaches the page only once this build lets
  it.
- **One row per model and target, over 90 days**, the last attempt, with the
  last success kept across a later failure. One metadata table may hold several
  targets. An `.env` file that sets `DBT_TARGET` (the one value of a file
  allowed to leave, 0012) stands for that target, the one dbt runs with when
  the file is loaded (`--target` reads it), so its environment keeps only that
  target's runs and is named with it. Without it, and for the manifest's own
  build, each target is shown by name. The runs under other targets are not
  counted, but their names are shown beside the environment, so a file whose
  `DBT_TARGET` no run used (an orchestrator's own target name, a value left
  unexpanded) says so instead of reading as never built. A lineage dot, which
  has room for one colour, takes the worst of a model's targets (error, then
  stale, then not built, then ok) and the box's hover card lists each; the
  Catalog lists every target's run and the Reports section counts the worst. At
  most 50 000 rows (a model and a target each), and an answer says when it was
  cut.
- **On a click only, one environment after the other.** Nothing at startup,
  nothing polled. The answer lives in the page (`S.builds`) until it closes;
  the server writes none of it and logs no row. The script's 60 s deadline then
  holds for each environment, and each is drawn as it lands. A failure of the
  connection or the script (any whose phase is not a query's) stops the rest,
  which would fail the same way and could open a sign-in each; the ones not
  asked say so.
- **The lineage colours one environment at a time (amends 0065)**: a dot on
  each model box for the environment the selector names, from the same read,
  not from `run_results.json`. It is stripped from an export, like the
  Halloween art (0106, 0107): a live read of other environments is not kept,
  and a file is.
- **The Reports section is listed from the index and empty until read**:
  it is a click's result and no file's news, so before a read it is neither
  the section the panel opens on nor a reason to light the button (0075). It
  lists every enabled model of the project, asked of the server once per read.
- **Ok, stale, error or none, by a threshold the user sets**: error when the
  last attempt failed, stale when the last success is older than
  `builds_stale_days` (7 by default, 1 to 365, in the Snowflake settings), none
  when no run was recorded. A skipped run built nothing that time: the last
  success still decides. A partial success (some batches of a microbatch
  model failed) counts as failed. The last attempt is ordered by when the model
  ran (`query_completed_at`, else `compile_started_at`), not by
  `run_started_at`, which is the invocation's start.

## Rejected

- **The `fct_dbt__*` views**: not there unless the project builds them.
- **A connection per environment**: a sign-in each, for tables one login reads.
- **Reading at startup, or polling**: warehouse time and a sign-in nobody asked
  for (0016).
- **Keeping a history** on disk: other people's runs would outlive the page.
- **Guessing the target of an `.env` file** from its name: a wrong green dot.
  Only a `DBT_TARGET` the file sets is believed.
- **`LAST_ALTERED`** from Snowflake's metadata: a different question (when the
  table changed, not whether dbt ran it), a later change.
- **A news chip on the Reports button (0066)**: there is no file written to
  watch.

## Consequences

The resolved relation is the config before `generate_schema_name` runs
(0113's trap), so a schema macro that rewrites the package's schema sends the
query to the wrong table, which the error then shows. Seeds, snapshots and
tests have their own `*_executions` tables and are not read yet.

Every run under one target is one, whoever ran it: two developers building
`dev` into their own schemas share one last attempt. Telling them apart would
need the model's `schema` column to leave the warehouse (who built where) and
the schema each environment really builds into, which `generate_schema_name`
decides and this tool does not evaluate (0113).
