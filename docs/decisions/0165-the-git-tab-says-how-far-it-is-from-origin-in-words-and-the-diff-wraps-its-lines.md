# 0165. The Git tab says how far it is from origin in words, and the diff wraps its lines

Date: 2026-11-09 · Status: accepted · Amends 0007

**Trigger:** read before changing the Git tab's branch row, its rows or its
commit button, or how a diff lays out, wraps or folds its lines.

## Context

The Git tab showed the distance to origin as two badges on the pull and push
glyphs, a file's name cut to its width with the folder beside it, a letter after
a lower-case `stage all`, and a commit button that said `Commit` whatever was
staged. The diff cut a long line at the pane's edge, so a Markdown paragraph or
a long SQL line could be read only by scrolling sideways in each pane, and it
painted an added line in the red of a removed one, because the merge addon marks
both panes with the class of its left view. The design draws the branch and the
words `1 behind` in one row, whole names, a count beside each section's name,
a button that says what it would commit, and a diff that wraps.

## Decision

- **The branch row says how far it is in words.** `2 ahead`, `1 behind` or both,
  each part in its own tone (the commits to pull in the warning colour), the
  title naming the upstream; nothing when level or when there is no upstream.
  The three glyphs carry no count: their titles say what each would do (`Pull 1
  commit (fast-forward only)`, `Push 2 commits`, `Nothing to push`). Push stays
  pressable with nothing to push, since its modal says why.
- **A row is a letter, the file's icon, its whole name and its folder.** The
  name is cut only when the row is too narrow for it, and the folder gives way
  first. Stage, Unstage and Mark resolved are a button of the row, shown on
  hover and on keyboard focus within it; the row is a stop of its own and opens
  the diff on Enter or Space. The row of the diff tab in front is marked. A
  section's action is a real button, in sentence case.
- **The commit button says what it would commit.** `Commit 2 files`, `Commit the
  merge` once nothing is conflicted, `Resolve the conflicts first` while
  something is, and `Stage a file to commit` with nothing staged, which is
  `aria-disabled` and outlined, not disabled: a click on it says the toast it
  always said. The shortcut the box already answers to is drawn beside it.
- **A diff wraps its lines, in either layout.** Side by side, the addon aligns
  wrapped lines by their heights; the page asks it to align again when the pane's
  width moves. The panes meet at a one pixel rule. A change is a tint of the
  green or the red of the page on the row and in its number gutters, the changed
  words a stronger tint; the `--diff-*` colours, read by nothing now, are gone
  from every theme. A folded stretch is a row that says `N unchanged lines`.
- **Inline is built on the page.** One read-only editor holds the lines of both
  sides in order, from `diff_match_patch` in line mode, the library the side by
  side view already uses, so both layouts agree on what changed. The number of
  each line in HEAD and in the working tree has a gutter, and stretches farther
  than 12 lines from a change fold as they do side by side.
- **Both layouts compare lines without their CRs.** Under `core.autocrlf` the
  working file is CRLF against an LF blob: comparing the CRs would mark every
  line, so the side by side view is given the text as CodeMirror shows it and the
  inline layout drops them too. A file changed only in its line endings then has
  no changed line, and the head says `line endings differ (CRLF here, LF in HEAD)`.
- **The layout is a choice of the page, not of the project.** One switch in the
  diff's head, kept in `S.diffLayout` until the page is closed and applied to
  every diff tab; changing it rebuilds the tab's body with the first line in
  view still in view.
- **Markdown reads the same in its tab and in its diff.** A heading is bold text
  in the text colour, the `#` marks quiet and not bold, inline code in the
  string colour, scoped to a Markdown editor because CodeMirror tags code as a
  comment and a SQL comment keeps its own.

## Rejected

- **A unified diff from the server.** The page already holds both sides, and a
  new route would be one more thing to guard (0015), and the unified text would
  have to be parsed back into the two sides the page already has.
- **Keeping the layout in the settings.** It is a way of looking at one page,
  not a fact about the project (0011), and a setting would put a row in the
  settings for something changed twice a day.
- **Wrapping off by default, or a switch for it.** A long line cut at the pane's
  edge is what the diff was changed for; a diff is read, not edited, so it has
  no use for the horizontal scroll the editor's tabs keep.
- **Keeping the two badges beside the words.** Two ways to say one number drift.

## Consequences

The commit's shortcut is still bound by the message box alone (Mod+Enter), so the
keycap beside the button is a reminder, not a second binding. The inline layout does not mark the words changed inside a line, only the line.
The collin comparison tab draws its head with the same classes as a diff's, so a
change to `.diff-head` is a change to both.
