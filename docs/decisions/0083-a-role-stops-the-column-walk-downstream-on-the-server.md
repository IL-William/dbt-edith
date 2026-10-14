# 0083. A role stops the column walk downstream, on the server

Date: 2026-10-14 · Status: accepted

**Trigger:** read before hiding part of a column graph by role, or making
the column walk skip an edge.

## Context

A column graph a few levels deep mixes what happened to the column right
here with everything that follows a window or an aggregate further down,
where the column is no longer the same value. The question asked is "what
does this column reach before it is windowed". Fading those edges, the way
a legend pick does, leaves them on screen and still pulls in what lies past
them.

## Decision

- **The walk downstream stops at the column an edge of a cut role reaches.**
  The edge and that column are drawn: without them the graph would not say
  how the column is used, only that it stops somewhere. Nothing that leaves
  that column is drawn, even to a column drawn for another reason. The
  server takes `cut=window,aggregate` on `/api/collineage`, so the depth
  spent is spent on what stays: hiding past the cut in the page would draw
  fewer levels than asked.
- **A column reached around the cut as well is walked on from**, as if
  the cut edge were not there: it is not only past the window.
- **Upstream is left whole.** Above the focus every edge leads to the
  column; a cut there would hide where it came from, which is another
  question.
- **What leaves a stopped column counts in its `hidden_down`**, so the +N
  badge says something goes on past it.
- **The roles are ticked in a stop at menu beside the depth boxes, not in
  the legend.** A legend click highlights, which only repaints; stopping
  changes what the server walks, as up and down do. Both on one legend
  entry, a click on the name and a ✕ beside it, was tried first: the two
  were a few pixels apart, and the ✕ read as "hide these edges" rather
  than "stop after them". The menu lists the roles the cache holds, busiest
  first, and the button names the ones ticked. Several can be ticked at
  once, unlike a pick: stopping is narrowing, and narrowing adds up. They
  are kept in the page across refocuses and dropped on reload.
- **Collin is not touched.** It names each edge's role; which edges a view
  follows is the viewer's business, and Snowflake's roles cut the same way.

## Consequences

Clicking the +N badge of a stopped column deepens the walk, which stops at
the same role: unticking it in stop at is the way back. Leaving the cut
edge out as well was tried first: the graph then said the column stopped
somewhere, not how it was used.
