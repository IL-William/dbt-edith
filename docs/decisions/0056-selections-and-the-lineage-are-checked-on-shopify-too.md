# 0056. Selections and the lineage are checked against dbt on Shopify too

Date: 2026-10-10 · Status: accepted · Amends 0033

**Trigger:** read before changing how a selection or the lineage resolves,
before moving the Shopify tag, and when the `dbt` job fails in CI.

## Context

0033 compares the Custom selection box with `dbt ls` on Jaffle Shop: 8 SQL
files, no source, nothing ephemeral. Model mode's lineage, `Graph::lineage`
and not the selection engine, was compared with nothing. dbt-collin had picked
Fivetran's Shopify package for the shapes Jaffle Shop lacks.

## Decision

`scripts/compare_with_dbt.py` also runs on Shopify's integration tests, a root
project reading the package from `../`, at the tag and commit dbt-collin pins:
cloned into a temporary directory, the commit checked, the dependencies
installed from the lock in `tests/fixtures/shopify/`. On both projects, the
lineage of each model at one, two and twenty levels each way, and at two with
its tests, must name what `dbt ls -s N+model+M` names. dbt answers from the
manifest it has just parsed, held in the process, a thousand answers in a
minute and a half. CI compares every model, check.sh every eighth past forty.

## Rejected

- **Copying the package**: 264 SQL files for a manifest rebuilt on every run,
  where a tag and a commit say what was read.
- **Committing dbt's answers**, as dbt-collin commits its caches: a new dbt
  could no longer move them, and catching that is why 0033 runs two versions.
- **Reading the lineage off dbt's graph in Python**: a second `+` written to
  check the first, where `dbt ls` is dbt's own.
- **dbt Labs' newer `jaffle-shop`**: it has no license, as dbt-collin found.

## Consequences

The first run found the fqn method reaching sources, which dbt's never does:
`*` listed Shopify's 87 sources, and a bare name selected a source table of
that name. That is fixed. Left out of the list: `path:` of a folder only the
package has, where dbt globs the root project's disk (0024), and `test_type:`
and `test_name:`, unknown to the box. dbt Fusion 2.0.6, by hand, agrees on all
but `file:shopify.yml`, where it lists none of the 20 tests the file declares
and dbt-core all of them; the box follows dbt-core. Offline, Shopify is
skipped and the run exits 2.
