# 0118. An indirect edge ends on its model's box and names its clause

Date: 2026-10-26 · Status: accepted · Amends 0097 and 0099

**Trigger:** read before changing what the page asks collin for, how a
downstream indirect edge is drawn or walked, what names an indirect role on
the page, or how many boxes a column's canvas draws.

## Context

The page ran collin without `--indirect`, so the cache it read held no
`filter`, `join_key` or `dedup_key` edge and 0097's button never showed. A
column used as the dedup key of a `QUALIFY` in one model had no line to that
model. Run with the flag, the cache grows several times over, and one edge of
such a role goes to every column of the model it keeps the rows of: a column
`stg_orders.status` used in a `WHERE` reaches each column of `orders`, so one
level downstream filled a canvas capped at 200 columns by itself.

## Decision

- **The page always runs collin with `--indirect`**, and so does `--column
  --run-collin`, whose cache the page reads too: neither run strips the edges
  the other wrote. `--column` still decides per answer whether it crosses them,
  with `--indirect`, column by column as before. A cache written before this
  needs one Regenerate: its mtime says nothing of a missing flag.
- **Downstream, an indirect edge goes to one box standing for the whole
  model**, not to each of its columns. The box sits one step past the deepest
  column that reaches it, is a leaf, counts toward the cap, and carries the strongest
  role of that column's edges into the model (`dedup_key`, then `join_key`,
  then `filter`). Only the focus and the columns the walk went on from get
  one; never one for the column's own model, nor from an upstream column or a
  stop. The walk follows no indirect edge past it, and a stop at a role never
  applies to one. Upstream stays column to column: the fan-in of a model is
  bounded by its keys. A box has a model's id, so a click previews its file and
  a double-click draws it in Models mode.
- **The page names the three roles by their SQL clause**: `dedup_key` is
  QUALIFY, `join_key` JOIN, `filter` WHERE / HAVING, collin merging the last
  two. Each has a hue of its own, which a lit edge keeps; the clause is
  written along an edge that ends on a model's box, and not on one between two
  columns, where labels pile on each other upstream, the legend and the card
  saying it there. The legend lists `indirect`, which lights all three, and a
  chip for each clause drawn, at the end. The command line and the JSON keep
  collin's words.
- **A column's card says what it keeps the rows of**, `keeps the rows of:
  orders (QUALIFY)`, beside what keeps its own, button on or off. The server
  sends both as `[name, role]` pairs, eight at most with the rest counted.
- **Stop at lists no indirect role**, on or off. 0097's listing of them is
  withdrawn: a stop at an edge that ends on a box stops nothing. A line
  under the menu counts the indirect edges drawn, so it still adds up to the
  status line.
- **The column canvas draws 1000 boxes, not 200**, and a click on `Draw up to
  3000` draws that many for that request alone, as 0087's does for a
  selection. The status line says where it was cut.

## Rejected

- **An edge to each column of the model**: the cap was reached at the first
  level, and the picture said nothing a box does not.
- **collin's words on the page**: `dedup_key` means nothing to someone who
  wrote a `QUALIFY`; the SQL is what they know.
- **Edith reading the SQL to find the clause itself**: it parses none (0002,
  0023), and collin already read it.
- **A fourth role for `HAVING`**: it needs collin to tell them apart, and a
  `HAVING` that is not also a `WHERE` on the same column is rare.

## Consequences

- The cache is several times larger and a run a little slower. The page's
  server reads and merges it once, at start and on Regenerate, but every
  `--column`, and every column question to the MCP server, reads, parses and
  merges the whole cache again, so each pays for the larger file. Measured on
  an invented project of 2000 models of 50 columns with four keys each: 99,950
  edges and 10 MB without indirect edges against 491,754 and 51 MB with them;
  parsing it takes 14 ms against 72 ms, merging it 42 ms against 178 ms.
  Keeping the merged graph per cache in the MCP server would spare it (left in
  `docs/state.md`).
- The command line answers per column where the page draws per model, so the
  two disagree on how many boxes an indirect edge makes: the page's is the
  picture, the command line's the list.
- A kept collin run from before this holds no indirect edge: the comparison
  of two versions leaves them out of both when only one side has them, and
  says so, so an upgrade does not read as a wave of edges gained.
- Where one column is in a WHERE and a QUALIFY of the same model, the edge
  shows the strongest one only.
- `Draw up to 3000` cannot say how many there were: the walk stops when it is
  full and does not count what it turned away.
