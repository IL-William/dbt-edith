# 0100. The settings open in a panel of their own

Date: 2026-10-19 · Status: accepted · Amends 0031, 0049, 0072 and 0079

**Trigger:** read before adding a setting, moving one between the settings
and another place, or changing what the gear, a status bar pill or a section
of the settings opens.

## Context

The settings were spread over the page. The column lineage menu sat in the
top bar, the snowflake beside it opened Snowflake's switch and the
orchestration's sources, and a user and a plug opened the profile. The
environments were named in a dialog behind the `env` pill's menu, the job
sources in another dialog, the venv's activate line behind its own pill, and
the update behind its pill and Edith's name. The three buttons in the top bar
were what broke it onto two lines under about 1000px (docs/state.md). 0091
sorted what speaks of the whole project into panels, and named the settings
as one of them.

## Decision

- **One gear in the top bar** rolls the settings down, by 0091's mechanism, in
  place of the column lineage menu, the snowflake and the profile's button.
  Its name unfolds on hover, as the Reports button's does, it is lit while
  the panel is down, and it carries a dot while the profile is missing or
  refused, as the profile's button did (amends 0049's button beside the
  snowflake, and 0031's menu in the top bar).
- **A section at a time**, picked from a rail, as the reports are: Project
  (the profile, the environments and Python), Column lineage, Snowflake,
  Orchestration, Updates, and Help and tour. Beside its name each row says
  what its section holds: the column lineage on screen, Snowflake on or off,
  a newer release, the profile's dot. It opens on the section asked for, or
  the one last shown.
- **The status bar's pills open their section**: `env` and the venv open
  Project, the update Updates (amends 0079's pill opening Edith's menu). The
  venv pill is a button whatever it says, and a click no longer types the
  activate line: the Python block lists each venv a click could offer, with
  the button that types its line, Enter still left to the user (amends
  0072). The Catalog's `env` pill keeps its menu, beside the locations it
  changes, and that menu's Manage environments opens Project.
- **The forms sit in their section**, not in a dialog: the environments'
  names and visibility, and the job sources. A form stays as it was left
  through another section and back, and goes when the panel rolls up, as it
  went with its dialog; the tour puts back one it rolled up. A clean read of
  the sources rolls the panel up onto the Jobs tab, when that tab is under it
  with a model, and otherwise gives its place to the coverage, which is
  where a Sources… in Reports came from. Rolled down over Reports, by a
  Sources… there or the gear, the panel gives Reports back when Escape, its ×
  or the gear rolls it up, as the dialog left Reports down under it.
- **A section is drawn from what the page knows**, and drawn again when that
  changes: Snowflake switched, a script starting, collin running, an update
  looked for, a venv the terminal reports. Only a block whose drawing would
  differ is replaced, so a poll that changes nothing leaves a row, its
  tooltip and a click half made on it alone. It draws the menus' own rows on
  the page, so the menus that stay, the Columns tab's column lineage menu,
  the Catalog's environment menu and Edith's, offer what it offers.
- **The tour's Column lineage step** rolls the settings down on that section
  and rings its list, where it rang the top bar's menu.

## Rejected

- **The panel with the top bar's buttons kept beside it**: the bar keeps its
  three buttons and its second line, and each setting is in two places.
- **Every section on one long page under the rail**: the job sources and the
  column lineage list run long, and a pill would land in the middle of them.
- **The pills keeping their menu, with a link to the settings**: a setting
  found in two places, behind two different gestures.
- **A dialog for the settings**: a box over the page with Escape and a focus
  of its own, which 0091 settles once for every panel.
- **Edith's menu moving into the settings**: it opens under the pointer to
  say which build serves the page (0086). The Updates section draws the same
  menu on the page.

## Consequences

On Jaffle Shop the top bar now breaks under about 860px, where it broke under
about 1050px, and under about 770px with Reload manifest folded into the
manifest's pill as arrows, in the same change; what is left of that is item 12
of docs/state.md. Picking an
environment from the status bar takes two clicks, as does activating a venv:
the pill, then the choice. The column lineage on screen is no longer named in
the top bar, only in the settings' rail and the Columns tab's menu.
