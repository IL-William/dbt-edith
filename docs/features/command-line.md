# Command line

Four questions dbt-edith answers without a page: what a selection matches, what
a column reaches, what a node is, and what the project holds. Each reads the
manifest dbt already wrote, prints and exits, so a script or an AI agent can
ask with one command and read the answer in few tokens.

## Without a server

`--select` and `--selector`, `--column`, `--describe` and `--overview` have
this in common
([0057](../decisions/0057-a-selection-is-answered-on-the-command-line.md),
[0102](../decisions/0102-a-node-and-the-project-are-described-on-the-command-line.md)):

- **Nothing is served.** No port, no key, no browser, no settings read or
  written, no watcher. `--column` alone ever starts a program, collin, and only
  when `--run-collin` asks (0099).
- **stdout is the answer and nothing else.** stderr says what it is worth: the
  manifest that answered, when it was written, by which dbt, and how many
  nodes, then a `freshness:` line, the verdict of the
  [freshness badge](lineage.md#is-the-lineage-still-true) naming the files
  newer than the manifest. The answer is the manifest's, not the working
  tree's: when that line says `stale`, run `dbt parse` and ask again.
- **Exit codes.**

  | Exit | Meaning |
  | --- | --- |
  | 0 | answered, an empty answer included, which stderr says |
  | 1 | refused: a selection or a name it cannot resolve, with the reason on stderr; `--describe` still prints the names it could |
  | 2 | nothing was asked: no project, no manifest, an unreadable one, a `--catalog` that is not a file, or a wrong flag |

  A flag that belongs to another mode is a wrong flag: `--sql` beside
  `--overview` or `--depth` beside `--describe` is refused, naming the mode it
  goes with, and never ignored.

- **`--manifest path/manifest.json`** names the manifest, else
  `<project>/target/manifest.json`. **`--catalog path/catalog.json`** names the
  catalog `--describe` and `--overview` merge, else the `catalog.json` beside
  the manifest, where `dbt docs generate` writes it. A relative path is
  relative to where the command is typed, not to the project.
- **`--output`** picks what is printed: `name`, the default, one name per
  line; `json`, one object per node; `compact`, tab-separated lines under a
  header. `--describe` and `--overview` print a text card unless it is `json`:
  a card is read, so `compact` prints the same text.

A call costs one process start and one read of the artifacts: the manifest for
a selection or a column, and for `--describe` and `--overview` the catalog and
the caches beside it too, which is why those two take longer on a project with
a wide catalog. The freshness walk runs after the answer is out, so it never
holds it back.

## A selection

`--select`, `--exclude` and `--selector` resolve a dbt selection, as the
Custom selection box does; the syntax, the exit codes and `--output json` are
in [selection.md](selection.md#from-the-command-line).

`--output compact` prints one line per node under a header, sorted by name:

```
dbt-edith path/to/project -s 'stg_orders+1' --exclude 'resource_type:test' --output compact
```

```
name	kind	materialized	path	tags	parents
customers	model	table	models/customers.sql		stg_customers,stg_orders,stg_payments
orders	model	table	models/orders.sql		stg_orders,stg_payments
stg_orders	model	view	models/staging/stg_orders.sql	staging	raw_orders
```

| Column | What it holds |
| --- | --- |
| `name` | what `--output name` prints: `source_name.table` for a source; the unique_id instead when another node of the project carries the same name |
| `kind` | dbt's resource type: `model`, `source`, `seed`, `snapshot`, `test`, `exposure`, `analysis` |
| `materialized` | `view`, `table`, `incremental`, `ephemeral`, `seed`, `source`, `test`; empty for an exposure, or where the manifest says nothing |
| `path` | `original_file_path`, the file the node is written in |
| `tags` | comma-separated |
| `parents` | the names of the nodes it depends on, comma-separated: each is a name that appears in the `name` column of the same answer when the selection reaches it; a name more than one node carries is written as the unique_id here too, so two parents of one name are two entries |

What `--output json` repeats and compact leaves out is what the other columns
already say: the `unique_id` is the kind and the name, the `fqn` is the path,
and an installed package's name is in its path. A tab, a newline or another
control character inside a field is written as a space, so every line has its
columns. An answer with no node has no header either.

## A column

`--column` walks one column's lineage over the cache beside the manifest, as
[column-lineage.md](column-lineage.md#from-the-command-line) describes, with
the same flags. `--output compact` prints one line per column reached under a
header, upstream first: by depth, then by model and column.

```
dbt-edith path/to/project --column stg_payments.amount --down all --output compact
```

```
depth	column	from	tests
0	stg_payments.amount		
1	customers.customer_lifetime_value	stg_payments.amount:aggregate	
1	orders.amount	stg_payments.amount:aggregate	not_null_orders_amount
1	orders.bank_transfer_amount	stg_payments.amount:aggregate	not_null_orders_bank_transfer_amount
```

`depth` is the levels from the column asked, negative upstream. `column` is
`model.column`, the model's unique_id standing for its name when two models of
the answer share it. `from` lists the columns of the answer it is read from,
each `model.column:role`, comma-separated; `tests` the data tests on it. What
the JSON adds is left to it: the `generated` values of a column its model's SQL
sets from no column, and `more_up` and `more_down`, the edges a depth or a stop
left out. Ask for `--output json` when the question is whether the walk was
cut.

## A node

`--describe NAMES` prints a node's card, what the Catalog shows of it, so
nothing has to be learned from the files:

```
dbt-edith path/to/project --describe stg_payments
dbt-edith path/to/project --describe orders,raw.orders --output json
dbt-edith path/to/project --describe orders --sql
```

This is the card of a model on Jaffle Shop, with its catalog beside the
manifest:

```
model stg_payments  view  models/staging/stg_payments.sql
  relation      "jaffle_shop"."main"."stg_payments"
  tags          staging
  yml           models/staging/schema.yml
  columns (4)
    amount	DOUBLE (catalog only)
    order_id	INTEGER (catalog only)
    payment_id	INTEGER	not_null, unique
    payment_method	VARCHAR	accepted_values
  parents (1)   raw_payments (seed)
  children (2)  customers (table), orders (table)
  tests (5)     accepted_values_stg_payments_payment_method__credit_card__coupon__bank_transfer__gift_card: accepted_values on payment_method; assert_order_amounts_match_payments (singular); assert_payments_are_positive (singular); not_null_stg_payments_payment_id: not_null on payment_id; unique_stg_payments_payment_id: unique on payment_id
  reach         1 upstream, 2 downstream
```

### Which node

A name is a node's unique_id (`model.shop.orders`) or its name: a model, seed,
snapshot, test or exposure by name, a source as `source_name.table`. Several
names go in one call, comma-separated, and each is printed, one empty line
between the cards; a node named twice is printed once. A name resolves by the
rules below. `--column` names its model by the same rules, among models, seeds,
snapshots and sources only, where `--describe` takes any node, a test
included:

1. a unique_id, exactly;
2. else the nodes whose name is exactly that and, for a name with a dot in
   it, those whose unique_id ends with a dot and the name: `dim.v2` for a
   version of a model, `shop.orders` for a package's; a source's
   `raw.payments` and a package `raw`'s model `payments` are then two nodes,
   refused rather than guessed;
3. else the same, case ignored;
4. among several, the enabled ones, then, for `--describe`, a model, seed,
   snapshot or source over a test or an exposure; still several, refused.

A refused name lists the unique_ids to write. A name that is no node is refused
with the nearest names, `no node named order; did you mean orders,
orders_daily`, models and sources before the tests that carry their names.
The names that do resolve are all printed first, and the exit code is 1 if any
did not.

### The card

The first line is the kind, the name, how it is built and the file it is
written in, under `dbt_packages/<package>` for a package's node, so the lines
under it need not say them again; a source's and a
test's `materialized` only repeat the kind, so their first line leaves it out.
Every line under it is left out when it has nothing to say.

| Line | What it says |
| --- | --- |
| `status` | `disabled`, for a node the manifest holds but dbt does not run |
| `relation` | where it is built: dbt's `relation_name`, else the database, schema and alias |
| `package` | the package it comes from, when it is not the project's |
| `tags` | comma-separated |
| `yml` | the properties file that documents it |
| `incremental` | an incremental model's strategy as its box shows it, when it was read, then its `unique key` |
| `description` | its own text, whole, with its line breaks |
| `columns (N)` | one line per column, tab-separated: name, type, tests, description on one line, the empty fields at the end left off with their tabs. A column only the warehouse has, from the catalog, says `(catalog only)` after its type |
| `parents (N)`, `children (N)` | `name (materialized)`, a source or an exposure saying its kind, sorted by name |
| `tests (N)` | each test on the node: `name: generic on column`, `name (singular)` for a query of its own, then `warn` and each setting off dbt's default (`where x`, `limit 5`); a test declared on another node, as a `relationships` is on the model it points at, names that node: `on orders.customer_id` |
| `reach` | how many nodes lie upstream and downstream, tests excluded; a source has only downstream |

A test's card says what it checks and how it is set instead:

| Line | What it says |
| --- | --- |
| `checks` | `not_null on orders.order_id`, the model it is declared on and the column, or `a query of its own` |
| `severity`, `settings` | `warn`, and the settings off dbt's default |
| `generic` | `dbt_utils.unique_combination_of_columns` for a generic named through a package |
| `parents (N)` | the nodes it reads, by name |

An exposure has its description and its parents. The columns line is the same
text the Catalog's Columns tab shows: a declared type wins over the catalog's,
which only fills a column whose YAML gives none. A control character other than
the tab in a description or a SQL file is left out of the text, so none of the
project's reaches the terminal as an escape.

### The SQL

`--sql` prints the node's own file after its card, under `sql`, indented. It
is the one file the command line reads besides the artifacts, through the
confined read the page uses, so it never leaves the project. A model, a
snapshot, an analysis, a Python model and a singular test have one. A generic
test shares its properties file, a source and an exposure are declared in one
and a seed's file is data, so those print nothing, and stderr says `name: no
SQL file of its own`. A node of a package `dbt deps` has not installed is not
read from the project's own folder of the same name: stderr says to run it.
A file that cannot be read is said on stderr too, `name: cannot read path:
reason`, with the card still printed and the exit code unchanged. The SQL is the
Jinja as written: the compiled SQL is under `target/compiled`, where the
manifest names it.

### As JSON

`--output json` prints one object per node, one line each, with dbt's key
names, and leaves out every key that is empty:

| Key | What it holds |
| --- | --- |
| `unique_id`, `name`, `resource_type`, `package_name` | who it is |
| `original_file_path`, `patch_path` | the file it is written in, as dbt gives it (inside its package for a package's node), and the properties file that documents it |
| `path` | where that file is, or would be, under the project, only when it is not `original_file_path`: `dbt_packages/<package>/...` for an installed package's node |
| `materialized`, `disabled` | how it is built; `true` when disabled |
| `database`, `schema`, `alias`, `relation_name` | where it is built |
| `incremental_strategy`, `unique_key` | an incremental model's only |
| `description`, `tags` | |
| `columns` | `name`, `data_type`, `description`, `tests` (the names of the test nodes), `undeclared` (`true` for a column only the catalog has) |
| `depends_on.nodes` | the unique_ids of its parents, sorted |
| `children` | the unique_ids of its children, sorted |
| `tests` | each test on the node: `name`, `test_name` (absent for a singular one), `column`, `host` (the unique_id of the node that declares it, only when that is not the node described: the `column` is its), `warn`, `settings` |
| `upstream_total`, `downstream_total` | the reach, tests excluded |
| `test` | on a test node: `checks` (the generic's name, or `singular`), `column`, `host` (the unique_id of the node it is declared on), `severity`, `generic`, `settings` |
| `sql` | with `--sql`, the file, whole, once it was read |

## The project

`--overview` prints what the project holds on one screen, the first thing to
ask of a project nobody has described yet:

```
dbt-edith path/to/project --overview
dbt-edith path/to/project --overview --depth 2 --output json
```

On Jaffle Shop:

```
project         jaffle_shop  dbt 1.11.16  adapter duckdb
manifest        /path/to/jaffle_shop/target/manifest.json  written 2026-10-19T17:14:10Z (1 min ago)
catalog         /path/to/jaffle_shop/target/catalog.json  38 columns typed
column lineage  none
nodes           5 models, 3 seeds, 23 tests
models          3 view, 2 table
macros          500, 0 the project's
selectors       around_stg_orders, changed (not resolvable here), customers_and_upstream, models_without_staging, orders_buildable, orders_cautious, orders_eager, orders_empty, orders_without_recon, recon_for_customers, recon_tests, shorthand_and_strings, staging_and_below, staging_children, staging_grown, +2 more
tags            raw 3, staging 3, recon 2
folders         (by folder, 3 levels)
  models     5 models (3 view, 2 table), 20 tests
    staging  3 models (3 view), 8 tests
  seeds      3 seeds
  tests      3 tests
```

| Line | What it says |
| --- | --- |
| `project` | the project's name, from `dbt_project.yml` when the manifest, from a dbt before 1.6, does not give it, the dbt that wrote the manifest and its adapter |
| `manifest` | its path, when it was written and how long ago |
| `catalog` | the catalog merged and the columns it typed, or `none`: column types then stay as YAML declares them |
| `column lineage` | each cache beside the manifest, who wrote it and when; one line each, newest first, or `none` |
| `nodes` | the enabled nodes by kind, then the disabled ones, which are counted nowhere else |
| `models` | the enabled models by materialization, most first; the incremental ones give the strategies the manifest names, `1 incremental (merge 1)`, and none in parentheses when it names none |
| `macros` | all of them, dbt's and the adapter's included, then the project's own once the manifest or `dbt_project.yml` names the project |
| `selectors` | the named selectors of `selectors.yml`, fifteen at most then `+N more`, marking the ones this build cannot resolve, since they compare with another run |
| `tags` | the tags of every enabled node, tests included, since `tag:` selects a test by the tags it is given: most used first, fifteen at most, then `+N more` |
| `folders` | every enabled model, source, seed, snapshot, test, exposure and analysis filed under the folders of its file, each folder counting its whole subtree |

`--depth N` sets how many folder levels `folders` goes down, three by default;
`--depth 0` leaves it out. The counts of a folder are the same whatever the
depth. A package's node whose file `dbt deps` has not put on disk is filed under
`dbt_packages/<package>`, where it would be, so it never counts among the
project's own. Numbered layer folders (`01_staging`, `02_marts`) sort the way
they are numbered.

`--output json` prints one object with the same facts as keys, a count each:
`project`, `dbt_version`, `adapter`, `manifest` (`path`, `written`),
`catalog` (`path`, `columns_typed`), `column_lineage` (a list of `file`,
`source`, `producer`, `written`), `nodes`, `materializations`,
`incremental_strategies`, `macros` (`total`, and `project` once the project is
named), `selectors` (`name`, `description`, and `unsupported` with the
reason), `tags` (all of them) and `folders`, a flat list in tree order,
parents before their children, each with its `path`, its `depth` and the
counts of its subtree, a zero left out.

## For an agent

What an agent needs of a dbt project is mostly a few of these calls, in this
order. None of them runs dbt, touches a warehouse or needs a profile, so they
are safe to run unasked.

| To know | Ask |
| --- | --- |
| what the project is | `dbt-edith . --overview`, once, first |
| what a model, source or test is | `dbt-edith . --describe name[,name...]`, on the nodes the question names; add `--sql` to read the model |
| a model's neighbours | `dbt-edith . -s 'model+1' --exclude resource_type:test --output compact`; `+model` for its parents |
| how many nodes sit in each layer upstream of a model | `dbt-edith . -s '+model' --exclude resource_type:test --output compact \| cut -f4 \| cut -d/ -f1-3 \| sort \| uniq -c` |
| what a column feeds | `dbt-edith . --column model.column --down all --output compact`; `--up all` for what feeds it |
| whether the answer still holds | the `freshness:` line of stderr; run `dbt parse` when it says `stale` |

Three habits keep the answers small and true:

- **Leave the tests out unless they are the question.** A test joins a
  selection whenever its parent is selected, as it does in `dbt ls`, and a
  project often has more tests than models. `--exclude resource_type:test`
  drops them, and `--describe` already lists the tests of a node.
- **Prefer compact for lists and the card for one node.** A compact line is
  a fraction of the same node's JSON line, which spends most of its bytes on
  the unique_id, the fqn and the package, all of which the name and the path
  say. Take `--output json` when a script reads the keys, or a column's cut
  matters.
- **Confirm before anything irreversible.** The answer is the manifest's. Before
  a run, a drop or a deletion acts on a selection, `dbt ls` with the same line
  says what dbt itself selects
  ([0057](../decisions/0057-a-selection-is-answered-on-the-command-line.md)).

The calls are plain commands with no protocol, so a skill that wraps them is
the caller's own: the recipes above are what it would hold.
