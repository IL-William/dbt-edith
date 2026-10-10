# 0030. Find is the app's own, and the shortcut list opens on ?

Date: 2026-09-28 · Status: accepted, amended by 0037

**Trigger:** read before adding a keyboard shortcut, or changing what
`Cmd/Ctrl + F` does anywhere.

## Context

`Cmd + F` in the editor reached the browser's own find, which cannot work there:
CodeMirror keeps only the lines on screen in the page, thirty of a 730 line
model, so a match further down was never found. The Compiled and Run tabs are
CodeMirror too. The Catalog's Columns table can run to hundreds of rows. And the
shortcuts that did exist were written down in the README and nowhere on screen.

## Decision

A find bar per CodeMirror, VS Code's in shape. It is reached through the three
commands CodeMirror's default keymaps already bind and leave undefined, `find`,
`findNext` and `findPrev`, so whichever editor has the focus answers, the diff
panes included. One pure function, `matchInLine`, decides what a match is for
the count, for the overlay that paints, and for a step, so the three cannot
disagree. A step reads the text rather than the counted list, which stops at
10 000, and typing searches from where the cursor was put, not from the last
keystroke's match.

`Cmd + F` outside an editor goes by pane: the Catalog focuses its column filter,
a click on the Compiled tab then `Cmd + F` searches the SQL under it, and the
lineage and the terminal keep the browser's find. The column filter matches
names only: a description mentioning `customer` would bury the columns called
that under every column that talks about one.

The shortcut list opens on `?` outside a text box, on `F1` anywhere, and from a
button in the top bar. `shortcutSheet` is its only source, and
`web/tests/keys.js` fails when `wireKeys` answers a key the list does not name.

## Rejected

- **`Cmd + H` for the list**, the first idea. macOS hides the browser on it
  before any page sees the key. `Ctrl + H` works on Windows, but a shortcut that
  exists on one of the two machines this runs on is a trap on the other.
- **`Cmd + /`**, the list's key in Google Docs and Slack. In an editor it is the
  comment toggle, and it stays free for that.
- **CodeMirror's search addon.** A prompt at the top, no count, no case or word
  toggle, a pattern typed between slashes, and two more vendored files.

## Consequences

A new global shortcut is a `case` in `wireKeys` and a row in `shortcutSheet`,
or the harness fails. `keyCombo` reads the physical key when Option is held on a
Mac, where Option turns W into `∑`: `Alt + W` never matched there before. That
is a US keyboard, and 0037 says what a French one does instead.
