# 0045. Merging a version bump releases it

Date: 2026-10-07 · Status: accepted

**Trigger:** read before changing how a release is started, what makes the tag,
what a release's notes say, or the headings of CHANGELOG.md.

## Context

A release took four steps by hand: a pull request bumping `Cargo.toml` and
turning Unreleased into the version's section of CHANGELOG.md, then, after its
merge, a signed tag pushed and a GitHub release written as a prose summary of
that section. Nothing checked one step against another, and the steps after the
merge are the ones that wait: one version was bumped on main and never tagged.

## Decision

- **A version on main with no tag is released.** `release.yml` runs on every
  push to main that touches `Cargo.toml`, reads the version, and when
  `v<version>` does not exist yet creates the tag and the release "Edith
  <version>" together, at the commit pushed. A release is the pull request
  that bumps the version, and nothing after its merge.
- **The notes are the version's section of CHANGELOG.md**, as
  `scripts/release_notes.sh` prints it, with one line saying no binary is
  attached. The changelog is already written entry by entry as the work lands;
  a summary written again at release time was a second copy of it.
- **The newest dated section of CHANGELOG.md is the version `Cargo.toml`
  names**, checked by that script in the `rust` job, which main requires. A
  bump without its section, or a section without its bump, does not land.
- **Still nothing is built.** A release stays the source (state.md says why).

## Rejected

- **The tag pushed by hand, kept as the trigger.** It is the step that gets
  forgotten, and a release then still has to be written after it.
- **GitHub's generated notes**, as collin uses. They list pull request titles,
  where the changelog says what changed for the person using it.
- **A release bot** (release-please and the like). It writes the bump from
  commit prefixes this repository does not use, and it is a third party action
  with write access to tags, which cannot be moved or deleted here.

## Consequences

The tag is no longer signed by hand, and it is lightweight. What vouches for a
release is the reviewed commit on main it points at. `build.rs` already
describes with `--tags`, which sees a lightweight tag, so the build stamp is
unchanged.

A version's date is written in its bump and stands for the day it merges: a
bump left open past that day has its date moved before it lands. A release
that fails leaves its version untagged, and `workflow_dispatch` on main starts
it again.
