# 0055. The same manifest answers the same way on every start

Date: 2026-10-10 · Status: accepted

**Trigger:** read before iterating a HashMap into anything a route returns, or
changing the order `Graph::build` reads the manifest in.

## Context

`Graph::build` numbered the nodes in the order the manifest's sections came out
of their HashMaps, which Rust seeds afresh on every start. Every list kept by
index followed that order, and so did every first-of-several taken from one:
the node a properties file opened on, the search hits that made the limit, the
tests a capped selection kept and so how many edges it drew, the nodes a capped
lineage kept, and the 5 000 names Copy held past that many. Three starts on the
project measured, asked the same 4 682 questions, disagreed on 95 of them under
1.1.0 and on 99 under 1.2.0.
Earlier fixes sorted one consumer at a time by name (0035), and the next one
read the index again.

## Decision

The manifest's `nodes`, `sources`, `exposures` and `disabled` are read into
BTreeMaps, so a node's index is its rank by unique_id within its section, the
same on every start. Counts sent to the page are BTreeMaps too, so an answer is
the same to the byte. Where a pick means something to a person, it goes by name
and then unique_id rather than by index: the node a file opens on, the order of
equal search hits, the names Copy keeps past its cap. A column test without
`attached_node` hangs on the node declaring it (0041), no longer on its first
parent, which for a source's `relationships` test was the model it points at.

## Rejected

- **Sorting each consumer**: what was done until now. The index still moved,
  and every list read off it, a payload's order included, moved with it.
- **The manifest's own order**: serde_json keeps it only with `preserve_order`,
  which brings in a crate (0003), and dbt promises no order between two parses.
- **A fixed hasher seed**: an order nobody chose, which a Rust release may
  change.

## Consequences

Three starts on that project now answer those questions to the byte, and its
manifest still loads in about half a second, release build, as it did before.
An index still means nothing to a person: a list someone reads is sorted by
what they read, the way the canvas already breaks its ties by name and then id.
