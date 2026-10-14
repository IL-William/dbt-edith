# 0078. The tour is offered once there is a model to show

Date: 2026-10-14 · Status: accepted · Amends 0074

**Trigger:** read before changing when the tour is offered, what a page with
no model is told, or when `tour.json` is first written.

## Context

0074 offered the tour to the first page to ask on a machine, whatever its
manifest held. A project cloned and opened before any `dbt parse` has no
`manifest.json`, and that is a likely first start for someone handed the
binary. The offer was spent there on a tour of six steps out of thirteen,
leaving out the editor, the lineage, the Catalog and the Jobs tab, and was
never made again: the full tour stayed in the shortcut list, where nobody new
to the tool looks. Its welcome also said "This manifest has no model" when no
manifest had been read at all, and "a model of this project" of a model the
pick took from a package, the project having none of its own.

## Decision

- **The offer waits for a page with a model to show**, the one the tour
  would open (0074's pick). The server decides, from the graph it serves,
  so the page and the tour cannot disagree on whether there is one.
- **A page with no model is told the offer is held, and nothing is written.**
  It asks again each time a manifest lands, from the watcher's reload or
  Reload manifest, so a `dbt parse` typed after opening the page brings the
  offer without a restart. The first page with a model claims it as before,
  by creating `tour.json`, so two tabs or two processes still cannot both be
  told yes.
- **The version is kept from the offer on**, not from the first page served
  (amending 0074). A machine that only ever served pages with no model was
  never offered the tour, and the first one with a model offers all of it,
  not what is new since. Once the offer is made, every page raises the
  version, with a model or without.
- **The welcome says which case it is**: no `manifest.json` to read yet, or a
  manifest with no model; and a model taken from a package names its package,
  saying the project has none of its own.
- **Without a model, the welcome rings the `?` button**, where the tour is
  taken again once `dbt parse` has written one, and names its key. Sending
  someone to "the shortcut list" by name told them nothing about where it is.

## Rejected

- **Keeping the version from the first page, with `offered` at 0 until the
  offer.** The claim would then rewrite a file that exists instead of creating
  one, and two tabs reading it at once could both be told yes.
- **Deciding in the page from its counts.** A model in the counts may be
  disabled or have no file, which the pick leaves out; the server's answer is
  the pick itself.
- **Offering the short tour, then the full one once there is a model.** Two
  offers of the same tour, and the first one shows little.
- **Waiting for the next start to offer it.** Whoever runs `dbt parse` in a
  page just opened is the one the tour is for, and the page already hears of
  the new manifest.

## Consequences

`scripts/walk_tour.py` starts dbt-edith before `dbt parse`: no offer and no
`tour.json` there, the shortcut list's tour saying there is no manifest, then
the offer once the parse lands. A page that never sees a model never offers the
tour, which stays in the shortcut list.
