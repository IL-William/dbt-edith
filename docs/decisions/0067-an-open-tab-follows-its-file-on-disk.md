# 0067. An open tab follows its file on disk, and a save never writes over a change it did not see

Date: 2026-10-12 · Status: accepted

**Trigger:** read before changing what a tab does when its file changes on
disk, what `PUT /api/file` checks, or when the page looks at the open files.

## Context

A tab read its file once, when it opened. After a `git pull` in the terminal,
a checkout, or dbt or another editor rewriting the file, the tab showed the
old text until the page was reloaded. Saving it then wrote that old text back
over the pull, which showed up later as a diff undoing someone else's commit.

## Decision

- **Every read of a file carries a stamp**: its size and modification time in
  nanoseconds, as a string, since that overflows a JavaScript number.
  `POST /api/file/stamps` answers with the stamp of each open tab's file, null
  for one that is gone. The page asks with the git poll, when a tab is shown,
  and after a git action from the panel.
- **A tab with no unsaved edit takes the new text in place**, as one replace of
  the stretch that differs, so the cursor and the scroll stay. The change has
  the origin `disk`, which marks nothing edited, and the undo history is
  cleared after it.
- **A tab with edits keeps them.** Its name turns amber and a toast says so.
- **A save sends the stamp its text started from**, and the server refuses with
  409 when the file is no longer that version. The page asks, and on yes sends
  the stamp it showed, empty for a file that was gone, so a second change
  meanwhile is refused too. The server checks and writes one save at a time,
  and a tab's second save waits for its first. Without a stamp it writes, as
  it always did.
- **A deleted file leaves its tab as it was**, struck through: its text may be
  all that is left of it, so saving the tab writes the file back, after
  asking, and closing it asks.
- **Text is compared with its line endings folded to `\n`**, which is what
  CodeMirror hands back. A CRLF file would otherwise reload at every poll.

## Rejected

- **A watcher pushing changes over a WebSocket.** It needs a crate (0003) or a
  walk of the whole tree, to follow the few files that are open, which a stat
  each every few seconds does for nothing.
- **Reading every open file at each poll and comparing.** Up to 4 MB each.
- **Looking only when the window gets focus back.** The pull is usually typed
  in the page's own terminal, and the window never loses focus.
- **Replacing the whole text.** The cursor would land at the end of it.
- **Undo across a reload, as in VS Code.** One Ctrl+Z took the pull back out,
  and the save after it, carrying the pull's stamp, wrote the old text over
  the pull without asking.

## Consequences

A pull typed in the terminal shows within one poll, five seconds at most, or
when the tab is clicked. A diff tab and the profile tab are not followed. A
pull landing in the instant between a save's check and its write is still
written over: git takes no lock the server could wait on.
