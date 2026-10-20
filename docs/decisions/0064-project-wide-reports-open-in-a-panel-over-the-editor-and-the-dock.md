# 0064. Project-wide reports open in a panel over the editor and the dock

Date: 2026-10-11 · Status: accepted, amended by 0070, 0075, 0091 and 0103 · Amends 0059 and 0063

**Trigger:** read before adding a report, or changing what opens or closes the
Reports panel, what its button counts, or when a report's file is read.

## Context

Two views spoke of the whole project and sat in two places: collin's report,
an editor tab behind a badge (0059), and the Jobs tab's Coverage, a view of a
dock tab (0063). More are asked for: the last run, the tests and docs, the
sources' freshness, the macros. A report is a list to work through and
wants the height of the window; a file opened from it is where the reader goes
next.

## Decision

- **One Reports button in the top bar** rolls a panel down over `#main`, the
  editor and the dock both, with a rail of sections on its left. The sidebar
  stays, so a file can still be picked. `Mod+Alt+R` opens and closes it.
- **It covers, it never replaces.** Nothing under it is hidden or measured
  again; while it is down, what is under it is `inert`.
- **Anything that shows a file or a dock tab rolls it up**: a tab activated, a
  link in a report, the palette, the Models list, a dock tab, Escape, the
  close keys. A tab shown because another closed does not, nor a dock tab an
  answer brings forward when it lands, after the panel was opened.
- **A section is listed only when it has something to show**, from an index
  that reads file times and settings alone. A section reads its file when it
  is shown, and the job files are looked at then too, a third moment beside
  the Jobs tab and its form (0063).
  *Amended by [0075](0075-the-reports-button-stays-in-view-dimmed-while-it-has-nothing.md):
  but when the guided tour puts back a panel it rolled up, which comes back
  as it was left, unread.*
  *Amended by [0103](0103-a-config-path-is-unused-when-no-fqn-of-its-kind-begins-it.md):
  the index also reads `dbt_project.yml`, to count the config paths that
  apply to nothing.*
- **collin's report is a section**, and its badge is news on the button, one
  chip per section, under 0059's rule: only for a run finished since the
  section was last shown. The comparison of versions stays a tab: it is
  opened from the menu for two runs, not worked through.
- **Coverage leaves the Jobs tab**, which keeps the node in focus.
- **A section on screen is never redrawn by itself.** Its Read again button
  says when the file behind it changed, which the freshness poll reports.
  Shown again, it keeps the folds and the scroll it was left with.

## Rejected

- **A dock tab.** Too short for a list of hundreds of models.
- **An editor tab per report**, 0059's choice. A report is not a file, and a
  tab bar of reports and files mixes two things to look for.
- **A modal.** It would hide the sidebar a report is worked through with.
- **A drawer from the right.** Too narrow for the lists.
- **Opening the panel when a run ends**, for 0059's reason.

## Consequences

The panel is the place for every later report: each brings a section, a field
in the index, and the file it reads, read when its section is shown.
*Amended by [0091](0091-what-speaks-of-the-whole-project-opens-in-a-panel.md):
the panel is one of several, each rolled down over the editor and the dock
by the same mechanism, one at a time.*
