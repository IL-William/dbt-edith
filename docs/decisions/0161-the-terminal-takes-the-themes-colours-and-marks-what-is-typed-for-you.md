# 0161. The terminal takes the theme's colours and marks what is typed for you

Date: 2026-11-09 · Status: accepted · Amends 0150 and 0071

**Trigger:** read before changing the terminal's colours, what the dock's row
shows while the Terminal tab is open, or how a command the page typed is told
from one the person typed.

## Context

The terminal kept Night's colours in every theme, written as hex in
`initTerm`, so Light showed a dark terminal and Halloween a blue cursor. The
design draws it as part of the page: the theme's surface, text and accent, five
ANSI colours, a venv chip and the shell named in the tab's row, Clear and
Restart beside them, and a chip at the end of a line the page typed.
Until now a command typed for the person was announced by a toast that left
with no trace of which page had typed it or whether Enter had been pressed.

## Decision

- **xterm.js reads the theme.** `termTheme` maps the page's variables to its
  theme: `--bg`, `--fg`, `--accent` for the cursor, `--select` for the
  selection, and sixteen `--ansi-*` colours, declared by Night, Graphite, Light
  and Halloween each. `applyLook` sets the theme again whenever the look
  changes, Follow the system included, and xterm.js repaints at once. The
  terminal has no background of its own: it is `--bg`.
- **The eleven colours the design does not draw are ours.** Black, magenta,
  white and the eight brights follow the One Dark family the five drawn belong
  to; Light's are One Light's, darker, and Graphite's black and white are plain
  greys. Every ANSI colour but black is held at 3:1 on its theme's page, and
  the text at 4.5:1, by `web/tests/themes.js`. The selection is the theme's
  translucent one, so the text under it keeps its colour.
- **Halloween follows too.** Its terminal is brown like the page, with the
  orange cursor, not the exception 0150 left.
- **The tab's tools sit in the dock's row,** where the lineage's and the Jobs
  tab's do: a chip with the venv in the words the prompt uses (`(dbt) py
  3.11.9`, `(dbt) inactive`, `no venv`) and the status pill's pictogram,
  the shell and the project's name, then two icon buttons, Clear and Restart
  the shell. Both are real buttons with names, and put the focus back in the
  terminal. The fold of the tabs still measures the lineage's tools alone.
- **Restart closes the socket,** which ends the shell on the server (`terminal_loop`
  kills it when its socket goes), resets the screen and opens a new socket. The
  old one's close says nothing, since it is no longer the page's.
- **A command typed for the person is marked where it was typed.** A decoration
  at the end of its line holds a chip, `typed from the Jobs tab · Enter is
  yours`, or `run from the manifest's card` for the one command whose Enter the
  page presses (0157). Each caller names where it was clicked, as the person
  saw it. It is drawn once the shell has echoed the command, which is why Enter
  for that one command follows the echo, a moment after the text; the chip would
  otherwise land after its output scrolled it. It goes at the next key or paste
  the person types, at the shell's next prompt (bash says so, 0069), on Clear
  and on Restart, and there is one at a time. It replaces the toast; if it
  cannot be made, because the shell did not echo the command within a
  second, the toast is said as before.
- **The text is 12.5px on rows of about 19px,** with 16px at the sides and
  10px above, as drawn. xterm.js multiplies the font's measured cell height
  (15px on macOS) by its `lineHeight`, so that is 19 / 15, not 19 / 12.5; the
  padding is on xterm.js's own element, which the fit addon measures.
  `allowProposedApi` is on, which `registerDecoration` needs.

## Rejected

- **An opaque selection.** It hides the colour of what it covers, and the
  theme already names a translucent one.
- **Leaving Halloween's terminal as it was.** The person asked for it to follow
  the theme with no exception left.
- **A toast alone.** It says nothing once it is gone, nor which page typed the
  line.
- **An overlay placed by hand from xterm.js's private cell size.** The
  decoration API follows the line as it scrolls and goes with it, which an
  overlay would have to be told.
- **A terminal background variable of its own.** It would equal `--bg` in all
  four themes.

## Consequences

- A theme added later declares sixteen `--ansi-*` colours or the test fails.
- The chip is drawn only when the shell echoes the command at the cursor: a
  program reading the keys itself, or a line the shell repaints, gets the toast.
  *Amended in 2.20.8: a line with no room left before the terminal's right
  edge for the whole chip gets the toast too; a resize after the chip is drawn
  can still clip it.*
- zsh says nothing at its prompt, so there a chip stays until a key, Clear or
  Restart.
- The command the page runs for the person is sent in two writes, and the Enter
  is left out when the person typed a key in between; no chip is drawn then,
  the line no longer ending with the command, and the toast says Enter is
  theirs. A restart in between says nothing.
  *Amended in 2.20.8: when the key typed in between is Enter, neither the chip
  nor the toast is shown: the line is the person's and has run.*
