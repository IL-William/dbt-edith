# 0163. The settings are one column of rows

Date: 2026-11-09 · Status: accepted · Amends 0100

**Trigger:** read before adding a setting, or changing how a section of the
settings lays out its controls.

## Context

0100 drew each menu the settings replaced as a block on the page, so a section
was a stack of those menus' rows, a heading over each, running the width of the
window. The design lays a section out as a column of cards, each row saying a
setting's name and why on the left and holding its control on the right, so a
setting is found by its name and not by the shape of the menu it came from.

## Decision

- **A section is a head and cards of rows.** The rail has a caption, a row 30px
  high with the current one marked by an inset accent bar, a glyph (a cube for
  Project, a tool's in its colour) and a hint at the right: the tool on screen,
  on or off, how many job sources there are, the newest release in the accent
  (`restart` once installed), the theme. A section's head is its name and one
  line of what it holds, a tool's (Column lineage, Snowflake, Orchestration)
  carrying its colour as a bar before the name, never a dot (0148).
  A row is a name and about on the left, the control on the right; a `wide`
  row, for a table or a form, puts it under the text.
- **Every setting and help text of 0100 stays**, in its section. Snowflake is
  one switch, as it was; the tools of Column lineage are radio cards with the
  sentence that says what picking one does; the environment shown first is a
  row of buttons, each in its colour; the job sources are a row each, **Edit**
  unfolding that source's form under it and **Add a source** a new one,
  unfolded; Python's venv is a path, chips and a state line, with the buttons
  that type the activate lines kept.
- **The popups keep their own drawing.** Edith's menu, the Columns tab's menu
  and the Catalog's environment menu are not changed: the section draws the
  same answers (`updateOffer`, `releaseRows`, `updateLook`, `fillSourceMenu`,
  `envChoices`) in its own layout.

## Rejected

- **What the boards draw and this tool has no feature for.** A "None" column
  lineage choice (there is no way to pick no tool), a profile and target picker
  (no target is shown), Snowflake's account line and three switches (one switch
  turns the features on), a single GitHub "Open" button (the two links stay,
  until the GitHub button has its own preview), a `.env` variable's value on its
  line (a value never leaves the server, 0012; the line says `DBT_TARGET`, how
  many variables the file defines and how often it matches), and a Change
  button for the Python (nothing here changes which one a terminal starts in).
- **A foot that hides whenever every form is folded.** A fold keeps what was
  typed, so an edit would be left with nothing to save it: the foot stays while
  a source was added, removed or typed in, or a save has answered.

## Consequences

The settings no longer say "On screen" or "Where it comes from" in a menu's
voice. A tour step ringing `#settings-view [data-live=columns]` rings the
cards. The Updates section is drawn by `paintSetUpdates`, so a change to what
Edith's menu says is made in two places.
