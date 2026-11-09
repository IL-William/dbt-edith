# 0166. The shell is laid out as the boards draw it

Date: 2026-11-09 · Status: accepted · Amends 0140, 0142 and 0075

**Trigger:** read before changing what the top bar or the status bar holds and
in which order, how either folds when it is short, what the dock's tabs keep
when their row is full, or what the status bar says on its left.

## Context

0140 gave the top bar its spaces and the folds that keep it on one line, and
left the rest as it had grown: the counts beside the tabs, the search a
button, the settings a gear whose name unfolded, the status bar's `env` pill
with a caret and its venv a bordered pill. The Theme boards draw all of it
again: a search field in the middle of the bar, plain squares on its right, the
counts and the place you are at in the status bar, and the dock's tab in front
keeping its name. Nothing the app does today is removed by that.

## Decision

- **The search is a field in the middle of the top bar**, between two growing
  spaces: the magnifier, "Search models, macros and files" and the shortcut. It
  is the button it was (`#palette-btn`, the same click and shortcut). The
  manifest pill moves before the right group's separator.
- **The right group is four plain squares**: `?`, the theme, GitHub and the
  settings, 28px, no border and no fill, a wash on hover and while their menu
  is open. The settings button draws the board's sliders instead of the gear and
  is lit while the settings are open. **Its name no longer unfolds**: it is the
  tooltip's and the label's, as its neighbours' are, and its triangle for a
  missing profile stays. GitHub keeps its mark and its issue and star menu, as
  a square like the others, the user having answered that the board's arrow is
  not wanted.
- **The counts leave the top bar for the status bar**, after the place and
  before the venv, in the UI font, cut with an ellipsis after the place has been.
- **The status bar's left says where you are in the space in front**: in Code the
  file's path in mono with its cursor and its state after it; in another space
  the space's name and, after it, the selection last drawn in Lineage (a line
  that is only `--selector X` shown as X, the space's name alone while there is
  none), the section in front in Reports, the section in front in the settings.
- **The top bar folds by measure, cheapest first**: the search field shrinks with
  the window down to 200px, its words cut with an ellipsis; then the words go
  and the field is its glyph and shortcut; then the project's name is cut down to
  120px; then the tabs not lit keep their glyph alone. This replaces 0140's order
  (the gear's name, the counts, the tabs): both have gone from the bar. Still
  measured with every name shown, so a click on a tab changes nothing.
- **The dock keeps its front tab's name longer.** When the row is full the tabs
  not in front keep their glyph alone (`folded`); then the tab in front folds as
  well (`bare`); only then does Export keep its glyph and chevron alone
  (`iconic`), the board having the tabs' names go at 1180px and Export's label at
  900px. The widest name is measured in front, so a click on another tab never
  changes what folds.
- **The status bar's `env` pill has no caret** (0142's pill carried one); it
  keeps its tooltip, `aria-haspopup` and its menu. The Catalog's pill keeps its
  caret. The `env` word on a filled pill is quieter by weight and opacity, on the
  outlined manifest pill by colour.
- **The venv is plain** in the status bar, a pictogram before its words saying
  the state: a check, a warning triangle with the words in amber, a dashed ring.
  The settings' own copy keeps its pill.
- **The sidebar and the editor are drawn as the boards draw them.** Open editors
  starts collapsed, behind one button the keyboard opens (`aria-expanded`), with
  save all and close all beside it; the editor's tabs list the same files. The
  rows the shell selects take the selection wash `--select`; a test holds git's
  letters and four kind marks at 3:1 on it in Light, and docs/state.md lists the
  kinds that stay under. The sidebar's
  tab row is as tall as the editor's tab bar (34px) so the line under the two is
  one. In the editor the cursor's line is lit, with its number (`--cur-line`, one
  line per document, painted again after a tab is switched to); the gutter is
  46px of numbers, their padding on top, with no rule; a tab has no accent line, takes the editor's colour when in
  front and shows its `x` only when in front or under the pointer or the keyboard
  (a modified tab keeps its pencil); the breadcrumb separates the path's parts by
  slashes, the file in the text colour, and keeps the chevron for what is inside
  the file. The gutter rules are the main editor's: the Compiled and Run tabs and
  the diff keep theirs.
- **The node card on hover is the board's**: on the card colour, 330px, the box's
  line at the head's right, the file in mono with the tags at its right, the
  description in italics, the counts as numbers in the text colour, the columns in
  two columns with their types quiet in capitals. The types are no longer amber.
- **The status bar's git branch keeps its glyph**, and loses it while git is
  paused (0121), where there is no branch to name.
- **A dimmed Reports (0075) is drawn at .8**, since .7 left its glyph at 2.7:1 on
  the bar in Light, under the 3:1 a control needs; a test holds it in the four
  themes.

## Rejected

- **The board's arrow for GitHub**: the mark says what it opens at a glance,
  and the user chose it.
- **Keeping the settings' name unfolding**: the square beside three others
  that carry none, and a name that moved the bar under the pointer.
- **A fold at a width** for the search, as the board draws one at 1180px: the
  room moves with the project's name and the news on the Reports tab.

- **Export's label folding before the tab in front**, as a first draft had it:
  the board drops the tabs' names at 1180px and Export's label only at 900px,
  and the label is the dock's one action word.

## Consequences

- A script or a test that read `#counts`, `nocounts`, `nolabel` or `.rlabel`
  reads the status bar's counts and the settings button's label instead.
- The page keeps a minimum width from the dock's tools, about 1017px on Jaffle
  Shop with no model open and 1052px with one, and runs past the window below
  it, as it did before; no fold here reaches it (docs/state.md).
