# 0184. Columns are searched from the palette, behind a chip, with Collin picked

Date: 2026-11-23 · Status: accepted, amended by 0185 · Amends 0154

**Trigger:** read before changing what `Cmd/Ctrl + K` finds, adding a column to
what the box searches by default, or letting a tool other than Collin search
columns.

## Context

The palette finds nodes, macros and files, never a column. A developer holding
only a column's name, `amount` or `customer_id`, had the Search tab, which reads
lines of files and so misses every column a model gets from a `select *`, from
the warehouse or from collin's cache, which `Graph::merge_col_lineage` adds to
`node.columns` beside the YAML's and `catalog.json`'s.

Searching columns is a scan of every column of every model, seed, snapshot and
source in memory, many more entries than the names `Graph::search` scans: no
collin run and no disk, but a request that is slower than a search by name, so
it is kept to the chip's own requests and its answer is cut to the rows kept. The
cost that decides it is the answer. A column called `id` exists in most models; typed in the box that
finds a model called `id_map`, the same query would put hundreds of columns
above the nodes and the files nobody was looking for.

## Decision

- **Columns have a chip, off at every opening.** A `Columns` chip in the box,
  switched by a click or by `Tab`, the focus staying in the input. On, the box
  searches columns and nothing else; off, it is the box it was. It is off again
  the next time it opens, so no one finds `Cmd/Ctrl + K` searching something
  other than what the shortcut has always searched. The reason is noise, not
  cost: nothing is run or read from disk in either mode.
- **Only with Collin picked.** The route and the page both gate on the tool in
  play (`cll_tool`). Collin's cache is what brings a model its columns when the
  YAML and the catalog name none, and Collin is the tool that reads from the
  compiled SQL, so a column found is one the lineage can then open. Snowflake
  fetches a column's lineage on click, from the warehouse, and a cache of
  another tool is one the user is not on. Without Collin the chip is shown
  greyed, with the tooltip `Pick Collin in the settings to search columns`,
  rather than absent, so the feature can be found; on a static site it is hidden,
  there being no server to ask. `/api/columns/search` answers 409 when the tool
  is not Collin.
- **The search never runs collin.** A cache that is missing or older than the
  manifest is said under the answer, in the words the Columns tab uses, and a
  column opened from a cache behind the manifest runs collin first, as a click
  in the Catalog does (0042).
- **`model.column` narrows it.** A query with a dot splits at the last one: the
  node's name holds the left part, the column matches the right. That is the way
  past `id`. Ranked by how the column's name matches (the name itself, its
  start, anywhere in it), then, for a dotted query, how the node's name matches
  the left part (itself, its start, anywhere in it), then the shorter name, then
  the model's name; models, seeds, snapshots and sources are searched, tests and
  exposures are not. Nothing after the dot lists the columns of each node the
  left part names, node by node, in the order the node holds them.
- **A dot may belong to a name.** A source is `raw.orders` and a version
  `orders.v2`, so a query that is a node's whole name lists that node's columns
  first, and the whole query may match a column's name too. A column that
  several of these reach is listed once.
- **A row is a column.** The model's dot, the column, its type when known, the
  model's name. Choosing it opens the column's lineage and the model's file as a
  node row opens the file, leaving the canvas on the column.

## Rejected

- **Columns always in the box, as a group.** Hundreds of rows for `id`, above
  what the box has always found; the groups' five rows each (0154) would hide
  the nodes below them.
- **Columns in the box from a number of letters.** A threshold moves the noise
  rather than removing it: `order` is four letters and a column in half the models.
- **The chip on for every tool.** Another tool's columns are the YAML's and the
  catalog's, which the Catalog already lists per model, and Snowflake's answer
  comes on a click; offering the chip there would promise columns a model may not
  have yet.
- **Running collin from the search.** It would make a keystroke cost a run, and
  collin is started only by a click on a column or Regenerate (0042).
