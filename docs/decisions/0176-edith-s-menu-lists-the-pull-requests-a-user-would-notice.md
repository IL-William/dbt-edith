# 0176. Edith's menu lists the pull requests a user would notice

Date: 2026-11-12 · Status: accepted · Amends 0079

**Trigger:** read before changing what Edith's menu or the Updates section
lists for a release, or what a look fetches from git to list it.

## Context

0079 listed one row per release: the title of the pull request whose commit
carried the tag, which was the release, since every pull request bumped the
version. Since 0175 a release holds several pull requests, gathered under one
version, the last one carrying the bump, so 0079's "a pull request is a
release" no longer holds. A row with the last one's title then says nothing of the
others, which someone behind by a release would want to read.

## Decision

- **A row for each pull request a release holds that changed
  `CHANGELOG.md`**, which is what someone using the tool would notice and what
  AGENTS.md asks each such change to touch. The first row has the version and
  the first change's title; the others have no version, one under the other,
  newest first, each linking to the release's notes. At most eight per release,
  then a row `+N more` linking the notes, and forty rows of changes in all:
  past them each further release shows its first title and the count of the
  rest, up to two rows. A
  release none of whose commits touched the file shows the subject of its
  tagged commit, so none is empty, and a release whose changes could not be
  listed keeps its one row, as in 0079.
- **The changes are the first-parent commits from a release's tag down to the
  next older tag, exclusive**, each a squashed pull request whose subject is its
  title, as in 0079 (`ReleaseBody.changes`, `title` staying the first change),
  kept when it changed `CHANGELOG.md` compared with its first parent.
- **They are fetched in one request, as 0079 fetched the titles**: the tags of
  a run of consecutive releases, from the newest one lacking its changes to the
  oldest, and the release just older than the run (the base), with
  `--filter=blob:none` and `--shallow-exclude` the tag older than that. GitHub
  answers with the commits since it and their trees, no file; two logs name
  the first-parent line and the commits on it that touched the file, and each
  goes to the nearest tag at or above it. The exclusion is one tag below the
  base because the first commit past an exclusion is a shallow root, with no
  parent to compare with, and a path-limited log counts a root as touching
  every path: with the base's commit in the fetch, every commit above it has
  its real parent. The walk must reach the base's commit on the first-parent
  line, or it is not the base's descendant and the answer is refused. The
  releases of the run that already have their changes are asked again, since a
  commit's release is only known with every tag between.
- **Kept with the answer**, in `update.json`, by version, beside the titles,
  which stay: a file an older dbt-edith wrote still reads, and an older one
  reads past `changes`.
- **The fallback is 0079's fetch**, one subject per tagged commit, used when
  there are not two releases older than the run to stop at, when git fails, or
  when a tag or the base is not on the newest one's first-parent line (the
  release above it would take its changes). A fetch that failed is not made
  again by a look on demand for the same run of releases, whose titles fill the
  menu; the next scheduled look, or another release, tries again.
- **Through git, not GitHub's API**, for the reason 0079 gives: the look
  already goes through the user's own git configuration, proxy included, and no
  HTTP client comes in (0003).

## Rejected

- **The CHANGELOG's bullets**, rejected by 0079 already: a file to fetch and
  prose rather than titles.
- **`--shallow-since` the base tag's date.** It needs the date, so a fetch of
  its own, and a commit made before it on another branch would come too.
- **One fetch per release.** Twenty requests where one will do.
- **Every pull request, whatever it changed.** The ones that touched no user's
  screen (a refactor, a test, a document) fill a release's rows without being
  read by anyone it is for.
- **`--filter=tree:0`**, 0079's, which is smaller but has no trees to tell which
  commit touched the file.

## Consequences

A release with many pull requests makes a longer menu, held to eight rows each
and forty of changes, then two for each further release; the rest is one click
away in its notes. A pull request that leaves CHANGELOG.md alone is not listed, whatever its title says. Old binaries keep
one row per release, the last pull request's title. A look that has to list
changes asks for the commits and trees since two releases below the run, as one
request, a few tens of kilobytes more than 0079's titles.
