# Catalog

What the Catalog tab says about a node, where it is built in each environment,
and the Compiled and Run tabs over the SQL dbt left under `target/`.

## The Catalog tab

The **Catalog** tab mirrors what dbt's own catalog shows: materialization,
column count, upstream and downstream counts, tags and description under
Preview, and a Column / Type / Description / Tests table under Columns. The
upstream and downstream lists navigate like the graph: click moves the lineage
and the catalog to that node, double-click also opens its file. The column count
opens the Columns tab.
An incremental model adds Strategy and Unique key beside the materialization,
the strategy as its box reads it, and a snapshot its Unique key. The key takes
one column per line, three at most, then `+N more, in Columns`, and each name
opens the Columns tab on its row. There, each key column carries a `KEY` tag,
and a Key sort puts them first in the key's order. A view or a table that a
folder's config gave a strategy or a key lists it, dimmed, under details: it
does not use it, but `config.incremental_strategy:` still selects it.
Once a [job source](jobs-and-reports.md#jobs) is named, Preview also lists the steps that run the
node, one chip each, `DAG › task` or `job › step`, green when the job is
scheduled, amber when its schedule is not read and grey when it is not
scheduled, in the Jobs tab's order. A click on one
opens the Jobs tab on that node at that step, even when it is a box clicked
once in the lineage rather than the node in focus. A dashed chip counts the
steps not resolved, or the files not read, that could run it too.
A column's Tests cell names every test guarding it, one chip each: two
`relationships` on one column are two tests and read as two. Where they would
not fit, the first few are shown and a `+N` carries the rest, which hover, click
or Enter reveals with the name dbt generated for each. A click or Enter on a
chip opens its properties file at the line that declares the test. The tests
that guard the model rather than a column, which is every singular test under
`tests/` and every generic with no column such as
`unique_combination_of_columns`, are listed under Preview, where clicking one
moves the lineage to it and double-clicking opens it where it is declared.
A test shown in the Catalog has Preview alone, since it has no columns of its
own. It opens on what it checks, as its hover card says it: the generic, or
`singular`, the model whose YAML declares it, its column, its severity, `warn`
in amber, and the count of models it reads, which moves the lineage to them.
Under config come the keys it moved off dbt's defaults, among `store_failures`,
`store_failures_as`, `where`, `limit`, `fail_calc`, `warn_if` and `error_if`,
and, when it keeps its failing rows, the relation they are stored in: the only
one dbt builds for a test, which is why it has no location table. A
`--store-failures` given on the command line is not in the manifest, so it is
not shown. Then the steps that run it, its tags,
the file it is declared in, and the models it reads.
Everything there comes from `manifest.json`, which already holds the merged YAML
(descriptions, tags, and the tests attached to each column). Column *types* are
the exception: the manifest only carries them when they are declared in YAML, so
run `dbt compile --write-catalog` under Fusion, or `dbt docs generate` under
dbt-core, and
`catalog.json` is picked up automatically, filling in
the real warehouse types and listing the columns that exist in the warehouse but
are not documented (shown in italics).

## Where a model lives

The Catalog preview shows each node's database, schema and alias at three
stages, because in a project driven by `env_var()` and `generate_*_name`
macros no single value tells the whole story:

| | |
| --- | --- |
| **as written** | the config exactly as authored, Jinja included |
| **resolved** | the config with its env vars evaluated |
| **built** | where the target that produced the manifest actually built it |

A last row gives the full relation for *resolved* and *built*, each with a Copy
button, quoted part by part the way dbt quotes the built one. The resolved
relation is only offered when it can be written for real: a database or schema
left to the target profile, a placeholder or a missing variable says why
instead.

With Snowflake's features on, a **Snowsight** menu sits beside each Copy and
**Clone into my schema** beside the Location heading, both in
[snowflake.md](snowflake.md).

Without an environment selected, *resolved* is the config as dbt parsed it,
which means with whatever env vars happened to be loaded at the time. That is
not necessarily any particular environment: a shell that sourced a CI `.env`
file yields CI values.

When a manifest comes from a developer sandbox, where nearly every model is
built into one `database.schema`, dbt-edith detects it once and says so, rather
than flagging every model as moved. A model built somewhere unexpected is only
highlighted when it is the exception. The comparison is always between the
parsed config and the built location, which come from the same parse. Case
alone is never counted as a move, since Snowflake folds unquoted identifiers,
and neither is a pair of literal quotes that dbt drops.

## Environments

The `env` pill in the status bar says which of the project's `.env` files the
*resolved* column is evaluated against, so a model's location can be read for
DEV, QA or UAT without running dbt. Built stays tied to the manifest's own
target. The file is picked in the menu in the header of the resolved column, or
in the Project section of the settings, which the pill opens; the pill takes a
colour per environment family (DEV, CI, QA, STG, UAT, PROD) so the one in use
shows at a glance.

- **Detection.** Every `.env` and `.env.*` file in the project root is offered,
  named after its suffix: `.env.uat` is UAT. A `DBT_TARGET` that disagrees with
  the suffix is flagged, since it usually means a file copied from another
  environment. Templates (`.env.example`), backups, and files that define none
  of the variables in use start hidden.
- **Evaluation.** `{{ env_var('NAME') }}`, with or without a default, and plain
  literals are evaluated. A single `if` / `else` is followed through the branch
  dbt actually took when it parsed, and marked as such. Anything else, filters
  or `target.*` for instance, is shown as not evaluated rather than guessed.
- **The file alone.** Values come only from the chosen file, never from the
  shell dbt-edith was started in. So *missing* means "not defined in this file",
  which is not quite what dbt would see after sourcing several files in a row.
  `${...}` interpolation is kept as text and not evaluated.
- **Flagged, not shown as real.** Variables set to a placeholder (`N/A`, `TODO`,
  `null` and the like) or left undefined.
- **Secrets.** `DBT_ENV_SECRET_*` variables are never substituted, and no
  variable value is ever sent to the browser except resolved locations and
  `DBT_TARGET`.

The same section, which *Manage environments* in the menu opens, renames or
hides files. It also reports, per file, how
often that file reproduces what dbt parsed: the file dbt actually had loaded
scores 100%, which makes it easy to spot which environment a manifest came from.

Those settings are stored per project in the user's config directory, never in
the project itself:

| | |
| --- | --- |
| macOS, Linux | `$XDG_CONFIG_HOME/dbt-edith`, else `~/.config/dbt-edith` |
| Windows | `%APPDATA%\dbt-edith` |
| anywhere | `DBT_EDITH_CONFIG_DIR` overrides both |

Each browser tab keeps its own environment. The stored one is only where a new
tab starts.

## Compiled and Run

Two tabs, over the two files dbt leaves per model under `target/`. **Compiled**
is `target/compiled/`: the model with its Jinja rendered, which is what you read
to check a macro or a `ref()` expanded the way you meant. **Run** is
`target/run/`: that same SQL wrapped in the statement dbt actually sent to the
warehouse, `create or replace transient table ...` and all, which is what you
read when the table is there but looks wrong.

Each names the date its file was written, with the age beside it:

```
last run 29/07/2026 16:48:06   54d ago, the model file changed after this was run
target/run/shop/models/marts/.../fct_orders.sql
```

That path is a link. Clicking it opens the file tree at the file, so the folder
dbt wrote, the files beside it and the file itself are one click away.

The bar turns amber when something the file was built from has moved since it
was written, and names what: the model, its schema file, `dbt_project.yml`, or a
macro. The Run tab asks one more question only it can ask, since dbt writes
`compiled/` on every compile and `run/` only on a run: has this model been
compiled again since it was last executed?

Age alone never colours anything. A file nothing has touched since is still
exactly what dbt would write, whether that was an hour ago or last month, and a
clock threshold only ever teaches you to ignore the colour. The date is there to
be read, not to expire.

Any macro counts, not just the ones your model calls. The manifest lists the
macros reached while parsing a node, which leaves out the ones those in turn
call and is mostly `macro.dbt.*`, built into dbt-core with no file to look at. A
macro edit makes every compiled file suspect until it is rebuilt, which is also
what dbt does about it. The same goes for a `git pull`: it moves the mtime of
everything it touches, so your artifacts predate the code you now have, which is
the answer [0025](../decisions/0025-freshness-is-mtimes-not-commits.md) gives
for the manifest too.

dbt-edith never compiles or runs anything itself. dbt runs where you run it, so
when a file is not there the tab names the paths it checked and offers to type
`dbt compile --select <model>` or `dbt run --select <model>` into the integrated
terminal, without pressing Enter for you.
