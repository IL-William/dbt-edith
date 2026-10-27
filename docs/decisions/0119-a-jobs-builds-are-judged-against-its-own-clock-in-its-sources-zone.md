# 0119. A job's builds are judged against its own clock, in the zone its source names

Date: 2026-10-27 · Status: accepted · Amends 0117, 0063 and 0115

**Trigger:** read before changing what Builds by job calls late, before reading
a cron or a time zone anywhere, or before adding a key to a job source.

## Context

0117 shows each model's last build per environment and calls one stale when its
last success is older than a threshold the user sets for the whole project. The
jobs of 0063 know which models each DAG or dbt Cloud job runs and when it
starts, so the question people ask, whether last night's run built what it
should have, can be answered per job. One threshold cannot: a weekly job is
stale for six days of each week and a job every few hours is stale after one
missed run.

## Decision

- **A second Reports section, Builds by job**, from the same read as Builds by
  environment (`S.builds`): no new query, nothing asked of Snowflake that was
  not. Models only, since the read has no seed or snapshot builds yet (0117);
  a job's seeds and snapshots are counted and said not to be read. Listed only
  when the index has both the builds and the jobs, and quiet like the first
  (0075, 0117): a click's result is no file's news.
- **A job's models** come from `GET /api/jobs/models`: for each job, the
  project's own enabled non-ephemeral models its executing steps reach, as the
  coverage counts them (0077), grouped by the step that runs them (the page
  works out the steps of a model from that). The ids go once in a table the
  jobs refer to by position, everything sorted (0055). A model's status is its last build whichever job ran it:
  `dbt_artifacts` does not record the DAG, so which job a model belongs to is
  inferred from the job's selections, and the section says so.
- **Late is judged by the job's own cron.** A model is late when it has no
  success since the fire before the last one, so a whole scheduled run went by
  without building it. That a run started at the last fire may still be going
  is why the last fire alone is not held against it: a model built since the
  last start is counted ("N of M since 05:00", with the day when the job does
  not run at least daily), never an alarm. Error and not built are as in 0117.
  The job's tone is its worst model's.
- **The cron is read in the zone of its source.** An Airflow source takes a
  `timezone` (Airflow's `default_timezone`, UTC when empty); a dbt Cloud job's
  cron is always UTC, so its source takes none and the server refuses one.
  The server holds only the shape (at most 64 characters of letters, digits and
  `_+-/`), having no tz database (0003); the page checks the name with `Intl`,
  and the sources form refuses a name the browser does not know. The key is
  written to `.dbt-edith/jobs.yml` and the settings like the others (0115).
  A DAG whose `start_date` carries a zone of its own is read by Airflow in
  that zone, not in `default_timezone`, and this tool does not read the
  Python: set that zone on its source, which then holds for every DAG of it.
- **The cron is walked in the browser**, in pure functions (`cronFields`,
  `cronPrevious`): five fields, lists, ranges, steps, month and weekday names,
  Airflow's presets, and a day of the month and a day of the week both set
  matching either as croniter, which Airflow uses, reads them: a step
  restricts its field, and a field spanning its whole range is unrestricted
  beside a `*` in the other. A cron whose hour field is not every hour is
  placed on the wall clock as Airflow 2.7 and later place it: a time the clock
  repeats fires once, at the later instant, and a time it skips fires shifted
  forward by the gap, so 01:30 in London on the night the clocks go forward is
  02:30, 01:30Z, whether the DAG runs at one time a day or at several. Only a
  cron that runs every hour (hourly, every quarter hour) counts in elapsed
  time: a wall time that does not exist is passed over and one that happens
  twice fires at both instants.
- **A job with no clock of its own is held to the global threshold** and says
  so: manual, CI, a merge, an asset or another job starting it, a start no
  file read (a templated trigger, whatever cron comes with it), a cron this
  cannot read (`L`, `W`, `#`, six fields, `@once`), a cron that never fires, a
  zone the browser does not know.
- **A job that names a target** (dbt Cloud's `settings.target_name`; the page
  ignores one that is templated, which no run is recorded under) is judged on
  that target's runs alone; one that does not, on the worst of them, as 0117. An environment kept to another
  target (its file sets `DBT_TARGET`, 0117) says so for that job and gives no
  verdict, rather than reading every model as not built.

## Rejected

- **The global threshold for every job**: wrong for every job whose period is
  not the threshold's.
- **Reading `dbt_custom_envs` to learn the real DAG** of an invocation: it
  needs the orchestrator to set a variable, and that column can hold secrets,
  which 0117 never selects.
- **A grace period after the start**: no file says how long a run takes, so any
  number is arbitrary. The count of models built since the start says what is
  known and leaves the judging to the reader.
- **A server that evaluates the cron**: it would need a tz database or a crate
  (0003), and the page has one.
- **The timezone as a key of the files**: it describes the deployment, not the
  DAG; the same files read in another Airflow have another zone.

## Consequences

A schedule that lives on one task, which only the DAG factory knows, is the
DAG's here. Jobs chained by an asset or by completion, and manual ones, have
no clock of their own and are judged like Builds by environment judges a model.
Two jobs that run one model each judge it by their own clock, and the model
reads late for one and fine for the other, which is the point. A timezone that
is wrong for the Airflow in use makes every job of the source early or late
by its offset.
