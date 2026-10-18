# 0096. A column a literal sets is generated, not raw

Date: 2026-10-16 · Status: accepted, amended by 0097 · Follows dbt-collin's 0045

**Trigger:** read before changing what a column box's badge says, or reading
a field of collin's cache other than its edges.

## Context

A column box with no edge into it in view was badged `raw`, whatever it was:
a source's column, `current_timestamp() as loaded_at` set in a model, or a
column whose lineage collin lost. collin 0.8.0 lists its roots beside the
edges: a model's column set from no column, with the literal (its 0045).

## Decision

- **`roots` is read beside the edges**, a root's column made as an edge's end
  is. A column box and an `/api/node` column carry `generated`: a value per
  branch of a union that sets it differently, none when collin could not tell
  all of it; a column named twice gets both lists, or none if either is.
- **A root is `generated` whatever reaches it.** Under `--indirect` the filter
  and join key edges of a model's reads reach every column, a root included:
  they say which rows exist, not what the column holds.
- **With roots in the cache (`roots_known`), `raw` is where data enters the
  project**: a source's or a seed's column, or one a `rel:` table's value
  reaches, never drawn since no node owns it, which the payload marks
  `outside`; a filter or a join key on such a table marks nothing. A model's
  or a snapshot's column with nothing feeding it that is no root has no
  badge: its lineage was lost, and `raw` would say it came from outside.
- **A cache without roots badges as before**: Fusion's, Snowflake's or an
  older collin's cannot tell a literal from a source, so every start stays
  `raw`, and the answers carry neither `roots_known` nor `outside`.
- **The value is on a hover card on the box**, in the box's label on one line,
  and in the Lineage cell before the counts, whose row opens the box even when
  nothing reads it. `--column` prints `generated` beside `from`, on a root.
- **The legend lists every badge drawn**, with or without a role on an edge.
- **A pink**, far from raw's green. No hue is left 15 from every colour of the
  canvas but a neon mint at the edge of sRGB; the pink is 12 from its
  nearest, farther than any role or raw but window is from theirs.

## Rejected

- **`generated` for every model column with no edge in**: a lost lineage and
  a literal would read the same, which is what the roots end.
- **No badge on such a column with any cache**: without roots it is as likely
  a literal as a lost lineage, and `raw` was what it always said.
- **The value on the box's second line**: it holds the model and the type, and
  `CAST(NULL AS TEXT)` would push both out.
- **An edge into a root taking its badge**, `filter` or `mixed`: the column
  still holds the literal.

## Consequences

With no root in the cache, every answer is main's byte for byte. An exported
page keeps a value in the box's label only, having no hover card.
