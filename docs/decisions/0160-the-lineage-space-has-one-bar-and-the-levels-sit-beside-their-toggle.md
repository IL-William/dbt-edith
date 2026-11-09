# 0160. The Lineage space has one bar, and the levels sit beside their toggle

Date: 2026-11-09 · Status: accepted, amended by 0162 (a column's box and its role tag) · Amends 0080, 0034, 0040 and 0092

**Trigger:** read before changing the Lineage space's bar, By folder and its
levels, how a model's box, a test's slip or the +N pill is drawn, or what the
space's card says and how it lays out its location.

Amends 0080 (the levels box), 0034 and 0040 (how a test looks, not where it
hangs or what it says), and 0092 (the space's head).

## Context

The space read as two rows and a box. A title row (a glyph and "Selection") said
what the top bar's Lineage tab already says; under it the selector's row and
above the canvas the tools; and By folder's levels opened over whichever of them
was under the pointer, so a touch screen could not reach them and the box covered
the count beside the selector (state.md item 13). The card's table of three
columns clipped a relation name. A model's tests were boxes, 26px tall, with
their name and column joined by a dot, and a box clicked was drawn in a white 2px
ring that no other mark of the page shares.

## Decision

- **One bar, 44px**: the selector on the left (Selectors, the input with the
  lineage glyph, the sum, Copy names, dbt ls), the canvas's tools on the right
  (Tests, By folder and its levels, Fit, Export). The glyph and the title go.
  The selector's row moved out of `#select-bar` into the space's head;
  `#select-bar` keeps the warnings, the cap's note and, under them, the find bar,
  floating over the canvas as before, so `headTop()` is unchanged. The box
  shrinks first (380px asked, 160 least), the sum goes below 1120px, and Copy
  names, dbt ls and the tools keep their glyph alone below 880: measured at 1440,
  1000 and 800px, one line, nothing cut. The 28px, 12px buttons apply to the
  space's head only; the dock's 30px row keeps its sizes.
- **By folder and its levels are one control.** A minus, the number and a plus
  follow the toggle behind a rule, only while it is pressed, in the dock as in the
  space since the tool moves between them. `#folder-levels` stays as the hidden
  input holding the number (`folderLevels()`, the tour and the redraw read it); a
  click writes it and fires one `change`. The hover box, its linger and
  `levelsHeld` are gone, with item 13 of state.md. The design hides the levels at
  640px and below; they stay, since nothing else sets them now.
- **A model's box is 220 by 48, radius 7**, one width for the dock and the space
  because the canvas moves between them without a redraw. The kind is a 3px bar
  inset 7px from the left and 10px from the top and bottom; the name is 13px at
  weight 500, the second line 11px mono, both starting at 19px; edges are 1.5px.
  The clip lengths follow (25 and 28 characters, set from the font's measured
  advance in `boxRoom`). A column's box keeps its look and its 180 by 40 until its
  own step: its box carries `.col`, and the rules for it are the old ones.
- **The box the lineage is on and the box clicked stay apart.** Both take an
  accent border at 90% on the focus fill; the clicked one also has a ring of 3px
  at 16% of the accent, a rect laid behind it (`select()`), because a CSS filter or
  a shadow per box is what a canvas of thousands cannot afford. In model mode the
  focus is the box clicked, so it has both. The white ring (`--sel-line`) is gone.
  The ring is not exported. A canvas has a dot grid behind it, as the design
  draws it, which an exported file does not.
- **The +N badge is a pill**, 24 by 18, wider for a longer number, in the place and
  with the click of the circle it replaces.
- **A model's tests are slips**: 22px, radius 5, the generic in the UI font and
  its column after it in mono and dimmer, as two runs with no dot (the label
  read out keeps it), clipped as before. The bar is 3 by 10, inset 6, each level
  keeping its shape (0040, 0148): one bar for a column, a bar and a thin one for
  the model, three pieces of 3 by 2 for a singular test, whose outline is dashed.
  Only the host's own stem and a group's are dashed, thin and in `--line-strong`;
  a line from another parent into a test keeps its dots. An exported file carries
  both, and not the ring.
- **The card lays out by a flag, not a width**: the Catalog rendered inside
  `#selpanel-card` passes `inCard` down, so dragging the card's edge never
  switches layouts under the pointer. Context is a grid of three
  (Materialization, Columns, Tests, Upstream, Downstream); Location is a list:
  one value where written, resolved and built agree, a line per run of equal
  stages where they do not, labelled by the run's last stage and titled with the
  stages it stands for, a key set nowhere shown as the value built with a dimmed
  `(default)`. Two stages merge only when their text and their class agree, so an
  unevaluated mark is never lost. The relation follows the same rule. The
  environment is picked by the pill beside the heading. The dock's Catalog keeps
  its row and its table. Back in the dock, the Catalog is drawn again.
- **Tests are counted apart**: `7 nodes · 6 edges · 11 tests` under the canvas and
  in the Export menu's head, a test no longer a node nor its stem an edge, folded
  tests counted in and said (`11 tests · 3 folded`). A capped selection counts as
  0087 does, and a canvas of tests alone keeps them as its nodes.
- **Halloween's three box styles keep their silhouettes**: their widths follow the
  box (210 and 192; a column's stay), every kind-bar rule sets the geometry it
  needs, the names keep the x they had, and the ring is not drawn, the states
  having colours of their own.

## Rejected

- **The levels box opening on a click**, which still covers the bar it opens over.
- **A narrow layout set by the card's width**: the drag edge would switch layouts
  under the pointer.
- **A CSS filter for the ring, and Light's node shadow**: a filter per box costs
  too much on a big canvas, for a shadow the design draws nearly invisible.
- **A different width for the dock's boxes**: the canvas moves without a redraw.

## Consequences

- The dock's tools row, wider by the stepper while By folder is pressed, overflows
  about 50px sooner (state.md item 48); it re-folds on the stepper like on any
  other change of its width, never on the pointer.
- A model's box is 20px wider: a canvas of the same nodes is a little wider, and
  its fit zooms a little less.
- Light has no node shadow.
