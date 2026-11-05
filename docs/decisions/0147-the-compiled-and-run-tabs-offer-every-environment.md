# 0147. The Compiled and Run tabs offer every environment

Date: 2026-11-05 · Status: accepted · Amends 0139, 0142

**Trigger:** read before changing what the top of the Compiled or Run tab
shows when environments exist, how an environment is picked from it, or what it
says with no `.env` file picked.

## Context

The tabs show a model's SQL for the environment picked in the status bar (0139,
0142). With none picked, which is the manifest, or `my compile` on a sandbox
manifest (0143), the tab said nothing about environments, so nothing on it told
a person that it could switch. Comparing QA, UAT and STG on one SQL meant going
to the status bar's pill for each, and back. 0139 had rejected a picker of its
own on the tab because it put two environments on screen, the tab's and the
Catalog's. A picker that sets the one shared choice does not.

## Decision

- **The tabs offer every environment as a button**, on a row at the top, whenever
  there is a file to pick, which is when the project's config reads an env var
  and at least one `.env` file is shown: the files shown, in the order of the
  menu and the settings, and the manifest's own choice (`manifest`, or `my
  compile`) last. It is drawn with a file picked and with none, and not at all
  when there is nothing to pick, so a project with no `.env` file sees what it
  saw before.
- **A button sets the one shared choice**, as the menu does: the status bar's
  pill, the Catalog's resolved column, the Builds dots and the other tab follow.
  The button of the choice in use is filled in its colour and pressed; a click
  on it does nothing. The keyboard keeps its place across the repaint.
- **Under the row, one line says what is shown.** For a file it is the line
  0139 has, `Shown for QA (.env.qa): 2 relations swapped`, with the findings
  folded under it. With none picked it says whose placement the SQL carries
  (`Shown as dbt wrote it, into <sandbox>`, or `Shown as dbt compiled it for the
  manifest's target`), and has no findings, since nothing is swapped.

## Rejected

- **A pill and a menu on the tab**: two clicks for a gesture repeated while
  comparing, where one button per environment is one click and shows all that
  can be picked.
- **A choice local to the tab**: 0139's reason, two environments on screen and
  the Catalog saying another.
- **A row only when a file is picked**: the case where a person most needs to
  learn that the tab can switch.

## Consequences

The status bar's pill and its menu stay as they are (0142). A script that waits
on the `Shown for` line still finds it with a file picked; with none, the row
of buttons is there and the line starts `Shown as`.
