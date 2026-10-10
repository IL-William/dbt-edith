# 0053. A project is read before anything in it runs

Date: 2026-10-10 · Status: accepted

**Trigger:** read before running any program found in the project, or changing
how the status bar learns which Python and which dbt a venv holds.

## Context

At startup, `venv::detect` ran `python --version` and `dbt --version` from every
folder at the project's root that looked like a venv. A repository can carry
such a folder, executable included, and opening it in dbt-edith ran that
executable before anyone clicked anything. The same folder's `python` was a
candidate for the Snowflake script. Nobody commits the venv they made for
themselves, which is exactly why nobody would look.

## Decision

- **Versions are read, never asked.** Python's comes from `pyvenv.cfg`,
  `version` or `version_info`; dbt's from the names of the `.dist-info` folders
  in `site-packages`: Fusion, which PyPI calls plain `dbt`, or dbt-core, then
  the adapters, told from dbt's other packages by the files their `RECORD`
  puts under `dbt/adapters/`.
- **A venv git tracks is never run.** `git ls-files` says whether it came with
  the repository. Such a venv is listed as `committed` in the payload and named
  in the status bar's tooltip, and is neither the one reported nor a candidate
  for the Snowflake script. The venv dbt-edith was started in, `VIRTUAL_ENV`,
  is the user's own choice and stays usable wherever it is.

## Rejected

- **A trust prompt per project**, as an editor's workspace trust. It guards the
  same thing with a click everyone learns to make, and the signal it waits for
  is already in git.
- **Guarding git hooks.** A clone brings no hooks: `.git/hooks` is not part of
  what is fetched, so a hook that runs on commit was installed by the user.

## Consequences

A project that arrives as an archive, with no `.git`, cannot be asked, and its
venvs are treated as the user's, as before. The status bar's dbt line now names
dbt-core and the adapter, where it used to show whatever `dbt --version` printed
first, `Core:` for dbt-core. Fusion's version is the one pip records,
`2.0.0rc196`, where its binary says `2.0.0-preview.196`. The Snowflake script still runs the project's own venv when that venv is
not tracked, once Snowflake is picked: that is the user's gesture.
