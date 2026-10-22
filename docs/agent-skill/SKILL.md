---
name: dbt-edith
description: "Answer questions about a dbt project from its manifest with the `dbt-edith` command, without running dbt: what a model, source or test is (materialization, incremental strategy and unique key, the database.schema.table dbt builds it as, its config per .env file, columns, tests, parents, children), lineage (upstream or downstream, the sources feeding a model, counts by layer, the impact of dropping a model), column-level lineage, which DAGs or jobs run a node and what a job runs, and a one-screen overview. USE FIRST for any such question, before grepping SQL files or running `dbt ls`."
---
<!-- written by dbt-edith {version}; `dbt-edith --skill` prints the text matching the installed binary -->

# dbt-edith

Run `dbt-edith` from the dbt project root as `dbt-edith . <flags>`. It reads the manifest, never runs dbt or reaches a warehouse, so every call below is safe unasked except `--run-collin`.

| To know | Run |
| --- | --- |
| what the project is, its job count | `dbt-edith . --overview`, once, first |
| a model, source or test: columns, types, tests, materialization, strategy, unique key, parents, children | `dbt-edith . --describe name[,name...]` (a source is `source_name.table`); `--sql` adds the file |
| children, parents, both of a model | `dbt-edith . -s 'model+1' --exclude resource_type:test --output compact`; `1+model` parents, `1+model+1` both |
| everything upstream of a model | `dbt-edith . -s '+model' --exclude resource_type:test --output compact`; `-s 'model+'` for downstream, what dropping it breaks |
| the sources feeding a model | `dbt-edith . -s '+model,resource_type:source' --exclude resource_type:test --output compact`; add `--count-by source` to count |
| a column's sources, or what it feeds | `dbt-edith . --column model.column --up all --output compact`; or `--down all` |
| which DAGs or jobs run a node; what a job runs | `jobs` line of `--describe name` (needs `.dbt-edith/jobs.yml`); `dbt-edith . --job name` |

`--count-by` counts a selection by `folder` (`--depth N`, layers), `source`, `kind`, `materialized`, `tag` or `package`. Never count with a pipe.

The card's `relation` is the parsed target's, `configured` the config as written; each `.env` line is that config per file, before the `generate_*_name` macros (not evaluated), so say a custom schema may differ. `<target>`: left to the target. `<VAR not set>`, `<VAR placeholder>`: missing or a placeholder. `<hidden>`: a credential. `?`: not evaluated or empty. `~`: one branch of an `if`.

Column lineage reads collin's or Fusion's cache beside the manifest, else fails. `--run-collin` runs collin over compiled SQL and rewrites its cache: ask first. After `dbt parse` alone an empty or partial answer is not the truth.

## Habits

- **Leave the tests out unless asked.** A test joins a selection with its parent: `--exclude resource_type:test` drops them, `--describe` lists a node's. Compact for lists, the card for one node.
- **Big project: count, then narrow.** A long list (hundreds of nodes) or several cards can pass a client's answer limit: `--count-by folder` or `tag` first, list a narrow selection, describe two or three names per call.
- **Several `--exclude` terms go in one quoted value.** `--count-by` counts sources, seeds and the start too: `--exclude 'resource_type:test resource_type:source resource_type:seed'` counts models, snapshots and analyses; subtract the start if selected.
- **Jobs touching a set, or models no job runs**: `dbt-edith . --overview --output json` names every job (the text cuts at 15), `--job name` lists the nodes each runs; intersect with the selection, or subtract their union from it. One call per job: with many, say which were not checked.
- **Confirm before anything irreversible**: check a selection with `dbt ls` before a run, drop or deletion acts on it.

## Freshness

stderr ends with a `freshness:` line. On `stale`, ask the user to run `dbt parse`, then ask again. A refused flag means dbt-edith is older than this text: update it, or run `dbt-edith --skill`.
