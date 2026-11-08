# 0156. The keys reaching a file are shown as written and struck where dbt replaces them

Date: 2026-11-08 · Status: accepted · Amends 0038

**Trigger:** read before showing where a node's config comes from, striking a
key of `dbt_project.yml`, or changing the dialog under the breadcrumb's violet
button.

## Context

0038 listed the keys of `dbt_project.yml` that reach a file and said "where,
never what wins": a level's name and a summary of what it sets, in a menu.
Reading such a list, the question is always the next one: which of these does
dbt actually keep? A model under `staging:` with `+materialized: view`, whose
project level says `table`, shows both, and nothing says one is dead. The new
design (step 11) shows the lines as the file writes them, with the replaced
ones struck.

## Decision

The dialog shows, for each level `configLevels` returns, the level's key line
and every config set directly under it with all of its own lines, in the file's
order, with their line numbers. Blank and comment-only lines are never shown, outside a block value;
a thin `⋯` marks where a line holding YAML was left out between two shown. The
line of the level naming the folder that holds the file is tagged and tinted.

A line is struck, with the line that replaces it, only where a deeper level of
**this file** replaces it, by dbt's own merge rules. They were read in
dbt-core 1.12.6: `ContextConfig` applies the levels of the resource block root
first, each through `BaseConfig.update_from`, which reads each field's own
merge metadata (`MergeBehavior.from_field`) in
`dbt/artifacts/resources/v1/config.py` and `model.py`. The `mergebehavior` list
of `dbt_common/contracts/config/base.py` is read only for the `config()` calls
of a file, so it is not the rule here; a field with no metadata, as
`snapshot_meta_column_names` is, is replaced whole.

- **Append** (`tags`, `pre-hook`, `post-hook`, `packages`): a deeper level adds;
  never struck. `pre_hook` is `pre-hook`, as `translate_hook_names` makes it.
- **Update** (`meta`, `docs`, `quoting`, `column_types`, `contract`,
  `latest_version_pointer`): a shallow dict update, so a deeper level
  replaces only the sub-keys it writes. The sub-key's lines are struck, the
  config's own line never.
- **Dict key append** (`grants`): the same, but a deeper sub-key written with
  a leading `+` extends the list, so only one written without it strikes.
- **Clobber**, everything else: the deepest level that sets it wins, and the
  upper setting is struck whole, its value lines too.

The struck lines say "replaced on line N", the line of the deepest setting that
replaces them. Two settings of one config at one level, a duplicate YAML key,
are left alone, and so is a value written in flow style (`{node_color: gold}`)
on either side: one line cannot be half struck, and a flow value's keys are not
read, so a deeper one strikes nothing above it. A project that writes both
`data_tests:` and `tests:` is read by dbt through `data_tests:` alone, so the
`tests:` levels are not shown then.

What this does not claim is the config the file ends up with. For a file of the
root project, a `config()` in the file and its properties YAML are applied after
every level (`patch_config_dict` and `config_call_dict`), and are not read here,
so the dialog's footer says they win over these lines. For a file of an
installed package it is the other way round: `calculate_node_config` applies the
package's own levels, its properties YAML and its `config()` first, then the
**root** project's levels, the ones this dialog shows (the
`own_config.project_name != active` branch). The footer then says these lines
win over the package's own `dbt_project.yml`, properties YAML and `config()`.
The strike says only what this file's deeper levels replace.

## Rejected

- **A table of keys, one row per config** (the design's second option). It
  loses the file's own order and its nesting, which is how a reader finds the
  line to edit.
- **The resolved config.** It needs the manifest's `config` and hides where
  each value was written, which 0038 already found; the manifest does not say
  which level a value came from.
- **Striking a flow value's sub-key**, or half a line. Not drawable on a line,
  and needs a flow parser the outline does not have (0003).

## Consequences

`projectPathKeys` carries where each level and each of its configs sits, and
`configExcerpt` reads the lines they own off the outline as plain data a test
reaches without a DOM. A
config dbt adds with a merge behaviour of its own is treated as a clobber until
`MERGE_*` in `web/app.js` names it. The dialog is placed from the editor pane's
rectangle, so it stays inside the editor.
