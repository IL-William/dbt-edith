# 0167. A hover card from the editor stays in the editor

Date: 2026-11-09 · Status: accepted

**Trigger:** read before changing where a hover card opens, or what bounds it.

## Context

The hover card was placed against the window: below what it describes when it
fits, above when it does not. From a line low in the editor it opened over the
dock, hiding what the dock shows and, being a floating surface of the editor,
looking like it belonged there. The Theme boards keep the card inside the editor.

## Decision

- **A card opened from the editor** (a ref, a macro, a var or env_var, a folder
  key, a missing ref, the unused paths of `dbt_project.yml`) **is placed inside
  the editor's rectangle**, cut by the window: below when it fits, above when it
  does not, clamped otherwise, and aligned within the editor's sides.
- **A card taller than that rectangle falls back to the window**, since a card is
  read whole and is never scrolled.
- **The canvas's and the Catalog's cards keep the window**: they have no
  rectangle of their own to stay in.
- The card is one element and one placement: `placeFloating` takes the room as a
  view that may start away from the window's corner, and `hoverEnter` an optional
  function giving the rectangle to stay in.

## Rejected

- **Putting the card in the editor's element**: the editor scrolls and clips, and
  the card must not move with a line.
- **Clipping a tall card to the editor**: half a card.

## Consequences

- A card from a low line opens above it, over the lines, not over the dock.
