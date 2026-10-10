# 0061. Each collin version keeps its last run, to compare with another's

Date: 2026-10-10 · Status: accepted · Follows 0059 and 0060

**Trigger:** read before changing what a collin run leaves under
`target/collin/`, or what the comparison of two versions counts.

## Context

With versions side by side (0060), a switch writes Collin's cache and report
again, and what the previous version read is gone. Telling whether a version
gained or lost lineage on this project meant noting the figures by hand before
switching.

## Decision

- **After a run, its cache and report are copied to `target/collin/<X.Y.Z>/`**,
  the version the running collin gives. Each version keeps its latest run only.
  Copied, not linked: collin run by hand writes its files in place, and a link
  would let that rewrite what was kept.
- **Two kept runs compare, B against A**, on the server: every counter of the
  two reports, the entries per kind of finding as the report page files them
  (0059), the edges by role and by origin, the edges only one side has or with
  another role on each, compared by node and column without case, and the
  models where the most moved, with how each side read them. A model a report
  leaves out is clean, so parsed.
- **A counter a report lacks is shown as absent, not zero**: an older collin did
  not count it. A role, an origin or a kind with no entry is a zero.
- **The comparison says what moved, never which side is right.** A run older
  than the manifest says so, since the project moved under it too.

## Rejected

- **Hard links.** No copy, and wrong the moment anything writes in place.
- **Keeping every run.** A run of a large project is tens of megabytes, and the
  latest of each version is what a comparison wants.
- **Comparing in the page.** Two caches of 130 000 edges each are a lot to send
  for a few hundred lines that moved.
- **Keeping them outside the project.** A run belongs to a manifest, and dbt's
  own `target/` is where its outputs already go.

## Consequences

`dbt clean` removes the kept runs with the rest of `target/`. On Fivetran's
Shopify package, 0.1.0 against 0.4.0: 4875 and 5548 edges, B has 703 A lacks
and A has 30 B lacks, 600 read with another role, the 630 inferred edges gone,
23 models moved, 16 from inferred and 7 from unresolved to parsed, and columns
with an edge from 4479 to 4996, as collin's own pull request for that change
measured.
