# 0146. A column is compared with another project's by its source columns

Date: 2026-11-04 · Status: accepted · Extends 0088, 0138 and 0141

**Trigger:** read before changing what `--against` prints, what makes two
source columns the same, how the other project's column is searched for, or
before offering the comparison over MCP.

## Context

Two projects often read the same source tables: a legacy star schema and the
project rebuilding it. The question that costs hours is which column of the
other is built from the same source columns, what each reads that the other
does not, and which rule one applies that the other lacks. collin's cache holds
the edges, the rules (0138) and the untraced columns (0141) of each project;
nothing set two caches side by side.

## Decision

- **A source column is `(schema, table, column)`**, lower case, quotes off, as
  the warehouse compares a name. The database is left out: each project renders
  an `env_var` database as its own compile environment set it. A `databases`
  line shows where the two differ.
- **The fingerprint is one upstream walk over every level**, indirect edges
  crossed. Each source column keeps the strongest role met, folded to `copy`,
  `computed` or `hashed` (a role the cache does not name is `computed`, not
  known to be a copy; collin's `inferred` edge is a copy too, and the line ends
  in `(inferred)`); those reached only through filter, join-key or dedup-key
  edges are `rows`. A column no value edge enters that is not a source's (a
  seed's, a snapshot's own, one the cache gave no edge), a column the SQL
  generates and a column read from a table no node owns are recorded as not
  shared; the walk goes on through their other edges, and through a snapshot
  to the source it reads. `(partial)` means a lost name, not `steps_capped`
  (0141), and with `(inferred)` it is read on the value's path only: a gap or a
  guess met only through a filter or a join key is not the sources' own.
- **The search is bounded**: one breadth-first walk downstream over value edges
  from each shared source column counts how many of them reach each column, and
  the fewest edges (hops) from one; the top twenty by hits, then by how far its
  hops are from this column's own depth, then hops, then name are fingerprinted, each walked once per call. A source read by two
  declarations of one table is one source, so a column both reach counts once.
  A row edge fans out to a whole model and is not walked. No pass over all the
  other project's columns.
- **Ranking is the share of the sources the two have together**, as an exact
  fraction, then the distance between the candidate's hops and this column's
  own depth (the fewest value edges from it up to a shared source, the measure
  a candidate's hops give downstream), then the fewer hops, then the more
  shared sources reached by the same class (a copy against a copy before a copy
  against a computed), then name; `shares a of b` is out of this column's own
  `b`. The depth comes first because in a copy chain every step reads the same
  sources and the fewest hops would put the step nearest them before the twin:
  the same step of the other chain is the likelier match, and a column deep in
  a layered project would never reach the shortlist. Hops then come before the
  name because a column built straight from the sources is likelier the match
  than the aggregates downstream of it, which an alphabet would put first.
  Columns equal on all of these (the same figure at one depth in several models)
  are told apart last by name: the one whose column name equals this column's
  (case-folded), then the one whose model name does too, then the alphabet. That
  is a tie-break between candidates the lineage cannot tell apart, in the
  shortlist and in the ranking, never a way to find a match, which "Matching
  columns by name" below rejects.
- **Rules are compared on what they read**: a rule is its clause and the source
  columns its columns resolve to (a model's column through its fingerprint), so
  `status <> 'x'` and `o.status != 'x'` meet and both texts are printed. The
  best match's models are the other side, and the card names it. A side with
  no rules is said so and nothing is compared. A rule's columns are resolved by
  their value sources alone, from one memo a side, each column once from the
  sets of the columns it is built from (a cycle of value edges sharing one
  set): the filters and join keys on the way up add nothing to a source set, so
  a full walk a column through them was thousands of walks on a deep project
  for the same answer. A group of rules is cut at twelve lines like a list,
  `+N` counting the rest and a note on stderr; JSON keeps all. Which models'
  rules are read is not cut: it is the fingerprint's whole walk.
- **Command line only**: `--against` names a path outside the served project
  (0110), so `against` and `with` are in `REFUSED`. Both projects are read from
  collin's cache, which alone holds rules and indirect edges: this side's too,
  so a Fusion cache never answers there unless `--column-lineage` names the
  file. A project without one is refused, exit 1.

## Rejected

- **Comparing the database**: a dev and a prod compile of one project would
  differ.
- **Matching columns by name**: the point is a figure rebuilt under another name.
- **A downstream walk per column of the other project**: its size would set the
  price of one question.
- **Comparing rule texts**: aliases and spelling differ for one predicate.
- **A directory named over MCP**: the model could read any project it names.

## Consequences

- A cache written without `--indirect` gives no rows and no rules; the card
  says so, and the steps that write one end in `--indirect`.
- Whole-project coverage and key composition against `unique_key` and tests
  (0127) are later increments.
