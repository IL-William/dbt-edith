# 0179. The VirusTotal badge reads the JSON's message

Date: 2026-11-12 · Status: accepted · Amends 0174

**Trigger:** read before changing the README's VirusTotal badge, the service
that draws it, or the colour it shows.

## Context

0174 put shields.io's endpoint badge in the README, pointed at
`releases/latest/download/virustotal.json`. shields.io refuses github.com as an
endpoint host, so the badge drew "custom badge | domain is blocked" in red. It
showed on the README right after 2.23.0, the first release to carry the file.

## Decision

- **The README's badge is shields.io's dynamic JSON badge**
  (`img.shields.io/badge/dynamic/json`), which accepts github.com. It reads the
  `message` of the same `virustotal.json` through the query `$.message`, with
  the label `VirusTotal`, and still links to `releases/latest`.
- **The colour is fixed, blue.** The dynamic badge cannot read the file's
  `color`, so the badge no longer goes from green to red with the flags. The
  count stays in the text (`0/75 · v<version>`), where a flagged file is as
  readable as before.
- **The file is unchanged.** `virustotal.json` keeps the endpoint format and
  its colour, and the release workflow is not touched: the file stays valid for
  an endpoint host later.

## Rejected

- **Keeping the colours by pushing the JSON to a `badges` branch read through
  raw.githubusercontent.com**, a host the endpoint badge accepts. It costs a
  commit per release and gives the release workflow write access to a branch.
- **Dropping the badge.** The release's notes carry the table, but the README is
  where a person deciding to download looks first.

## Consequences

*Amends 0174*: only how the README reads the file changes. The badge no longer
turns orange or red for a flagged file; only a run forced past a flag can
publish one, and its text then shows the count. It still depends on shields.io
being up and on the latest release carrying the file.
