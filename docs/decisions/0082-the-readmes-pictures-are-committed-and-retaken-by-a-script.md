# 0082. The README's pictures are committed, and retaken by a script

Date: 2026-10-14 · Status: accepted

**Trigger:** read before adding a picture to the README or the docs, or
changing where one is kept or how it is taken.

## Context

A README without a picture leaves whoever is deciding whether to install
dbt-edith to imagine it. A picture of an interface goes stale as the
interface changes, and once committed every version of it stays in the
history for good. It may show nothing of a real project: no name, no size, no
figure (0014).

## Decision

- **Taken on Jaffle Shop**, the copy in `tests/fixtures/`, built on duckdb,
  with the invented jobs of `tests/fixtures/jobs/` and an invented Airflow DAG,
  by `scripts/take_shots.py`, which drives headless Chrome with the client
  `walk_tour.py` already has (0074) and the standard library alone.
- **Committed under `docs/shots/`**, as PNG at 1440 by 900, one device pixel
  per CSS pixel, around 150 KB each. A tag's README shows that tag's
  interface.
- **Retaken all at once, when a change alters what one shows**, and not at
  every release: each retake adds its bytes to the history for good.

## Rejected

- **A gist**, as #16 used for its pictures: light, but the README then hangs
  on a gist that can be deleted, and a tag no longer shows its own interface.
- **Images uploaded to an issue or a pull request**: lasting links, but
  nothing versions them and no script can replace them.
- **Twice the pixels**: sharper on a dense screen, several times the bytes at
  every retake, for a picture GitHub shows well under 1440 pixels wide.
- **Taken by hand**: they stop matching the interface the first time nobody
  remembers how one was set up.

## Consequences

The script is not part of `check.sh`, since it writes into the repository. A
change that breaks one of its steps shows the next time it is run.
