# 0144. The pictures also illustrate the feature pages, and are retaken by name

Date: 2026-11-04 · Status: accepted · Amends 0082

**Trigger:** read before adding a picture to a page under `docs/features/`,
retaking one, or changing what `scripts/take_shots.py` takes or copies.

## Context

0082 gave the README six pictures and had them all retaken when one changed.
The pages under `docs/features/` say what each part does in words, and a
feature that is a screen is read faster beside the screen. Twenty-four
pictures at 150 KB each, all retaken at every change, would add around 3.5 MB
to the history each time, and a change to one screen would rewrite twenty-three
that had not moved.

## Decision

- **A feature page carries the pictures of the screens it describes**, as
  `![alt](../shots/<name>.png)`, each beside the text it illustrates: a
  section may hold several, and one may sit in the middle of a section, where
  its feature is described. The alt text is the README's kind: one sentence
  saying what is on the picture. No caption besides it. `command-line.md` and
  `look.md` have none. The README keeps its six.
- **`scripts/take_shots.py` always takes them all**, 24 in all, on Jaffle Shop
  as in 0082, with invented jobs, invented releases and a fake Snowflake
  connector: no warehouse is reached and GitHub is not asked. The update
  picture is taken from a copy of the binary built from a clone kept under
  `/tmp` (its own folder of `TAKE_SHOTS_CACHE`, marked, and locked while it
  builds), so that the page offers its command and shows only temporary paths.
  That clone and its debug build take about 1 GB of `/tmp` and a few minutes
  the first time and whenever the sources change; the rest of a run is
  minutes of browser time and writes nothing outside `/tmp`.
- **It copies only the names it is given** into `docs/shots/`, or all of them
  when it is given none. A change that alters one screen commits that picture
  alone, and the history grows by it alone. The rest of the set is left as it
  was.
- **`--scale 2 --out DIR`** makes a sharper set outside the repository, for
  design work. It is never committed: a scale above 1 without `--out` is
  refused, so a double-size picture cannot land in `docs/shots/`.
- **0082's "retaken all at once" no longer holds**, and so no longer does the
  line in AGENTS.md that said it: a change retakes the pictures whose screen it
  altered.

## Rejected

- **Retaking all of them at every change**: about 3.5 MB to the history each
  time, most of it pictures that did not change.
- **One picture per page only**: the pages hold several features each, and one
  picture covers one of them. One per feature was chosen.

## Consequences

The script is still not part of `check.sh`, since it writes into the
repository. Which names to copy is the author's call, from what the change
touched; a picture left out of the copy is stale until someone names it.
