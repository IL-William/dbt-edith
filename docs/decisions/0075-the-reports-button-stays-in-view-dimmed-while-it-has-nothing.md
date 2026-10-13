# 0075. The Reports button stays in view, dimmed while it has nothing

Date: 2026-10-13 · Status: accepted · Amends 0064, 0070 and 0074

**Trigger:** read before hiding the Reports button, or changing what it shows
or says while no report has anything.

## Context

0064 listed a section only when it had something to show, and 0070 kept the
button hidden while the only section was the empty job coverage. So the button
appeared after the first `dbt run`, `build`, `test` or `compile`, a collin
report, or a job source. A first start is none of those: the manifest comes
from a `dbt parse`, which writes no `run_results.json`, and nobody has named a
job source yet. Whoever opened dbt-edith for the first time never saw that
Reports exist, and the panel was where a job source is added (0070). The
guided tour (0074) had to show the button for its own step.

## Decision

- **The button is always in view**, from the first paint of the index.
- **While no section has anything, it is dimmed**, at half opacity and full
  under the pointer or the keyboard's focus, and carries no chip.
- **Its tooltip then says what will fill it**: the last dbt run from
  `target/run_results.json`, collin's report, and the job coverage once the
  Jobs tab has a source, which is added from it.
- **A click opens the panel on the empty coverage**, as `Mod+Alt+R` already
  did, which is where a source is added (0070). The sections listed, and
  which one the panel opens on, are 0064's and 0070's as before.
- **The guided tour never asks to look at the job files** (0063). Its Jobs
  step and the dock it puts back show the jobs as last read; the server reads
  them once if nothing has yet, as opening the Jobs tab does, to pick a
  model a job runs (0074). A Reports panel the tour rolled up comes back as
  it was left, its section still drawn and not read again, the one showing
  that does not read its file (amending 0064); its Read again says when the
  file changed meanwhile. Reading the coverage again looked at every job
  file and dropped the jobs the Catalog had read, on an orchestration
  repository that can be large.

## Rejected

- **Hidden until a report has something**, 0064 and 0070's rule: a part of the
  page nobody finds is the problem the guided tour was written for.
- **Shown at full strength with nothing in it**: a button that looks like the
  others promises a report, and opens on an empty section.

## Consequences

The tour's Reports step no longer shows the button for its duration, and
says it is dimmed until a report has something.
