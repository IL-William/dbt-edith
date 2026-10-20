# 0108. The silk has dewdrops and the selected box grows a web

Date: 2026-10-20 · Status: accepted

**Trigger:** read before changing what the Halloween look draws on an edge, the
web on the selected box, or what `select()` moves in the canvas.

## Context

Under the Halloween look (0106) the edges are thin silk, and the boxes carry
art up to a count (0107). The user chose two more touches: dewdrops along the
strands, and a small web that grows on the box just selected.

## Decision

- **The dewdrops are a second path per edge**, the same `d`, a dash that is a
  round cap and a gap, in a layer of their own between the edges and the boxes
  (`g.hw-beads`), written by `paintArt()` and so drawn under the same count as
  the box art: past `ART_MAX` the edges stay the plain silk strand. They are
  not in the edges' layer because `select()` moves the lit edge to the end of
  that layer so it is drawn on top, and nothing there may be wrapped or paired
  by position: a bead path carries `data-a` and `data-b`, `select()` lights it
  with its edge, and `paintPick()` fades it with its edge. A role's hue
  (`--edge-col`) is copied onto it and the stylesheet lightens it; a lit edge's
  are orange. An edge that is dashed to mean something, a test's, a back edge
  or an indirect one, has none, so that its dashes still read.
- **The web is one element at a time**, a clone of the template `#hw-web-src` of
  `index.html`'s hidden defs, put in the selected plain box by `select()` and
  taken off with it. It grows from the box's top right corner out into the gap
  beside it, where it covers neither the ring nor the names: threads drawn in
  by `stroke-dashoffset` (the paths have `pathLength="1"`), then three arcs,
  then a spider dropping on its own thread, about 700 ms, again for each new
  selection and left as it ended for the same box selected again, as a redraw
  does. It is cloned, not `<use>`d, because an animation inside a `<use>` is
  the original's and would not start again for each selection. It is drawn
  under any skin, past `ART_MAX` too, since it is a single element. Under
  reduced motion it is static. Both are taken out of the clone `snapshot()`
  writes.
- Measured in headless Chrome on a Mac (2026-10-20), on a 400 box canvas with
  507 edges drawn by `Lineage.render`, the dewdrops included: a pan cost the
  same as with the look off to within 0.2 ms of 16.6 ms for each skin, drawing
  the canvas took 40 to 60 ms against 35 without art, and laying a skin on a
  drawn canvas took 4 to 5 ms.

## Rejected

- **Beads as the edge's own dash:** one path cannot be a thin line and bigger
  dots, and a strand drawn in dashes would lose the dashes that mean a test, a
  back edge or an indirect one.
- **A bead path after each edge, in its layer:** `select()` and the legend would
  have to move the pair together.
- **A web on every box:** a spider per box is noise, and the single web says
  which box is the one selected.

## Consequences

A new state an edge can be in has to reach `paintBeads()`'s paths too, by a
class mirrored from the edge. `scripts/walk_export.py` checks that neither the
dewdrops nor the web reaches an export.
