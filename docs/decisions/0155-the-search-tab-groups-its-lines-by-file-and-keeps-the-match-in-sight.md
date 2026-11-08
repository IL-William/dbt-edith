# 0155. The Search tab groups its lines by file and keeps the match in sight

Date: 2026-11-08 · Status: accepted

**Trigger:** read before changing how the Search tab lays out its answer, what a
click on a file's header or on a line does, how a long line is cut, or how many
lines a file shows.

## Context

The tab listed every matching line under a file name, in a plain list. A file
with twenty hits pushed the next file out of sight, a long line was cut at its
end, so in a narrow sidebar the letters searched for were often the part
hidden behind the ellipsis, and nothing said which file was the one already
open in the editor.

## Decision

- **One group per file, in the server's order.** The header is a button that
  stays at the top of the list while its lines scroll under it: the file's
  icon (0151), its name, its folder, and how many lines it holds (the ones
  shown plus the ones the server left out). The file open in the editor says
  `· open` after its name, in a span of its own so a long name's ellipsis never
  hides it, and keeps saying it right as tabs change.
- **A click on a header folds the file, a click on a line opens it.** Opening
  the file at its first hit from the header, as the list did, made the header
  both the way to look at a file and the way to hide it; a fold is the
  question a long list asks most, and a line already says where it opens. The
  one on the status line folds or unfolds every file. The folded files are
  kept from one query to the next and forgotten when the box is cleared.
- **The match is always in the row.** A line loses its indentation, and when
  the match ends past column 24 it is cut at the front, at the first of a
  space, `.`, `(` or `,` found 2 to 16 characters before the match (8 before it
  when there is none, one earlier if that falls inside an emoji), and starts
  with `…`. The end of the line is what the row's own ellipsis may hide.
- **Twenty lines a file stays the server's.** The page holds nothing past it,
  so `+N more in this file` is a line of text, not a button that would have to
  ask again.
- **The keys are the menus' way**: down from the box goes to the first header,
  the arrows move over the visible rows, up from the first goes back to the
  box, left and right fold and unfold a header, left on a line goes to its
  header, Escape gives the focus back to the box.

## Rejected

- **A header that opens the file** and a separate fold button: two targets in
  a 26 pixel row, one of them a few pixels wide.
- **Cutting every line to a fixed window around the match**: it hides the
  start of lines that were short enough to show whole.
- **Raising the server's cap** to make the `more` line a button: a search for a
  common word would then return most of a project to the browser.

## Consequences

- The result on screen is kept (`grepLast`), so a fold redraws from it and never
  asks the server again.
- A request still on its way when the box is cleared is dropped: clearing
  counts as a new query.
- `grepLine` is pure and built on `splitMatch`; `web/tests/grep.js` runs it.
