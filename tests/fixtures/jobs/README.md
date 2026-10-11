# Job fixtures

Invented jobs over Jaffle Shop (`../jaffle_shop`), never taken from a real
orchestrator (0014). `scripts/compare_with_dbt.py` points dbt-edith at these
three folders, from outside the project it reads, and compares what each step
selects with `dbt ls` (0063).

- `airflow/`: DAGs in the shape dbt-edith proposes, with its default keys.
- `airflow_renamed/`: the same idea with every key renamed, as a source's
  settings would name them, plus a folded command, a `run-operation` and a
  sensor on a DAG of the first folder.
- `dbt_cloud/`: dbt-jobs-as-code's schema, with anchors, merges, a job started
  by another one's end, and a CI job whose `state:` selection stays unresolved.
