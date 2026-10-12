# 0071. A click types a dbt command and leaves Enter to you

Date: 2026-10-12 · Status: accepted · Amends 0025

**Trigger:** read before passing `run` to `sendToTerminal`, or making any
button press Enter in the terminal.

## Context

`sendToTerminal` types a command and stops at the prompt unless `run` is
passed. 0025 passed it for the freshness badge because the command is the
whole point of that button. A Run button on a node's file, planned in
`docs/state.md`, is also a button whose whole point is the command, and by
that reasoning it would press Enter on `dbt build --select +name+`.

## Decision

- **A button types its command and leaves Enter to the user**, whatever the
  button is for. dbt runs where the user runs it (0002), and pressing Enter for
  them is running it.
- **The freshness badge stays the only exception**, for a narrower reason than
  0025 gave: `dbt parse` reads the project and writes `target/`, and reaches no
  warehouse. A command that can reach one is typed, never run.
- **The user is told that the command is waiting**, in a way that lasts until
  they act on it. A typed command that looks as if it has run is the failure
  this decision accepts in exchange.

## Rejected

- **Running on a click, with Alt-click to only type.** It was chosen first and
  then dropped. One misclick on `+name+` under `build` writes part of the
  project to whichever target the shell holds, and the page cannot know that
  target for certain: the shell may have exported another since.

## Consequences

Every button that types a dbt command takes one more gesture than one that runs
it. The note on `sendToTerminal` now cites this decision instead of 0025's
reason.
