# Snowflake

Everything that only works on Snowflake, behind one switch. Snowflake's
column lineage is in [column-lineage.md](column-lineage.md).

## The switch

**Snowflake's features are a setting of the project**, a switch in the
Snowflake section of the settings, behind the gear in the top bar. They are marked alpha: none of them has yet been
checked against a real warehouse. Until you choose, they follow the manifest:
on when its adapter is `snowflake`, off otherwise. Off, Snowflake is not in the menu at all,
nor are the caches it wrote, and the server refuses every Snowflake route.
More features that only make sense on Snowflake will sit behind the same
setting, as the query history below does.

## Snowsight

With Snowflake's features on, a **Snowsight** menu sits beside each Copy of Catalog > Location: the
object, its schema and its database, each opened in Snowsight in a new tab. The
account is the one your profile's target connects to, which the first menu of
a page asks Snowflake for, so that click may bring a sign-in tab. The address
is built from the name, as Snowflake keeps it: an unquoted part in upper case,
a quoted one as written. A view opens as a view, a dynamic table as a dynamic
table, and the rest as a table, a source included, since the manifest does not
say what a source is
([0051](../decisions/0051-a-relation-opens-in-snowsight-by-its-name.md)).

## Clone into my schema

With Snowflake's features on, **Clone into my schema** sits beside the Location
heading of a model, a seed or a snapshot. It writes the statements that copy the
node and its direct parents from an environment into your own schema, so you can
work on it without building everything above it. Nothing is sent to Snowflake:
you read the statements, copy them and run them yourself.

- **From** is one of your `.env` files, each node's relation being the one the
  Location table resolves with it, or *built*, where this manifest's target put
  it. A node the file cannot place says why and is left out, rather than copied
  from somewhere else.
- **Into** is `database.schema`, filled in from your profile's active target:
  `DBT_TARGET` if dbt-edith was started with it, otherwise the profile's
  `target:`. What you type instead is kept for the project. Each copy keeps the
  name of what it copies, so your local `ref()` finds it.
- **As** is Clone, zero-copy for a table, an incremental model, a seed or a
  snapshot, or Create as select, a real copy of the rows. A view is always
  copied as a table of its rows, after dropping a view of that name a local run
  may have left. Every copy is transient, since Snowflake cannot clone a
  transient table, which is what dbt builds, into a permanent one.

An ephemeral parent has no object, so what it reads is copied instead. Sources
are listed and left where they are. Untick a node to leave it out. The notes
under the table say when the manifest is behind your files, so the parents
listed may not be the ones the model reads now
([0058](../decisions/0058-a-model-is-cloned-into-your-schema-by-statements-you-run.md)).

## Query history

With Snowflake's features on, the dock has a **Query history** tab: the queries
your Snowflake user ran in the last seven days, from dbt in the terminal, from
Snowsight worksheets or from anything else, newest first. It is read from
Snowflake when you open the tab, which is the click that may bring a sign-in
tab, and again when you press Refresh; nothing polls, and nothing is kept once
the page is closed. The same script that fetches column lineage reads it, with
the same profile and target, so a column lineage tool other than Snowflake
changes nothing here. It is alpha, like every Snowflake feature.

Twenty queries show at first, and **Load 20 more** goes further back, until
Snowflake has nothing older: it gives back your newest 10 000 queries of the
week, so a role you rarely use may run out before the week does. Each row says
when the query started on your clock, how it ended, how long it took, its role,
warehouse and type, the first line of its SQL with dbt's leading comment left
out, and the error under a failed one. Click a row for the statement itself, the
first 10 000 characters of it, with buttons to copy it or its id. **Snowsight**
opens the query's own page there, in a new tab, with its profile and every
detail this tab does not have.

The **Role** menu decides whose queries: by default the role of the target the
script connected with, then the role of every other target in your profile that
signs in as the same user, then every role. A target that signs in as another
user, a service account for instance, is listed greyed: its queries are that
user's history, not yours. So is a target whose role is a Jinja expression,
which the script does not evaluate.

The queries dbt-edith runs itself carry the query tag `dbt-edith`, and are left
out. Reading the history needs no privilege beyond your own role: it is
`INFORMATION_SCHEMA.QUERY_HISTORY_BY_USER`, which shows any user their own
queries ([0048](../decisions/0048-the-query-history-is-read-live-and-never-kept.md)).

