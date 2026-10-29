# 0127. A key is a column list from unique_key or a uniqueness test

Date: 2026-10-29 · Status: accepted · Amends 0116 · Builds on 0076

**Trigger:** read before changing which columns the Catalog tags `KEY`, what
makes a key tag dashed, the three key rules of the Checks section, or reading a
test's kwargs for anything else.

## Context

The Columns tab tagged `KEY` the columns of an incremental model's or a
snapshot's `unique_key` and nothing else. A column tested `unique` and
`not_null`, or a list tested by `dbt_utils.unique_combination_of_columns`, is a
key as much, and a key whose column may be null is the one worth a second look:
dbt's `unique` skips nulls (`default__test_unique` has `where col is not
null`), `unique_combination_of_columns` groups by the columns, nulls included,
and a merge or a delete+insert on a null key matches nothing and inserts the
row again. This was checked against dbt-core's and dbt_utils' macros, and
against the manifests of dbt-core and Fusion 2.0, which both write the test's
`kwargs.combination_of_columns` as a list.

## Decision

- **A key is a list of columns**, declared by a `unique_key` (an incremental
  model's or a snapshot's), by an enabled `unique` test on one column, or by an
  enabled `unique_combination_of_columns` test. Two declarations of one list
  are one key, compared without order or case and with quotes stripped, as the
  page's `columnKey` compares a name. Several tests of one list merge too.
  `src/keys.rs` works them out and `/api/node` sends them as `keys`.
- **One word, `KEY`, whatever declares it.** `KEY 1/2` only for a key of
  several columns: the place in the `unique_key` when the key is declared
  there, else in the first test's list. A column of one key says `KEY` alone.
- **Letters `A`, `B`... as soon as the node has two different keys**, a key of
  one column included. The order is the same on every start (0055):
  `unique_key` first, then the fewest columns, then the lowercased column list.
- **A dashed tag when a column of the key has no enabled `not_null`**, whatever
  declares the key. Coverage is read from the node's tests by their column
  name, not from the columns of the YAML, so a key column the YAML and
  catalog.json do not list still counts. The hover says the key, where it is
  declared (the `unique_key`, each test by name) and the columns left.
- **Three rules in Reports > Checks**: keys with a column not tested
  `not_null` (composites included); an incremental model's `unique_key` that
  no `unique` or combination test checks; and models with no `unique` and no
  combination test. One row per key for the first two, with the key and the
  columns left in its detail, and a row opening the Columns tab on the first
  column left. Models only, the root package's and enabled, like the other
  four (0116).
- **Only what the manifest says.** A test with a `where:` checks part of the
  rows and declares nothing, and a `not_null` with one covers nothing; a kwarg
  that is not a list of strings is Jinja left unrendered; a combination test
  named through a namespace other than `dbt_utils`, or a `unique` or `not_null`
  through one other than `dbt`, is another package's test of the same name.
  Disabled tests never reach the graph (0116).
- **A `unique_key` entry that is an expression** (`order_id || '-' || line`)
  stays an entry, counted in `1/2`, and is not checked for `not_null`: it is
  not parsed, as 0076 left it. It merges with a `unique` test written on the
  same text, which does check that expression, and with no other.
- **Checked against dbt.** `scripts/compare_with_dbt.py` works the three lists
  out again from the manifest dbt wrote, on Jaffle Shop with the shapes added
  to its copy, a combination test included (a bare generic of the project's,
  with dbt_utils' SQL, since Jaffle has no packages), and on Shopify, whose
  models are a package's and give the rules nothing to list.

## Rejected

- **A word for each source** (`KEY`, `UNIQUE`, `PK`). One word reads at a
  glance, and where a key is declared is in the hover.
- **Counting `constraints`, `dbt_constraints` and
  `dbt_expectations.expect_compound_columns_to_be_unique`.** Contract
  constraints are not enforced on every adapter, and the packages' tests have
  kwargs of their own; each is a rule to add with its own reading.
- **Reading `not_null` through the Columns' tests.** A key column the YAML
  does not list would read as uncovered.
- **Tagging sources, seeds and snapshots in the three rules.** The other four
  hold models; a snapshot's `unique_key` is tagged in the Columns tab all the
  same.

## Consequences

Disabled and package nodes are not looked at by the rules. A model with two
tests of one list is one key naming both. `--describe` and MCP do not carry
keys. A `unique` test with a `where`, which is a key of the rows it covers,
reads as no key. The test's kwarg is read as `kwargs.combination_of_columns` only,
the rest of the kwargs being dropped as the manifest is read.
