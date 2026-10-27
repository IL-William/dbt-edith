# 0124. A job's answer carries its order as the files write it

Date: 2026-10-28 · Status: accepted · Amends 0115

**Trigger:** read before changing what `--job` prints of the order between
tasks and DAGs, how a sensor or a task's `depends_on` is worded there, or
whether that order is followed from job to job or checked.

## Context

The Jobs tab already says, under a task, what it runs `after` and `before`,
which sensors of other DAGs it is `awaited by`, and which assets it updates and
what they start, and on a DAG what its sensors wait for. `--job` (0115) printed
none of it, though `jobs.rs` had parsed all of it for the tab: an agent asked
which of two jobs runs first read the orchestrator's YAML by hand, which is
what the command exists to spare it.

## Decision

- **The answer prints the order the files declare, and nothing derived from
  it.** Under a DAG: its sensors, the dbt Cloud jobs its end starts, and what
  its tasks with no dbt command update with the DAGs scheduled on it. Under a
  step: one `order:` line, `after`, `before`, `awaited by` and `updates … which
  starts …`, joined by ` · ` and left out when empty. The JSON carries the same
  as fields, each skipped when empty, the step fields keeping the Jobs
  payload's names, and the flags that say a value is unread (`after_unread`,
  `assets_unread`) beside the value.
- **Nothing is followed and nothing is checked.** The answer does not walk from
  one job to the next, does not say that a job runs before another, and does
  not judge a missing sensor. That is a question about the order between jobs,
  which this answer is the input for.
- **A sensor is named by its id**, since a task's `depends_on` names it so and
  that is how a reader learns which tasks it gates. The id is kept in `sensors`
  and in `waited_by`; a sensor written without one is the DAG alone, in the
  Jobs tab too, where a `›` with nothing after it read as a lost name.
- **A sensor on a DAG's whole run is marked**, `(whole run)` after it in
  `waited_by`, because it counts as awaiting every dbt task of the DAG and four
  tasks would otherwise read as four separate waits. The Jobs tab keeps its
  wording: it has the node in focus to say which.
- **Names are the file's.** `after` lists what `depends_on` wrote, a sensor's id
  or a task that runs no dbt included; `before` lists the tasks that run dbt,
  as the tab does. The file's order is not the run order: every task with no
  `after` starts with the DAG's run. Whether a sensor also holds back tasks that
  do not name it is the factory's to say, which the file does not: dbt-edith
  prints the file.

## Rejected

- **Ordering the jobs here.** A graph of jobs across assets, sensors and
  triggers is another answer, with cycles and unread assets to judge; it has
  its own place, and printing the declared order first is what it reads.
- **Marking the sensors of a DAG no file reads.** A DAG with no dbt task is not
  kept by name, so every ingestion DAG a sensor waits on would read as missing.
- **Rewording the Jobs tab to match.** The page shows the node in focus and the
  command line a whole job; only the sensor without an id was a defect.

## Consequences

`--job` and the MCP tool answer more per call, and the text of an answer has
new lines under a DAG and a step. Nothing else about what the page shows
changes.
