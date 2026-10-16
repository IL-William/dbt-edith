# 0087. A capped selection draws what is nearest its start

Date: 2026-10-15 · Status: accepted · Amends 0035

**Trigger:** read before changing how many nodes a selection draws, which of
them a capped canvas keeps, or what it says when it is capped.

## Context

The canvas of a selection stopped at 400 boxes and kept the first 400 by name.
A source's tables grown by one step, `source:stripe+1`, can match more than
that, and then the cut kept the sources, whose names sort together, and the
few children that sorted before them, and left out every other child. The
canvas showed sources read by nothing, which looked like a broken lineage. The
corner said `400 of … drawn`, and nobody read it.

## Decision

- **A selection draws up to 3 000 nodes**, the ceiling the route already
  allowed, now a design cap. Model mode keeps 400: its depth boxes narrow a
  picture that is too large, while a selection can only be narrowed by
  rewriting it. A larger canvas does not fail, but every step of zoom and pan
  takes longer as boxes are added, and on a slow machine much longer, which
  is what the cap guards.
- **Past the cap, a click draws the rest, this time only.** A button beside
  the warning, `Draw all N`, says it is slower to zoom and pan, and asks for
  up to 20 000 nodes, a ceiling on what one answer weighs and one canvas
  holds. The server sends that ceiling with every answer, so the button never
  offers more than it would draw. `N` leaves out the tests a named selector
  keeps that the tests eye keeps off the canvas, since no click draws them,
  and so do the warning and the corner, which counts them on their own. The
  raise holds for that line and that tests box: another selection, an
  emptied box, the tests eye, or a return from model mode is cut at 3 000
  again. Nothing remembers it.
- **Past the cap, the canvas keeps what is nearest the selection's start.**
  The start is what its criteria matched before their operators grew it: the
  sources of `source:stripe+`, the model of `+orders`. In an intersection a
  criterion that no operator grows only filters the others, so
  `+orders,resource_type:model` starts at `orders` and not at every model
  feeding it. Nothing written under an exclude is a start, however deep. The
  resolver records the start as `starts`, in a walk of its own after the
  answer. A selection that is all start, `tag:nightly` or `*`, says nothing
  about where to begin, and is drawn from its roots like one with no start.
- From the start the cap goes breadth first over the edges the canvas draws,
  in either direction, taking each node's neighbours by name. Every node kept
  past the start is drawn next to one kept before it. A test in the start
  stands for the model it hangs under. The parts that no start reaches begin
  together, at their roots, the nodes nothing in the selection feeds, so no
  part is drawn to its end before another begins. Tests still come after the
  models and sources, each model's first test before any model's second
  (0035), the model nearer the start first; context comes after the tests
  (0032).
- **A capped canvas says so under the box**, before any other warning, as
  well as in the corner. The warning points at Copy names, which an
  exported page does not have; that page already lists what was left out by
  name, so it does not repeat the warning. Copy names sends at most 5 000
  names, so past that the warning and its toast say it lists the first ones.

## Rejected

- **By name**: the bug above.
- **Whole chains, depth first** (a node after all its parents, its children
  straight after it): one long chain from the first start fills the cap
  before the second start is drawn, and the reader looks first for what the
  selection names. For the same reason the parts no start reaches are not
  drawn one after the other.
- **Layers from the selection's roots**: for `+orders` they keep the sources
  at the far end and drop `orders`, the model the selection named.
- **Each start with its first step before the next start**: no start would
  stand alone, but some starts would be dropped, and they are what was asked
  for. The warning and the `+N` badges already say what was left out.
- **Recording the starts while the answer is computed**: a named selector's
  answer is computed once however often it is named, and its starts would
  then depend on where it was named first. Keying the memo on that evaluated
  it twice, for an answer that does not change.
- **400 with a button to draw the rest**: a second click to get an answer the
  canvas can draw straight away.
- **No cap**, or a higher one by default: every selection that large would
  be slow to move around, and on a slow machine a click on it would answer
  late, for a picture too dense to read before zooming in.
- **Remembering the raise**: the next large selection would be slow without
  having been asked, and the cost would no longer be said.

## Consequences

When the starts alone fill most of the cap, as `source:*+` can, the canvas
draws the starts and few of their children. Each start carries a `+N` badge,
and the warning says what was left out. A start that an exclude or an
intersection removed from the answer does not count as one, and a selection
left without any is drawn from its roots.
