---
name: dbt-edith-mcp
description: "Answer questions about a dbt project from its manifest with the `edith` MCP tool: what a model, source or test is (materialization, incremental strategy and unique key, config per .env file, columns, types, tests, parents, children), lineage (upstream or downstream, counts by layer, sources feeding a model, what dropping one breaks), column lineage, which DAGs or jobs run a node and what a job runs, and an overview. The tool is deferred: load it first. USE FIRST for any such question, before grepping SQL files or running `dbt ls`."
---
<!-- written by dbt-edith {version}; `dbt-edith --skill mcp` prints the matching text -->

# dbt-edith over MCP

Load the deferred `edith` tool first (Claude Code: ToolSearch `edith`). One `line` per call: flags below, no `dbt-edith`, no project. It never runs dbt or reaches a warehouse: every call is safe unasked but `--run-collin`.

| To know | Line |
| --- | --- |
| the project, its job count | `--overview --depth 2`, once, first |
| a model, source or test: its card | `--describe name[,name...]` (a source is `source_name.table`; a macro: args, callers); `--sql` the file, `--compiled-sql` dbt's SQL; `--columns` columns only |
| downstream of a model, what dropping it breaks | `-s 'model+' --exclude resource_type:test --output compact`; `model+1` children only, `1+model` parents, `1+model+1` both |
| everything upstream of a model | `-s '+model' --exclude resource_type:test --output compact` |
| the sources feeding a model | `-s '+model,resource_type:source' --exclude resource_type:test --output compact`; add `--count-by source` to count |
| a column's sources, what it feeds, how it is computed | `--column model.column --up all --output compact`; or `--down all`. `steps`, `expression` columns (empty: a copy, rename, hash, literal like `'x' as channel`, untrusted compile, cache before collin 0.11.0 or Fusion's): `(cut)`, `32+` (a recursive CTE) or `(partial: x)` in `from` (collin gave up on x in that model) means an incomplete cache there: read the SQL (`--describe name --sql`, `--compiled-sql`), trace by hand; `--casts` adds casts. `--rules` instead: the models' WHERE, HAVING, JOIN, QUALIFY on the path; a card: `--describe name --rules` |
| which jobs run a node; what a job runs; what no job runs or orders | `jobs` line of `--describe name` (needs `.dbt-edith/jobs.yml`); `--job name`; `--job-coverage`; `--job-order` |

`--count-by` counts a selection by `folder` (`--depth N`, layers), `source`, `kind`, `materialized`, `tag` or `package`.

`relation` is the parsed target, `configured` the config as written; an `.env` line is that config per file, before the `generate_*_name` macros (not evaluated): it may differ (a source's is exact). `<target>`: left to the target. `<VAR not set>`, `<VAR placeholder>`: missing, placeholder. `<hidden>`: a credential. `?`: not evaluated, empty. `~`: one `if` branch.

Column lineage reads collin's or Fusion's cache beside the manifest, else fails; `--rules` collin's only. `--run-collin` rewrites it from compiled SQL: ask first. After `dbt parse` alone answers may be incomplete.

## Habits

- **Tests out unless asked.** A test joins its parent's selection; `--exclude resource_type:test` drops them.
- **Big project: count, then narrow.** Long answers pass the limit: `--count-by folder`/`tag` first, list a narrow one, describe 2-3 per call.
- **Several `--exclude` terms go in one quoted value.** `--count-by` also counts sources, seeds, the start: `--exclude 'resource_type:test resource_type:source resource_type:seed resource_type:exposure'` counts models, snapshots, analyses; subtract the start.
- **Jobs touching a set**: `--overview --depth 0 --output json` names every job, `--job name` its nodes and step order; intersect with the selection, one call per job, say which were skipped.
- **Irreversible?** `dbt ls` the selection first.

## Freshness

Answers end on `freshness:`. On `stale`, have the user run `dbt parse`, ask again. A refused flag: dbt-edith is older than this text; update it or run `dbt-edith --skill mcp`.
