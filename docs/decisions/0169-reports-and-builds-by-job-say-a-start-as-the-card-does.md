# 0169. The Reports and Builds by job say a start as the card does

Date: 2026-11-10 · Status: accepted · Amends 0159 and 0148

**Trigger:** read before changing how a job's start is drawn outside the Jobs
tab (the Reports' lists, Builds by job, the Catalog's chips), or what the clock
means.

## Context

0159 changed the Jobs tab's card alone. The `Jobs read` list and Builds by job
kept the green **Scheduled** pill with the bare cron, and the Catalog's chips
kept green, grey and amber fills, so one job was worded three ways. The board
draws every chip on `--line`.

## Decision

- **One line says a start everywhere,** built by one helper used by the card,
  the `Jobs read` list and Builds by job's cell: the clock for a job that runs
  on its own, the words, the cron and its zone; `Manual` for one nothing
  starts; the warning's text colour for one not read. A CI or merge job has its
  `CI` or `Merge` tag, so its row is not empty.
- **Builds by job draws the zone under the line** whenever the cron has one,
  not only when the job is judged. The column is sized from the line's text,
  which the tooltip repeats, then what the start means, then the verdicts' rule.
- **The Catalog's chips are all outlined alike** on `--line`, scheduled jobs
  still first, and say `step 2 of 3`. The tooltip gives the job, its start in
  words with cron and zone, what that means, then the command.
- **`@quarterly` is worded:** `The 1st of January, April, July and October at
  00:00`. The command line and the job order keep the older words (`step 2/3`,
  the cron as written): the server writes them for both.
- **The clock has three meanings, and they now meet.** The Jobs tool's glyph
  (violet, beside its name), the "under way" state (amber, alone) and the
  card's muted "runs on its own" clock before a start's words. The tool's and
  the start's clocks meet in the coverage section: the source chip at the head
  of the section and its Jobs read list. 0148 said they never meet; the company
  each keeps tells them apart.

## Rejected

- **A second clock on a chip,** which already holds the Jobs tool's.
- **Keeping the chips' start colours:** the card has none.

## Consequences

A chip no longer tells a scheduled job from a manual one by sight: their order
(scheduled first) and the tooltip do.
