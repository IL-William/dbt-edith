# 0070. A source is picked from folders the server lists

Date: 2026-10-12 · Status: accepted, amended by 0075 · Amends 0063 and 0064

**Trigger:** read before listing a folder outside the project, changing what
the sources form's picker shows or how it writes a path, or listing a report
section that has nothing to show.

## Context

A job source was a path typed by hand (0063), usually into another
repository, and a typo was only found by saving. A browser cannot help: its
file dialog hands a page a file's contents, never where it sits
(`C:\fakepath\`), and its folder dialog uploads every file under the folder.
A source could only be added from the Jobs tab or the menu, not from Reports,
where the coverage is read.

## Decision

- **The server lists one folder at a time** for the picker under a source's
  path: `GET /api/jobs/browse`, located as a source's path is. What leaves is
  what a walk of it would read: the names of the folders it would enter and
  of its `.yml` and `.yaml` files, never a hidden entry, a symlink or a
  junction, never a line of a file. Folders first, then by name (0055), at
  most 2000, past which the file the path names stays listed.
- **A path the picker would not stand on lists the nearest folder above it**:
  one that leads nowhere, into a hidden folder or onto a link. The folders the
  project sits in are taken as they are, hidden or not, and a link further up
  a typed path is followed, as a source's own path is: `/tmp` is one on
  macOS. A folder the system will not list is shown empty, with the reason,
  and left by its up button.
- **That is anywhere the user can read**, as 0063's own path is, for 0063's
  reason: the key a page needs (0052) already gives it a shell.
- **A pick is written from the project** when it sits at most two folders
  above it, as a repository beside the project's does, and in full otherwise.
- **A source being added opens with its picker**, from the form's buttons and
  from Add a source.
- **The coverage is listed in Reports before any source is named**, as where
  one is added, amending 0064's rule for it alone. Empty, it neither shows the
  button nor is opened on while another section has something.
  *Amended by [0075](0075-the-reports-button-stays-in-view-dimmed-while-it-has-nothing.md):
  the button is shown all the same, dimmed.*

## Rejected

- **A native dialog started by the server** (PowerShell's folder browser,
  `osascript`): a program run on a VM where scripts may be blocked, a window
  that can open behind the browser, and a dialog picks a folder or a file,
  where an Airflow source is either.
- **Every name in the folder.** The others are not the form's business, and
  a `.env` stays out of sight as the walk keeps it out.
- **Windows drives above `C:\`.** Probing each letter can hang on a
  disconnected network drive; the picker's own path field takes `D:\`.

## Consequences

The page can learn the names of folders anywhere the user can, as the
terminal already could.
