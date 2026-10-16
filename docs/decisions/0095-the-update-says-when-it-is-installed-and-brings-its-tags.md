# 0095. The update says when it is installed, and brings its tags

Date: 2026-10-16 · Status: accepted · Amends 0068, 0079

**Trigger:** read before changing what the update types, what Edith's menu
offers once it has run, or how the build stamp names a release.

## Context

The first update run from Edith's menu on the Windows VM, 1.25.0 to 1.28.0,
worked, and three things in it read wrong to the person running it:

- The update was a row like the menu's other rows, under a heading, and read
  as one: nothing said it was the button. What it does, and that Enter has to
  be pressed in the terminal, came three lines and the whole release list
  below it.
- Once cargo had installed the new binary, the menu and the pill still offered
  the update, since the page is served by the old one until it restarts.
  Nothing said the install was done and the restart was left, and the command
  run again would stop on renaming the running `.exe` over the `.old` it now
  runs from.
- After the restart, the build stamp read `v1.25.0-9-gb32671a`: the pull names
  `origin main`, and a pull naming a branch fetches no tag, so `git describe`
  in the clone named the last release the clone had heard of, not the one it
  built.

## Decision

- **The update is drawn as the app's primary button**, named after the
  release, `Update to 1.28.0`, and right under it is what it does: types the
  update into the terminal below, Enter there runs it, then dbt-edith is
  stopped where it was started and started again. How it goes (the clone it
  pulls, the Windows rename) stays below the release list.
- **The server watches its own binary's file.** At start it notes the time the
  file at its path was last written, and each answer to the page says whether
  that time has changed since: another binary sits there, an update
  installed. Only that time is read, never the file, as only which files
  exist tells how it was installed (0068). A path left empty, while the update
  has renamed the running `.exe` on Windows and cargo is still building, is not
  one, and neither is the `.exe` renamed back when the build fails, which
  keeps its time.
- **Once installed, the menu says so and offers no command**: `Installed:
  restart dbt-edith to run it`, and the version still serving the page. The
  pill reads `restart for 1.28.0`. The page asks every five seconds for half
  an hour after the button is pressed, and whenever the menu opens, so it says
  so soon after cargo is done, and after an update run by hand too.
- **The pull brings the tags**: `git pull --ff-only --tags origin main`. The
  build stamp names the release it builds. A clone holding a tag the
  repository moved has a `main` that cannot fast-forward either, so the
  command stops there as it did.

## Rejected

- **Pressing Enter for the user.** The update builds for minutes and replaces
  the binary: 0068 keeps it a command the user runs.
- **Running the new binary to learn its version.** It would run a file the
  server did not start from, for one word of the menu; that it is newer than
  the one running is all the restart needs.
- **Reading the version from the clone's `Cargo.toml`.** It says what the
  clone holds, not what was installed, and reads inside a file outside the
  project, which 0068 avoids.
- **An `echo` at the end of the command.** It would land in the terminal the
  update ran in, written for two shells, and say nothing to the menu or the
  pill, which are what was misread.
- **`git describe --match` on the release `Cargo.toml` names.** It hides the
  tags a clone lacks rather than bringing them.

## Consequences

An update left half done, the `.exe` renamed and cargo stopped, still shows
the button, and the command run again stops on the rename. The binary's time
is read on every answer to the page, a few times a minute at most.
