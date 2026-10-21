# 0111. The update ends on a line and a card, and a failed build is a failure

Date: 2026-10-21 · Status: accepted · Amends 0095

**Trigger:** read before changing what the update command prints or ends on,
what the page shows once it is typed or installed, or where a message in
the middle of the page replaces a toast.

## Context

0095 made the update a button, said under it what to do, and told the menu
and the pill once the new binary was in place. It still read as missed: after
the click, a toast of three seconds at the bottom said the command was ready,
and the restart was written only in the menu, which closes on the click. The
person ran the update, saw cargo end, and kept the old version running.

Putting the line where the person is looking found a fault in the command. On
Windows the Posix variant ends in `(build || mv -f "x.old" "x")`: when the
build fails and the file is put back, the subshell exits 0, so anything chained
after it would claim an install that never happened.

## Decision

- **The command ends on one line, only when the install worked**: `dbt-edith
  is installed. Stop dbt-edith where you started it (Ctrl+C) and start it again
  to run it.` `compose` chains it after the install, `printf` for the Posix
  shells and `Write-Host` for PowerShell, with no quote, `$` or backtick in the
  text, since it sits inside a string in both.
- **The put-back after a failed build ends in `false`** (`|| { mv -f ...;
  false; }`), so the chain stops and the binary is still restored. In
  PowerShell the line sits in the success branch of `$?` after the build and
  the restore in the other; `$?` stays true after the restoring `Move-Item`,
  so there a failed build prints no line, but its status is still a success.
- **A card in the middle of the page** says it too, one at a time, over every
  backdrop like a toast (80), with no backdrop of its own, closed by its button
  or Escape pressed in the terminal (where it stops at the card), on the card or
  on the bare page; in the editor, a find bar, the tour or a menu Escape is
  theirs. A click on the card keeps the focus where it was, so Enter still
  reaches the terminal. After the button types the command, in place of the
  toast: Press Enter, and the restart, gone after fifteen seconds. When an
  answer first makes the menu say installed (`replaced`, of a newer release)
  after one that did not, whether or not the button was pressed on this page: it
  is installed, and it stays until closed. Neither on the first answer after the
  page loads, which the pill already covers, nor on every poll.
- **The cards replace the toast for the update alone.** The collin update row
  and the freshness badge go on typing with their toast.

## Rejected

- **The `echo` 0095 left out.** It was refused for landing in one terminal
  written for two shells, and for saying nothing to the menu or the pill.
  `compose` already writes a variant per shell, and the cards now carry what
  the menu and the pill said alone, so the line is a second place that says it,
  not the only one.
- **A line printed whatever the build did.** It would say installed over a
  failed build, the sentence the swap fix exists to avoid.
- **A modal with a backdrop.** It would cover the terminal where Enter is to
  be pressed, and take the clicks meant for it.
- **Every toast as a card.** Most answer a click and are gone in three
  seconds; a card for each would be the noise that hides this one.

## Consequences

A command typed before this version has no closing line, and a page loaded
from one still shows the toast. The line is printed in the terminal by the
shell, so a user who closed the terminal sees only the card.
