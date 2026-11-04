# 0140. The top bar switches between spaces; a panel is a space

Date: 2026-11-04 · Status: provisional · Amends 0091, 0092, 0100, 0101 and 0137

**Trigger:** read before adding a space or a panel, changing what the top bar's
tabs or the gear do, what Escape or a click on a lit tab does, what the
Lineage space opens on, or how the top bar folds when it is short.

## Context

The editor was the base and three panels rolled down over it: Custom
selection and Reports, wide, and the settings, narrow. Each had its own
button in the top bar, which toggled it, and each had an × in its head. It read
as a panel over the editor, closed with an ×, when what the reader reaches
for is a place to be in: the code, the lineage, the reports. The top bar had
also outgrown its width: under about 1000px its chips broke onto several
lines and under about 850px it ran past the window (docs/state.md).

## Decision

- **A tab for each space**, right after the project's name: Code, Lineage and
  Reports, each a glyph with its name, the one in front lit and marked
  `aria-current`. A space is a panel without an ×, and **Code is no panel
  down**. The mechanism stays 0091's: `PANELS`, `openPanel`, `closePanel` and
  `escapePanel` keep every rule they had, and every `PANELS` paint hook calls
  `paintSpaces()`, which lights the tab of `S.panel`.
- **A tab is a place, never a switch.** One `goSpace(name)` replaces the three
  toggles, and a click on the space in front does nothing. Code closes the
  panel, giving the focus back; it first forgets that the settings were
  opened over Reports, so the settings' `closed` hook does not bring Reports
  back. Escape keeps its meaning: it closes what a space opened inside itself,
  the selection's card, then goes back to Code, or to Reports for the settings
  opened over it, by a Sources… or by the gear. It stays the keyboard's way back, since Cmd or
  Ctrl with a digit belongs to the browser and Option alone cannot carry a
  shortcut (0037). `Mod+Alt+R` still opens and closes the Reports.
- **The gear is a space with no tab.** It stays at the right, lit while the
  settings are open, its name unfolded then, and a click on it open does
  nothing. The settings are `wide` now, over the sidebar like the other
  spaces, their blocks keeping the reading width they had. Only Code has the
  sidebar. This reverses 0137's "the settings stay narrow", and the settings
  no longer give the sidebar back when opened from Sources….
- **Lineage is the Custom selection's space, renamed.** The panel, its
  labels, the tour and the toasts say Lineage; inside it the mode is still
  Selection, with the frame glyph the button had. The tab takes the dock's
  Lineage glyph. It opens with an empty selector box seeded with
  `1+<name>+1` when a model, a seed or a snapshot is in front, in the name the
  Catalog copies, a version's `name.v2` included (0104), and never over a
  selection already typed. The dock's Custom selection button keeps its
  label and opens the same space (0101); its tooltip says so.
- **Reports keep what they had on their tab**: dimmed while no report has
  anything (0075), and the chips of what is new (0066).
- **The bar folds by measure, never by width.** Nothing in it breaks onto a
  second line. The project's name is cut with an ellipsis, its full name in
  the tooltip, down to 120px of it, and what it could still give up counts as
  room. Then, cheapest first: the gear's name stays folded, so the lit gear
  shows its glyph alone; the counts go; and the tabs not lit keep their glyph
  alone, named by their tooltip and still read out, the lit one keeping its
  name. It is measured with every name shown, as the dock's tabs are, so a
  click on a tab never changes what folds. While the gear's name can still
  unfold, on hover and while the settings are open, the room it will take is
  held back from the bar's slack: neither can push the bar past the window,
  and opening the settings and coming back folds nothing differently.

## Rejected

- **A tab that toggles**: the lit tab closing its own space sends the reader
  to a place they did not name, and the bar would say two things by one
  colour. Code is the way back.
- **A × on a space**: with a tab to go back by it was a second way to say the
  same, and it kept the look of a panel over the editor. None of the three
  has one: the Reports' and the settings' heads lost theirs too. The way out
  is the Code tab, the gear's own tab being none, or Escape.
- **The settings staying narrow, beside the sidebar**: a settings page has
  nothing to read beside, and Code's sidebar under a space that is not Code
  was the one place the bar's tabs and the window disagreed.
- **A container query for the fold**: the room moves with the project's name,
  the counts and the news chips, which no breakpoint knows.
- **The Jobs space and the project list now**: wanted, but their scope is
  still to settle; each is its own change, as the Lineage space taking every
  mode is.

## Consequences

The Reports, the settings and the selection lose their toggle and their ×: a
script or a test that pressed one to close them presses Code or Escape. `#select-btn`,
`#reports-btn` and `#selpanel-close` are gone, replaced by `#space-code`,
`#space-lineage` and `#space-reports`. On Jaffle Shop the bar no longer wraps
at any width, and runs past the window only under about 650px, where the
project's name is already cut to 40px.
