# 0129. The update brings the clone up to the tag it announced

Date: 2026-10-30 · Status: accepted · Amends 0095

**Trigger:** read before changing the git step of the command the update types,
or where on the network the version in that command comes from.

## Context

The command the menu types for a binary built from a clone fetched the tags,
then ran `git pull --ff-only origin main` (0095), and built what it held. A
release is tagged at the commit that bumped the version (0045), and `main`
keeps moving after it, so a commit merged after the release was built and
installed under the release's number, the build stamp being the only thing that
told the two apart. The menu announces a release; the person was getting
something newer, untagged, and possibly not yet meant to ship.

## Decision

- **After the tags are fetched, the clone is fast-forwarded to the announced
  tag and no further**: `git -C "<clone>" merge --ff-only vX.Y.Z`, in place of
  the pull. Release tags are lightweight (`git cat-file -t` names a commit), so
  the merge names a commit. It works whatever the clone has out, checked with
  git 2.54 on a scratch repository: on `main` or another branch the branch
  moves and stays the branch out, so a plain `git pull` later still works; on
  a detached HEAD, HEAD moves; a clone already at the tag or past it prints
  "Already up to date" and exits 0, so it builds what it holds; a clone with a
  commit the tag does not contain cannot fast-forward, the merge fails, and the
  command stops there, as the pull did.
- **The version is the one the check found, put into the command for each
  answer.** The command used to be made once at start; a clone's `How` now keeps
  the parts that do not change (`update::Plan`) and `How::for_release` composes
  the line for the release `/api/update` announces. The version comes off the
  network (or the kept answer in the config directory), so it enters a typed
  line only if it is three runs of at most five ASCII digits, the check the
  collin versions already use (`collin_run::is_version`). Anything else, or
  none, gets no clone command and the reason `release`, which the menu says. A
  Windows download beside it is still offered: it names no version, and
  `releases/latest` stays as 0123 has it.
- **0111 and 0120 hold as they were**: no quote, `$` or backtick in what is
  printed, `DONE` only after a working install, the clean after the install and
  outside the restore.

## Rejected

- **`git checkout vX.Y.Z`.** It leaves the clone on a detached HEAD, where a
  plain `git pull`, which someone who stays on `main` will type next, fails with
  "You are not currently on a branch".
- **Keeping the pull of `main` and relying on the build stamp.** The stamp tells
  the build apart from the release but does not stop the build from being one.
- **Composing the command once, with `latest` resolved by git in the shell.**
  Tags would have to be sorted by version in a typed line, in two shells.

## Consequences

The command is typed by the running binary, so the first update after this
release still types the pull of `main` (0095's trap): the clone it updates is
brought up to `main`, which holds this release, by the old command, and the next
update is the first to stop at a tag.

A clone that is ahead of the tag, with commits of its own on top of `main`,
builds what it holds, as it did before, and is told nothing: "Already up to
date" scrolls past in the terminal. A tag that exists on the announcing
repository but not in a fork the clone's `origin` points at stops the command
at the merge, which names the tag it could not find.
