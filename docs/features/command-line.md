# Command line

Six questions dbt-edith answers without a page: what a selection matches, what
a column reaches, what a node is, what the project holds, what a job runs, and
which dependencies the jobs leave unordered. Each reads the
manifest dbt already wrote, prints and exits, so a script or an AI agent can
ask with one command and read the answer in few tokens. An agent with no
terminal can ask the same questions [over MCP](#over-mcp).

## Without a server

`--select` and `--selector`, `--column`, `--describe`, `--overview`, `--job`
and `--job-order` have this in common
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
  | 2 | nothing was asked: no project, no manifest, an unreadable one, a `--catalog` that is not a file, or a wrong flag; for `--job` and `--job-order`, no `.dbt-edith/jobs.yml`, or one that does not read |

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
  header. `--describe`, `--overview`, `--job` and `--job-order` print text
  unless it is `json`: a card is read, so `compact` prints the same text.

A call costs one process start and one read of the artifacts: the manifest for
a selection or a column, and for `--describe` and `--overview` the catalog and
the caches beside it too, which is why those two take longer on a project with
a wide catalog. The job files are read only by a project that has a
`.dbt-edith/jobs.yml`, and then once per call. The freshness walk runs after the answer is out, so it never
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

### Counting a selection

`--count-by KEY` counts what the selection matched instead of listing it, for
the questions whose answer is a number: how many models sit in each layer, which
sources feed this model. It goes with `--select` or `--selector` and takes
`--exclude`, and the selection is resolved, refused and signed on stderr exactly
as a list is
([0112](../decisions/0112-a-selection-may-be-counted-instead-of-listed.md)).

```
dbt-edith path/to/project -s '+customers' --exclude resource_type:test --count-by folder
```

```
count	folder
1	models
3	models/staging
3	seeds
7	(all)
```

| Key | Counts | Rows |
| --- | --- | --- |
| `kind` | dbt's resource type | `model`, `source`, `seed`, `test` |
| `materialized` | how it is built | `view`, `table`, `incremental`, `source`, and `unset` where the manifest says nothing |
| `source` | the source a source table belongs to, by the name the manifest gives it | the source's name, and `(not a source)` for every other node |
| `folder` | the folders of its file, the file's own name left off, down to `--depth N` levels (default 3) | `models/staging`, and `(no folder)` for a file at the project's root; an installed package's node whose files `dbt deps` has not put on disk falls under `dbt_packages/<package>`, as in `--overview` |
| `tag` | each tag, once per tag a node carries | the tag, and `(no tag)` |
| `package` | the package it belongs to | the package's name, and `(no package)` where the manifest names none and neither does the project |

Rows go by count, the most first, then by value. `folder` goes in path order, so
numbered layers (`01_staging`, `02_marts`) read in order. A row counts a node
once, in its own folder cut at `--depth`, and not in the folders above it:
unlike `--overview`, where a folder counts its whole subtree, `models` above
holds the one model that sits directly in it. The last row, `(all)`,
is the number of nodes selected: a node with two tags is in two `tag` rows and
once in `(all)`, so those rows can add up to more. `--depth` goes with
`--count-by folder` and is refused beside any other key; `--depth 0` is refused
there too, since it counts no folder.

`--output name` and `--output compact` print the same text. `--output json`
prints one object, `depth` only for `folder`:

```
{"by":"folder","depth":3,"total":7,"counts":[{"key":"models","count":1},{"key":"models/staging","count":3},{"key":"seeds","count":3}]}
```

A selection that matches nothing prints no row, says so on stderr and exits 0;
as JSON it prints the object with `total` 0 and an empty `counts`.

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
2. else the nodes whose name is exactly that, `dim.v2` for a version of a
   model, together with, for a versioned model's bare name, `dim`, each
   package's latest version of it, as a `ref()` reaches it (0104); and, for a
   name with a dot in it, those whose unique_id ends with a dot and the name:
   `shop.orders` for a package's; a source's `raw.payments` and a package
   `raw`'s model `payments` are then two nodes, refused rather than guessed;
3. else the same, case ignored;
4. among several, the enabled ones, then, for `--describe`, a model, seed,
   snapshot or source over a test or an exposure; still several, refused.
   A bare name that is the latest version in one package and a seed, a model
   or a versioned model in another is therefore refused, as it is for any two
   packages, and a disabled model of that name gives way to the versions.

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
| `relation` | where it is built by this parse's target: dbt's `relation_name`, else the database, schema and alias |
| `configured` | the database, schema and alias the project writes, the ones that are set, when any differs from the built one (before dbt's `generate_*_name` macros, which are not evaluated): `database {{ env_var('DBT_DB_SHOP') }}  schema marts`. A model, seed, snapshot or source that is built (not an ephemeral one) only |
| `.env.qa`, one line per file | what the project's config writes for a node whose location reads a variable, with that file's values filled in: `SHOP_QA.marts_qa.fct_orders`. It is the configured value before dbt's `generate_*_name` macros, not the relation dbt builds there. A database or schema nothing writes is `<target>`, the target's, which this parse does not know for another environment (a source's schema is the exception: dbt's own default, its name, in every environment, so it prints as parsed); an alias nothing writes is the one this parse built. A variable the file lacks and `env_var()` gives no default reads `<DBT_DB_SHOP not set>` in its place, one it defines as a placeholder (`TBD`) `<DBT_DB_SHOP placeholder>`, a value withheld `<hidden>`, and `?` stands for Jinja that is not evaluated (an `{% if %}` with no branch to pick, any other expression, a `DBT_ENV_SECRET_*` variable) or for an empty value. A value taken from one branch of an `{% if %}` by what dbt parsed, as the Catalog does, ends in `~`: the condition was not evaluated. The parts are joined by dots, so a value with a dot in it reads ambiguously; the JSON is the exact form |
| `package` | the package it comes from, when it is not the project's |
| `tags` | comma-separated |
| `yml` | the properties file that documents it |
| `incremental` | an incremental model's strategy as its box shows it, when it was read, then its `unique key` |
| `description` | its own text, whole, with its line breaks |
| `columns (N)` | one line per column, tab-separated: name, type, tests, description on one line, the empty fields at the end left off with their tabs. A column only the warehouse has, from the catalog, says `(catalog only)` after its type |
| `parents (N)`, `children (N)` | `name (materialized)`, a source or an exposure saying its kind, sorted by name |
| `tests (N)` | each test on the node: `name: generic on column`, `name (singular)` for a query of its own, then `warn` and each setting off dbt's default (`where x`, `limit 5`); a test declared on another node, as a `relationships` is on the model it points at, names that node: `on orders.customer_id` |
| `reach` | how many nodes lie upstream and downstream, tests excluded; a source has only downstream |
| `jobs (N)` | only in a project with a `.dbt-edith/jobs.yml` ([below](#a-job)): the steps of the orchestrator's jobs that run it, scheduled jobs first, `DAG shop_daily › task marts (scheduled 0 5 * * *) via +tag:daily` or `job Daily build › step 3/3 (manual)`, the terms of the step's selection that reached it after `via`; `none` when no step does and every source was read, then `N steps not resolved` for the ones that could not be read or resolved and could act on it, `N job files not read`, and `1 of 3 sources (nothing at this path) not read` for a source whose folder or file could not be walked or was cut at a bound, which says `jobs  not read: 1 of 3 sources (...)` instead of `none`. A file with `jobs: []` says `no sources in .dbt-edith/jobs.yml`, which is not `none`. A file that does not read gives `not read, .dbt-edith/jobs.yml:` and the reason in its place. Left off a test, an exposure and the nodes with no reach line |

A test's card says what it checks and how it is set instead:

| Line | What it says |
| --- | --- |
| `checks` | `not_null on orders.order_id`, the model it is declared on and the column, or `a query of its own` |
| `severity`, `settings` | `warn`, and the settings off dbt's default |
| `generic` | `dbt_utils.unique_combination_of_columns` for a generic named through a package |
| `parents (N)` | the nodes it reads, by name |

The per-file lines are the Catalog's resolved column for each environment (see
[the Catalog](catalog.md#environments)), through the same guards and two more
(0113). They show the files the Catalog shows by default: a template or a
backup, and a file that defines none of the variables the project's locations
read, are left out. They are the location as written, with the variables filled
in: a `generate_database_name`, `generate_schema_name` or `generate_alias_name`
macro runs after the config, and none is evaluated. `relation` is dbt's own
name for this parse's target; a per-file line is the configured value before
those macros. dbt's default `generate_schema_name` builds a custom schema
`stg_qa` as `<target schema>_stg_qa`, and a project may override any of the
three, so a line is not where that environment builds the node. Only the
resolved locations leave a `.env` file this way (0113): a variable named like a
credential (`DBT_API_KEY`), a `DBT_ENV_SECRET_*` one and a value shaped like
one (a URL with a password in it) are never printed, and nothing else of the
file is. A written key that reads such a variable shows `<hidden>` on the
`configured` line, expression and default included. Over MCP the files are the
server's project's.

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
| `configured` | `database`, `schema`, `alias` as the project writes them, the keys that are set, only when one differs from the built one |
| `by_env` | for a node whose location reads a variable, per `.env` file that is not hidden: `database`, `schema`, `alias` the config resolves to with that file's values, before `generate_*_name`, and, apart, the keys that did not resolve: `not_set` and `placeholder` (variable names), `from_target` (keys nothing writes), `branch` (keys whose value, still given, is one branch of an `if` picked by what dbt parsed) and `unresolved` (keys left out because Jinja was not evaluated, the value is empty or it is withheld). A `generate_*_name` macro runs after it and may change the value, as dbt's default does a custom schema |
| `incremental_strategy`, `unique_key` | an incremental model's only |
| `description`, `tags` | |
| `columns` | `name`, `data_type`, `description`, `tests` (the names of the test nodes), `undeclared` (`true` for a column only the catalog has) |
| `depends_on.nodes` | the unique_ids of its parents, sorted |
| `children` | the unique_ids of its children, sorted |
| `tests` | each test on the node: `name`, `test_name` (absent for a singular one), `column`, `host` (the unique_id of the node that declares it, only when that is not the node described: the `column` is its), `warn`, `settings` |
| `upstream_total`, `downstream_total` | the reach, tests excluded |
| `jobs`, `jobs_unresolved`, `jobs_files_refused`, `jobs_not_read`, `jobs_sources`, `jobs_refused` | with a `.dbt-edith/jobs.yml`, for the nodes that have a reach: `jobs` the steps that run it, each `kind` (`airflow` or `dbt_cloud`), `job`, `step`, `schedule`, `via` and `file`, an empty list when none does; `jobs_unresolved` the steps not resolved that could act on it and `jobs_files_refused` the job files not read, when not zero; `jobs_not_read` the sources that were not read (`sources`, `of`, `reason`), so an empty `jobs` is not an answer; `jobs_sources` how many sources the file names, when none or when some were not read; `jobs_refused` why the sources file was not read, instead of `jobs` |
| `test` | on a test node: `checks` (the generic's name, or `singular`), `column`, `host` (the unique_id of the node it is declared on), `severity`, `generic`, `settings` |
| `sql` | with `--sql`, the file, whole, once it was read |

## A job

`--job NAME` prints one DAG or dbt Cloud job step by step, with the nodes each
step runs and the order its files declare, the question the other way round
from a card's `jobs` line
([0115](../decisions/0115-job-sources-may-live-in-the-project.md),
[0124](../decisions/0124-a-jobs-answer-carries-its-order-as-the-files-write-it.md)):

```
dbt-edith path/to/project --job jaffle_daily
dbt-edith path/to/project --job 'Nightly build' --output json
```

The jobs are the ones the project names in its own `.dbt-edith/jobs.yml`, which
the Jobs settings write when you choose to keep the sources in the project
([Jobs](jobs-and-reports.md#jobs)), read as the Jobs tab reads them over the
same files, and never the settings: the command line reads none. Without that
file the call exits 2 with `no .dbt-edith/jobs.yml in this project; name the
sources in the Jobs settings`, and with one that does not read, exits 2 with its
reason and line. The call never runs dbt.

```
DAG jaffle_daily  scheduled 0 6 * * *  jaffle_daily.yml:2
  task load_seeds  dbt seed  (1 node)
    order: before stage
    seed raw_orders
  task stage  dbt run --select tag:staging  (2 nodes)
    order: after load_seeds · before marts
    model stg_customers
    model stg_orders
  task marts  dbt run -s +customers orders --exclude tag:staging  (2 nodes)
    order: after stage · before reconcile · updates jaffle_marts, which starts jaffle_on_marts
    model customers
    model orders
  task reconcile  dbt test --selector recon_tests  not resolved: no selector named ...
    order: after marts
nodes 5
```

A job is found by a DAG id or a dbt Cloud job's name, written exactly; else with
the case ignored; else by the identifier dbt-jobs-as-code gives a dbt Cloud job.
Several jobs of the name are refused with their sources (`nightly: shop_daily`)
and none with the nearest names, as `--describe` does, exit 1. A task is no job.

The first line is the job, how it starts and where it is written, as the Jobs
tab's pill puts it: `scheduled 0 6 * * *`, `schedule not read: ...`, what
starts it when no clock does (`CI`, `on merge`, `on an_asset`), or `manual`.
Under it each step in order: its command, as parsed (the values of `--vars`
and `--args` are `…`), then how many nodes it runs and the nodes themselves,
`kind name`, sorted by name. A step that could not be read or resolved says why
instead; one that only compiles, lists or previews says `dbt compile runs no
node`. `nodes N` closes the answer: the distinct nodes the steps run, once
however many steps run them.

What the files say of the order is printed, nothing more: it is not followed
from job to job and nothing is checked. Under a DAG's first line come its
sensors, `sensor wait_marts  waits for jaffle_daily › marts` (the sensor's id,
which a task's `depends_on` names it by, then the DAG and the task it waits for,
`D › a, b` for the tasks of `external_task_ids`, `D › task group g, whose tasks
are not read` for an `external_task_group_id`, the DAG alone when it names
neither and so waits for the whole run; a sensor written without an id leaves
the id out), then `updates X, which starts D` for what its tasks with no
dbt command update and the DAGs scheduled on it, and for a dbt Cloud job
`starts A, B when it ends`. Under a step, an `order:` line, left out when it
has nothing to say: `after` the tasks its `depends_on` names, as written, a
sensor's id or a task that runs no dbt included; `before` the tasks that run
dbt and name it, a task that runs none left out as the Jobs tab leaves it;
`awaited by` the sensors of any DAG, its own included (a sensor on its
previous run), as `dag › sensor`, that wait for it, a sensor on the whole run
marked `(whole run)` on every task of the DAG instead of read as that many
waits, and a sensor on a task group on none, since which tasks it holds is not read;
`updates` the assets it writes and the DAGs scheduled on them. The file's order
is not the run order: every task with no `after` starts with the DAG's run. A
sensor may also hold back tasks that do not name it, which the factory that
builds the DAG decides: dbt-edith prints what the file says. A dbt Cloud job's
steps do run in the order they are listed. The sensor's task is read from
`external_task_id` (the key a source may rename), else `external_task_ids`, else
`external_task_group_id`, the last two under Airflow's own names.

`--output json` prints one object: `kind`, `name`, `source`, `ident` (a dbt Cloud
job's), `file`, `line`, `schedule`, `steps` (each `n`, `name`, `line`,
`command`, `unresolved`, `runs_nothing`, `warnings`, `upstream`, `downstream`,
`waited_by`, `outlets`, `feeds`, `node_count` and `nodes`, a list of `id`,
`kind` and `name`) and `nodes`, the total, and `not_read` when a source was not
read: the text answer then ends on a `not read:` line, and a refusal for a name
no job has says it too, since the job may be in that source. At the job's level
`sensors` (each `id`, `dag` and, structured, `task` for one task, `tasks` for
several, `task_group` for a group; none of the three is a sensor on the whole
run of `dag`), `assets` with `assets_unread`, `after` with
`after_unread`, `triggers`, `outlets` and `feeds`, each left out when empty.
The step's `waited_by` is a list of `dag › sensor` strings, the Jobs tab's
wording, with `(whole run)` after a sensor on the whole run.
The file is the one [Jobs](jobs-and-reports.md#jobs) describes: paths from the
project, never hidden, never a profile, never the home folder, a link along the
path included.

## The order between jobs

(Reports' Job order section draws the same answer, over the sources in force,
the settings' included: [jobs-and-reports.md](jobs-and-reports.md#reports).)

`--job-order` lists the dbt dependencies between the steps of the jobs that
start on their own which the job files do not order
([0125](../decisions/0125-the-jobs-are-checked-for-the-dependencies-they-do-not-order.md)):
a step that builds a model whose parent another step builds, with nothing in
the files making it wait for that step. It is the question an agent otherwise
answers by joining `--job` for every DAG with the parents `--describe` gives and
the YAML, by hand.

```
dbt-edith path/to/project --job-order
dbt-edith path/to/project --job-order --output json
```

It reads the jobs as `--job` does, from the project's own `.dbt-edith/jobs.yml`,
and exits 2 in the same cases, with the same sentences. An answer exits 0
whether it finds a pair or none. It never runs dbt.

```
pairs 2: 1 nothing orders them, 1 only the clock, 0 the other way
nothing orders them  1
  DAG jaffle_marts › task build  should wait for  DAG jaffle_stage › task stage
    scheduled 0 6 * * *  ·  scheduled 0 6 * * *
    customers reads stg_customers
    orders reads stg_orders through int_orders
only the clock orders them  1
  DAG jaffle_late › task build  should wait for one of  DAG a › task x, DAG b › task y
    scheduled 0 7 * * *  ·  scheduled 0 6 * * *, scheduled 0 5 * * *
    orders reads stg_orders
2 steps not resolved, which could hide more
1 job nothing starts left out: jaffle_full_refresh
```

**What counts as a dependency.** A step is a consumer of the nodes it builds and
a producer of the same, where a built node is a model that is not a view, an
ephemeral or a dynamic table, a seed or a snapshot, and not disabled. A
materialized view is built: dbt refreshes it on every run, so the step that
runs it is the one a reader waits for. Each parent of a consumed
model is looked at. When the step builds the parent itself, dbt orders them. When
another step builds it, that step is one it should wait for. When the parent is
a view, an ephemeral or a dynamic table, whoever selects it, the walk goes on
through it to its own parents (an ephemeral is inlined into the model that reads
it, a view reads its parents live, and a dynamic table refreshes on its own
lag); what is crossed is named after `through`. A source, or a table or incremental model that no step builds, ends
the walk. Tests are neither consumers nor producers, and a step that only
re-creates one of them consumes nothing.

**What counts as ordered.** The files order a step after another when the second
is an ancestor of the first in a graph made of:

- an Airflow DAG's tasks, the tasks with no dbt command, its sensors and a start
  per DAG: each task after its DAG's start, and after the tasks its
  `depends_on` names;
- a sensor, after each task it names in the DAG it names (`external_task_id`,
  or every one of `external_task_ids`), or after every task of that DAG only
  when it names neither a task nor a group. A sensor on a task group
  (`external_task_group_id`) orders nothing, since which tasks the group holds is
  not read, and is never taken for the whole run. **A sensor orders only the tasks that name it,
  directly or through `depends_on`**, which is Airflow's rule: a sensor in the
  DAG waits, but holds back only the tasks that depend on it. Whenever the files
  put a sensor of this DAG after a step the pair should wait for (its task, the
  DAG, or a later task of it) and this task does not depend on it, the pair says
  so in a `note:` (`idle_sensors` in JSON; a sensor with no id, which no task can
  name, is always one, and reads "a sensor" in the note). A sensor on its own DAG waits
  for the previous run and adds no order within this one. When the step depends
  on a sensor that waits for a task group of a DAG holding a step it should wait
  for, the pair says so in a `note:` too (`group_sensors` in JSON), since the
  group may well hold that step and its tasks are not read;
- a DAG scheduled on assets, after the tasks of any DAG that update one of them,
  and after a DAG with no dbt task that does. An asset updated by several tasks
  starts a run on the first update, so only what all of them are after is before
  it; across the DAG's assets it waits for all;
- a dbt Cloud job's steps, each after the one written before it, and a job
  started by another's end, after that job's last step.

A step is never compared with itself. A DAG whose `depends_on` loops is not one
Airflow loads: it is left out as a consumer and as a producer, and listed as
`DAG x has a depends_on cycle, which Airflow does not load: left out`.

**The three kinds of pair.**

| Group | Meaning |
| --- | --- |
| `none`, "nothing orders them" | Nothing the files say, and not a clock either: the same DAG or job, the same cron in the same zone, a side scheduled on an asset or a trigger, or a schedule that was not read. |
| `clock`, "only the clock orders them" | A step it should wait for is in another job, both jobs have a cron, and the (cron, zone) differ. Both schedules are shown; the tool never works out which cron runs first. |
| `reversed`, "ordered the other way" | Every step it should wait for runs after it, by the files: the model reads what a later step builds. When only some do, the pair keeps its group and lists them (`later` in JSON, `T runs after it` in text). |

A cron is compared with its spaces collapsed and a preset spelled out
(`@daily` is `0 0 * * *`), in the zone of its Airflow source, UTC when it has
none, and a dbt Cloud job's is UTC ([Jobs](jobs-and-reports.md#jobs)). A
schedule is shown with its zone when that is not UTC. Pairs are listed by group
as above, then by the consumer's job and step, then by what it should wait for.
A pair lists its edges, `orders reads stg_orders`, at most five in text and then
`+N more`; JSON lists them all.

**Scope.** Only the steps that execute (`run`, `build`, `test`, `seed`,
`snapshot`, `source freshness`) of jobs that may start on their own count, a
job a clock starts, or one only a schedule not read, an asset or a trigger may
start. A job nothing starts (manual, CI, merge) is left out on both sides, or a
single ad hoc run of `+x` would flag the whole project; the last line of the
answer names them. `N steps not resolved` counts the steps of the scoped jobs
that could build a node and could not be read, since each could hide a pair,
and `not read:` says when a source was not read, as `--job` does. On a project
with many jobs the text answer is long: its first line gives the counts, so a
cut answer still says how much was cut, and `--output json` is the shape to
filter.

**Approximations**, none of them coded:

- a DAG with no dbt task that updates an asset is one point, and its own
  sensors and `depends_on` are not read; a sensor on any of its tasks, or on
  its whole run, counts as after all of it;
- a DAG started by a `TriggerDagRunOperator` reads as manual and is left out,
  and an Airflow task that starts a dbt Cloud job is not read;
- a sensor between two different crons, without the `execution_delta` or
  lookback that fits them, may wait for a run that never comes, and still
  counts as ordering;
- `trigger_rule` is not read, so a task that runs on a failed upstream counts
  as after it;
- an asset updated by several tasks is as above, not the update that really
  started the run.

`--output json` prints one object: `configured` (the sources file names at
least one source, which tells an empty answer from no job read at all),
`checked` (how many steps were looked at: those of jobs that start on their own
that build a node, which tells an empty answer that holds from one that looked
at nothing), `counts` (`none`, `clock`, `reversed`), `pairs`, `unresolved`, `left_out` (each
`source`, `job`), `cyclic` (the same) and `not_read`, when a source was not
read. An `unresolved` entry has `source`, `job`, `label`, `shown` (the command as
parsed), `file`, `line` and `why`, to be listed and checked by hand. A pair has
`group` (`none`, `clock` or `reversed`), `step` and `waits_for` (a list), each
side being `kind`, `job`, `source`, `label` (`task build`, `step 2/3`),
`schedule`, `cron`, `timezone` (empty ones left out), `file` and `line`;
`edges`, each with `node`, `node_name`, `parent`, `parent_name` and `through`, a
list of `id` and `name`; and, when there are some, `later` (sides),
`idle_sensors` (each `id`, absent for a sensor written without one, and `waits`,
what it waits for as the Jobs tab words it) and `group_sensors` (the same, and
`task`, the step's task that depends on it). The text's notes are built from
these.

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
| `jobs` | only in a project with a `.dbt-edith/jobs.yml`: how many jobs it names and their names, `12 from .dbt-edith/jobs.yml: shop_daily, ..., +N more`, fifteen at most, in the files' own order, then `1 of 3 sources (...) not read` when a source could not be read; `no sources in .dbt-edith/jobs.yml` for `jobs: []`; `not read, .dbt-edith/jobs.yml:` and the reason when it does not read |
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
reason), `jobs` (all of them: `kind`, `name`, `schedule`, `steps` and `file`,
present only with a sources file, `jobs_sources`, `jobs_not_read` for sources
that were not read, and `jobs_refused` in its place when the file does not read), `tags` (all of them) and `folders`, a flat list in tree order,
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
| a model's neighbours | `dbt-edith . -s 'model+1' --exclude resource_type:test --output compact` for its children; `1+model` for its parents, `1+model+1` for both |
| everything upstream, or downstream, of a model | `dbt-edith . -s '+model' --exclude resource_type:test --output compact`; `model+` for what dropping or renaming it breaks |
| how many nodes sit in each layer upstream of a model | `dbt-edith . -s '+model' --exclude resource_type:test --count-by folder`; `--count-by source` for the sources that feed it |
| what a column feeds | `dbt-edith . --column model.column --down all --output compact`; `--up all` for what feeds it |
| which DAGs or jobs run a model, when the project names its jobs | the `jobs` line of `dbt-edith . --describe model` |
| what a DAG or dbt Cloud job runs, and what each step waits for | `dbt-edith . --job name`, step by step |
| the dbt dependencies the jobs do not order | `dbt-edith . --job-order`, then `--output json` to filter |
| whether the answer still holds | the `freshness:` line of stderr; run `dbt parse` when it says `stale` |

Six habits keep the answers small and true:

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
- **On a big project, count and then narrow.** A list of hundreds of nodes, or
  several cards, can pass the limit a client puts on an answer. `--count-by
  folder` or `tag` says how much there is, then a narrow selection is listed
  and two or three names are described per call.
- **Put several `--exclude` terms in one quoted value, and know what
  `--count-by` counts.** The flag takes one value, as in `--exclude
  'resource_type:test resource_type:source'`. A count includes the sources,
  the seeds, the exposures a downstream selection reaches and the node the
  selection starts from, so those are excluded to count models, and the start
  is subtracted when it is selected.
- **For the jobs that touch a set of nodes, or the models no job runs, go job
  by job.** `--overview --depth 0 --output json` names every job, where the
  text stops at fifteen, `--job` lists the nodes each one runs, and the agent
  intersects them with the selection or subtracts them from it. It costs one
  call per job, so an agent with many says which it did not check. `--job` also
  gives each step's order, the sensors a DAG waits on and what its tasks update,
  as the files write them; whether the jobs run what feeds a model before it is
  no job-by-job question, and `--job-order` answers it for all of them at once.

Start from these commands, or from the skill that wraps them
([below](#the-skill)). Use MCP only where there is no terminal to run them in;
an agent without one can ask the same questions [over MCP](#over-mcp).

### The skill

The recipes and the habits above are shipped as a skill, a `SKILL.md` an agent
client loads
([0114](../decisions/0114-a-skill-is-shipped-and-printed-never-installed.md)).
`dbt-edith --skill` prints the one that matches the installed binary, and
nothing else: it takes no project, reads nothing, writes nothing and starts
nothing. Putting it where the client looks is the caller's step:

```
mkdir -p .claude/skills/dbt-edith && dbt-edith --skill > .claude/skills/dbt-edith/SKILL.md      # Claude Code
mkdir -p .github/skills/dbt-edith && dbt-edith --skill > .github/skills/dbt-edith/SKILL.md      # Copilot
```

In Windows PowerShell 5.1 `>` writes UTF-16, which a client may not read: run
it in Git Bash or cmd.

The same file is [docs/agent-skill/SKILL.md](../agent-skill/SKILL.md) in the
repository, with a literal `{version}` in a comment under its frontmatter that
`--skill` fills in with the installed version. dbt-edith never writes it into a
client's folder or settings. Print it again after updating dbt-edith: a flag the
installed binary refuses means it is older than the text. The tests check that
every long flag written anywhere in the file is one this version has, that the
keys it lists for `--count-by` are keys, that every selection it cites resolves
on a small graph, and that every command it cites parses and is accepted, over
MCP too. They do not check what a recipe means.

What the description says decides whether an agent loads the skill at all: a
one-line description was not loaded where one that names the questions it
answers was, so the shipped one lists them, the kinds of node, the lineage in
both directions, the column lineage, the jobs that run a node and the
overview, and says to use it before grepping SQL or running `dbt ls`. Change it
with care. It describes the card's per-`.env` lines as [the card](#the-card)
does, the config before the `generate_*_name` macros and not where dbt builds.
An agent that has no terminal, or whose client lists tools rather than skills,
needs no skill: it can ask the same questions [over MCP](#over-mcp), where
`help` is the reference and `--skill` is refused.

## Over MCP

Where the client has a terminal (Claude Code, Copilot in agent mode with
auto-approve), prefer the skill: the commands of
[For an agent](#for-an-agent). The server is the exception, for a client that
cannot run a command, such as Claude Desktop chat or a Copilot mode with no
terminal. It answers about twice as fast per call (4.9 ms against 9.8 ms on
Jaffle Shop, 34.9 ms against 53.0 ms on the Shopify fixture), but that is
milliseconds nobody waits on. Context does not decide it either: the tool's
definition is about 153 tokens in each conversation of a client that lists it up
front, the skill's description about 190 and its body about 800 more once
loaded. What decides it is in [What it is for](#what-it-is-for).

`dbt-edith --mcp` answers an MCP client over stdin and stdout instead of
printing one answer
([0110](../decisions/0110-an-agent-may-ask-over-mcp-through-one-tool.md)). It
is opt-in: nothing configures it, no file of a client's is written, and the
command line and [the skill](#the-skill) keep working beside it. The server
reads nothing until a call asks, so a client can be configured before
`dbt parse`, and it ends when the client closes its input.

```
dbt-edith --mcp [PROJECT] [--manifest path/manifest.json] [--catalog path/catalog.json]
```

The project, the manifest and the catalog are named here, once, and are the
same for every call; the defaults are the command line's.

### The tool

There is one tool, `edith`, taking one argument, `line`: the flags of the
questions above and `--output`, written as on the command line and with no
project.

```
--overview
--describe orders --sql
--job jaffle_daily
--job-order
-s 'stg_orders+1' --exclude resource_type:test --output compact
--column stg_payments.amount --down all --output compact
```

- **`help`** (or `--help`) answers the reference of every flag. It is not in the
  tool's description, so it enters the context only when the model asks.
- **No shell.** Quotes group words; a pipe, a redirection, `;`, `&`, a backtick
  or `$` outside quotes is refused with its reason. A count is asked with
  `--count-by`, so no pipe is needed
  ([0112](../decisions/0112-a-selection-may-be-counted-instead-of-listed.md)); a
  recipe that filters a list in a pipe is done on the answer by the one who
  reads it. A leading `dbt-edith` and `.` are dropped, so a recipe pastes as it
  is.
- **Only the questions' flags pass.** `--port`, `--manifest`, `--catalog`,
  `--column-lineage`, `--compiled`, `--shell`, `--no-open`,
  `--no-update-check`, `--mcp` and `--skill` are refused, and so is any flag added to the
  command line later until someone decides it is a question.
- **A selection holding spaces is quoted**: `-s 'stg_orders orders'`. The dbt
  habit `-s stg_orders orders` is answered with that advice, not with a wrong
  selection; a third bare word is clap's "unexpected argument".
- **`--run-collin` goes in a line** under the rule of
  [0099](../decisions/0099-a-column-may-run-collin-first-when-asked.md): it
  writes collin's cache beside the manifest, takes up to 600 s, and is stopped
  when the client cancels the call or closes its input. Because of it the tool
  is not marked read-only, and a client that approves the tool approves collin
  runs with it. On Windows a client that ends the server with
  `TerminateProcess` can leave collin running until it finishes.

The result is the command line's text in two blocks, stdout then stderr, an
empty one left out, with `isError` set when the exit code would not be 0. The
second block carries the lines of the command line's stderr: the manifest that
answered, when it was written, and `freshness:`. The answer is still the
manifest's: when the line says `stale`, run `dbt parse` and ask again: the
server reloads the manifest when its file changes. `column lineage capped at N
edges` is the one message that goes to the server's own stderr instead.

### Configuring a client

Claude Code, run in the project:

```
claude mcp add dbt-edith -- dbt-edith --mcp "$PWD"
```

The tool is then `mcp__dbt-edith__edith` in a permission rule. VS Code, in
`.vscode/mcp.json` of the workspace (or the user-level file):

```json
{ "servers": { "dbt-edith": { "type": "stdio", "command": "dbt-edith", "args": ["--mcp", "${workspaceFolder}"] } } }
```

On Windows the command is `dbt-edith` when it is on the PATH, else the whole
path of the `.exe`, with its backslashes doubled in the JSON. A client that
starts the server in another folder needs the project named, as above; a
relative `--manifest` is made whole against the folder the server started in.

### What it is for

It is for the clients that cannot run a command. Where one can, the skill is
the better choice. The tool is the cheaper in context (its
definition against the skill's description, and the skill's body once loaded),
but the skill needs no server to configure and works with any agent that has a
terminal. When both were tried with agents, what decided it was how the
agent finds them: Claude Code defers an MCP tool, showing the agent only its
name until it searches for it, and an agent given the tool then mostly ran
`dbt ls` instead; listed up front, the tool was used every time. A client that
defers tools therefore needs the skill, or the tool listed. What else was
weighed is in [0110](../decisions/0110-an-agent-may-ask-over-mcp-through-one-tool.md).

A client may cut a long answer (Claude Code stops at 25 000 tokens): ask for
`--output compact`, or a narrower selection.

On a large project the manifest load dominates a command-line call, which reads
it again each time. The server keeps the graph, and the jobs read, between
calls, so the later calls are the cheaper ones.

A client that defers MCP tools can pair the server with a small skill that
tells the agent to load the `edith` tool (with ToolSearch) and call it. That
variant, a `SKILL.md` using the tool instead of the commands, is not shipped. It
was tried only with Claude Code on Jaffle Shop: with it an agent used 1 to 3
calls where, without a skill, it used 9, and it did not fall back to `dbt ls`
in those runs.
