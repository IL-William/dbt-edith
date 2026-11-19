# 0181. A newer release keeps its colour in every theme

Date: 2026-11-19 · Status: accepted · Amends 0148, 0150

**Trigger:** read before changing the colour or the drawing of what says a
newer release is out: Edith's badge, the status bar's pill, the menu's update
button or the Updates section's cards and button.

## Context

Every sign of a newer release read the accent: the arrow beside Edith's name,
the status bar's pill, the menu's "Update to" button, the Updates section's
Newest card, and the version beside Updates in the settings' list. In Graphite
the accent is grey, so on a real install all five went grey, and a state that
wants to be noticed was the quietest thing on the bar.
The user compared two mockups of a way out and chose the badge, drawn below.

## Decision

- **Three names, in the blocks at the top of `web/app.css` (0150).** Night
  holds them as literals: `--update` and `--update-dim`, its accent pair, and
  `--on-update`, white, the arrow drawn on the badge. Light and Halloween
  restate the pair as their own accent pair (Light's blue, Halloween's orange,
  as its ghost's glow). Graphite restates nothing, so it keeps Night's blue
  and never takes its grey accent.
- **Edith's badge.** A disc in `--update`, 14px, in the corner of the name and
  inside the button, ringed by a halo of the same colour. The disc is a round
  ground, allowed on the list of `web/tests/shapes.js` with its reason; the
  Updates arrow on it, still the `--m-newer` mask in `--on-update`, is the shape
  that carries the meaning (0148).
- **The status bar's pill is filled**: `--update-dim` with an `--update` border
  and a small halo, text in the page's text colour.
- **The menu's update button, and the recommended install button of the
  Updates section, share one look**: `--update-dim` fill, `--update` border, a
  still halo around it. The second command of a choice keeps the menu's other
  buttons' look, with no halo. The section's Newest card takes its border and
  its version in `--update`, and so does the version beside Updates in the
  settings' list (`.sethint.new`), with a 12% wash of it.
- **The halo is still.**

## Rejected

- **The arrow alone in blue, with a glow behind it.** Less visible: at 9px it
  is a speck.
- **Graphite's own way, an inverted grey button.** It fits the theme and
  catches the eye less, which is the fault this answers.
- **A pulsing halo.** A static one is enough: nothing should move for a state
  that lasts hours.
- **Leaving a release's colour to each theme's accent**, as it was: the
  reason for this decision.

## Consequences

A fourth set of names in the theme blocks, which a theme restates only if its
accent is a colour that says "new". The test of the blocks holds Halloween to
Graphite's set plus the pair. The arrow is white on every theme, 2.5:1 on
Night's blue: the disc, its ring and its halo are what is seen, the arrow what
is read, and it does not stand alone.

*Amends 0148*: the Updates arrow, the one marker of a newer release, is now
drawn on a round ground of its own, which 0148's list of round shapes allows
with its reason; the arrow is still the shape that says it.

*Amends 0150*: a meaning's colour may be held apart from the accent, so that a
theme whose accent is grey does not take it. The update colour is held in Night
and restated by Light and Halloween, and Graphite takes none.
