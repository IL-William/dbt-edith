# 0178. The site is written inside the project, into a folder Edith marks

Date: 2026-11-12 · Status: accepted

**Trigger:** read before changing where `--site` writes, what it empties, or
adding a way to write anywhere else.

## Context

`--site` is the first mode that writes a folder of its own size, and it empties
the folder before it writes, or the cards of an earlier run would sit beside
this one's. A command that deletes what is in a folder it was handed is one typo
from deleting a project: `--site .`, `--site ..`, `--site models`. The rule that
nothing outside the project is written (AGENTS.md) holds here too.

## Decision

- **The folder is strictly inside the project, never the project itself, and
  either empty or carrying the marker `.dbt-edith-site`.** The default is
  `<project>/target/edith-site/`. Anything else is refused with the reason, exit
  2, and nothing is touched.
- **A marked folder is emptied and rewritten, marker first**, so a run that stops
  half way leaves a folder the next run still knows as its own. A link inside it
  is removed, not followed.
- **The place is judged by where it really is.** The nearest folder that exists
  is resolved, so a link on the way that leads out of the project is refused; a
  link at the folder itself is refused; a `..` and a `.git` in the path are
  refused outright.
- **The manifest is read, and the data built, before the folder is touched.**
- **Writing outside the project is not offered.** A pipeline that wants the site
  elsewhere moves the folder after the run.

## Rejected

- **Any folder the user names.** `--site .` would empty the project.
- **A `--force`.** The marker is the force: a folder Edith wrote is Edith's to
  rewrite, and nothing else is, with no flag to say otherwise.
- **Writing the files over what is there.** A bucket of an earlier run, with
  more nodes, would stay behind and be loaded by no one, and a stale `index.js`
  could outlive a failed run.
- **Writing to a temporary folder and swapping it in.** A rename across file
  systems is a copy, and it leaves two folders of the site's size on disk.
- **Refusing every folder that exists.** The second run, in a pipeline, would
  fail.
