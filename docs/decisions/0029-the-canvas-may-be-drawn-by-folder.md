# 0029. The canvas may be drawn by folder, and an edge against the folders is dashed

Date: 2026-09-28 · Status: accepted · Amends 0027

**Trigger:** read before changing which folder a node is filed under, the order
the folders go in, or what the canvas does with an edge between two of them.

## Context

A project laid out in numbered folders, `10_raw`, `20_clean`, `30_vault`, is
read as those layers. The canvas orders by edges alone (0027): on a 64 model
neighbourhood, 194 pairs of boxes sat in the reverse of their folders' order.
0027 rejected laying out by folder, since the manifest states no layering, and
deferred an opt-in band per folder.

## Decision

A **folders** checkbox gives each folder its own run of columns (`dagFolders`,
`dagLayers` in `web/lineage.js`):
- the folder is the first below where the drawn models and sources stop sharing
  a path, per canvas; one filed higher up keeps its own folder rather than
  lifting the level. Seeds and snapshots go under their own path, another
  package under its name, a test with the latest model it tests;
- folders go by the edges between them, a numbered one never before a lower
  number; where the edges leave a choice, by where their nodes sit without the
  bands, then by name;
- an edge from a later folder to an earlier one is turned for the layering and
  dashed, as a loop is in column mode, and the line under the canvas counts it;
- the names are pinned over the top of the window; an export writes them in.

Measured on the 18 825 node project: that neighbourhood went from 8 to 13
columns, crossings 52 to 54; a 157 model one from 13 to 19, crossings 1 191 to
1 752, 4 088 to 6 720 px tall; 400 boxes lay out in about 6 ms.

## Rejected

- **Refusing the bands when two folders feed each other**, as state.md planned.
  The canvas where a layer is read by an earlier one is the one most worth
  seeing in layers, and there the checkbox would have done nothing.
- **Ordering by the edges alone.** One edge from 30 to 20, the only one a canvas
  drew between them, put 30_vault first on that canvas and nowhere else.
- **One level for the whole project.** Every path from the server, and a single
  band whenever what is drawn sits in one folder.
- **Merging folders that feed each other into one band.** It hides that edge.
- **Names drawn at the canvas's scale.** Unreadable on a fitted graph.

## Consequences

A way of reading, not a fix: more columns, often more crossings, and a source
only a late model reads stays in the first band at the end of a long edge.
Without a band argument the layering is unchanged, which `web/tests/folders.js`
checks along with the rest.
