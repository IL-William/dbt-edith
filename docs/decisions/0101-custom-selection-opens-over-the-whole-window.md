# 0101. Custom selection opens over the whole window, its card on a click

Date: 2026-10-19 · Status: accepted · Amends 0091 and 0092

**Trigger:** read before changing what a panel covers, where the Custom
selection is opened from, or when its card shows.

## Context

0092 opened a selection in a panel over the editor and the dock, by the
button in the dock's lineage bar, the sidebar beside it and a card beside the
canvas from the start, saying to click a box until one was. Seen in use, it
was not where it was looked for, and too narrow: the reports open from the
top bar and a selection did not, and of the window's width the sidebar and an
empty card took about half, for a drawing that is read across its width.

## Decision

- **A button in the top bar** opens and closes it, beside Reports and drawn
  like it and like Settings, a glyph whose name unfolds on hover and while
  its panel is down, lit then. The dock's button stays, as the way in from a
  model's lineage.
- **The panel covers the sidebar too**, from the top bar to the status bar.
  `PANELS` marks it `wide`, and the sidebar and its gutter go inert with the
  rest while it is down. The reports, and any panel not marked, keep the
  sidebar beside them as 0091 has it. A file is reached from the search, or
  by a double-click on its box.
- **The card opens on a click on a box**, and the canvas has the panel's
  width until then. Its ×, Escape or a selection drawn without that box
  closes it, and the box goes out of the light with it. Escape closes the
  card before the panel: `PANELS` takes an `escape` hook for what a panel
  opened inside itself. Its left edge is dragged to resize it, as the
  sidebar's and the dock's gutters are, and for the session only as theirs
  are: from 280px to what leaves the canvas 320px. Let go, the canvas is not
  fitted again, unlike under those gutters, as the reader may have zoomed
  into the part beside the card.
- **Opening the card moves the canvas along only if the card covers the
  box**, by as little as brings it back into sight. A double-click must find
  the box where its first click left it, so a box the card would cover, or
  the canvas move, waits out a double-click, 500 ms, the default on Windows
  and macOS, before its card opens. A box clear of it gets its card at once.
  A pan or a wheel zoom in that wait leaves the canvas where the hand left
  it, as Lineage's `onMove` says; a press alone, on a box or the background,
  does not count.

## Rejected

- **Every panel over the sidebar**: the reports and the settings are lists,
  read beside the files they lead to; only a drawing wants the width.
- **The card over the canvas, which keeps its width**: it hides the same part
  of the drawing and leaves the band names and the legend under it.
- **The card held back after every click**: half a second on each, for the
  boxes the card never reaches.
- **The dock's button removed**: from a model's lineage it is the shortest
  way to a selection.
- **One pill with Reports**, as the manifest's age shares one with its
  reload: a selection is not a report, and nothing ties the two together.
- **A glyph alone, never named, or the word "Selection" as Search has**:
  the first says nothing while its panel is down, the second is wider for a
  button no busier than Reports, which takes the glyph and the name.

## Consequences

The dock's lineage and its Catalog still move into the panel as 0092 says;
only the card's place and its showing changed. A click whose card waits sees
it come half a second later, the box already lit. One more button in the top
bar brings its break about 50px sooner: on Jaffle Shop under about 830px
rather than 780px, and about 930px while the panel is down, its name
unfolded; item 12 of docs/state.md follows it.
