# 0116. The tool's own checks are listed apart from dbt's warnings

Date: 2026-10-22 · Status: accepted · Builds on 0103 and 0105

**Trigger:** read before adding a rule to the Checks section of Reports,
changing which nodes one of its four looks at, or reading a source's
`freshness` for anything else.

## Context

Warnings (0103) says what dbt itself would warn of as it parses, and keeps
that meaning: a reader who sees a line there can find dbt saying it. Four
things worth a look in a project are warned of by dbt in no command: a source
with no freshness threshold, a model nothing tests, a model with no
description, a source no node reads. They are rules of this tool, and the
names of dbt-project-evaluator's `fct_sources_without_freshness`,
`fct_unused_sources` and `fct_undocumented_models` are the ones they borrow,
from its documentation and not from running it here. Listed under Warnings,
they would read as dbt's.

## Decision

- **A Checks section in Reports, after Macros**, apart from Warnings, with a
  title that says these are this tool's own and not dbt's. Each rule is a fold
  with its count, open while it lists fifty or fewer, its rows grouped by
  folder as the job coverage groups its own, a name opening the node
  with its Catalog tab.
- **Sources without freshness are dbt-core's `has_freshness`**, checked
  against 1.12.6: `bool(source.freshness)`, true when `warn_after` or
  `error_after` holds both a count and a period. The manifest of 1.12.6 has a
  top-level `freshness` and an identical `config.freshness` on every source
  table, a source's threshold already inherited by its tables; `null` on a
  table is written `null`, `{}` inherits, and nothing, with no source-level
  threshold, is a threshold of nulls: all falsy, as half a threshold is. The
  top-level key is read, else the config's. `loaded_at_field` is not required,
  as dbt's own selector does not.
- **A key that is absent gives no verdict.** The graph keeps `Some(has)` once
  the key is there, `null` included, and `None` when it is not, as in another
  producer's manifest: such a source is never listed as missing a threshold.
- **A model with no test is one no test counts for**: a generic test declared
  on it or one of its columns, by the test's `attached_node`, else the node
  whose YAML declares it as the column tests do; a singular test, which has no
  attached node, for each model it selects from; a unit test for the model its
  `depends_on.nodes` names. A `relationships` test also has the model it points
  at as a parent, and does not count for it. Models only: a seed or a snapshot
  is left alone. dbt-project-evaluator's rules about tests were not read for
  this, so the definition is this tool's own.
- **A model with no description** has none once trimmed, doc blocks already
  resolved into it by dbt. Models only.
- **A source no node reads** has no child among the graph's nodes: a model,
  snapshot, analysis or exposure reading it counts, a test of it does not.
- **The project's own enabled nodes only**, as the job coverage leaves them:
  the root package's, by the project's name, and never a disabled one. An
  installed package's models and sources are the package's to document, test
  and read, and would bury the project's.
- **Quiet: no chip, no news, and no light on the button.** The count rarely
  goes down, so a chip would stay on, as 0105's macros carry none, and nearly
  every project has a finding, so a lit button would say nothing. The section
  is listed in the rail while the index has it, drawn like any other, but it
  never lights the button, shows in its tooltip, or is the section the panel
  opens on, the one read last included.
- **The index counts from the graph in memory**, as the macros entry does, and
  reads no file, within 0064 as 0103 amended it. `/api/reports/checks` lists
  the four, each row its id, name, file and folder, by folder, file and name.
- **Checked against dbt itself.** `scripts/compare_with_dbt.py` works the four
  lists out again in Python from the manifest dbt wrote, by the rules above
  read off dbt's fields, on Jaffle Shop, on Shopify and on a copy of Jaffle
  Shop given every shape the rules tell apart, and compares them with the
  report; the copy's lists are also written there by hand.

## Rejected

- **Lighting the button like any other section**, the first version, turned
  down by the user because it left the button lit on almost every project and
  the first click no longer opened the job coverage (0075, 0070).
- **Listing them under Warnings.** That section's meaning is "dbt says so".
- **A chip.** It would not go away once the project has a finding, and nobody
  acts on a count that has stood for months.
- **Counting an installed package's nodes.** A package's models would bury the
  project's own, and the project cannot document what it does not own.
- **Requiring `loaded_at_field`.** dbt's `has_freshness` does not, and a
  source without one is better told so by dbt's own freshness run.

## Consequences

A manifest with no `attached_node` on a model's tests, which dbt-core always
writes, falls back on the node whose YAML file declares the test: two models in
one file read the first of them as declaring a `relationships` test the other
holds. A test named `source_...`, dbt's prefix, prefers a source of its file to
a model, so a source's test with a custom `name:` and no `attached_node` still
falls to the previous rule, the first parent in the file. A unit test is read through all of its `depends_on.nodes`, which in
dbt-core 1.12.6 holds the tested model alone: a producer that adds its inputs
would read them as tested. dbt Fusion's manifest was not looked at: where it
writes `freshness` otherwise, or not at all, its sources are given no verdict
rather than a wrong one, and where it writes `null` for one with no threshold
they read as having none. Checks does not light the button, so 0075 and 0070 hold
unchanged: the button stays dimmed while no run, warning, collin report, macro
finding or job source gives it something, a click on it opens on the job
coverage as before, and Checks shows in the rail below it.
