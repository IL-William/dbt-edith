# 0103. A config path is unused when no fqn of its kind begins it

Date: 2026-10-19 · Status: accepted · Amends 0018, 0063 and 0064

**Trigger:** read before showing a warning dbt gives as it parses, judging
whether a key of `dbt_project.yml` applies to anything, or letting the
Reports index read a file.

## Context

dbt warns once, as it parses, of the config paths of `dbt_project.yml` that
apply to no resource: `UnusedResourceConfigPath`, I051 in dbt-core, dbt1097
in Fusion. A misspelt folder, a folder renamed or emptied, a `docs:` written
without its `+`: what the key sets reaches nothing, and the warning scrolls
past among the parse's lines. 0036 dashes a key whose folder is not on disk,
which is a different question: a folder can exist and hold no node the
manifest knows, and dbt warns of its key all the same.

## Decision

- **dbt's rule, as dbt-core writes it** in `_get_config_paths` and
  `warn_for_unused_resource_config_paths`. Under each of eleven blocks,
  `models` to `functions`, a key holding a mapping is a path segment unless it
  starts with `+` (0039). A path is recorded once the mapping it names holds
  anything else, a `+` key or a value that is no mapping, so a folder of
  folders is no path and `{}` is none. A path is used when it begins the fqn of
  a resource of its block's kind, or of any disabled node, whatever its kind.
  `tests:` is read as `data_tests:` when there is no `data_tests:`, and left
  out when there is.
- **Against the manifest, never the disk**: `Fqns` keeps every fqn of the
  manifest, the disabled nodes, the unit tests, metrics, semantic models,
  saved queries and functions included, joined by a character no name holds
  and sorted, so a path is two binary searches. The file is read whole by
  `src/yaml.rs` (0063), which now gives each value the line of its key, and a
  key reached through an alias the alias's line: the anchor's own keys are
  another path, which may be used.
- **Checked against dbt itself.** `scripts/compare_with_dbt.py` compares the
  list on Jaffle Shop, Shopify, and a copy of Jaffle Shop given such paths,
  with the warning dbt-core sends as an event; Fusion 2.0.8 gave the same list
  on that copy. An exposure and a unit test there each keep a path used that
  no folder's fqn begins, so leaving their fqns out fails it.
- **A Warnings section in Reports**, after Last run, listed while a path is
  unused, each opening `dbt_project.yml` at its key and saying why. Its chip
  counts from none seen on each page, unlike the run's and the coverage's: dbt
  says this once, so a path stays news until the section is looked at, and
  then only a rise is news (0066). In the editor, a mark in a gutter of its
  own on each such key, placed by the text's own chain of keys, so an edit not
  yet saved moves it; its hover card says why, its click opens the section.
  Paths on one line (an alias and a path inside what it names, the keys of
  one flow mapping) share one mark, as a gutter keeps one per line, and its
  card lists each with its reason.
- **Nothing on the button while the manifest is behind**: `dbt_project.yml`
  saved after it, or other files, by the freshness badge. A folder added since
  the last parse is in no manifest, so its key reads unused though dbt will
  not warn of it after the next parse. The section still lists the paths, the
  reason first, and the marks turn grey.
- **A file the reader refuses lists the section, with no chip**, to say at
  which line, since nothing else would.
- **The index reads `dbt_project.yml`** for the count, which 0064 kept to file
  times and settings: a few kilobytes of the project's own file, against a
  manifest already in memory. Its time joins the stamp the freshness poll
  carries, so a save is heard of. With no manifest there is no section, since
  every path would read unused.
- **A manifest older than the file is said first**, and the freshness badge's
  state otherwise, as 0036's card does: a folder added since the last parse is
  in no manifest.

## Rejected

- **The disk, as 0036 places a key.** It answers whether a folder exists, not
  whether dbt configures anything through it, and would contradict dbt's own
  warning both ways.
- **Reading `logs/dbt.log`** (item 6 of state.md): no log when the manifest
  came from elsewhere, rotated at 10 MB, moved by `--log-path`, and the last
  command's only.
- **The fqns of the graph's nodes**, joined by dots in `Node.fqn`: disabled
  tests and every disabled variant past the first are not kept there, nor are
  unit tests' and four other kinds', and a dot inside a folder's name would
  let `v1` begin `v1.2`.
- **The editor's own scanner for the paths**, as 0036 does: it skips the
  blocks whose keys are not folders, `sources:` among them, and follows no
  anchor or merge, which dbt does.
- **The count seen kept in the browser's `localStorage`**, so a standing path
  is news once rather than on each page: 0011 rejects `localStorage`, and a
  store on the server is a write route for one number.

## Consequences

A key YAML reads as a number, `2023:` unquoted, is text here; dbt-core stops
on it. A file `src/yaml.rs` refuses, a duplicate key or a tag, gives no
warnings and says at which line, where dbt takes the last of two keys. A
standing path is news on each page until the section is shown. The other
warnings item 7 of state.md ranks can join the section, each with its own
check against dbt.
