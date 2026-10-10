# 0047. collin's numbers restart at 0.1.0

Date: 2026-10-07 · Status: accepted

**Trigger:** read before changing `WANTS` in `src/collin_run.rs`, or what the
menu decides from the version collin gives.

## Context

0043 has the menu offer "Update collin" to a collin older than the one this
dbt-edith wants, which it set at 0.2.0, the first collin to answer
`--version`. collin's repository was then started again under FSL-1.1-MIT,
like this one (0046), and its numbers with it: the first release of the new
repository is 0.1.0. Asking for 0.2.0 would offer every collin from it an
update that reinstalls the same version.

## Decision

- **dbt-edith wants collin 0.1.0 or later.** Every collin from the new
  repository answers `--version`, and so did the old repository's 0.2.0, the
  last it published; no collin before that answers at all. A collin giving no
  version is still the one to update, which is the line 0043 drew.
- **The old 0.2.0 passes.** It reads compiled files and writes the report this
  dbt-edith reads, as the new 0.1.0 does, so nothing tells them apart that
  matters here.

## Rejected

- **Releasing the new collin as 0.2.0 or later**, so nothing here moves. The
  old repository published an MIT 0.2.0, and a second, different 0.2.0 under
  another license would make a version name two things.

## Consequences

The version stops being a promise that it only rises: an old 0.2.0 compares
above a new 0.1.0. Nothing here relies on the order between them, only on a
version being given at all, until `WANTS` rises past 0.2.0 again.
