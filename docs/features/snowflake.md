# Snowflake

Everything that only works on Snowflake, behind one switch. Snowflake's
column lineage is in [column-lineage.md](column-lineage.md).

## The switch

**Snowflake's features are a setting of the project**, a switch in the menu
behind the snowflake in the top bar. They are marked alpha: none of them has yet been
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

## If something looks off

| Symptom | Cause | Fix |
| --- | --- | --- |
| the column lineage menu says `Snowflake · failed` | the script could not start; its tooltip and the Columns tab say why | usually no `snowflake-connector-python` in the Python it found, or no `profiles.yml` it can read |
| Snowflake is not in the column lineage menu | Snowflake's features are off for the project, which is the default when the manifest's adapter is not `snowflake` | switch them on in the menu behind the snowflake in the top bar |
| a clicked column comes back with no lineage | the object was not built by a query Snowflake could analyse, or the role cannot see it | check with `sf_lineage.py probe`, and check the environment pill names the objects you mean |
| a query in Query history has no Snowsight link | Snowflake gave no organization name for the account | copy the query id from its row and filter Snowsight's own history by it |
| Snowsight says a relation's object does not exist | the *resolved* relation is the config before the `generate_*_name` macros, or a source that is a view was asked for as a table | open the *built* relation instead, or the object from its schema's page |
