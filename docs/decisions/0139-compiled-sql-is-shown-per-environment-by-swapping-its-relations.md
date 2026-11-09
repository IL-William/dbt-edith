# 0139. The Compiled and Run SQL is shown per environment by swapping its relations

Date: 2026-11-04 · Status: accepted, amended by 0143 and 0164 (how a swap is marked in the SQL) · Builds on 0113 and 0058

**Trigger:** read before changing what the Compiled or Run tab shows when an
environment is picked, which text the page replaces in the SQL, what the
payload of `/api/compiled` carries, or what the page says the SQL may differ
in.

## Context

The tabs show the SQL dbt left under `target/` for the one target the manifest
was built with. To paste a model's SQL into Snowsight for QA, UAT or STG, the
person recompiled under each `.env` file. The page already resolves every
node's location per file (0113), so the relations the SQL reads can be written
for another file without compiling anything. The risk is the SQL that branches
on the environment: `target.name`, an `env_var()`, a call that asks the
warehouse, and a project's `generate_*_name` that follows the target.

## Decision

- **The tabs follow the environment selector** of the status bar, the choice
  the Catalog and the Snowflake relation already follow. With none picked they
  show the file exactly as dbt wrote it.
- **Each relation the SQL holds is swapped, as text.** `/api/compiled` carries
  the node's own relation, its parents' and, through an ephemeral parent, that
  one's parents (dbt inlines it and gives it no relation), each with its
  location resolved for every `.env` file whether or not anything is written,
  as the clone plan does (0058). The page finds each `relation_name` on
  identifier boundaries, longest first, and writes the output in one pass, so
  a swapped name is never read again. A run's statement names the node's own
  relation with `__dbt_tmp` and `__dbt_backup`; those are swapped too.
  `scripts/compare_with_dbt.py` checks on Jaffle Shop and Shopify that dbt
  writes every non-ephemeral parent's relation verbatim in its child's
  compiled SQL, which is what the swap rests on (0033).
- **A relation the file cannot place is kept as compiled and marked amber**,
  with the reason: a variable the file lacks, a placeholder, a database left to
  the profile, or a model dbt built elsewhere than its config says, which means
  a `generate_*_name` macro moved it and that is not evaluated. The bar counts
  them.
  *Amended by [0143](0143-a-sandbox-manifests-relations-are-swapped-where-the-file-places-them.md):
  in a sandbox manifest a model built into the sandbox is not kept for its
  database or schema having moved (an alias move still is); it is written where
  the file places it.*
- **What may differ beyond the relations is named, not evaluated.** The server
  scans the model's SQL and every macro it reaches, transitively, for a key of
  `target` other than `type` and `threads`, `env_var()`, `run_query`,
  `load_result`, `load_relation`, `adapter.get_*` and `is_incremental()`, and
  for a project override of `generate_schema_name`, `generate_database_name` or
  `generate_alias_name` that reads the target. Each finding names the macro and
  the line in it, in the model's file for its own SQL; a macro finding opens the
  macro at that line. An override that reads the target only through a macro it
  calls is named through that macro and gives no line, the line being in
  another text. Macros of dbt and of
  the adapter are left out, `is_incremental` excepted. An `env_var` the node's
  own location reads is not a finding: the swap handles it. The name of one
  that reads as a credential is withheld at the scan (0012, 0090).
- **Only findings are kept at load**, never the bodies: each macro's
  `macro_sql` and each node's `raw_code` is scanned once while the manifest is
  read, and `Macro.body` stays limited to the strategy macros (0076).

## Rejected

- **Evaluating the Jinja** for another environment: this tool does not run
  dbt (0002), and it would be a second dbt to keep right (0009).
- **A picker of its own on the tab**: two environments on screen at once, and
  the Catalog saying another.
  *Amended by [0147](0147-the-compiled-and-run-tabs-offer-every-environment.md): the tab offers the
  environments as buttons that set the one shared choice, so there is still
  only one environment on screen and the Catalog says the same.*
- **Refusing the whole view** when any relation cannot be placed: most of the
  SQL is still right, and the amber marks say which part is not.
- **A fixed banner** that the SQL may differ: it is always there, so it is read
  never. The findings say where, by name and line.
- **Porting the per-environment rule to Rust**: the page already holds it
  (`lineageRelation`), and two copies drift apart.
