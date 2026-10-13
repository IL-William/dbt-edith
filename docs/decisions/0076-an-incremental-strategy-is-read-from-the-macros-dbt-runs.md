# 0076. An incremental strategy is read from the macros dbt runs

Date: 2026-10-13 · Status: accepted · Amends 0018 and 0028

**Trigger:** read before showing what an incremental model does on a run,
following `adapter.dispatch`, keeping any `macro_sql`, or reading another
block of `dbt_project.yml`.

## Context

A box on the lineage said `incremental` and nothing of what a rerun does. Most
incremental models on a large real project configure no strategy, so the
adapter picks one. And on Snowflake a merge with no unique key is a plain
insert: the box would say merge where every row is appended. Both answers
live in macros, which a project or a package can override by name.

## Decision

- **The default and the insert are read from `macro_sql`**, the way dbt
  reaches each macro: by name from the model's package, the root project,
  then dbt's own packages; by `adapter.dispatch` for dbt's own, in the
  order `dispatch:` gives the `dbt` namespace, else the root project then
  dbt's, the adapter's prefix, its parents', then `default__` in each. The
  materialization's `config.get('incremental_strategy') or '<x>'` names the
  fallback; a default macro whose body is one `return(...)` names the rest.
- **Anything else names nothing.** A project's materialization, a condition
  in a default macro, two parent adapters, an unreadable `dispatch:` with a
  package that could come first: the box keeps `incremental`, and the card
  says the default could not be read.
- **The insert is claimed only on three known macros**, by package, name and
  their own test of the key: Snowflake's merge when the key is none, dbt's
  merge and delete+insert when it is falsy. The box then reads `append` with
  the strategy struck through.
- **Only those bodies are kept.** `RawMacro` reads `macro_sql` and drops it
  at once unless it names an incremental strategy macro or the strategy.
- **The 0018 scanner reads `dispatch:` too**, a sequence of mappings, and
  calls the whole block unreadable rather than half read.

## Rejected

- **A table of defaults per adapter**: wrong as soon as a project overrides
  a macro, and stale with each adapter release.
- **Every macro body kept**, which is the bulk of a large manifest.
- **Evaluating the Jinja**: a template engine, for one fallback and one test.
- **Claiming the insert for any macro testing `unique_key`**, DuckDB's
  among them: a body read loosely is a claim nobody checked.

## Consequences

The selector still matches the configured strategy, as `dbt ls` does (0033),
so a model on the default is not `config.incremental_strategy:merge`; the
card says where the strategy comes from for that reason. A dbt release that
rewrites a known macro under the same name stops the claim until the test
it is known by is updated here.
