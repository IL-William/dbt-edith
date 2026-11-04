# 0145. A column collin traced in part is marked on the canvas

Date: 2026-11-04 · Status: accepted · Follows 0141

**Trigger:** read before changing what a column box or its hover card says about
a name collin gave up on, which names are marked, or how the card opens the SQL
at them.

## Context

0141 reads collin's `partial` list and marks the column in the compact
`--column` answer. The column canvas still drew such a column as whole: a box
with every edge it had and nothing to say that one input of its model was
missing. A person reading the graph needs to see that, read which names, and be
taken to the SQL to trace the rest.

## Decision

- **One rule for what is marked.** One test, `marked`, decides it: a gap whose
  reason is not `steps_capped` and whose name is not empty. `ColLineage::lost`
  gives a column's marked names, sorted, each once, and the compact `from` field
  and the canvas read it; the count on the `column lineage:` line reads `marked`
  too, so the three cannot disagree. An unknown reason is marked like the four
  collin names today: a later collin may add one, and a name it gave up on is
  still a name given up.
  The JSON `partial` list keeps every gap.
- **A pill on the box.** `/api/collineage` sends the names on the column's node
  (`partial`, absent when there are none) and the box draws an amber `PARTIAL`
  pill above it on the left, after the steps pill when there is one. The
  colours are inline, as the other pills', so an export keeps them. The box's
  label for a screen reader says `partly traced:` and the names.
- **The card says which, and where to read.** Hovering the box opens the card
  even for a column that has nothing else to say. It adds `partial  a, b` in the
  warn colour (eight names, then how many more), a line saying collin could not
  trace the name and to read the SQL, and two links: the model's file and its
  compiled SQL. A control character in a name is shown as a space, as the
  command line shows it.
- **Both links light the names.** Each opens the text and the editor's own find
  bar on every name at once, longest name first in the pattern so one that begins
  another is still lit, as whole words, case aside, with the first reached and
  the focus in the text. Only that bar's own query is set: what other bars open
  with is left as it was. The compiled link
  opens the Compiled tab of the column's model, which need not be the focused
  one, so `showDock` takes the model to load. A text that holds none of the
  names is shown as it is.
- **The status line counts them.** ` · N partly traced` follows the columns
  count when a drawn column has names.
- **Canvas only.** The Catalog's rows and the model canvas say nothing of it.

## Rejected

- **An amber outline or a `?` mark on the box**: an outline is lost among the
  role and test colours already on a box, and a `?` says nothing a person can act
  on. A pill names the fact in a word, in the place the other facts about the
  column already sit.
- **A Catalog row and a count on the model canvas**: the fact is about one
  column's sources, and the canvas of that column is where a person is reading
  them. A count on the model would make a mark of what is a few names in one
  column.
- **One link to the file only**: the compiled text is what collin read, and the
  file is what a person edits; each answers a different next step.

## Consequences

- A cache from a collin before 0.12.0 draws no pill and the card is what it was.
- collin errs towards silence (its 0051): a box with no pill may still have lost
  a name collin does not claim.
- The compiled link lights the names only when the pane painted that model's
  SQL; a text dbt has not compiled, an error or a newer load leaves it alone.
