# 0032. Selections are checked against dbt itself, on a copy of Jaffle Shop

Date: 2026-09-30 · Status: accepted · Follows 0013, 0024 and 0031

**Trigger:** read before changing how a selection resolves, before touching
`tests/fixtures/jaffle_shop/`, and when the `dbt` job fails in CI.

## Context

`src/select.rs` and `src/selectors.rs` re-implement dbt's selection (0024,
0031). Their unit tests assert what the code's author read in dbt's source, so
a misreading sits in the code and in its test alike, and both pass. The only
check against dbt itself was a one-off run of `dbt ls` on a client project,
which cannot live in a public repository (0014).

## Decision

`tests/fixtures/jaffle_shop/` is dbt Labs' `jaffle_shop_duckdb`, Apache-2.0,
copied at a pinned commit, plus a `selectors.yml`, three singular tests and two
tags, which give every `indirect_selection` mode an answer of its own.
`scripts/compare_with_dbt.py` parses a temporary copy with dbt, serves the
manifest with dbt-edith, and compares every named selector and a list of typed
lines with `dbt ls`, name for name.

It needs dbt-core and dbt-duckdb, so it skips itself without them and
check.sh still needs nothing installed (0013). CI installs dbt-core 1.11 and
1.12, the two manifest shapes 0031 reads, and fails when it skips. `DBT` runs
an executable instead, which is how dbt Fusion is compared, locally.

## Rejected

- **The new `jaffle-shop`**: it needs `dbt deps`, so the network.
- **Cloning the fixture in CI**: the network again, and an upstream that can
  move or be archived. The copy is 31 KB and never part of the binary.
- **Expectations written by hand only**: they share the author's reading.
- **Comparing with the tests box off**: it has no equivalent in `dbt ls`.

## Consequences

The `dbt` job depends on PyPI. A new dbt-core is compared only once the matrix
names it, and Fusion only on a machine that has it.
