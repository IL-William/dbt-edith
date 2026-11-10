# 0148. A meaning has one shape, and no marker is a round dot

Date: 2026-11-05 · Status: accepted, amended by 0151 and 0169 (the clock) · Amends 0049, 0079, 0100, 0117 and 0137

**Trigger:** read before adding a marker for a kind, a state, an environment or
a tool, or before drawing anything small and round in the page.

## Context

The page drew five different meanings as a coloured round dot: a model's kind, a
state (the manifest's freshness, a query, a job's builds, a model's last build,
the venv, a missing profile), an environment, a tool, and a column's role. The
greens did not agree with one another: the same green said "a table", "up to
date" and "the DEV environment". A person had to learn the colours of five
vocabularies that shared one shape, and a colour alone cannot be read by
everyone.

## Decision

- **A kind is a bar.** The thin bar on the left of a node's box is its kind, in
  the box's colour, and every `.dot` before a name is the same bar (3px wide), so
  a model looks the same in the sidebar, the palette, the Catalog, the cards and
  the legend.
- **A state is a pictogram.** A check is done well, a triangle is worth a look,
  a cross is failed, a dashed ring is nothing known or recorded, a clock is under
  way, a circle with a slash is cancelled. They are masks over the colour that
  says which (`--m-ok`, `--m-warn`, `--m-bad`, `--m-none`, `--m-clock` and
  `--m-cancel` in app.css's `:root`), so a theme re-points the colour and the
  shape stays. The canvas draws the first four as paths (`BUILD_MARK` in
  lineage.js), since the canvas is SVG, where a path is the natural mark. A
  query that was cancelled is the slashed circle in the dim colour, apart from
  the red cross of a failure, and a status the page does not know is the dashed
  ring.
- **An environment is a tag.** Its name, in its colour (`data-tone`), tinted;
  the manifest's own is only outlined, as the status bar's pill is. The name is
  the label, so no colour is said twice. The Compiled and Run tabs' bar is the
  exception that follows from it: the picked button of its row is filled in the
  environment's colour and the line under it names the environment, so that
  line carries no tag of its own.
- **A tool is its glyph.** The columns glyph for collin, a flake for Snowflake,
  a clock for the jobs; the flake and the clock are the drawings of the buttons,
  the columns glyph is new. A column lineage tool without a glyph of its own
  (Fusion, one not yet named) shows the columns glyph in its colour, and its
  name is always written beside it. The empty tool, none picked, keeps the ring,
  which says "nothing known".
- **The clock has two meanings that never meet.** It is the Jobs tool's glyph
  and the "under way" state. A tool's glyph always sits beside its name, a
  state's pictogram stands alone, and the colour differs: the tool's violet, the
  state's amber.
- **A role stays a square.** The 8px `.rolesq` in the stop at menu, the legend
  and the column card, until roles are drawn as lines and tags.
- **Unsaved edits are a pencil,** in the colour the dot was (the tab's close
  button's own, the Open editors row's amber), in the tab's close slot and that
  row, where the pointer turns it back into the cross.
- **No round marker.** `web/tests/shapes.js` holds the list of what may be round
  (a switch's knob, two round buttons around a glyph) and fails on any other
  rule with a round corner, on a marker without a mask, on a state the code
  produces that has none, and on a marker the Halloween look restyles.

## Rejected

- **Keeping the dot and varying its colour more**: the problem is one shape
  carrying five meanings, not too few hues.
- **A letter or a word on every state**: the words already sit beside the marks,
  and the pictograms are what a glance reads at 10 to 12px.
- **A mask for the canvas's build mark**: the canvas is drawn in SVG, where a
  path takes a stroke and a fill from the colour variables as the rest of the
  drawing does, so the mark is a path with the geometry of the masks.
- **An image file per pictogram**: they are data URLs in `:root`, so the stylesheet
  stays one file: a mask needs a value held in a variable, and each extra file
  would need embedding in the release and a route to serve it.

## Consequences

- A new state needs its shape in `:root` and its rule next to the others; the
  test names the ones that are missing.
- The pictograms are few and 16 units wide: one that has to say more than its
  neighbours is a sign the state is two states.
- The kind bar is not on the editor tabs yet.
