---
name: dbt-edith
description: "Answer questions about a dbt project from its manifest with the `dbt-edith` command, without running dbt: what a model, source or test is (materialization, incremental strategy and unique key, the database.schema.table dbt builds it as for the parsed target and what its config gives for each .env file, columns and their types, tests per column, parents, children), lineage (parents or children, everything upstream or downstream, the sources that feed a model, counts by layer or source, the impact of dropping or renaming a model), column-level lineage (trace a column back to its source column or forward to what it feeds) and a one-screen overview of the project. USE FIRST for any such question, before grepping SQL files or running `dbt ls`."
---
<!-- written by dbt-edith {version}; `dbt-edith --skill` prints the text matching the installed binary -->

# dbt-edith

Run `dbt-edith` (on the PATH) from the dbt project root as `dbt-edith . <flags>`. It reads the manifest and never runs dbt or reaches a warehouse, so every call below is safe unasked except `--run-collin`.

| To know | Run |
| --- | --- |
| what the project is | `dbt-edith . --overview`, once, first |
| what a model, source or test is: columns, types, tests per column, materialization, incremental strategy, unique key, parents, children | `dbt-edith . --describe name[,name...]` on the nodes asked about (a source is `source_name.table`); `--sql` adds the file |
| where a node is built | the card: `relation` is what dbt builds for the parsed target, `configured` the config as written, each `.env` line that config with the file's values |
| neighbours of a model | `dbt-edith . -s 'model+1' --exclude resource_type:test --output compact` for its children; `1+model` for its parents, `1+model+1` for both |
| everything upstream of a model | `dbt-edith . -s '+model' --exclude resource_type:test --output compact` |
| everything downstream, or what dropping or renaming a model breaks | `dbt-edith . -s 'model+' --exclude resource_type:test --output compact` |
| the sources feeding a model | `dbt-edith . -s '+model,resource_type:source' --exclude resource_type:test --output compact`; add `--count-by source` to count |
| nodes per layer upstream of a model | `dbt-edith . -s '+model' --exclude resource_type:test --count-by folder` |
| where a column comes from, or what it feeds | `dbt-edith . --column model.column --up all --output compact`; `--down all` for what it feeds |

`--count-by` counts a selection, instead of listing it, by `folder` (`--depth N`), `source`, `kind`, `materialized`, `tag` or `package`. Never count with a pipe.

A per-`.env` line is the config before the `generate_*_name` macros, which are not evaluated: dbt builds a model's custom schema as `<target schema>_<custom>` unless overridden, so say the line may differ (a source's schema is exact). `<target>`: left to the target. `<VAR not set>`, `<VAR placeholder>`: the file lacks it or holds a placeholder. `<hidden>`: a credential. `?`: not evaluated or empty. `~`: one branch of an `if`.

Column lineage reads collin's or Fusion's newest cache beside the manifest, else it fails. `--run-collin` runs collin first over compiled SQL (after `dbt parse` alone its answer is partial or empty, exit 0) and writes `target/column_lineage.collin.json` and `target/collin.report.json`: ask before using it.

## Habits

- **Leave the tests out unless asked.** A test joins a selection when its parent does: `--exclude resource_type:test` drops them, and `--describe` lists a node's tests.
- **Compact for lists, the card for one node.** JSON only to read keys.
- **Confirm before anything irreversible.** Before a run, drop or deletion acts on a selection, check it with `dbt ls`.

## Freshness

stderr ends with a `freshness:` line. When it says `stale`, ask the user to run `dbt parse` (or run it if allowed), then ask again: the answer is the manifest's.

A refused flag means the installed dbt-edith is older than this text: update it, or run `dbt-edith --skill` for the matching text.
