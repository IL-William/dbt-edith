# 0080. A band may be cut by its sub-folders, a level at a time

Date: 2026-10-14 · Status: accepted · Amends 0029

**Trigger:** read before changing how deep the folder bands go, the order of
the sub-folders inside a band, or how one is drawn inside another.

## Context

0029 draws one band per folder, at the first level where the drawn models stop
sharing a path. A project laid out in layers often splits a layer into folders
of its own too: `staging/shop` and `staging/payments`, `marts/core` and
`marts/finance`. A canvas that reaches across the layers drew each layer as
one block, so the folders inside one were not told apart. The only way to see
them was a canvas drawn inside that layer alone, which loses the layers around
it.

## Decision

A **levels** box takes 1 to 4, at 1 first (`dagFolders`, `dagBands` in
`web/lineage.js`). It opens under the by folder toggle, over the canvas, while
the toggle is pressed, once the pointer reaches the toggle or the keyboard
does, and closes 2.5 s after the pointer leaves, or after the typing in it
ends (`wireFolderLevels` in `web/app.js`):
- 1 draws as 0029 did. Past 1, each band is cut by its sub-folders, that many
  levels down in all, by the path alone: a folder that holds a single
  sub-folder is drawn with it, so a level is the same depth on every band;
- each sub-folder is a run of columns inside its folder's, and sub-folders go
  in the order 0029 gives folders, among themselves only: by number, then by
  the edges between them, then where their nodes sit, then by name. A folder
  never moves for what it holds;
- the nodes filed in a folder beside its sub-folders are a run of their own
  there, with no rectangle of their own, placed by their edges alone, since
  the folder's number says where the folder goes, not where they go inside it;
- an installed package is never cut, its paths being the package's own;
- an edge from a later sub-folder to an earlier one in the same folder is
  turned for the layering, dashed and counted, as one between bands is;
- a sub-folder is drawn as a rectangle inside its folder's, in a hue per
  level, its margin a tenth of the gap between columns narrower each level
  down, so the folder shows round it and between two of them. The names go on
  a row per level, pinned over the top of the window as 0029's are, and an
  export writes each row in. The line under the canvas counts the
  sub-folders.

On the harness, 400 models in five folders, each cut twice, lay out in about
12 ms.

## Rejected

- **Sub-folders as rows inside the band.** The stages of a layer run in an
  order, and only the columns say which comes first; rows would also cross
  every edge running left to right.
- **A colour per sub-folder.** Neighbours would need as many hues as a folder
  has sub-folders, and the hue would no longer say how deep a rectangle is.
- **Skipping a level that holds one folder only.** Level 2 would then mean a
  different depth on each band, and the number in the box would not say which.
- **More than four levels.** The fourth's margin is already 6 px; a fifth
  would have none, and its rectangle would touch its neighbours.
- **The box always in the bar, beside the pressed toggle.** It moved Fit,
  Export and Copy image each time it came or went, and took room from a bar
  that folds its tabs once full. Under the toggle, opening it moves nothing.
- **Closing it as soon as the pointer leaves.** The pointer crosses a gap to
  reach it from the toggle, and a box that closes then cannot be reached.
- **Keeping it open while the toggle has the focus.** A click on the toggle
  leaves the focus there, so the box would stay until a click elsewhere; only
  the keyboard's focus keeps it, since without a pointer nothing else opens it.
- **Keeping the box across launches.** The toggle itself is not kept.

## Consequences

Every level costs width: a band cut into four sub-folders takes four columns at
least, and each level adds a row of names above the graph. One level, the
default, draws exactly what 0029 drew, which `web/tests/folders.js` checks
along with the rest.
