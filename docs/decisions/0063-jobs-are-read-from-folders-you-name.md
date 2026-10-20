# 0063. Jobs are read from folders you name, outside the project

Date: 2026-10-11 · Status: accepted, amended by 0077 and 0103 · Amends 0017 and 0018

**Trigger:** read before changing which job files are read, what leaves them,
or how a job's dbt command is resolved.

## Context

Which job runs a model lives with the orchestrator, usually in another
repository: dbt-jobs-as-code YAML for dbt Cloud, or one YAML file per DAG
behind an Airflow factory. 0017 lets nothing outside the project be read but
the profile, and 0014 forbids building the reader around one client's layout.

## Decision

- **Folders you name.** Each source is a folder or a file, kept per project
  in the settings (0011), absolute or relative to the project. The page names
  it, which 0017 refused for `/api/file`. That is safe here because the key a
  page needs (0052) already gives it a shell through `/ws/pty`. Within a
  folder, only `.yml` and `.yaml` files are read, never `.env*`, hidden folders,
  symlinks or junctions, at most 1 MiB each, and the walk is sorted (0055).
- **Only what was parsed leaves.** That means names, schedules, and each
  command as it was read, with the values of `--vars`, `--args` and `--inline`
  shown as `…`, since that is where a secret would sit (0012). A line that
  cannot be read shows its first words, never past its first flag. File text
  never leaves, and a file that yields no dbt command contributes nothing.
- **Two formats.** dbt Cloud jobs are read in dbt-jobs-as-code's public
  schema. Airflow has no standard, so this tool proposes a shape (one DAG per
  file, a list of tasks, a command per task) in which every key can be renamed
  by a dotted path.
- **A YAML tree, written here** (`src/yaml.rs`). Anchors and `<<` merges are
  common in jobs-as-code files, and a line scanner like 0018's cannot follow
  them. A file is read whole or refused whole, at a line, with a fixed message.
  *Amended by [0103](0103-a-config-path-is-unused-when-no-fqn-of-its-kind-begins-it.md):
  `dbt_project.yml` is read with it too, and each value knows the line of its
  key, or of the alias that names it.*
- **The engine resolves, never dbt** (0024, 0032). Each step is resolved with
  the verb's resource types and tests in the universe. A flag this tool does
  not know, a `state:` method, or a template makes the step unresolved, with
  the reason, rather than guessed.

*Amended by [0077](0077-a-start-no-file-says-is-neither-scheduled-nor-unscheduled.md):
a file with no dbt task leaves the schedule and the outlets of its DAG, to
tell what starts the DAGs on those assets, and nothing else.*

## Rejected

- **Detecting keys by name.** The list would grow with every repository, and a
  false match is silent.
- **One fixed format.** Every repository but ours would need a converter.
- **A launch flag.** It would have to be retyped on every start.
- **A file in the dbt project.** It would put a file of this tool's into the
  client's repository.

## Consequences

Some answers are not visible from the YAML: a factory that builds tasks in
Python, a paused DAG, a task-level schedule only the factory interprets,
`--vars` or `--target` changing what is enabled, and sources without a
freshness config. A `--project-dir` is taken to name this project. Each step's
`dbt ls` button is the check, and `scripts/compare_with_dbt.py` runs it on
invented jobs over Jaffle Shop.
