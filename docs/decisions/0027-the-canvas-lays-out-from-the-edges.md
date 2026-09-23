# 0027. The canvas lays out from the edges, and depth stays a distance

Date: 2026-09-23 · Status: accepted · Amends 0006

**Trigger:** read before changing where a box sits on the lineage canvas, or
before reading `n.depth` for anything.

## Context

0006 said the layout layered by longest path from the focus. The code used
`n.depth` as the column, and `n.depth` is the server's BFS distance from the
focus. A model reached by a short path and by a long one sat at the short one,
so a model it feeds could be drawn to its left. On a 112 model neighbourhood,
27 of 152 edges pointed left or stayed inside one column, and edges passed
behind boxes 199 times.

## Decision

The browser lays the canvas out from the edges it draws (`dagLayout` in
`web/lineage.js`):
- columns by longest path, a node with more readers than parents pulled right
  to just before its first reader;
- a lane in every column a long edge skips, shared per kind once a node has
  four long edges;
- median sweeps that keep the order with the fewest crossings;
- heights from Brandes and Kopf, so long edges run level wherever no other lane
  crosses them, unless it spends more than half again the tallest column's
  height, as with tests ticked; a compact isotonic step then takes over where
  it comes out shorter.

Column lineage from access history can loop, so one edge per cycle is turned
round for the layering and drawn dashed.

`n.depth` keeps its meaning. `exportCommand` turns it into dbt's
`N+model+M`, which counts shortest hops, and a +N badge adds one BFS level.

Measured on that graph: no edge pointing left, crossings 316 to 57, no pass
behind a box, 14 to 21 columns, a millisecond or so.

## Rejected

- **Laying out by folder** (numbered folders such as `10_raw`, `20_clean`),
  the first idea. It is a project's convention, which the manifest does not
  state. An opt-in band per folder, ordered by the edges between folders, is
  deferred in state.md.
- **dagre or ELK.** 0006's reasons stand. The lanes, the room for role tags
  and the loops would still be written around them.
- **The layout, or a longest-path `depth`, on the server.** Box sizes are
  chosen in the browser, and the export's `dbt ls` line would be wrong.
- **Network simplex** for the columns. It moves the marts to the end, but
  costs about 110 delicate lines and adds crossings on the graph measured.
- **Brandes and Kopf alone:** 15 615 px with tests ticked, for 8 092 needed.
  **The isotonic step alone:** compact, but one long edge in seven bends.

## Consequences

The canvas is wider. 21 columns is the floor for that graph, whose longest
chain has 20 edges, so Fit zooms out further. The header's levels no longer
match the column count, and should not: one is reach, the other is order. A
test sits one column right of its model. The layout functions live in a slice
the harnesses evaluate, so they take sizes as arguments and touch no DOM.
