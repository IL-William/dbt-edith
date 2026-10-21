# 0113. The card resolves a location per .env file

Date: 2026-10-21 · Status: accepted · Amends 0017, 0019, 0102 and 0110

**Trigger:** read before printing anything derived from a `.env` file on the
command line or over MCP.

## Context

0012, kept by 0017, lets resolved locations leave the server; 0019 and 0090
added guards, `sensitive_name` for the vars route only. 0102 and 0110 read no
file but `--sql`'s and reach nothing outside the project. The Catalog resolves
a location per file; the card showed one parse's, so an agent read macros.

## Decision

- **`--describe` and the `edith` tool print the Catalog's resolved locations**
  of a built model, seed, snapshot or source: `configured` (the keys as
  written, when one differs from the built one) and a line per `.env` file,
  from `envs::discover` and `envs::resolve_place`; JSON `configured`, `by_env`.
- **The Catalog's default files**: a template, a backup and a file defining
  none of the variables read are skipped, and a name with a control character.
- **The guards hold, and two more apply.** `DBT_ENV_SECRET_*` is never
  substituted; a variable named like a credential and a value carrying a
  password (0090) print `<hidden>`, which the location resolver does not check
  (0019), and a written key reading one hides its default, set or not.
- **Before the macros**: a line is the configured value for that file, and
  `generate_*_name` run after it (the default builds a custom schema as
  `<target schema>_<it>`), so it is not where dbt builds; `relation` is dbt's.

## Rejected

- **Reading `.env` in `--overview`**, or printing values other than locations.
- **Evaluating the `generate_*_name` macros**: beyond the scanner (0009).

## Consequences

The command line and MCP now read the project's `.env` files, which 0102 and
0110 said they did not. `discover` follows a `.env` symlink out of the project,
as the page does.
