# 0137. Reports open over the whole window, in one grammar

Date: 2026-11-02 · Status: accepted, amended by 0140 · Amends 0064, 0091, 0101 and 0131

**Trigger:** read before changing what the Reports panel covers, how its rail
is grouped or counts, how a section lays out its lists, or what its filter
searches.

## Context

0101 left the reports beside the sidebar: lists, read next to the files they
lead to. In use, the job sections read worst. Their rows wrapped `job › task`
onto two lines and cut commands and paths off, each pair of Job order was one
long bold sentence, Warnings said one sentence under each path, and a Checks row
listed a key's columns twice. Only the last run and collin's report had figures
on top, findings grouped by what fixes them, and folds of a title, a count, one
line and compact entries.

## Decision

- **Reports cover the sidebar too.** `PANELS.reports` is `wide`, its section
  sits in `#body` beside the Custom selection's, and the sidebar and its gutter
  go inert while it is down. The settings stay narrow: opened from Sources…
  they give the sidebar back, and their `closed` hook puts the wide Reports back.
  *Amended by [0140](0140-the-top-bar-switches-between-spaces.md): the
  settings are wide too, and the `closed` hook still puts Reports back on
  Escape.*
- **The rail is grouped by who reads the files**: dbt, collin, Jobs, Snowflake.
  A row carries a count with a title when it has one, and every count is a
  number its section shows: from the index, or, for Job order, from what the
  section left when it was last drawn, since counting it would ask for the job
  files. The Builds have none, one number across environments matching nothing
  on the page.
- **Each source has its colour** in the rail, as a rule down its group, and in
  a chip at the head of its sections, always with its name beside it, never by
  colour alone. Jobs has its own, away from the others.
- **Every section is figures, notes, a filter, folds.** The filter is held
  against the data of each fold (`find`), not the rows drawn, so it reaches past
  a cap such as Show all's hundred. A fold says `n of m` while it narrows, and a
  fold or group with nothing left is dropped.
- **The coverage's counts are tiles** on the page: the sentence 0131 words for
  the command line stays the command line's alone, and the page draws the same
  counts as figures over its lists.
- **An explanation is folded to its first sentence**, the rest behind **why**.
  The filter, what Show all opened, the open folds, the unfolded whys and the
  scroll are kept per section, and put back in that order.

## Rejected

- **A filter on the drawn rows**: it would miss what a cap hides.
- **One dot per row for its tool**: the group says it already. The dot stays on
  the last run alone, where it says how the run ended.
- **A fetch to count the rail**: the counts are what the page already knows.
