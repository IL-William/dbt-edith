# 0058. A model is cloned into your schema by statements you run

Date: 2026-10-10 · Status: accepted · Amends 0017

**Trigger:** read before writing SQL for the user to run, changing what a clone
copies or where it lands, or reading anything else out of the profile.

## Context

Developing a model with a deep lineage means either building everything above
it locally or working out by hand which objects of a shared environment to
copy, and whether a clone is even allowed: dbt-snowflake creates tables
transient, and Snowflake refuses to clone a transient table into a permanent
one. The Catalog already resolves each node's relation under every `.env`
file; nothing wrote the statements.

## Decision

- **Text, never a call.** A modal writes the statements and the user copies
  them. Nothing starts and nothing reaches Snowflake.
- **Behind Snowflake's features** (0031): the button is absent while they are
  off, `/api/snowflake/clone` answers 409, and the settings menu describes it.
- **The model and its direct parents.** An ephemeral parent has no object, so
  its own parents stand in for it. Sources are listed, never copied. A semantic
  view is skipped, a disabled node refused. Parents by name, the model last.
- **The source is the Location table's relation** under the chosen file, by the
  same function, or where the manifest built it. Never the manifest's values
  in place of a file's, and never a database or schema the config leaves to the
  target, which the file does not decide.
- **One destination**, prefilled from the profile's active target, picked as dbt
  picks it: `DBT_TARGET` from this process, then `target:`, then `default`. Its
  `database` and `schema` are read by the profile's line scanner (0003, 0018),
  and only when written plainly. What the user types instead is kept per
  project (0011). The copy keeps the source's name, quotes included.
- **Always transient.** A clone for a table, an incremental model, a seed or a
  snapshot; a create as select for the rest, a view first dropping the one a
  local run may have left. A create as select for everything on request.
- **Amends 0017:** a second route reads the profile, and sends the active
  target's name, database and schema, or why there are none. Nothing else.

## Rejected

- **`/api/node` per parent:** a request per parent and per ephemeral.
- **Running the statements**, which would make dbt-edith write to a warehouse.
- **An age threshold on the manifest**: 0025 judges what moved, not how old.
- **Permanent copies**, which a clone cannot be, for no gain in a sandbox.
- **`DBT_TARGET` from the project's `.env`**, which dbt does not read (0049).
- **Evaluating the profile's other Jinja**, or emulating `generate_schema_name`:
  a guessed schema puts a copy where nobody looks.
- **Cloning a view**: the clone would keep reading the shared environment.

## Consequences

None of it has run against a warehouse yet (docs/state.md). A legacy snapshot
configured with `target_schema` is refused under an environment, since that key
is not read. A package's files are never checked for changes since the parse.
