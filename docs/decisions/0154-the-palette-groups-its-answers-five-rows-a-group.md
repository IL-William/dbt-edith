# 0154. The palette groups its answers, five rows a group

Date: 2026-11-08 · Status: accepted · Amends 0084

**Trigger:** read before changing what `Cmd/Ctrl + K` shows of an answer, how
many rows a group keeps, how a cut answer is counted, or what its last row
hands over to.

## Context

0084 put nodes, macros and files in one flat list of up to a hundred and twenty
rows under a box that took half the window. A short answer left an empty box
and a long one buried the macros under forty models: nothing said which row was
which kind, and a file hit looked like a node beside it. The search tab holds
the answer to a different question, the lines inside the files, and the palette
had no way to reach it with the letters already typed.

## Decision

- **Rows are grouped, in a fixed order**: the nodes (models, sources, seeds,
  snapshots and exposures), the macros once something is typed, the files, then
  one row, "Inside the files", from three letters (the Search tab's own
  minimum). Each group has a sticky header that names it. The node group is
  titled by the kinds it holds, in the legend's order: "Models and seeds",
  "Models, sources and seeds", "Sources".
- **A group shows five rows and a `+N more` row**, which shows the whole group
  where it is: the box stays open, the focus stays in the input and the
  selection stays on the same index, now the sixth row. What was opened is
  forgotten when the next key draws a new answer. The box is as tall as its
  rows, up to a ceiling, and the list scrolls inside it.
- **A cut is said as a floor.** Each request asks for 41 and keeps 40; one that
  came back with 41 is cut, so its count ends in a plus (`40+`, or fewer when a
  file already shown as a node was dropped), the foot reads `N+ results`, its
  `+N more` row `35+ more`, and whenever the group shows all its rows, opened or
  five or fewer, it ends on a line that cannot be picked, "The search stops at
  40 a group: type more letters for the rest."
  Nothing claims a number the server did not give.
- **The last row hands the letters to the Search tab.** It closes the box and
  any panel in front (which brings Code back, and its sidebar), shows the
  Search tab, puts the query in its box and runs it. The Search tab keeps its
  own results; the palette does not read file contents.
- **A row says what it is**: a node its kind's bar, its name, how it is built
  and its file; a macro its glyph, its call, who uses it and its file; a file
  its icon (0151), its name and its folder; the letters typed are marked in the
  name, or in the path for a hit found by its path alone.
- **The foot names the keys** (move, open, close) and counts the hits of the
  three groups; the rows that open something else are not results.
- **`#branches`, the branch switcher, keeps `.palette-box` and `.pres`**: the
  new box is `.pal-box`, so a change to one does not move the other.

## Rejected

- **One flat list, as before**: it is the tall box this replaces, and it hides
  the kind of a row.
- **A request for every match**, to count them exactly: a one-letter query
  matches most of a project, and the count is not worth the walk. A floor that
  says it is one is exact about what it knows.
- **Opening the Search tab's results inside the palette**: two places would read
  file contents, and the tab already paints, caps and links them.

## Consequences

- A new kind of row is a new group in `paletteGroups` and a row builder, never a
  special case in the list.
- The capped line is not a row: the arrows, Enter and the count skip it.
