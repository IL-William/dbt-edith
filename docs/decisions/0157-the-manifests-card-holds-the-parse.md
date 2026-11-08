# 0157. The manifest's card holds the parse

Date: 2026-11-08 · Status: accepted · Amends 0071 and 0025

**Trigger:** read before changing what the manifest pill's click does, or what
its card's button runs.

## Context

The pill's badge was the parse: a click typed `dbt parse` in the terminal and,
by the exception 0071 keeps for it, pressed Enter, while hovering showed a card
that explained what the pill counted. The design gives the pill a card that
lays out what it counts (the files by group, where the branch sits) with the
parse as a button inside it.

## Decision

- **The badge opens the card.** Under the pointer after a rest, a click holds it
  and gives it the focus, as every menu of the app does (0086), on the Run
  button when there is one so that Enter twice opens no file. Escape closes it
  and returns the focus to the badge; Tab closes it and goes on from the badge.
- **Nothing changes but where the exception sits.** The one button that presses
  Enter (0071's exception, since `dbt parse` reaches no warehouse; only typed
  while the shell reads the current folder, 0130, the hint beside it saying
  which) moves from the freshness badge's click to the manifest card's Run dbt
  parse button.
- **No button when the manifest is fresh.** The card says there is nothing to
  do, or gives the server's advice (pull the default branch first) as a line.
- **The card is drawn again in place** when a poll changes the answer, the focus
  kept on the same button.

## Rejected

- **Keeping the click as the parse and the card hover-only.** A click that runs
  something you meant to read about.
- **A card without the button.** One more trip to the terminal.

## Consequences

- Parsing from the pill takes two gestures instead of one: open the card, press
  the button.
