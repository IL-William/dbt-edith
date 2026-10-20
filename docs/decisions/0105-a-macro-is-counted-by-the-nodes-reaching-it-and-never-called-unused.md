# 0105. A macro is counted by the nodes reaching it, and never called unused

Date: 2026-10-19 · Status: accepted · Amends 0028 and 0075

**Trigger:** read before changing what a macro's card, its palette row or the
Macros section of Reports says about who calls it, or reading
`depends_on.macros` for anything else.

## Context

Every node and macro carries `depends_on.macros`, which 0028 rejected only to
link a call. dbt records each one's own calls, `adapter.dispatch` resolved
for the manifest's adapter, and a node's are those dbt rendered when it
parsed: a branch on `execute`, `is_incremental()` or `target.name`, a
`{% call %}` body, and under dbt Fusion every hook, are left out. dbt Fusion
also lists a macro reached through another as a direct call. And dbt runs
some macros itself: `generate_schema_name`, materializations, strategy
macros, its own macros a project overrides by name.

## Decision

- **A macro is used by the enabled nodes reaching it**: listing it, through
  a macro that does, walked up breadth first, and only then calling it in
  their SQL or hooks where dbt recorded no call, a method, a filter, a test
  or a `{% raw %}` block being no call. Counted by kind.
- **What dbt runs itself is walked down**: the project's macros it runs by
  name or for the nodes naming them, a materialization only where dbt-core
  would pick it (the nearest the adapter, then the project, a package, dbt,
  a package giving way where dbt has one; two packages tied for it both
  count, since no position is a tie-break: `named()` and the positions come
  from HashMaps, and the manifest answers the same on every start, 0055),
  and its own and its adapter's but the helpers it keeps for projects to
  call, under `macros/utils/`, `tests/generic/` and
  `macros/generic_test_sql/`, unless its own call them, bare, which reaches a project's override. So a helper of
  `generate_schema_name` or a dispatch variant dbt picked says so.
- **The card says it, and its first line opens the list** in Reports'
  Macros section; the palette row says `used by N`. Summaries are worked out
  once per macro and kept with the graph.
- **No macro is ever called unused.** One no node reaches has `no reference
  found` and its likeliest reason: dbt calls it, dbt runs it for N nodes,
  a dispatch variant not picked, a generic test no test uses, only macros no
  node reaches call it, or nothing in the manifest does.
- **The section lists them all, and only what nothing explains lights the
  Reports button**, the user's choice: the last three reasons, or a
  materialization or strategy macro no node names. What dbt runs itself, a
  dispatch variant included, stays listed and never lights it, nor does a
  card's click, which lists the section empty. It carries no news (0066),
  and is opened on rather than an empty coverage, by 0070's rule. Amends
  0075: the button lights for it, and its tooltip and the tour name it. It
  sits after the coverage, so the panel opens on the first section with
  something over all of them, Warnings (0103) included, and falls back on the
  first listed only when every one is empty. Warnings keeps its own chip and
  its own rule; the button is dimmed only while neither has something, nor
  any other section.

## Rejected

- **Counting the nodes listing it alone**: every helper would read uncalled.
- **Saying `unused`**: the manifest cannot see half the ways a macro runs.
- **The list in a menu off the card**: too short for hundreds of tests.

## Consequences

Reasons are read from names: a macro `x__y` beside a macro `y` reads as a
dispatch variant. A call dbt writes as `dbt.x(` (`dbt.string_literal(`, say) reaches dbt's own
`x`, never an override, yet a project macro of that name reads as called by
dbt. A method call `a.y(` in dbt's macros (`config.get(`,
`information_schema.replace(...)`) is recorded as a call of the macro `y` dbt
would resolve, the project's first, so a project macro named like a method
(`get`, `replace`, `split`) reads as called by dbt. Both err toward explained.
Two packages tied for a materialization (`kit` and `kat` both defining
`materialization_fancy_default`) both count, though dbt runs exactly one, the
later in `manifest.macros` order. Both cards say dbt runs it for each model
materialized that way, and whatever only the skipped one calls reads as called
by it, which dbt runs itself: such a macro lands under "Called by dbt itself"
and does not light the Reports button. That was chosen because it is stable
across starts (0055) and errs toward explained. Left for later:
`manifest.json` keeps dbt's macro order as its key order, so reading that
order would give dbt's actual pick deterministically.
"Through other macros" reads differently under dbt Fusion.
