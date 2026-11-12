# 0174. A release shows its VirusTotal result

Date: 2026-11-12 · Status: accepted · Amends 0152 · amended by 0179

**Trigger:** read before changing what the `virustotal` job hands to the
release, the table in a release's notes, the `virustotal.json` asset, or the
README's VirusTotal badge.

## Context

0152 scans the two Windows files before a release is published, but the result
lived only in the run's summary, which few people open and which expires. The
README cannot carry the number itself: the scan runs after the merge, so a
commit with the result would be a second commit per release, pushed past the
branch protection.

## Decision

- **The job writes what it read as two files** before any flag can stop it,
  `virustotal.md` and `virustotal.json`, and uploads them as the artifact
  `virustotal`. A run blocked by a flag uploads nothing, as it publishes
  nothing; one forced past a flag uploads the flagged result.
- **`virustotal.md` ends the release's notes**: a sentence, the table of the
  run's summary (file, SHA-256, flagged over engines, link to the report), and
  for a flagged file the verdict of each engine that flagged it.
- **`virustotal.json` is attached to the release**, in shields.io's endpoint
  format: `{"schemaVersion":1,"label":"VirusTotal","message":"0/75 · v<version>",
  "color":...}`, built with `jq --arg` and never by concatenation. It holds the
  file with the most flags. The colour is `brightgreen` for none, `yellow` for
  one, `orange` for two and `red` for three or more; only a run forced past a
  flag can publish a non-zero one. It is not attested: it is not a binary anyone
  runs.
- **The README's badge reads `releases/latest/download/virustotal.json`**
  through shields.io's endpoint badge. GitHub redirects that URL to the newest
  release's file, so the badge follows each release with no commit.

## Rejected

- **A bot committing the badge to the README on main.** It pushes past the
  branch protection and adds one commit per release.
- **A static "scanned" badge.** It would carry no number.
- **An SVG attached to the release.** GitHub's image proxy wants an image
  content type, and a release asset is served as `application/octet-stream`.

## Consequences

*Amends 0152*: the check is unchanged, only its result is kept where people
read it. The badge depends on shields.io being up and on the latest release
carrying the file. It shows the scan at publication, never a later re-scan of
the same bytes, and the releases published before this change have no file.
