# 0081. Find on the lineage centres the box it finds

Date: 2026-10-14 · Status: accepted · Amends 0030

**Trigger:** read before changing what `Cmd/Ctrl + F` does over the lineage,
what a box is found by, or where finding one moves the view.

## Context

0030 left the lineage to the browser's own find. That find reads the names as
the canvas draws them, clipped to a box's width, so a long name was found by
its first letters only. It never moves the view, so a match outside the window,
the usual case once a graph is zoomed into, was counted and never shown. And a
test folded under a chip (0035) or a closed group's row (0040) is not drawn at
all, so it could not be found.

## Decision

- **`Cmd/Ctrl + F` over the lineage opens a find bar of its own**, the editor
  bar's shape without its three toggles, under the selector bar when that one
  is open, and under the note a canvas not of the file in front carries. The
  folder names move down under both bars. `Escape` closes it from the canvas
  too, and gives the focus back to what had it.
- **It searches what the canvas was sent**: every source, model, seed and, with
  the tests box on, test, a folded one included. A box is found by its name,
  and also by a source's table alone, a test's generic and column, since dbt
  shortens a long test name with a hash, and in column mode the column's model.
  In any case, as a substring: `.` and `_` mean themselves.
- **Typing lands on the best named box**: a name typed whole before a name it
  starts, before a name it is inside, then the first in reading order, which
  is column by column from the left and down each, a model's tests after it.
  `Enter` and `Shift + Enter` step in that order, round the canvas, and so do
  `Cmd/Ctrl + G` and `F3` from the canvas itself. A step or a click on a box is
  where the next step and the next query read from, as the cursor is in an
  editor's find.
- **The box landed on is brought to the middle** of what the bars, and the
  rows of folder names under them (0080), leave of the canvas, zoomed to its
  drawn size if the view was smaller, since a fitted graph can be too small to
  read the name looked for. A test folded away is brought there as the chip or
  the row it is under, painted as the current match. A step opens it, as a
  click on that chip or row would, two turns at most: the first step forward
  from it opens it rather than pass it by, unless it was opened or closed by
  hand since. Typing never does, so a keystroke never draws the canvas again,
  and what the first letters of a query land on stays folded once the query
  moves on.
- **Every match is outlined in amber, the current one filled**, as in the
  editor. An exported file or image carries none of it.

## Rejected

- **The editors' query, `findPrefs`**: a box is looked for by a name, and the
  last query in an editor is as often a piece of SQL.
- **Match case, whole word and regular expression**: dbt names are lowercase,
  and the order above already puts the name typed whole first.
- **The whole project**: `Cmd/Ctrl + K` searches it already, and moving the
  focus would redraw the canvas being read.
- **Reading order from the focus**: the first match from the left is the one a
  reader can predict.
- **Panning without zooming**: on a fitted graph the box found is a few pixels
  of amber with no readable name.
- **Opening a folded test as typing lands on it**: every keystroke that
  landed on one drew the whole canvas again, and `o` on the way to `orders`
  opened a chip that stayed open after `Escape`.
- **Folding back what a step opened**, on the next step or on `Escape`: a
  step opens it as a click would, to be read, and a click's opening stays.

## Consequences

`Lineage` holds the search, since only it knows where each box is drawn and
what a folded test stands under; `app.js` holds the bar. The reading order and
the names each node is found by are kept from one drawing to the next.
`web/tests/graphfind.js` slices `web/lineage.js` from `function findNames` to
`function pastSlop`, and the bar's part of `web/app.js`.
