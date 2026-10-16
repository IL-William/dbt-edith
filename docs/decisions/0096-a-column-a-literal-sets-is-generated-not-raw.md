# 0096. A column a literal sets is generated, not raw

Date: 2026-10-16 · Status: accepted · Follows dbt-collin's 0045

**Trigger:** read before changing what a column box's badge says, or reading
a field of collin's cache other than its edges.

## Context

A column box with no edge into it in view was badged `raw`, whatever it was:
a source's column, `current_timestamp() as loaded_at` set in a model, or a
column whose lineage collin lost. collin 0.8.0 lists its roots beside the
edges: a model's column set from no column, with the literal (its 0045).

## Decision

- **`roots` is read beside the edges**, a root's column made as an edge's end
  is, and a cache without them reads as before. A column box and an
  `/api/node` column carry `generated`: a value per branch of a union that
  sets it differently, none when collin could not tell all of it.
- **A root is `generated` whatever reaches it.** Under `--indirect` the filter
  and join key edges of a model's reads reach every column, a root included:
  they say which rows exist, not what the column holds.
- **`raw` is where data enters the project**: a source's or a seed's column,
  or one read from a `rel:` table, never drawn since no node owns it, which
  the payload marks `outside`.
- **A model's column with nothing feeding it that is no root has no badge.**
  Its lineage was lost, which collin's report may mark with a ! in Catalog >
  Columns, or the cache cannot say, being collin 0.7.0's, Fusion's or
  Snowflake's. `raw` said it came from outside, wrong either way. A
  snapshot's column is read as a model's.
- **The value is on a hover card on the box** and in the Lineage cell, before
  the counts. `--column` prints `generated` beside `from`, on a root only, so
  every other line reads as before; `--output name` stays a name a line.
- **A pink**, far from raw's green, which sits beside it wherever a model
  reads a table and sets a literal.

## Rejected

- **`generated` for every model column with no edge in**: a lost lineage and
  a literal would read the same, which is what the roots end.
- **The value on the box's second line**: it holds the model and the type, and
  `CAST(NULL AS TEXT)` would push both out.
- **An edge into a root taking its badge**, `filter` or `mixed`: the column
  still holds the literal.

## Consequences

With no root in the cache, every answer is main's byte for byte, but for
`outside` on a column read from a `rel:` table. The canvas changes even so: a
model's column with nothing feeding it loses `raw`. An exported page names a
value in the box's label only, having no hover card.
