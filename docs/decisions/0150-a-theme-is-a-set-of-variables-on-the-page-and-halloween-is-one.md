# 0150. A theme is a set of variables on the page, and Halloween is one

Date: 2026-11-05 · Status: accepted, amended by 0151 · Amends 0106 and 0107

**Trigger:** read before adding or changing a theme, the theme button or its
menu, what `look.json` holds, which theme an export is drawn in, or the
Halloween theme.

## Context

The page had one look and a switch for a second: Halloween, a class on the body
that re-pointed nine colours and held twenty-seven others at their old values,
with a pumpkin to switch it on and a speaker for its sounds (0106). The
redesign wants three more, Night as the default, Graphite in greys and Light
for daylight and screenshots, picked from the top bar and the settings. 0148
and the variable work before it had already put every colour of the page in a
named variable, so a theme could at last be only a set of them. Three things
had to be settled: where a theme sits on the page, how its colours are written
without four copies of two hundred names, and what an export takes of it.

## Decision

- **Four themes, `night`, `graphite`, `light` and `halloween`,** named Night,
  Graphite, Light and Halloween in the page. `data-theme` on `<html>` says
  which; `index.html` ships `night`. The design canvas's `nuit` and `clair` are
  its French and are not ids.
- **One block per theme, at the top of `web/app.css`.** Night is the full
  literal block, `:root, [data-theme="night"]`, with the values of 2.0 and every
  name a rule reads. Graphite, Light and Halloween each state the 83 names their
  design gives them (surfaces, text, accent, states, the kinds, tests, roles,
  environments and tools, code) and a colour scheme. The rest, about seventy of
  app.css's own names (a hover, a wash, a line, a pill's edge), are derived once
  by `color-mix` from those, in one rule the three share. A `var()` resolves on
  the element that carries the attribute, so the same rule serves a theme on
  `<html>`, a card's preview and an export's probe. The proportions were fitted
  against Night's own values, within 16 of 255 per channel, so what Night draws
  as a relation carries over. A name no theme restates keeps Night's value in all
  of them: the terminal's, the other files' colour, and in Graphite and
  Halloween the standalone meaning colours that need no twin. Light adds darker
  twins for those, from the same hue: 3:1 for a mark on the page and on the
  sidebar's `--bg-2`, 4.5:1 for code on the page. `web/tests/themes.js` holds
  the floor, and that no block repeats a name the derived rule sets.
- **Ten names that twinned another are folded.** `--veil`, `--head-bg`,
  `--more-bg`, `--more-line`, `--line-faint`, `--diff-hatch`, `--diff-gap`,
  `--label-bg`, `--label-fg` and `--fg-quiet` were kept apart only so the
  Halloween look could hold them while the rest moved. It is a theme now, so
  their readers read `--bg`, `--bg-2`, `--bg-3`, `--line` and `--fg-2`.
- **Halloween is a theme on `<html>`, its rules keep their weight.** Each rule
  of its section, `body.halloween`, becomes `html[data-theme="halloween"]`, both
  (0,1,1), and the box style's `hw-boxes-*` and the night's `hw-night` move to
  `<html>` too, a class beside the attribute, so `html[data-theme="halloween"].hw-boxes-X`
  weighs what the body's pair did. A class left on the body would gain one level
  and let the grey of one rule beat the ring of another. The section's colours
  that were literals stay literals, as the art's drawing colours, like
  index.html's gradients: a box style is a picture, not a set of colours a theme
  re-points. Its own colours are its block, and the 27-value hold goes with its
  reason, so its lines, hovers and code are the warmer browns of the design.
- **`look.json` holds `{ theme, follow, apparitions, boxes }`.** `theme` is
  one of the four, Night when missing or spelt otherwise, `follow` false by
  default, the rest as in 0106. A file 2.1 wrote is read once: its `halloween`
  is the Halloween theme when no `theme` is there, its `sound` is dropped, and
  the next write holds neither key. `POST /api/look` wants all four, refuses
  an unknown theme, and `halloween` or `sound`, with a 422.
- **Follow the system is a switch over the pick.** Off by default. While on, the
  page shows Light while the system is light and Night once it is dark, and it
  tracks the system live (`prefers-color-scheme`). Picking a theme, from the
  menu or a card, turns Follow off and keeps that pick; turning Follow off keeps
  the theme on screen. The "new" tag beside it is static text.
- **The picker is a button between `?` and GitHub, and a section of the
  settings.** The menu sits under its button, held inside the window from the
  right, one radio row per theme with a swatch painted in that theme's own
  variables (it carries its own `data-theme`). The section has four cards, each
  with a preview that paints itself the same way, Follow, and a dimmed group of
  what only Halloween has: its apparitions and the box style, named Headstone,
  Coffin and Spellbook, the stored values staying `tombstone`, `coffin` and
  `grimoire`. Cards and rows read only names every theme declares.
- **An export takes the theme on screen, and Night under Halloween.**
  `exportCanvasVars()` reads the variables from a probe element that carries the
  export's theme, appended to the page for the read and removed, not from the page
  itself, whose Halloween block would leak the art's browns into a file that has
  none of the art. Light on screen is a Light file, its `color-scheme` in the
  file's `:root` line, and Copy image paints its background from the same read.
  `Lineage.snapshot()` still takes the `hw-boxes-*` classes off for the length of
  its call, now on `<html>`. 0107's "an export is the usual blue" becomes "an
  export is Night, or the theme on screen".
- **The Halloween sound is removed**, with its code, its speaker and its tests.
  The apparitions stay, silent.
- **The terminal keeps Night's dark colours in every theme.** A light terminal
  wants all sixteen ANSI colours as light twins, and the design has five: that
  is the terminal's own step (Deferred 32 in `docs/state.md`).

## Rejected

- **A class on the body, as before.** A preview, a swatch or an export's probe is
  an element in the page, and none of them can sit under a class of the body
  with another theme's: a variable block keyed on an attribute can sit on any of
  them.
- **`localStorage` for the theme.** 0106's reason holds: 0011 keeps what is kept
  outside the page, and a cleared profile or another port would lose it.
- **195 literals times four.** Every name in four blocks is four places to keep
  in step and a theme that drifts by one edit; the derived rule is one place and
  Night is the only full list.
- **Exporting Halloween.** Its art is `<use>` of defs a file does not have and
  its serif cuts names to its own room (0107); a file with the browns and none of
  the art would be neither.
- **Keeping the sound.** The user's answer: it goes, and the speaker with it.
- **Serving `index.html` with the theme already set,** to avoid a flash of
  Night on load: a later step, with the config read before the first byte.

## Consequences

A rule of the Halloween section that forgets `html[data-theme="halloween"]`
leaks into every other theme, and `web/tests/halloween.js` fails on one. A new
colour is a name in Night and, if no theme can do with Night's value, in the
three blocks or in the derived rule, which the theme test holds to the same set.
The page is Night until `/api/look` answers, so a Light user sees one flash on
load, and another open tab keeps its theme until it is reloaded.
