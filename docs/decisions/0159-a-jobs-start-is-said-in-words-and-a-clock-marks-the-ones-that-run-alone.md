# 0159. A job's start is said in words, and a clock marks the ones that run alone

Date: 2026-11-08 · Status: accepted · Amends 0063 and 0077

**Trigger:** read before changing how the Jobs tab words a job's schedule,
what marks a job as running on its own, or what the tab's header says.

## Context

The Jobs tab drew each job as a card with a green **Scheduled** pill that held
the cron, a grey **Manual** one, an amber **Not read** one, and `CI` written
inside whichever pill the job had. A cron such as `0 5 * * 1-5` made the reader
do the arithmetic, and the colour of the pill was the only thing that told a job
that runs on its own from one that does not. The design draws a card per job
with its tool's colour down its side, the schedule said in words beside the
cron, CI and merge as tags, and a header that names where the jobs were read
from.

## Decision

- **A cron is worded only where it reads as one short sentence.** Presets
  (`@daily`, `@weekly`, `@monthly`, `@yearly`, `@hourly`), a time or up to four
  times a day on every day, on weekdays, on weekends or on a few named days, one
  day of the month from the 1st to the 28th (a 29th to 31st is skipped by the
  months without it, so no sentence says "each month"), every hour, every N hours or minutes where N divides the day
  or the hour. Anything else (both day fields restricted, a month, `L`, `W`,
  `#`, a range of hours, six fields, a templated string, `@once`) is left as
  the cron alone. The words are built on `cronFields`, which already refuses
  what is not a cron, so a malformed one is never worded.
- **The cron stays beside the words,** and the zone it is read in after it,
  always: the Airflow source's, UTC where it names none, and UTC for a dbt Cloud
  job, as the builds by job judge it (0119). A time without its zone is not
  exact, and a zone shown only when set would leave the reader guessing which
  clock an unmarked one is on.
- **A clock marks the jobs that run on their own,** in place of the green pill.
  Not read, other and manual jobs are told by their words and the schedule's
  tooltip; the unread ones are worded in the warning's colour. One meaning, one mark
  (0148).
- **CI and merge are tags** in the card's head, each with a tooltip saying it is
  never started by a clock (a job only they start draws no schedule to carry
  one), and `on each pull request` and `on each merge` are said on the line
  under it with the environment and the target, not inside a pill.
- **The header names the sources and the age of the last look:** "Jobs that run
  customers", how many jobs run it and how many steps could not be resolved (the
  Catalog chip's figure), then "read from a and b, 1 min ago". Only a source
  that was read is named there: one whose path failed or whose walk was cut at
  its bound is said apart, in the warning's colour, as "c not read (nothing at
  this path)", and with no source read at all only that part is drawn, with no
  age. The age is the page's last answer to a request that looked at the job
  files, rewritten in place by the five-second poll, not the server's read
  time.
- **The step lists why it is there in one line** (`selects customers by name ·
  runs after the task staging`), the commands of a card in one grid so they line
  up, and a reason the server gave in a warning box.
- **Sources… and Coverage in Reports sit in the dock's tab row,** where the
  lineage's tools sit while it is open, and the row's fold measures the
  lineage's tools alone.
- **Three fields leave the server:** the name of each source read (its label,
  else its path, as 0012 already lets leave) in the order read, the names of
  those not read, and `not_read`, the same count and reason every other answer
  gives for them (`unread`), so the header says where the jobs came from and
  where they did not.

## Rejected

- **A cron library.** No new dependency (0003), and `cronFields` already reads
  the grammar the builds by job use (0119).
- **Words for every cron.** A sentence for `15 3,9 1-7 * mon` is longer than the
  cron and says less.
- **The Scheduled pill kept beside the clock.** Two marks for one meaning (0148).
- **The age of the server's read.** It says when the files were read, not when
  they were compared with the disk, which is what a reader asks.

## Consequences

- A cron that is not worded shows only itself, so two jobs may differ in how much
  they say.
- The header's age is of the page: any request that looked at the job files
  stamps it (the tab, the sources form, the coverage, the job order, Builds by
  job), and a page where none has shows no age.
- The Catalog's chips, the Reports lists and the command line keep the older
  wording (`jobsScheduleText`, `Scheduled` pill): only the card changed.
