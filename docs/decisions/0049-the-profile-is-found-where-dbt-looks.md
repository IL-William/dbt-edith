# 0049. The profile is found where dbt looks, whatever the warehouse

Date: 2026-10-08 · Status: accepted · Amends 0017 and 0031

**Trigger:** read before changing which `profiles.yml` dbt-edith opens, or
what decides whether it can be opened.

## Context

0017 made the dbt profile reachable as the file the Snowflake script named, and
0031 closed it whenever Snowflake's features are off. On a Snowflake project
with column lineage on `none`, the script never runs, so the top bar offered no
`profiles.yml` and nothing said why. And every dbt project has a profile, read
by every dbt command typed in the terminal, whatever its warehouse.

## Decision

- **dbt-edith finds the profile itself**, in dbt's order: `DBT_PROFILES_DIR`,
  then `profiles.yml` in the project, then `~/.dbt` (`USERPROFILE` on Windows).
  Its environment is read once at startup, the one the terminal and the script
  inherit, and a relative `DBT_PROFILES_DIR` is taken from the project, where
  both run. `src/profiles.rs` holds the order; `tools/sf_lineage.py` repeats it.
- **The route still takes no path.** Only where it looks has changed: what
  0017 says about `/api/file`, writing atomically and never creating a file
  stands.
- **No Snowflake gate.** A button beside the snowflake opens it on every
  project, grey with a dot and the path dbt would read when there is no file
  there. Its icon is a user and a plug, since the file says who dbt connects
  as and how; a key would read as the `.env` secrets, which never leave the
  server.

## Rejected

- **Remembering the path the script last named**, across restarts. It still
  needs Snowflake picked once per project, and says nothing on another
  warehouse.
- **Showing it in the file tree.** The tree is the project, and 0017 keeps the
  profile out of the route that serves it.
- **Reading `DBT_PROFILES_DIR` from the project's `.env`.** dbt does not, so the
  file opened would not be the one dbt reads.

## Consequences

A `DBT_PROFILES_DIR` exported by the user's shell startup files reaches dbt in
the terminal but not dbt-edith, which then opens another file. Starting
dbt-edith from a shell that has it set avoids that.
