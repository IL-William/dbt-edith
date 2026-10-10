# 0044. The terminal copies and pastes the way Windows Terminal does

Date: 2026-10-06 · Status: accepted

**Trigger:** read before handling a key inside the terminal, or changing what
Ctrl + C does there.

## Context

On the Windows VM nothing could be copied out of the terminal or pasted into
it. xterm.js turns Ctrl + C into `^C` and Ctrl + V into `^V` and sends both to
the shell, cancelling the key, so the browser never fires its copy or paste.
On a Mac nobody noticed: Cmd + C and Cmd + V are not control characters, and
xterm.js leaves them to the browser. Ctrl + Shift + C, the Linux habit, opens
the element picker of Chrome and Edge. Only Ctrl + Insert and Shift + Insert
worked, and nobody reaches for them.

## Decision

Outside a Mac, `termClipboard` answers the key before xterm.js does, as
Windows Terminal and VS Code on Windows do: Ctrl + C copies when something is
selected and interrupts otherwise, Ctrl + V pastes, and Ctrl + Shift + C and
Ctrl + Shift + V always copy and paste. A copy empties the selection, so the
next Ctrl + C interrupts, and says nothing unless it fails. A paste is refused
to xterm.js and left to the browser, whose paste event xterm.js reads as
typing, bracketed paste included. On a Mac nothing changes. Ctrl with Alt is
AltGr on a Windows keyboard and is left alone, since it types a character.

## Rejected

- **Reading the clipboard with `navigator.clipboard.readText` on Ctrl + V.**
  Chromium asks for a permission the first time, on a page whose user expects
  a paste, and the browser's own paste needs none.
- **Ctrl + Shift + C and Ctrl + Shift + V alone**, leaving Ctrl + C an
  interrupt in every case. That is the Linux convention, and the people this
  is for are on Windows, where Ctrl + C copying a selection is what every
  console does.
- **Copying on select, and pasting on right-click**, the PuTTY and conhost
  habits. Neither is what Windows Terminal does by default, and both reach
  for the clipboard where nobody asked.

## Consequences

`web/tests/terminal.js` holds the keys, a Mac and the release of a key among
them. Any other key the app wants inside the terminal goes in the same custom
key handler, since xterm.js sends the shell whatever it is not told to leave
alone.
