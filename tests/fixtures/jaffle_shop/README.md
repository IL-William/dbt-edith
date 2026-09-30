# Jaffle Shop, as a test fixture

dbt Labs' [jaffle_shop_duckdb](https://github.com/dbt-labs/jaffle_shop_duckdb),
Apache-2.0 (see `LICENSE`), copied at commit `20cc904` of 2026-09-28 so the
comparison with dbt runs without a network. Only what `dbt parse` reads was
kept: no images, editor settings, lock files or Docker setup.

Added for dbt-edith (0033), and nothing else changed:

- `selectors.yml`;
- three singular tests under `tests/`, which give indirect selection its
  cases: one reading two models neither upstream of the other, one reading a
  model and its own ancestor, and one with a single parent;
- the `raw` tag on the seeds and the `staging` tag on the staging models, in
  `dbt_project.yml`.

It is read by `scripts/compare_with_dbt.py` only, from a temporary copy, and is
never part of the binary.