## Last built

With Snowflake's features on and the **dbt_artifacts** package in the project
(its `model_executions` and `invocations` models in the manifest), dbt-edith
can say **when each model last ran in each environment** and how it ended. It
reads the package's own tables, in the schema each `.env` file places the
package in, the way the Location table places any model. Three places show it:
the Catalog's **Last built** section, a dot on each lineage box for the
environment picked in the selector, and the **Builds by environment** section
of Reports. See [catalog.md](catalog.md), [lineage.md](lineage.md) and
[jobs-and-reports.md](jobs-and-reports.md).

It is read **only when you press Read from Snowflake** (Refresh once there is a
read), one environment after the other, and a sign-in tab may open for the
first. Nothing is read at startup or polled, and nothing is kept: the answer is
in the page until it closes. Per model and target it brings the last run's
status, when the invocation started and when the model began and ended, the
last success, the invocation id, whether it was a full refresh, rows affected,
dbt's message (cut to 600 characters), the dbt command, the target's name and
the dbt Cloud job and run ids. It never reads the invocation's `env_vars`,
`dbt_vars`, `dbt_custom_envs` or `invocation_args`, nor a model's
`adapter_response`. The last 90 days are looked at, and at most 50 000 rows (a
model and a target each) per environment; past that the answer says it was cut.
A read stops at the first environment whose connection or script fails, since
the rest would fail the same way and could each open a sign-in tab; the ones
not asked say so.

An environment whose `.env` file cannot place the package says why and is not
asked. Several files that place it in one schema are asked once. One table may
hold the runs of several targets. A file that sets `DBT_TARGET` stands for that
target, the one dbt runs with when the file is loaded, so its environment keeps
only that target's runs and is named with it ("DEV · target dev"). Without it,
and for the manifest's own build, each target's run is shown by name, and a
lineage dot takes the worst of them (error, then stale, then not built, then
ok), the box's hover card listing each. The runs under other targets are not
counted, but their names are shown beside the environment, so a file whose
`DBT_TARGET` no run used says so instead of reading as never built. Every run
under one target is one, whoever ran it: two developers building `dev` into
their own schemas share one last attempt.

A model is **stale** when its last success is older than the threshold, 7 days
until you change **Stale after** in the Snowflake section of the settings (1 to
365). It is in **error** when its last attempt failed, **ok** otherwise, and
**not built** when no run is recorded in the window. A skipped run built
nothing that time: the last success still decides
([0117](../decisions/0117-the-last-build-per-environment-is-read-from-dbt-artifacts.md)).
The schema the package is placed in is the config before `generate_schema_name`
runs, so a project whose macro rewrites it sends the query to the wrong table,
which Snowflake's error then names.

## If something looks off

| Symptom | Cause | Fix |
| --- | --- | --- |
| the column lineage menu says `Snowflake · failed` | the script could not start; its tooltip and the Columns tab say why | usually no `snowflake-connector-python` in the Python it found, or no `profiles.yml` it can read |
| Snowflake is not in the column lineage menu | Snowflake's features are off for the project, which is the default when the manifest's adapter is not `snowflake` | switch them on in the Snowflake section of the settings, behind the gear in the top bar |
| a clicked column comes back with no lineage | the object was not built by a query Snowflake could analyse, or the role cannot see it | check with `sf_lineage.py probe`, and check the environment pill names the objects you mean |
| a query in Query history has no Snowsight link | Snowflake gave no organization name for the account | copy the query id from its row and filter Snowsight's own history by it |
| Last built says an environment "is not defined in .env.x" or "was not resolved against" it | the `.env` file cannot place the package's tables, as the Location table could not | define the variable in that file, or read the manifest's own environment |
| Last built has no run for a model that did run | the package's tables of that environment are in another schema than the config says, or the run is older than 90 days | open the table in Snowsight; the section names the relation it asked |
| Snowsight says a relation's object does not exist | the *resolved* relation is the config before the `generate_*_name` macros, or a source that is a view was asked for as a table | open the *built* relation instead, or the object from its schema's page |
