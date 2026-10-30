# 0134. Columns are printed alone, by one flag, for a card and for a selection

Date: 2026-10-30 · Status: accepted · Amends 0057 and 0102

**Trigger:** read before changing what `--columns` prints, before giving a
selection anything the merged graph holds, or before adding another flag that
narrows a card.

## Context

An agent asking which columns the models of a layer have read a card per model
with `--describe`, each a page of parents, children, tests and reach, or read
their files. 0102 left two ideas open: a line per column of every selected node,
and a card of the columns alone. A selection reads the manifest alone (0057),
and a column's type is the catalog's to give.

## Decision

- **One flag, `--columns`**, beside `--describe NAMES` or `--select` /
  `--selector`. It narrows what is printed of the nodes found; it finds none.
- **With `--describe`**: each node's first line, `status  disabled` if it is, and
  its `columns (N)` block as the card has it, or with `--output compact` the
  same five-field rows with a header as a selection gets, not card text; as JSON
  the node's `unique_id`, `name`, `resource_type`, `disabled` and `columns` as
  `--describe` gives them.
- **With a selection**: `--output name` prints `node.column`; `compact` a row of
  five fields under `node`, `column`, `type`, `tests`, `description`, every
  field written as the other compact modes do, tests joined by a comma; `json`
  one object per node. The node is its name, or its unique_id where another
  node shares it. The card keeps its own `, ` and drops trailing empty cells.
- **One function makes a column's cells** for the card and for `--columns`
  (`column_cells`), so the two cannot drift.
- **A node with no column prints nothing.** stderr counts those of a selection
  and names those of `--describe`. A macro named to `--describe` is one: nothing
  on stdout, `name: a macro has no columns` on stderr, the same exit code.
- **The merged graph is loaded for `--columns` only**, as `--describe` loads it,
  and a selection without the flag still reads the manifest alone. Selecting
  reads nothing the merge changes (a test holds the two to the same ids). Over
  MCP the graph the server keeps for `--describe` answers (0110).
- **Refused beside** `--count-by`, `--sql`, `--compiled-sql` and `--column`, by
  clap.

## Measured

2026-10-30, release build, medians of 20 calls, `-s resource_type:model`: Jaffle
Shop (31 nodes) 9.1 ms with and without the flag; Fivetran's Shopify v1.10.0
(555 nodes, 2 667 declared columns) 57 ms without, 124 ms with, 128 ms with a
catalog of every declared column. The merged graph is about 65 ms of that.

## Rejected

- **`--output columns`**: output says how an answer is written, not what it
  holds, and `json` and `compact` would each need a second meaning.
- **A flag per mode** (`--describe-columns`, `--select-columns`): two names for
  one narrowing, and a third for the next mode with columns.
- **The merged graph for every selection**: 65 ms on Shopify for the calls that
  ask for names.
