# 0107. The box art is drawn under a count and never exported

Date: 2026-10-20 · Status: accepted

**Trigger:** read before changing what the Halloween look draws on a lineage
box, how many boxes it draws it on, `Lineage.setSkin`, or what
`Lineage.snapshot()` measures and copies.

## Context

The Halloween look (0106) first drew its three box styles, tombstone, coffin
and grimoire, in CSS alone: the box's rect, filled by one gradient and clipped
to a silhouette, and its kindbar bent into a strip, a plate or a ribbon. It
looked little like the mockup the user chose, which has a cross, a crack and a
moss fringe, a gold plate with a gem and studs, a banded spine and a page edge.
That detail wants shapes, and the first answer to "more shapes" had been
refused: a pattern fill on every box made a pan of 3,000 boxes about three
times slower. The user chose full art under a box count, and the plain look
above it.

## Decision

- **The art is drawn by `web/lineage.js`**, which builds the boxes, and
  `Lineage.setSkin(name | null)` lays it on, or takes it off, without drawing
  the canvas again; `render()` lays it again after each redraw. A plain box,
  not a test hanging under a model nor a chip, gets one `g.hw-art` over its
  rect: a silhouette path and a trim path written from the box's size, and a
  few `<use>` of symbols (the cross, the fringe, the plate and the gem, the
  spine, the pages). The symbols are fixed drawings in a hidden `<svg><defs>` of
  `index.html`, outside `svg#graph`. The kind's colour reaches the gem, the
  moss and the ribbon through `currentColor`, from an inline `color` on the
  group, as the kindbar's fill is set inline: no rule of the stylesheet can
  outrank it. `applyLook()` asks for `halloween ? boxes : null`.
- **At most `ART_MAX` plain boxes, 400**, the cap of a model's lineage. Past it
  nothing is added, and the look's plain CSS, one gradient and a clip per rect,
  shows as before. A pan with the art on 400 boxes, in headless Chrome on a Mac
  (2026-10-20), cost the same as with the look off to within 0.2 ms of 16.6 ms,
  for each skin, where a pattern fill on 3,000 boxes had cost three times that.
  Laying the art on 400 boxes took under 10 ms of script. 800 boxes was tried
  and passed the pan, but drawing the canvas, which happens on every move of the
  focus, took 52 to 81 ms against 27 to 33 without the art, too much for a
  machine with no GPU. The art is laid only when the skin changes: `setSkin`
  returns at once for the skin it already has, which is what every click in the
  Theme section asks. The count has to be measured again when the art grows:
  add paths to a symbol only after a pan of 400 boxes, and the drawing of the
  canvas, are.
- **The rect is only the hit area while art is there** (the canvas root has
  `hw-on`): transparent, unclipped, so the whole box takes the click. Every ring,
  the dashes of an ephemeral box included, is a stroke of the silhouette path,
  so it follows the headstone's arch or the coffin's chamfers instead of
  falling back to a rectangle. Above the count the rect keeps the clip, and its
  clipped corners take no click, which falls through to the canvas.
- **A name is cut to the room its skin leaves it**, a character budget per skin
  and mode, the same in every state, and starts at one x per skin and mode
  (a transform on the text, never on `.nd`, whose transform is the box's
  place). Past the budget the name ends in an ellipsis, as it does without
  the look; the hover card has the whole of it.
- **The art is never exported.** `snapshot()` measures the lines it writes on
  the live canvas, and a themed canvas would measure the look's serif and
  smaller size and pin the file's lines to that: so the body's `halloween` and
  `hw-` classes come off for the length of the call, with no paint between, and
  go back. The clone drops every `.hw-art` and `hw-on`, whose `<use>` would
  point at defs the file does not have, and its names, which the art cut to its
  own room, are cut again to the plain look's before anything is measured. An
  export of a themed canvas is then the export of the same canvas without the
  look, character for character, which `scripts/walk_export.py` checks in
  Chrome for the three styles; `web/tests/export.js` only reads the source.

## Rejected

- **Art on every box:** a pan of a big selection would pay for thousands of
  shapes it cannot show at that zoom, and the plain look already tells the kinds
  apart.
- **A pattern, a filter or a second rect per box for the texture:** each was
  measured or ruled out in 0106 as the cost of a pan; the mockup's noise
  filters are not drawn.
- **Art in `app.js` or in CSS alone:** the boxes are built in `lineage.js`, and
  CSS has no way to add a shape to an SVG group.
- **Art in `exportCanvasCss`:** an export is the usual blue (0106), and a file
  opened elsewhere has no defs for a `<use>`.

## Consequences

A new part of a skin is a symbol in `index.html`, an entry in
`artGeometry`, and a budget in `ART_ROOM`. Past 400 boxes the look changes
shape under the user's hand, which is the point and is said in
[docs/features/look.md](../features/look.md).
