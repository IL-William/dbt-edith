# 0097. An indirect edge is hidden by default, drawn dashed when shown

Date: 2026-10-18 · Status: accepted · Amends 0096 and 0089

**Trigger:** read before changing how an edge of a role is drawn or walked,
what the legend groups, or which edges a column's badge or counts read.

## Context

collin's `--indirect` writes `filter`, `join_key` and `dedup_key` edges (its
0011): the column on the left decided which rows a model keeps, and the edge
reaches every column of the model. They were walked and drawn as value edges,
so a generated column's card said "from no column" beside a line from the
column filtered on.

## Decision

- **Hidden by default.** Off, the server crosses none of them in either
  direction, and no +N, status line or Catalog figure counts them; the status
  line and an export say `indirect edges hidden`. An `indirect` button, an eye
  beside stop at, shown when the cache holds such roles in any case, walks
  them; it holds across refocuses and is off after a reload. Stop at offers no
  indirect role while hidden; `--column` leaves them out unless `--indirect`.
- **Shown, they are thin, dashed (`3 2`), in one teal**, apart from a test's
  dots and a back edge's dashes; `.edge.hi` keeps the dash. One legend entry,
  `indirect`, collin's word, with a dashed swatch, lights them.
- **No badge reads them**: a column only they reach reads as one nothing
  feeds, by 0096's rule, `hidden_up_rows` telling a +N of them.
- **The card names what keeps the rows**, `rows kept by: raw_orders.status
  (filter)`, button on or off: the column route names eight such columns at
  most, and a count, which `--column` never builds. A column only they reach
  or leave reads `indirect only` in the Catalog and still opens, its
  `/api/node` row marked `indirect`; its card, when they reach it.

## Rejected

- **Shown by default**: it drew `status` as feeding `loaded_at`, which is
  false of its value; the user chose them hidden.
- **Three legend entries, or the word `rows`**: collin calls them indirect.
- **The card only with the button on**: what keeps a column's rows is worth
  saying when the edges are hidden, which is most of the time.

## Consequences

A cache with no indirect role answers as before, and no button shows.
