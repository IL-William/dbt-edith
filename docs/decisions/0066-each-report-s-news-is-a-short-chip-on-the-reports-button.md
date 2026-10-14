# 0066. Each report's news is a short chip on the Reports button

Date: 2026-10-11 · Status: accepted, amended by 0077 · Amends 0064

**Trigger:** read before giving a report news on the Reports button, changing
what a chip says, or letting the index count anything new.

## Context

The button carried news from two reports: the last run's errors and failed
tests (0065) and collin's models to investigate (0059). The coverage of the
jobs had none: nothing happens to a list of jobs between two looks, but the
project changes under it, and a model added with no job is the news there.
With three reports, chips of words made the button long.

## Decision

- **One chip per report with news**, in the sections' order, each in its
  colour: the run's red, collin's pink, the coverage's orange. Alone, a chip
  says its words, "5 to investigate"; with others, its count alone, "1", "5",
  "+1". The tooltip says every one in full, and the panel's rail gives the
  counts beside the sections.
- **The coverage's news is how many more models, seeds and snapshots no
  scheduled job runs than when it was last shown**, run by no job or by
  manual ones only. Fewer is no news, and what was last seen is lowered to
  match, so a later rise counts from there.
- **The index counts the coverage only once the jobs were read**, by the Jobs
  tab, the sources form or the section itself, and never looks at the job
  files to do it (0063). A new manifest reads them again, as the Jobs tab
  would; until a first read, the coverage has no news, which needs a look to
  be measured from anyway.

*Amended by [0077](0077-a-start-no-file-says-is-neither-scheduled-nor-unscheduled.md):
a model run only in jobs whose schedule is not read is no news.*

## Rejected

- **News for every model in no job.** On a large project hundreds are, and
  for good reasons: a chip that never goes says nothing.
- **Reading the job files when the page starts**, to have a count from the
  first second. 0063 reads them when asked, and a count before any look has
  nothing to compare with.
- **A single total on the button.** "8" would not say which report to open,
  and the colours do.
