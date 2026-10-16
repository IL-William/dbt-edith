# 0089. A column count says what it counts: the canvas, or the whole chain

Date: 2026-10-16 · Status: accepted · Amends 0083

**Trigger:** read before changing a figure the column lineage shows: the stop
at menu's counts, or the Catalog's Lineage cell.

## Context

0083's stop at menu counted each role over the whole cache, beside a status
line counting the graph. The Catalog's Lineage cell counted one level, `←0 1→`,
for a column whose graph ran on for several: it read as how far it goes.

## Decision

- **The stop at menu counts each role's edges on the canvas**, from the
  answer last drawn for that column, busiest there first, then in the cache's
  order, and a line counts the edges with no role: it adds up to the status
  line. A role the canvas lacks is dimmed, not left out: a deeper walk may
  meet one. The project's count is on hover. An open menu recounts when a
  tick's graph arrives and keeps its order, so no row moves under the pointer.
- **The Lineage cell counts every column upstream and downstream, the whole
  chain** in the lineage loaded, as "in total" does for a model in Preview,
  and hovering it adds the one level the canvas opens on at a depth of 1.
- **The chain is walked on `/api/colreach`, which only the Columns table
  asks**: in `/api/node`, tried first, it made every hover card several times
  slower on a dense cache. Each side is one pass for all the node's columns:
  the columns reached are cut into strongly connected parts, and the starts
  reaching each part go as bits from part to part in topological order, so
  each edge is crossed once. A walk per column took seconds there.

## Rejected

- **Both figures in the cell**: a wider cell for a number few want at a glance.
- **One level kept, the header renamed Direct lineage**: right, but not the
  figure looked for, which is how far a column goes.
- **A walk per column, under a memo per node**: the first open still pays,
  and a graph merged in place by a Snowflake fetch would have to clear it.
- **A cap on the walk, `10000+`**: a figure that stops saying how far.
- **A worklist walking a column again while it gains a bit**: slower than a
  walk per column when the starts join one long chain at staggered depths.
