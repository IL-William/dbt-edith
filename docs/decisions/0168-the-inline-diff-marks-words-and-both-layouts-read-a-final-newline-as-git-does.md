# 0168. The inline diff marks words, and both layouts read a final newline as git does

Date: 2026-11-10 · Status: accepted · Amends 0165

**Trigger:** read before changing how a diff marks the words that changed, how it
treats a file's final newline or its line endings.

## Context

Inline marked lines, not the words in them. A final newline ended its line there and
started an empty last line side by side, so a file that gained one showed as nothing
in Inline, and an added empty line sat after the text beside it where `git diff`
puts it before. The head called a side CRLF when any line ended so, hiding a HEAD of
mixed endings and reading a lone `\r` as LF.

## Decision

- **Inline marks the words that changed**, with the tints side by side uses. Each block
  of changed lines has its removed text diffed against its added text in characters
  (`diff_main`, no line mode); every removed part is marked on the lines that hold it,
  every added part likewise, one mark per part, spanning lines where the part does and
  never painting the newline at its end. A line only added or only removed is marked
  whole. The whole file has one second for it: a block reached after that is marked
  whole, not diffed.
- **A final newline ends its last line and starts none, in both layouts**, through one
  function (`diffSides`): each side loses its one final newline before the layouts
  compare it. The exception is a file with the newline on one side only, both sides
  holding text and the view whole: that side shows an empty last line, as the editor
  does side by side and as one appended line does in Inline, and the head says `no
  final newline here` or `no final newline in HEAD`, after the line endings note. A cut
  view and a binary one say nothing of it.
- **Line endings are CRLF, LF, CR or mixed.** The head says `line endings differ`
  with both names (`mixed here, CRLF in HEAD`). Two mixed sides differ only where they
  hold the same line with another ending, so a line added never flags a mixed file.

## Rejected

- **A diff per line pair**: it pairs by position and marks words never edited when
  the counts differ.
- **A second for every block**: many blocks would wait that many seconds.
- **Keeping the final newline as a line in Inline**: a change of nothing shows as a
  change, and an added blank line lists after the text, against git's order.

## Consequences

- A block past the second is marked whole, so a huge rewrite may be marked differently
  on a slower machine, as side by side's own diff already is.
- Side by side no longer ends with the editor's empty line when both sides end in a
  newline.
