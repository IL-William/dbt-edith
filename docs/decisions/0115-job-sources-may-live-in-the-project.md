# 0115. Job sources may live in the project

Date: 2026-10-21 · Status: accepted · Amends 0011, 0057, 0063 and 0110

**Trigger:** read before changing where the job sources are kept, what
`.dbt-edith/jobs.yml` holds or who writes it, or what the command line and
MCP read of the orchestrator's jobs.

## Context

0063 keeps the folders of Airflow and dbt Cloud YAML in the per-user settings
(0011), so only the page knows them. An agent asking over the command line or
MCP (0057, 0110), which read no settings, could not ask which DAGs and jobs run
a model or which models a DAG runs, though the Jobs tab knows. A team sharing
a project shares its sources only by each person typing them.

## Decision

- **A file of the project, `.dbt-edith/jobs.yml`**: a `jobs:` list of the
  settings' sources (`kind`, `path`, `label`, `keys`). The page, the command
  line and MCP read it.
- **It wins over the settings** when it exists; they are ignored, not merged.
  A file that cannot be read gives no source and says why, at its line, and
  never falls back to the settings: an answer must not change silently when
  someone breaks a file (0055).
- **Written on the person's click only**, by the sources form's save with
  `place: project`, through `files::resolve` (0094) and an atomic write,
  refusing a file changed on disk since the form read it. "For me only" writes
  the settings and no file. Nothing deletes it; emptied, it is `jobs: []`.
- **The file takes a narrower path than the settings.** A source in it is a
  path from the project, at most two folders above it, with no hidden folder
  or file anywhere in it and never a `profiles.yml`; one that is not refuses
  the whole file at its line, and the page's save refuses it too. Hidden is a
  dot in the name or the system's own mark (macOS' `UF_HIDDEN`, which a user's
  `Library` has, Windows' hidden and system attributes, which `AppData` has).
  The same holds where the path really leads, a link along it followed (a
  Windows short name too, which resolves to its long one), as of each read, and
  a source may not lead to the user's home folder or any folder above it (a
  rule skipped when `HOME`, or `USERPROFILE` on Windows, is not set, and then
  only the hidden-folder checks hold). The
  folder walk of such a source opens no `profiles.yml`, `profiles.yaml` or
  hidden entry at all. A cloned
  repository supplies the file, and `keys` can rename any key of a YAML file,
  a profile's password included, into the DAG name the page shows. Sources the
  person keeps in the settings are held to nothing of this (0063). Reading
  stays bound by 0063 (YAML only, no symlink, only what was parsed leaves), and
  collections nest 64 deep at most. The file is written by a small emitter and
  read by `src/yaml.rs` (0003).
- **The command line and MCP read the file, never the settings.** `--describe`
  and `--overview` gain a `jobs` line, and `--job NAME` prints one DAG or dbt
  Cloud job step by step with the nodes each runs. No path enters a line, and a
  project without the file pays nothing. A source that could not be read is
  said as such, never as `none`. MCP keeps the read while the graph, the file
  and the job files are as they were.

## Rejected

- **The command line reading the settings**: a per-user file the project
  cannot share, and 0057's promise that none is read.
- **`--jobs KIND:PATH`**: a path in a line (0110), retyped by every caller.
  Deferred.
- **Writing the file when the page opens**: a file in someone's repository
  nobody asked for (0011).
- **Any path in the file**, as the settings take: a repository could then aim
  a source at `~/.ssh` or a profile, and read it through its `keys`.
- **Merging the file and the settings**: an answer that depends on which side
  moved last.

## What remains

Key renames of a source from the file can still lift a value out of any
visible, non-home, non-hidden YAML file inside the project or up to two folders
above it: a sibling repository's files, a compose file next to the project.
The home folder itself and everything above it are never a source, and nor
are the application folders the system hides, but the home folder's visible
subfolders that lie within that zone still are: `../../Documents` from
`~/code/proj` passes. The user accepted what is left, and it is why the zone is
so small.

## Consequences

0063's rejected "file in the dbt project" is accepted, on request and shared.
Whoever opens a project that commits the file reads the folders it names, from
the project or two folders above it, never a hidden one and never the home
folder. The command line
and MCP now read a file of the project besides the artifacts and a node's own
SQL (0102).
