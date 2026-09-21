# 0024. A selector expression is resolved from the manifest, never by running dbt

Date: 2026-09-19 · Status: accepted · Follows 0002

**Trigger:** read before changing how the Selection box matches, before adding a
selector method, and before reaching for `dbt ls` to answer anything.

## Context

The Lineage tab takes a dbt node-selection expression, `stg_customers
dim_customers+ fct_orders`, and draws what it matches. The obvious
implementation is to run `dbt ls --select "<expr>" --output json` in the
project's venv and draw the answer: dbt is right there, and its answer is
correct by construction.

## Decision

The expression is parsed and resolved in `src/select.rs`, against the graph
already built from `target/manifest.json`. dbt is not started.

Three reasons, in order. `dbt ls` re-parses the whole project, which is tens of
seconds on the project this tool was built for and cannot sit behind a
keystroke; resolving it here answers a 5 700 node selection in under ten
milliseconds. It needs a profile, and on some adapters a warehouse connection,
to do it. And 0002 already settled that this binary never runs dbt: dbt runs
where the user runs it, under their credentials.

The engine is dbt-core's `graph/selector_methods.py`, re-implemented:
`is_selected_node` for the implicit fqn method, tried again without the package
the way `QualifiedNameSelectorMethod` does, an fnmatch-style glob written by
hand (0003), `@`, `+` and `N+` combined as `collect_specified_neighbors`
combines them, whitespace as union, comma as intersection and `--exclude` as a
difference.

## Rejected

- **Shelling out to `dbt ls`.** Correct by construction, and unusable: seconds
  per keystroke, a profile required, and 0002 forbids it for reasons that have
  not changed.
- **Reading `target/graph.gpickle` or `partial_parse.msgpack`.** A Python pickle
  needs Python to read it, and neither file is a format dbt promises to keep.
- **Matching exact names only.** `dim_customers+` is the reason anyone
  types a selector at all. Without the operators the box is a worse search field
  than the one already in the sidebar.
- **A regex crate for the glob.** 0003. The matcher is forty lines, and `*` with
  `?` is all any selector in this project uses.

## Consequences

**The fidelity risk is ours.** A dbt release that adds a selector method makes
this box wrong for that method, and a wrong answer looks exactly like a right
one. Two things hold that down, and only one is code: an unsupported method is a
`400` naming what is supported, never a silent pass that would quietly draw the
whole project for `state:modified`; and the **dbt ls** button types the command
that should print the same list into the terminal tab, without running it, so
any disagreement is one Enter away from being visible.

The known divergences are listed in `src/select.rs`'s header and beside the code
that causes them. They are: `path:` matches the path the manifest recorded
rather than globbing the working directory; the glob is `*` and `?` only, with
`[seq]` literal, and case sensitive on every platform where Python folds case on
Windows; hooks are not in the universe at all; tests take part only when the
tests checkbox is on; indirect test selection is applied once at the end rather
than per union component, which differs only when one parent of a test is
excluded and another is not; and a versioned model's `v` suffix is not special
cased, this project having no versioned models.

Named selectors from `selectors.yml` are not supported. That file is one this
tool does not read, and resolving it is the next piece of this work.
