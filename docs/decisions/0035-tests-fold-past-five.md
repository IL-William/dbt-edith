# 0035. A model's tests fold past five, and a capped canvas shares its room

Date: 2026-09-30 · Status: accepted, amended by 0040 · Amends 0034

**Trigger:** read before changing how many tests show under a model, or which
tests the canvas keeps when it is capped.

## Context

Hanging under its model (0034), a test costs 26 px of height, and the most
tested node of the project measured carries 134: a pile of about 3 500 px. The
cap of 400 boxes, 3 000 at most, kept the canvas from failing but cut unevenly.
Model mode added tests model after model, in a HashMap's order, so the first
models kept all of theirs and the last none, a different last after every
reload; a selection kept tests by name across every model at once.

## Decision

Under a model the first five tests by name show, and a chip counts the rest
and opens them; opened, a chip folds them again. That is per model, and lasts
until the page reloads. A folded test is not drawn, nor any edge into it; the
line under the canvas counts them, and the open tests eye counts the tests sent.

When the cap cuts tests, the server takes one test per model a turn, each
model's by name, so every model drawn keeps some.

## Rejected

- **Lowering the depth past a threshold**: in Custom selection the depth is
  part of the answer, which Copy and dbt ls hold; in model mode it removes
  models, which are what the view is for.
- **Folding on the server**: the payload would stop holding the answer, and a
  chip would be a round trip.

## Consequences

With a chip closed, the canvas draws less than the payload holds, and the chip
and the status line say how much less.
