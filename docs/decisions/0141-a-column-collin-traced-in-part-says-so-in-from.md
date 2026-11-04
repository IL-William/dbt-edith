# 0141. A column collin traced in part says so in `from`

Date: 2026-11-04 · Status: accepted · Amends 0138 · Follows dbt-collin's 0051

**Trigger:** read before changing what the compact `--column` line carries in
`from`, what the `column lineage:` line on stderr counts, or how dbt-edith reads
collin's `partial` list.

## Context

collin 0.12.0 adds a list to its cache beside `edges`, `roots`, `rules` and
`expressions`: `partial`, one entry `{node, column, reason, name}` for each name
the walk gave up on in a column that kept an edge. Before it such a column
looked complete: its edges were listed, and nothing said one input was missing.
An agent reading the compact answer took the `from` field for the whole of it.

collin names five reasons. Four are a name dropped inside the model:
`name_unresolved`, `relation_unknown`, `cte_lacks_name` and
`window_key_unbound`. The fifth, `steps_capped`, drops nothing: the edges are
all there and the chain is longer than `steps` counts, which `32+` already says.

## Decision

- **Read the list tolerantly.** `partial` is one more additive field, read with
  a default, as `rules` and `expressions` were. A cache without it reads as
  before; one saved here keeps it. An entry for a column the manifest lacks is
  dropped, as an expression's is.
- **Mark the column where its sources are read.** The compact `from` field ends
  with one `(partial: name)` item for each name dropped, after the edges, names
  sorted and each once whatever its reasons. `steps_capped` gives no item, since
  the `steps` field says `32+` for it. JSON carries every entry, the fifth reason
  included, as `partial: [{reason, name}]` on the column.
- **Count it where the cache is named.** The `column lineage:` line on stderr
  ends with `, N columns partly traced` when the cache has any column with a
  name dropped, and is what it was otherwise. It counts what compact marks,
  `cte_lacks_name` included, so it can be above collin's own `columns_partial`,
  which leaves that reason to the finding on the unbacked read.
- **The skill says what to do with it**, beside `(cut)` and `32+`: a partial
  column means collin gave up on that name inside that model, so the SQL is read
  (`--describe name --sql`, `--compiled-sql`) and the rest traced by hand.
- **`WANTS` moves to 0.12.0** so the Collin pill offers the update. The note
  that a cache lists no expression still names 0.11.0, the first collin to write
  one.

## Rejected

- **A sixth column after `expression`**: a partial mark is about the column's
  sources, and an empty field more on every line costs every answer for a mark
  most lines do not have.
- **The reason in compact**: it says why collin gave up, which the SQL says
  better, and the JSON keeps it.
- **A marker on the canvas** in this change: the command line is where an agent
  reads the lineage, and the canvas is its own change.

## Consequences

- An answer from a collin before 0.12.0 has no mark and no count; nothing on the
  line says the list is missing, as nothing said it before.
- collin errs towards silence (its 0051): a column with no mark may still have
  lost a name collin does not claim, a bare one among them.
