# 0142. The status bar's `env` pill opens the environment menu

Date: 2026-11-04 · Status: accepted · Amends 0100

**Trigger:** read before changing what a click on the status bar's `env` pill
does, what its menu offers, or where an environment is picked.

## Context

0100 made the status bar's pills open their section of the settings, the `env`
pill's being Project, so picking an environment took two clicks and rolled a
panel down. Then the pill only moved the Catalog's resolved column. Since 0139
it also drives the Compiled and Run tabs, so switching between QA, UAT and STG
is a reading gesture, repeated many times over one SQL, and the settings panel
rolling down covers the very text being compared. 0100 rejected the pills
keeping their menu with a link to the settings, a setting found in two places
behind two gestures. That no longer holds for `env`: picking an environment is
now a reading gesture repeated while comparing the SQL (0139), while naming and
hiding environments stays in one place, the settings.

## Decision

- **The `env` pill opens the environment menu**, the one the Catalog's pill
  and the header of its resolved column open: the files shown, the manifest as
  dbt parsed it, and Manage environments... at the end, which opens the
  settings' Project section. A second click on the pill closes it, and the menu
  opens above the pill when there is no room below. The pill carries the caret
  and the tooltip the Catalog's has, and `aria-haspopup="menu"`.
- **Naming and hiding environments stays in the settings**, as 0100 has it.
  The venv pill keeps opening the settings.

## Rejected

- **Keeping the two clicks**: the pill, then a choice in a panel that hides
  what the choice changes.
- **A picker on the Compiled tab**: 0139 rejected it, two environments on
  screen and the Catalog saying another.

## Consequences

Picking an environment from the status bar takes one click again. A script
or a test that expected the pill to open the settings opens the menu and picks
Manage environments... instead.
