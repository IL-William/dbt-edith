# 0031. A named selector is read from the manifest and resolved whole

Date: 2026-09-29 · Status: accepted · Amends 0024

**Trigger:** read before changing how `--selector` resolves, how a criterion
brings its tests along, or what the tests checkbox does in the Selection mode.

## Context

0024 left `selectors.yml` unread. dbt parses that file into the manifest, under
`selectors`, and not always alike. dbt-core 1.13, and dbt Fusion since a fix
of 2026-08-18, keep `method: selector` as a reference; dbt-core 1.11 writes it
out in place. Both keep each criterion's `indirect_selection` and write
`exclude` as a list. Older Fusion, which wrote the manifest this tool is
measured on, writes a reference out in place, an exclusion as one expression,
and drops `indirect_selection`. There, 84 of the 86 selectors keep only tests,
and which tests is what `buildable` and `empty` decide.

## Decision

`src/selectors.rs` reads each of these into the tree dbt builds, and
`src/select.rs` resolves it as `select_nodes_recursively` does: each criterion
brings its own tests, by its own mode, before the sets are combined. Typed
expressions go through the same engine, which ends 0024's "applied once at the
end" divergence, and `+` and `@` walk into tests, as dbt's graph does. A
selector is resolved whole or refused whole, with the reason: `state:`,
`result:` and `source_status:`, a reference to nothing, and a cycle. When the
manifest carries no `indirect_selection` but `selectors.yml` sets one outside a
comment, the answer says so; that one word is all the file is read for.

For a named selector the tests checkbox filters the drawing, never the answer.
On, each selected test is drawn with the models it belongs to, dimmed when the
selector did not select them. Counts, Copy and dbt ls stay dbt's answer.

## Rejected

- **A scanner for `selectors.yml`** (0018). dbt already stored the normalised
  tree; a scanner would re-derive dbt's normalisation, Jinja included, and
  disagree with it in places no test would find.
- **Running `dbt ls --selector`** (0002, 0024): seconds per click, a profile.
- **Dropping a criterion this build cannot read.** The rest would look right.
- **The checkbox changing the resolution**, as for a typed line. A selector of
  tests alone would come out empty and disagree with dbt ls.

## Consequences

On an older Fusion manifest every criterion answers eagerly, and the note is
the only sign. A reference written out in place has lost its own `+`, as it
has in that dbt, so the answer is still the one that dbt gives.
