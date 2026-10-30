---
name: dbt-edith
description: "Answer questions about a dbt project from its manifest with the `dbt-edith` command, without running dbt: what a model, source or test is (materialization, incremental strategy and unique key, where dbt builds it, its config per .env file, columns and types, tests, parents, children), lineage (upstream or downstream, the sources feeding a model, counts by layer, the impact of dropping or renaming a model), column-level lineage (a column to its source), which DAGs or jobs run a node and what a job runs, and a short overview. USE FIRST for any such question, before grepping SQL files or running `dbt ls`."
---
<!-- written by dbt-edith {version}; `dbt-edith --skill` prints the text matching the installed binary -->

# dbt-edith

Run `dbt-edith` from the project root as `dbt-edith . <flags>`. It reads the manifest, never runs dbt or reaches a warehouse, so every call below is safe unasked except `--run-collin`.

| To know | Run |
| --- | --- |
| the project, its job count | `dbt-edith . --overview`, once, first |
| a model, source or test: columns, types, tests, materialization, strategy, unique key, parents, children | `dbt-edith . --describe name[,name...]` (a source is `source_name.table`); `--sql` adds the file |
| downstream of a model, what dropping it breaks | `dbt-edith . -s 'model+' --exclude resource_type:test --output compact`; `model+1` children only, `1+model` parents, `1+model+1` both |
| everything upstream of a model | `dbt-edith . -s '+model' --exclude resource_type:test --output compact` |
| the sources feeding a model | `dbt-edith . -s '+model,resource_type:source' --exclude resource_type:test --output compact`; add `--count-by source` to count |
| a column's sources, or what it feeds | `dbt-edith . --column model.column --up all --output compact`; or `--down all` |
| which jobs run a node; what a job runs; what no job runs or orders | `jobs` line of `--describe name` (needs `.dbt-edith/jobs.yml`); `--job name`; `--job-coverage`; `--job-order` |

`--count-by` counts a selection by `folder` (`--depth N`, layers), `source`, `kind`, `materialized`, `tag` or `package`. Never count with a pipe.

The card's `relation` is the parsed target's, `configured` the config as written; each `.env` line is that config per file, before the `generate_*_name` macros (not evaluated), so say it may differ (a source's is exact). `<target>`: left to the target. `<VAR not set>`, `<VAR placeholder>`: missing or placeholder. `<hidden>`: a credential. `?`: not evaluated or empty. `~`: one branch of an `if`.

Column lineage reads collin's or Fusion's cache beside the manifest, else fails. `--run-collin` runs collin over compiled SQL and rewrites its cache: ask first. After `dbt parse` alone an empty or partial answer (exit 0) is incomplete.

## Habits

- **Leave the tests out unless asked.** A test joins its parent's selection: `--exclude resource_type:test` drops them, `--describe` lists a node's. Compact for lists, the card for one node.
- **Big project: count, then narrow.** A long list (hundreds) or several cards can pass an answer limit: `--count-by folder` or `tag` first, list a narrow one, describe 2-3 names per call.
- **Several `--exclude` terms go in one quoted value.** `--count-by` counts sources, seeds and the start too: `--exclude 'resource_type:test resource_type:source resource_type:seed resource_type:exposure'` counts models, snapshots and analyses; subtract the start if selected.
- **Jobs touching a set**: `--overview --depth 0 --output json` names every job (text cuts at 15), `--job name` lists the nodes each runs and the steps' order; intersect with the selection. One call per job: with many, say which were skipped. Models no job runs: `dbt-edith . --job-coverage --output compact`.
- **Confirm before anything irreversible**: `dbt ls` the selection before a run, drop or deletion.

## Freshness

stderr ends with a `freshness:` line. On `stale`, have the user run `dbt parse`, then ask again. A refused flag means dbt-edith is older than this text: update it or run `dbt-edith --skill`.
