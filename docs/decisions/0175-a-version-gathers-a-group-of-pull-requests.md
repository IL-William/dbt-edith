# 0175. A version gathers a group of pull requests

Date: 2026-11-12 · Status: accepted · Amends 0045

**Trigger:** read before changing which pull request bumps the version, who
chooses the versions of a batch, what a pull request adds to CHANGELOG.md, or
what `## Unreleased` holds between releases.

## Context

Every pull request bumped `Cargo.toml`, so every merge was a release. The dated
headings of CHANGELOG.md show it: 16 releases on 11-10 and 11-11 (2.20.1 to
2.22.1), and 7 and 10 on 11-08 and 11-09. A release checks its files on
VirusTotal (0152), whose free key allows 500 requests a day at about 25 a
release, so 16 releases took four fifths of a day's quota. Every pull request
also touched `Cargo.toml` and the root entry of `Cargo.lock`, so each merge
made every pull request still open conflict on them. And a release per fix told
the person reading Edith's menu nothing about which changes belong together.

Nothing forces the bump to ride on every pull request. `release_notes.sh` skips
`## Unreleased` and wants only the newest dated heading to be the version
`Cargo.toml` names; the changelog check wants a line when `src/`, `web/` or
`tools/` change; the release workflow fires on a push that touches `Cargo.toml`
with an untagged version (0045). The update installs tags, so a pull request on
main reaches nobody until a bump releases it.

## Decision

- **A pull request never bumps.** It adds its line under `## Unreleased`, and a
  step of the `changelog` job refuses a line added under a dated heading by a
  pull request that leaves the version as it was. That job is a required check
  of main, with `rust` and `audit`.
- **Whoever merges a batch groups its pull requests into versions:** a large
  feature is a minor version of its own; small fixes share a patch, or take
  several when they are unrelated; an unrelated batch is its own release; a
  major only on the maintainer's word.
- **The bump rides on the last pull request of its group that adds a CHANGELOG
  line**, never a `no changelog` one, added at that one's turn: `Cargo.toml`, the root entry of `Cargo.lock`, and in CHANGELOG.md
  a dated `## <version> - <merge day>` cut from `## Unreleased`, which stays
  above it, empty. Never a pull request of its own. Merging it is still the
  whole release (0045).
- **A batch ends with no line waiting under `## Unreleased`**, so nothing a
  person would notice stays unreleased. A fix needed at once may still bump
  alone.
- **A Shipped paragraph of state.md names its decision and no version**, since
  a pull request no longer knows its own; CHANGELOG.md says which version
  shipped it.

## Rejected

- **A release bot** (release-please and the like). A dependency, and the choice
  of what goes in a version leaves the person who knows the work.
- **A release pull request of its own.** An extra CI cycle, and an empty commit
  in the menu that someone a release behind reads.
- **A release on a timer.** It releases whatever is on main, with no
  grouping, and the version number says nothing about the change.

## Consequences

*Amends 0045*: the bump is no longer in every pull request, but a version on
main with no tag is still released and its section is still the notes.

Only the `Cargo.toml` and `Cargo.lock` conflicts go away. Lines added under
`## Unreleased` meet at the same place, so a pull request rebased on another
keeps both.

Edith's menu shows one row per release, the title of the pull request that
carries the bump (0079), so a group's other titles are not seen there. That is
a known cost. A pull request merged after the last bump waits for the next one;
the update does not install it.
