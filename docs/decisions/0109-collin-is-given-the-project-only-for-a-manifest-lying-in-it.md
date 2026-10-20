# 0109. collin is given the project only for a manifest lying in it

Date: 2026-10-20 · Status: accepted · Amends 0098 and 0099 · Follows 0042

**Trigger:** read before changing the arguments dbt-edith hands collin, what
`WANTS` in `src/collin_run.rs` asks for, or where a seed's columns come from.

## Context

collin 0.10.0 reads a seed's columns from the header of its CSV, for a seed the
catalog does not list (collin's 0047). It looks for the file in the project it
is given with `--project`, then at the manifest's own `root_path`. A manifest
named without `--project` is read alone: the folder collin runs in is not its
project, and no seed is looked for there (collin's 0046, 0047). Checked in
collin's `crates/collin-cli/src/main.rs`, where `--project` sets
`Options::project_named`, and in its decision 0047.

dbt-edith always handed collin `--project <folder>` and ran it in that folder.
That was harmless while the project named nothing `generate` reads once a
manifest is named. With 0.10.0 it is not: `dbt-edith --manifest
/artifacts/manifest.json` started in another project's folder, as a script or
an agent does (0099), would give that manifest the seeds of the folder it was
started in, and a seed of the same name there would put its header's columns,
and edges from them, into a lineage they do not belong to.

## Decision

- **dbt-edith wants collin 0.10.0 or later.** An older one reads no seed's
  header, and `WANTS` is how a release says which collin it needs (0043,
  0098). The 0.9.0 that `--compiled` needs is below it.
- **`--project` is passed only when the manifest lies under the project
  folder**, in `collin_run::manifest_in_project`. Otherwise the manifest is
  named alone, and collin looks for its seeds only where the manifest says
  (`root_path`). Every other argument, and the folder collin runs in, are as
  before. All four paths are named, and for `--run-collin` they are made
  absolute first (`ls_column.rs`), so nothing is read or written relative to
  where it runs. The page's run holds that when those paths are absolute or
  dbt-edith was started in the project folder itself; a relative `--manifest`
  or `--catalog` given from anywhere else is resolved by collin from the
  project folder, as before this change.
- **Both paths are resolved before they are compared**, links followed
  (`canonicalize`, then `files::plain` for Windows' `\\?\` form), so a manifest
  that is a link out of the project is outside it, a project reached by a link
  is the same folder, and the real case of a name on Windows and macOS is the
  one compared. They are then compared **a component at a time**
  (`Path::starts_with`), so `jaffle_shop_2` is not under `jaffle_shop`, which a
  string prefix would say.
- **A path that cannot be resolved is not under**: a manifest that is not there
  gets no project, and collin, run without the flag, says it finds no
  manifest, as it does with one.

## Rejected

- **Always passing it, as before**: the case in Context.
- **Never passing it**: a manifest in the project would lose the seeds of the
  project it was written for, the case 0.10.0 is for.
- **Comparing strings or lexical paths**: `..`, a link, a trailing separator
  and a sibling's name prefix all change the answer.
- **Deciding at startup**: a Job names its own project and manifest, and the
  command line's `--run-collin` and the page's Regenerate build one each; the
  rule belongs where the command is built.

## Consequences

- **A manifest of this same project kept outside its checkout**, a CI run's
  artifacts as 0099 documents for `--run-collin`, no longer reads seed headers
  through `--project`. Its `root_path` names the CI machine, so its seeds stay
  unknown rather than wrong, as before 1.43.1 with collin 0.9.0.
- **A `target` folder that is itself a link out of the project** (pointing at a
  build folder elsewhere) puts its manifest outside it, so collin gets no
  project. A manifest parsed on this machine still finds its seeds through its
  absolute `root_path`, when that holds a `dbt_project.yml`; only a manifest
  parsed elsewhere loses them.
- **A collin older than 0.10.0** shows the update line until it is replaced.

## Known limits

- The rule is tested in `collin_run`'s unit tests, a module that is unix-only,
  and on the command line path in `ls_column`'s; dbt-edith's CI has no Windows
  job, so the `\\?\` handling of `files::plain` is not exercised there.
- A refinement left for later: also passing `--project` when the manifest's
  `metadata.project_name` equals the project's name (`crate::project::name`),
  which would keep the seeds for the two cases above without giving them to
  another project's manifest.
