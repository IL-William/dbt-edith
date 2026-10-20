# 0098. collin is wanted at 0.8.0, and a version installed by tag is never called older

Date: 2026-10-19 · Status: accepted, amended by 0109 · Amends 0043 and 0047 · Follows 0096

**Trigger:** read before changing `WANTS` in `src/collin_run.rs`, or what the
menu and the Columns tab say about the version of the collin that runs.

## Context

0096 badges a column `generated` from the `roots` collin 0.8.0 lists in its
cache, and `WANTS` still asked for 0.1.0 (0047), so nothing offered that
collin: an older one, the old repository's 0.2.0 included, got "Check for a
newer collin" and never drew the badge. At 0.1.0 no collin giving a version
was older. Raised, the server would have called older a version installed by
its tag too, and the Columns tab would have said "the column lineage menu
updates it" over a menu that has no update line for such a version (0060).

## Decision

- **dbt-edith wants collin 0.8.0 or later**, the first to list its roots. The
  old repository's 0.2.0 compares below it again, as 0047 said it would, and
  gets "Update collin".
  *Amended by [0109](0109-collin-is-given-the-project-only-for-a-manifest-lying-in-it.md):
  collin 0.10.0 or later, the first to read a seed's header.*
- **A version installed by its tag is never called older**, whatever its
  number: neither the update line nor the Columns tab speaks of an update
  while one runs. It was picked on purpose, the menu says it runs and lists
  the default above it, and the hint's place keeps the alpha caveat.
- **The server decides**, in one pure function, `collin_run::update_offered`,
  over the collin that runs and the versions installed by tag, so the hint,
  the update line and any later reader of `outdated` agree.

## Rejected

- **Keeping the old 0.2.0 passing**: it writes no roots, so the badge never
  shows under it.
- **A hint "older than 0.8.0" for a version installed by tag**: a sentence
  with nothing to do, on every model's Columns tab, in place of the caveat.
- **Offering such a version the update**: its tag has nothing newer, and the
  command reaches the default one (0060).
- **Hiding the hint in the page alone**: the server's field would still say
  older, and its next reader would say it again.

## Consequences

A collin older than 0.8.0, a copied `collin.exe` included, shows the update
until it is replaced, and where cargo cannot reach GitHub the command typed
fails: a newer copy is the remedy there. A version installed by tag older than
0.8.0 draws no `generated` badge, and nothing says why.
