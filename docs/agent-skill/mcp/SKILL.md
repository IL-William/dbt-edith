---
name: dbt-edith-mcp
description: "Answer questions about a dbt project from its manifest with the `edith` MCP tool: what a model, source or test is (materialization, incremental strategy and unique key, its config per .env file, columns and types, tests, parents, children), lineage (upstream or downstream, counts by layer, the sources feeding a model, what dropping a model breaks), column lineage (a column to its source), which DAGs or jobs run a node and what a job runs, and an overview. The tool is deferred: load it first. USE FIRST for any such question, before grepping SQL files or running `dbt ls`."
---
<!-- written by dbt-edith {version}; `dbt-edith --skill mcp` prints the matching text -->

# dbt-edith over MCP

The `edith` tool is deferred: load it before anything else (Claude Code: ToolSearch `edith`). Call it with one `line` per question: flags below, no `dbt-edith`, no project. It never runs dbt or reaches a warehouse: every call is safe unasked but `--run-collin`.

| To know | Line |
| --- | --- |
| the project, its job count | `--overview --depth 2`, once, first |
| a model, source or test: columns, tests, materialization, strategy, unique key, parents, children | `--describe name[,name...]` (a source is `source_name.table`; a macro: args, callers); `--sql` the file, `--compiled-sql` dbt's SQL; `--columns` columns only (`-s` too) |
| downstream of a model, what dropping it breaks | `-s 'model+' --exclude resource_type:test --output compact`; `model+1` children only, `1+model` parents, `1+model+1` both |
| everything upstream of a model | `-s '+model' --exclude resource_type:test --output compact` |
| the sources feeding a model | `-s '+model,resource_type:source' --exclude resource_type:test --output compact`; add `--count-by source` to count |
| a column's sources, what it feeds, how it is computed | `--column model.column --up all --output compact`; or `--down all`. `steps`, `expression` columns: `(cut)` a cut text, `32+` a recursive CTE, `--casts` adds casts. `--rules` prints instead the models' WHERE, JOIN, QUALIFY on the path; a card: `--describe name --rules` |
| which jobs run a node; what a job runs; what no job runs or orders | `jobs` line of `--describe name` (needs `.dbt-edith/jobs.yml`); `--job name`; `--job-coverage`; `--job-order` |

`--count-by` counts a selection by `folder` (`--depth N`, layers), `source`, `kind`, `materialized`, `tag` or `package`.

`relation` is the parsed target, `configured` the config as written; an `.env` line is that config per file, before the `generate_*_name` macros (not evaluated): it may differ (a source's is exact). `<target>`: left to the target. `<VAR not set>`, `<VAR placeholder>`: missing, placeholder. `<hidden>`: a credential. `?`: not evaluated, empty. `~`: one `if` branch.

Column lineage reads collin's or Fusion's cache beside the manifest, else fails. `--run-collin` rewrites it from compiled SQL: ask first. After `dbt parse` alone an empty or partial answer is incomplete.

## Habits

- **Tests out unless asked.** A test joins its parent's selection; `--exclude resource_type:test` drops them, `--describe name` lists a node's.
- **Big project: count, then narrow.** Long lists or many cards can pass the limit: `--count-by folder`/`tag` first, list a narrow one, describe 2-3 per call.
- **Several `--exclude` terms go in one quoted value.** `--count-by` also counts sources, seeds, the start: `--exclude 'resource_type:test resource_type:source resource_type:seed resource_type:exposure'` counts models, snapshots and analyses; subtract the start if selected.
- **Jobs touching a set**: `--overview --depth 0 --output json` names every job, `--job name` lists the nodes each runs and the steps' order, sensors included; intersect with the selection, one call per job, saying which were skipped.
- **Confirm before anything irreversible**: `dbt ls` the selection first.

## Freshness

Answers end on a `freshness:` line. On `stale`, have the user run `dbt parse`, ask again. A refused flag: dbt-edith is older than this text; update it or run `dbt-edith --skill mcp`.
