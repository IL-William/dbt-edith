# 0125. The jobs are checked for the dependencies they do not order

Date: 2026-10-28 · Status: accepted · Amends 0063, 0077 and 0115

**Trigger:** read before changing what `--job-order` lists, what counts in it as
ordered, which jobs it looks at, or what a job file may say that orders a step.

## Context

The jobs of 0063 say which nodes each step runs and when each job starts. Whether
they run in an order that matches the models is the question that decides
whether a night's build is right: a step that builds `orders` reads
`stg_orders`, and if another DAG builds `stg_orders` with nothing making the
first wait, it reads the previous run's. An agent could only answer it by
joining `--job` for every DAG with the parents of every model and with the
YAML, by hand.

## Decision

- **A mode of its own, `--job-order`**, over the same read as `--job` (the
  project's `.dbt-edith/jobs.yml`, once per call, 0115), in text and JSON, over
  MCP through the same line (0110). It lists pairs: a step that consumes a node,
  the steps that build a parent of it, and the models and parents that make the
  link. It answers 0 whether it finds pairs or none, and 2 where `--job` does.
- **Who is in it.** The steps that execute of jobs that may start on their own,
  `Start::Yes` or `Start::Unread` (0077). A job nothing starts (manual, CI, a
  merge) is left out on both sides, or one ad hoc `+x` run would flag the whole
  project; the answer names them. A step that could not be resolved and could
  build a node is counted: it could hide a pair.
- **Consumers and producers are built nodes**: a model that is not a view, an
  ephemeral or a dynamic table (a dynamic table refreshes on its own lag, and
  dbt's build of it is empty when it is unchanged, so a run neither leaves it
  current nor reads from it; a materialized view is built, since dbt refreshes
  it on every run), a seed, a snapshot, not disabled; a model with no materialization
  in the manifest is dbt's default, a view. A step consumes the built nodes it
  runs and produces the same. Tests are neither, so a DAG that
  gates its tests on a tag is not found; that waits for the
  tests to count as readers (`docs/state.md`).
- **The walk.** For each built node a step runs, its parents are looked at in
  order of distance. A built parent the step runs itself ends the walk, since
  dbt orders inside a step. A built parent that other in-scope steps build is a
  requirement: the step should wait for one of them. A built parent nothing
  builds, or a source, ends it. A view, an ephemeral or a dynamic
  table is always walked through, whoever selects it, to its own parents, and is named after `through`:
  an ephemeral is inlined, and a view reads its parents live, so re-creating one
  reads no data and never makes a step a consumer.
- **Ordered is an ancestor in a graph the files draw**, nothing else. Its points
  are one per dbt step, per task with no dbt command, per sensor and per DAG
  with no dbt task that updates an asset, and one start per job. A task is after
  its DAG's start and after the tasks its `depends_on` names; a sensor after
  each task it names in the DAG it names, or after every task of it only when it
  names neither a task nor a task group (a group's tasks are not read, so a
  sensor on one orders nothing and is no whole-run sensor); a DAG on
  assets after the tasks that update them, and after the DAGs with no dbt task
  that do; a dbt Cloud step after the one written before it, and a job started
  by another's end after its last step. **A sensor orders only the tasks that
  name it**, directly or through `depends_on`, as Airflow does: a sensor no task
  depends on holds nothing back. A pair whose DAG has a sensor the files put
  after a step it should wait for (its task, the DAG, or a later task of it),
  and which this task does not depend on, says so (`idle_sensors`; a sensor
  with no id, which no task can name, is always one). A sensor on its own DAG
  waits for the previous run, so it adds no order within this one. A pair whose
  step depends on a sensor waiting for a task group of a DAG holding one of the
  steps it should wait for says so (`group_sensors`): the files may order them,
  and the group's tasks are not read. An asset several tasks
  update starts a run on the first update, so a DAG on it is after what ALL its
  updaters are after, and after every one of the DAG's assets.
- **Three groups.** `reversed` when the files run every step it should wait for
  after it; else `clock` when a step it should wait for is in another job, both
  jobs have a cron, and their (cron, zone) differ, where a preset is spelled out
  and spaces are collapsed first, an Airflow zone is its source's (0119) and
  dbt Cloud's is UTC; else `none`. The clock is shown, never ranked: whether
  `0 5 * * *` finishes before `0 6 * * *` is not in the files. Some runners
  after it and some not leaves the pair in its group and names them as `later`.
- **Approximations, not coded**, and stated in the page: a DAG with no dbt task
  that updates an asset is one point, its sensors and `depends_on` unread, and a
  sensor on any of its tasks or its whole run counts as after all of it; a DAG
  started by a `TriggerDagRunOperator` reads as manual; an Airflow task that
  starts a dbt Cloud job is not read; a sensor across crons without the right
  `execution_delta` counts as ordering; `trigger_rule` is not read; several
  updaters of one asset are as above. A DAG whose `depends_on` loops is not one
  Airflow loads: it is left out as a consumer and as a producer and listed
  (`cyclic`), found from the graph and not from where a walk starts, so the
  answer does not change with the order pairs are looked at. A loop between
  DAGs, through sensors or assets, is no such error: a point met again on the
  walk adds nothing, and every point is settled in one fixed order first.
- **Output.** Text opens with the counts, `pairs N: a nothing orders them, b only
  the clock, c the other way`, so an answer cut by a client still says how much
  there was; JSON has `configured`, `counts`, `pairs`, `unresolved`,
  `left_out`, `cyclic` and `not_read`, no total apart from them. Pairs and edges are sorted by names
  (0055), at most five edges a pair in text.
- **A task with no dbt command is kept** with its `depends_on` and `outlets`,
  per task, for the order through it. `Job.outlets` stays what starts DAGs.

## Rejected

- **Counting tests as readers.** A test reads the model it tests, and a
  DAG that gates its tests on a tag does that. It would also add every test of
  every model to the count, which the first answer should not carry; left in
  `docs/state.md` with what it needs.
- **Ranking two crons.** It needs a clock, a time zone database (0003) and the
  run lengths, and the answer would look more exact than the files are.
- **Including manual jobs.** One ad hoc run of a manual DAG is a state of the
  night nobody scheduled.
- **A sensor orders its whole DAG.** It is Airflow's rule that a task waits only
  for what it depends on; reading it the other way would hide the commonest
  mistake, a sensor placed and not wired.
- **A job argument, `--job-order NAME`.** It would only narrow the consumers
  to one job, while the steps it should wait for are in the others; a narrowing
  is left to a later `--select`.

## Consequences

*Amends 0063* (the Airflow reading keeps tasks with no dbt command, one by one),
*0077* (a job's start now also decides whether its steps count) and *0115* (the
sources file feeds a fourth mode). The mode is in `src/job_order.rs`; the
reading change is `Job.links` in `src/jobs.rs`.
