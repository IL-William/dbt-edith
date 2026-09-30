# 0034. A test hangs under the model whose YAML declares it

Date: 2026-09-30 · Status: accepted · Amends 0027

**Trigger:** read before changing where the canvas draws a test, or what
`attached` in a lineage payload means.

## Context

0027 lays every box out right of all its parents, and tests were boxes like any
other, so a test took the column after its model's. A model with seventeen
tests grew a column of seventeen full boxes beside it, reading as seventeen
models built from it rather than as checks on it.

## Decision

A test with a drawn model among its parents leaves the grid. It hangs under
the model whose YAML declares it, dbt's `attached_node`, which the server sends
as `attached`; a singular test has no YAML, so it hangs under the parent in the
furthest column, the last of its inputs to be built. The layout runs on the
rest, each host taller by what hangs under it, and the tests hang as one-line
boxes, by name, joined to the host by a stem from its lower edge.

Another parent's edge reaches a hanging test from the left, loops out on the
right from the host's own column, and comes back dashed from a later one, as
a turned edge does. A test no model reads keeps a column. With nothing to hang,
the canvas is laid out exactly as before.

## Rejected

- **The furthest parent for every test**: no edge would run backwards, but a
  relationships test would hang under the model it points at.
- **Full-size boxes**: seventeen tests would still be a column of seventeen.
- **Lanes for the other parents' edges**: a lane needs the test in the grid.

## Consequences

An edge from another parent into a hanging test is a plain curve and can pass
behind a box, which 0027 rules out for the grid.
