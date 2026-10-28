# 0077. A start no file says is neither scheduled nor unscheduled

Date: 2026-10-14 · Status: accepted, amended by 0125 · Amends 0063 and 0066

**Trigger:** read before changing which jobs count as scheduled, what a DAG
file with no dbt task leaves behind, or what the coverage calls unscheduled.

## Context

A job counted as scheduled only on a cron it could read or a chain back to
one, and every other job as one nothing starts (0063). Three kinds were
counted wrong. A dbt Cloud job whose schedule trigger is on, its cron left
to the vars file, is on a clock. A DAG on an asset only an ingestion DAG
updates runs on that DAG's clock, but a file with no dbt task left nothing,
its outlets included. And a schedule in Jinja or written as a mapping, an
asset written as a mapping or one no file read updates, or a job no file
defines may start one or not.
The coverage listed all three as *Only in unscheduled jobs*, and its news
counted them.

## Decision

- **Three answers, not two**: scheduled, not, and not read. A chain takes the
  best of what pulls it. A DAG waits on all its assets, so it takes the worst
  of them. An asset no file read updates is not read, and so is one written
  as a mapping, as dag-factory writes one by its `uri`, or an empty list.
  Cycles still never start.
- **dbt Cloud's schedule trigger is the clock.** On, the job is scheduled
  whatever its cron says; templated, in `{{ }}` or `{% %}`, it is not read.
- **A DAG file with no dbt task leaves its schedule and its outlets**, once it
  has a DAG id and an outlet, and nothing else: it is no job, is listed
  nowhere and is never refused. A task with no command in a dbt DAG updates
  its assets too.
- **Not read is listed apart.** The coverage has *Only in jobs whose schedule
  is not read*, the news counts only what is surely not scheduled, and the
  pill says **Not read**, in amber, ranked after the scheduled jobs.

## Rejected

- **A templated schedule counted as scheduled**: it may render `None` in one
  environment, and 0063 resolves nothing by guessing.
- **Every DAG file read as a job**: the lists would fill with DAGs that run
  no node.
- **A note alone**, the counts left as they were: the coverage would go on
  saying unscheduled where it does not know.
- **`schedule_interval` or a timetable found by its key**: 0063 rejected
  detecting keys by name, and a source renames `schedule`.

## Consequences

The coverage moves on a project with such jobs, models leaving *Only in
unscheduled jobs* for a scheduled job or the new list. An asset updated from
outside Airflow, through the API or a watcher, reads as not read even when it
comes every hour. `/api/jobs/node` and the coverage's jobs carry
`schedule_unread` and `assets_unread`, and the coverage `unread`.
