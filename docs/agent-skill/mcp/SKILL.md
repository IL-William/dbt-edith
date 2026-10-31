---
name: dbt-edith-mcp
description: "Answer questions about a dbt project from its manifest with the `edith` MCP tool: what a model, source or test is (materialization, incremental strategy and unique key, where dbt builds it, its config per .env file, columns and types, tests, parents, children), lineage (upstream or downstream, counts by layer, the sources feeding a model, what dropping a model breaks), column-level lineage (a column to its source), which DAGs or jobs run a node and what a job runs, and an overview. The tool is deferred: load it first. USE FIRST for any such question, before grepping SQL files or running `dbt ls`."
---
<!-- written by dbt-edith {version}; `dbt-edith --skill mcp` prints the text matching the installed binary -->

# dbt-edith over MCP

The `edith` tool is deferred: load it before anything else (Claude Code: ToolSearch `edith`; elsewhere the deferred tool named edith). Call it with one `line` per question: the flags below, no `dbt-edith`, no project. It never runs dbt or reaches a warehouse: every call is safe unasked except `--run-collin`.

| To know | Line |
| --- | --- |
| the project, its job count | `--overview --depth 2`, once, first |
| a model, source or test: columns, types, tests, materialization, strategy, unique key, parents, children | `--describe name[,name...]` (a source is `source_name.table`; a macro: args, callers); `--sql` the file, `--compiled-sql` its compiled SQL; `--columns` columns only (`-s` too) |
| downstream of a model, what dropping it breaks | `-s 'model+' --exclude resource_type:test --output compact`; `model+1` children only, `1+model` parents, `1+model+1` both |
| everything upstream of a model | `-s '+model' --exclude resource_type:test --output compact` |
| the sources feeding a model | `-s '+model,resource_type:source' --exclude resource_type:test --output compact`; add `--count-by source` to count |
| a column's sources, or what it feeds | `--column model.column --up all --output compact`; or `--down all` |
| which jobs run a node; what a job runs; what no job runs or orders | `jobs` line of `--describe name` (needs `.dbt-edith/jobs.yml`); `--job name`; `--job-coverage`; `--job-order` |

`--count-by` counts a selection by `folder` (`--depth N`, layers), `source`, `kind`, `materialized`, `tag` or `package`.

The card's `relation` is the parsed target's, `configured` the config as written; each `.env` line is that config per file, before the `generate_*_name` macros (not evaluated); say it may differ (a source's is exact). `<target>`: left to the target. `<VAR not set>`, `<VAR placeholder>`: missing or placeholder. `<hidden>`: a credential. `?`: not evaluated or empty. `~`: one branch of an `if`.

Column lineage reads collin's or Fusion's cache beside the manifest, else fails. `--run-collin` runs collin over compiled SQL and rewrites its cache: ask first. After `dbt parse` alone an empty or partial answer is incomplete.

## Habits

- **Leave the tests out unless asked.** A test joins its parent's selection: `--exclude resource_type:test` drops them, `--describe name` lists a node's.
- **Big project: count, then narrow.** A long list or several cards can pass a limit: `--count-by folder` or `tag` first, list a narrow one, describe 2-3 per call.
- **Several `--exclude` terms go in one quoted value.** `--count-by` counts sources, seeds and the start too: `--exclude 'resource_type:test resource_type:source resource_type:seed resource_type:exposure'` counts models, snapshots and analyses; subtract the start if selected.
- **Jobs touching a set**: `--overview --depth 0 --output json` names every job, `--job name` lists the nodes each runs and the steps' order, sensors included; intersect with the selection. One call per job; say which were skipped.
- **Confirm before anything irreversible**: `dbt ls` the selection before a run, drop or delete.

## Freshness

Answers end on a `freshness:` line. On `stale`, have the user run `dbt parse`; ask again. A refused flag means dbt-edith is older than this text: update it or run `dbt-edith --skill mcp`.
