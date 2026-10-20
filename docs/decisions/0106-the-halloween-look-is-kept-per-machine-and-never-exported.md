# 0106. The Halloween look is kept per machine and never exported

Date: 2026-10-20 · Status: accepted

**Trigger:** read before changing the Halloween look, its switch or its
sounds, what `look.json` holds, or what an export or Copy image carries of the
page's colours.

## Context

The page can be dressed for Halloween: orange and black colours, drawings on
the brand, the dock, the switches and the lineage, and the odd apparition with
a sound. It is a game, off by default, switched on by a pumpkin in the top
bar. Three things had to be settled: where the choice lives, how the theme is
kept out of what leaves the page, and where the sounds and drawings come from.

## Decision

- **The choice is kept on the server, per machine, in `look.json`** in the
  config directory beside `tour.json` and `update.json`: `{ "halloween": bool,
  "sound": bool, "apparitions": bool, "boxes": "tombstone" | "coffin" |
  "grimoire" }` and nothing else. The look and the sound are false, the
  apparitions true and the boxes tombstones when the file is missing or not
  its shape. A file written before the apparitions switch or the box choice
  existed reads the missing key as that default, and a `boxes` spelled
  otherwise in the file reads as tombstone without costing the other keys.
  The apparitions, the ghost that crosses and the eyes, can be switched off in
  the settings without leaving the look, and the boxes are the shape a lineage
  box takes under it. `boxes` is an enum on the server, not a free string: a
  POST naming a style no stylesheet draws is refused with a 422 rather than
  kept to be read back. `GET /api/look` reads the file and `POST /api/look`
  writes it, every key wanted, behind the Host and Origin guard and the launch
  key like every route (0015, 0052). A look is the user's, the same in every
  project, as the tour offer is (0074), so it is not in the per-project
  settings. Without a config directory nothing is remembered: a read gives the
  defaults, and a write changes nothing on disk yet answers with the value, so
  the page wears the look for its session.
- **Every rule of the theme that changes how something looks is scoped under
  `body.halloween`.** The rules that are not name elements the look owns and
  say what they are with it off: `.hw-ghost`, `.hw-witch` and `#hw-layer` are
  `display: none` until the class is there, `.hw-cross` and `.hw-eyes` are made
  by script only while it is, and `.lookbtn` and the two `#look-btn` rules lay
  out the pumpkin and the speaker and swap the pumpkin's glyph by its pressed
  state, which belong to the buttons and not to the look. None of them is in
  the canvas. The export's
  stylesheet is the hand-copied list `exportCanvasCss`, and `Lineage.snapshot()`
  clones the canvas root only, so a rule that does not name the class cannot
  reach either. `web/tests/export.js` asserts the export's CSS holds nothing of
  the theme. Copy image draws from that same snapshot, so a picture for a
  ticket is never orange.
- **Sounds are synthesised in the browser** with Web Audio: an oscillator and
  filtered noise, no audio file, so there is no licence to carry and nothing
  to vendor. They are muted by default behind a speaker button. The audio context
  is created on the first click or key anywhere once the look and the sound are both
  on, since browsers allow one only after a gesture.
- **Drawings are inline SVG written by hand**, so they look the same on a Mac
  and on Windows. What the lineage's boxes draw under it, and how many, is 0107.

## Rejected

- **`localStorage`**: 0011 puts what is kept outside the page, and a browser
  profile that is cleared, or another port, would lose the choice.
- **The per-project settings** (`src/settings.rs`): the look would change with
  the project opened, and would sit in a file meant for the project's choices.
- **Emojis for the drawings**: each OS draws its own, so a pumpkin or a ghost
  would differ between the Mac and Windows.
- **CC0 audio files**: a file to vendor, a licence to track in
  `THIRD_PARTY_NOTICES.md`, and weight in a binary that embeds its frontend.

## Consequences

A new rule of the theme that forgets `body.halloween` leaks into the page when
the look is off, and into an export if the rule is copied into the export's
list; the export test is what catches the second. `look.json` is rewritten in
full on every switch, and a hand-edited file with a key it does not know is
read for the four it does and written back without the rest.
