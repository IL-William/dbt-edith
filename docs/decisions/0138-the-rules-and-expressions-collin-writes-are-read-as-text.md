# 0138. The rules and expressions collin writes are read as text

Date: 2026-11-04 · Status: accepted · Amends 0088, 0097 and 0102 · Follows dbt-collin's 0050

**Trigger:** read before changing what `--rules` or `--casts` print, what the
compact `--column` answer carries after `tests`, what a column box counts or its
card says of an expression, or what the card of a node reads from collin's cache.

## Context

collin 0.11.0 adds two lists to its cache, beside the edges and the roots of
0096: `rules`, the predicates each model applies (`WHERE`, `HAVING`, `QUALIFY`,
`JOIN`, with the columns each reads), written under `--indirect`, and
`expressions`, the loudest expression behind a column that is more than a copy,
the number of steps of its chain of CTEs that did something, and whether the
text was cut. Until then dbt-edith knew only the names of the columns a
predicate reads, fanned out to every column of the model (0097), and nothing of
how a value is computed. Both questions are the ones an agent otherwise answers
by reading the SQL.

## Decision

- **Read as text, in the places an agent already asks.** The compact `--column`
  line gains `steps` and `expression` after `tests`; `--column ... --rules`
  prints the predicates of the models on the walk, upstream first, a heading
  `model  (depth N)` and a line each; `--describe ... --rules` adds a
  `keeps the rows where` block to the card. The empty fields at the end of a
  line are left off with their tabs, so a copy or a rename costs what it did.
- **A cut and a recursion are marked where they happen.** A text collin cut
  ends in ` (cut)` in every text form. A chain of 32 steps or more prints
  `32+` in compact and `recursive` on the canvas card, never the number, which
  is where collin's limit fell and not a count. JSON carries the raw `steps`
  and `cut`.
- **A cast is folded in compact.** Its text is everywhere and says little; its
  step is counted, and `--casts` prints the text. JSON always carries it.
- **`--rules` is the flag that reads the cache for a card.** `--describe` reads
  no cache today, and a cache that holds every edge of every model, read for every card, would slow it. Without
  `--rules` the card is byte for byte what it was and no cache is opened; with
  it, the cache is merged into a copy of the graph so the card's own columns
  stay the manifest's and the catalog's. Over MCP the same copy is made, so the
  command line and the tool answer alike.
- **A cache without the lists says so on stderr.** Compact keeps its header and
  its four fields per row, and one line says why `steps` and `expression` are
  empty: the cache was written by a collin older than 0.11.0 (the only case
  that names 0.11.0), it is Fusion's, or it lists no expression, which a
  current collin does for a cache whose columns are all copies, renames,
  hashes or literals: it writes an expression only for a cast, transform,
  aggregate or window column. It first chose silence, which read as columns with
  nothing to compute. `--rules` prints nothing and stderr says the cache
  predates collin 0.11.0 or was written without `--indirect`; exit 0.
  `--describe --rules` prints the `column lineage:` line and the staleness
  warning of `--column`, so a rule of a model since changed is not read as
  current. `WANTS` moves to 0.11.0 so the Collin pill offers the update.
  *Amended by [0141](0141-a-column-collin-traced-in-part-says-so-in-from.md):
  `WANTS` is 0.12.0; the note that a cache lists no expression still names
  0.11.0.*
- **The canvas moves as little as it can.** A column box shows a small count
  above its box, on the left, only when `steps` is above one (`×3`, `32+`), the
  expression in its label and its hover card, the badge having no tooltip
  of its own (a native one would land over the card). The hover card opens for an expression too and
  adds `expression` and `steps` or `recursive`. The `keeps the rows of` line of
  0118 stays, and gains under it the predicates that read the column, of the
  models it names, when the cache has rules.

## Rejected

- **A cache version 2 in dbt-edith's reading**: both lists are additive fields,
  read with a default, as `roots` was.
- **`--describe` reading the cache always**: it would slow every card to print
  a block most cards do not need.
- **The rules on every compact line**: a predicate belongs to a model, not to
  each of its columns, and repeating it per column is the fan-out 0097 left.
- **A count on every box**: one step is what the role badge already says.

## Consequences

- The cache of a collin before 0.11.0 still answers; the Collin pill offers
  the update.
- On the canvas a column's card names only the predicates that read that
  column; every rule of a model is seen with `--describe ... --rules`, and
  `--column ... --rules` prints every rule of each model on the walk.
- `--rules` reads collin's cache when no tool is named, so a newer Fusion
  cache, which carries no rules, does not hide them; `--column-tool fusion`
  with it prints nothing and says why.
