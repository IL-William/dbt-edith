# 0092. Custom selection opens in a panel

Date: 2026-10-16 · Status: accepted

**Trigger:** read before changing where a selection is drawn, what a click or
a double-click on a box does there, or what goes back to the dock when its
panel rolls up.

## Context

A selection was drawn in the dock, a third of the window under the editor,
the selector bar floating over the top of it. It speaks of the whole project,
not of the file in front, and it is read at length: the dock's height was the
first thing given up to it, and it was asked to be seen whole, as the reports
are. 0091 sorts views of that kind into a panel.

## Decision

- **The dock's Custom selection button opens a panel** over the editor and
  the dock, the sidebar staying beside it, by the mechanism of 0091.
- **One canvas.** The dock's lineage pane comes up into the panel, the
  selector bar over it, the canvas's own tools in the panel's head, the tests
  eye, by folder, Fit, Export and Copy image, and the Catalog beside it. All
  of it goes back as the panel rolls up. Moved, not copied: the canvas keeps
  its listeners, its find bar and its folder names.
- **A click on a box shows its card** beside the canvas, the canvas staying
  put, and opens no file. Until a box is clicked the card says so, even with
  a model open: its card would read as a box of the selection clicked. **A
  double-click opens its file**: the panel rolls up, and the dock's lineage
  moves onto that model.
- **Rolled up any other way**, by Escape, its ×, a file from the sidebar or
  the terminal's key, the panel leaves the dock as it found it: the model's
  lineage or the column's, drawn again, and the Catalog's card. A line typed
  and not yet drawn is dropped with it, or the panel would come down again,
  and the dock's canvas get the answer. The selection, its line and its
  history stay, and the button draws it again; 0087's raise still holds for
  that line only.
- **Whoever draws a selection opens the panel**: the button, the tour, a `+N`
  badge, a named selector picked. A model or a column drawn, from a
  double-click or a column in the Catalog beside the canvas, rolls it up.
- **The depth boxes, the modes and stop at stay in the dock**: they speak of
  one model's lineage, which a selection has no use for.

## Rejected

- **A canvas of its own in the panel**: `lineage.js` is one canvas, with its
  find, its folded tests, its legend pick and its hover. A second would mean
  making all of it per canvas, for one view that is never seen beside the
  other.
- **A taller dock instead**: the dock would cover the editor without the rules
  a panel follows, what goes inert, what Escape closes, what rolls it up.
- **A click opening the box's file, as in the dock**: the file's tab would
  roll the panel up at the first click, when the reader came to read the
  selection.
- **No card beside the canvas**: in the dock a click fills the Catalog, which
  the panel covers; a selection read without it sends the reader back and
  forth.

## Consequences

The dock's lineage is one model's or one column's. A model's lineage answer
is drawn only while the canvas still asks for that model, since the panel
rolling up and a file opened can each ask for one. The card is the Catalog at
420px, where its location table wraps.
