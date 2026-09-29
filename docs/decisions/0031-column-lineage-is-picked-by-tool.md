# 0031. Column lineage is picked by tool, and Snowflake sits behind its own setting

Date: 2026-09-29 · Status: accepted · Amends 0016, 0017 and 0021

**Trigger:** read before adding a source of column lineage, adding anything
that only makes sense on Snowflake, or changing what the top bar's column
lineage menu does.

## Context

0021 let the user pick among the cache files beside the manifest, and 0016 put
a switch for Snowflake beside that choice. Neither ever worked in the browser:
the page never read the list of caches `/api/meta` sent, and picking one called
a helper that did not exist. The only place the source was named was a read-only
chip in the top bar, so a project with collin's cache showed "collin" and
offered no way to change it.

Snowflake was also offered to every project, including ones on another
warehouse, where its switch, its profile link and its error messages are noise.
More features that only mean something on Snowflake are coming.

## Decision

- **The menu offers tools, not files: Fusion, Collin, Snowflake.** A cache
  belongs to a tool by the producer its header names, never by its file name,
  so a synthetic cache cannot pass for the warehouse's answer (0008). A tool
  stands for its newest cache. A tool with no cache is shown greyed, with the
  file it lacks in its tooltip. Every other file, an older one of a tool's
  included, stays pickable under "other caches", as 0021 requires.
- **It lives in the top bar**, where the read-only chip was, and again in the
  Columns tab. Each tool has its own colour, on its own attribute rather than
  the environments' `data-tone`, and the tool is always named beside it.
- **Picking Snowflake is the switch.** One request, `POST /api/collineage/source`,
  sets which cache the graph holds and whether a click fetches, so the two
  cannot disagree. Snowflake with no cache yet loads nothing, rather than
  leaving another tool's edges under Snowflake's name. `POST /api/sidecar` is
  gone. A fetch adds to the Snowflake cache on screen, so a `dump` being read is
  not hidden behind a new file holding one column (amends 0021's "its own file").
- **Snowflake's features are one setting per project**, under a settings menu.
  Until the user chooses, it follows the manifest's `adapter_type`. Off,
  Snowflake is not offered at all: not its tool, not a cache it wrote, not the
  profile link, and the server refuses its routes, `/api/profiles` included
  (amends 0017: the exception exists only while it has a use). The gate is
  checked on every request, because a reloaded manifest can name another
  adapter.

## Rejected

- **Greying Snowflake when its features are off.** Still offering it is exactly
  what the setting exists to stop.
- **Mapping tools by file name.** The name is the one thing about a cache
  anybody can change without changing what is in it.
- **Keeping the Snowflake switch apart from the menu.** Two controls for one
  choice is how the graph ended up holding collin's edges with Snowflake on.
- **A Fusion producer now.** Its index is parquet under `target/index/`, which
  needs a reader this binary does not have (0003). The entry waits, greyed, for
  a `column_lineage.fusion.json` written by something else.

## Consequences

A project on Snowflake behaves as before until someone turns the setting off.
Anything new that only works on Snowflake checks the same gate,
`AppState::snowflake_allowed`, and adds its row's description to the settings
menu rather than a switch of its own.
