# Column lineage

Edges between columns, from one of three tools picked in the top bar.

## Column lineage

Column-level edges come from one of three tools, picked in the menu in the top
bar, next to the model counts. The same menu sits in Catalog > Columns.

| | Tool | Where its edges come from |
| --- | --- | --- |
| orange | **Fusion** | `target/column_lineage.fusion.json`. Nothing here writes one yet: dbt Fusion keeps its lineage as parquet under `target/index/` |
| magenta | **Collin** | `target/column_lineage.collin.json`, parsed out of the compiled SQL by [collin](https://github.com/IL-William/dbt-collin), which dbt-edith runs for you (0023, 0042) |
| cyan | **Snowflake** | `SNOWFLAKE.CORE.GET_LINEAGE`, fetched when you click a column |

The button names the tool whose edges are on screen, in its colour, and how
many there are; its tooltip names the file and who wrote it. A tool with
nothing to offer is greyed, and its tooltip says which file it lacks. A tool
stands for its newest cache: any other file beside the manifest, an older one
of a tool's or one written by something else, is listed under "other caches"
and can still be picked. The choice is remembered per project.

**Snowflake is the one tool that fetches.** Pick it, then click a column. The
edges come from `tools/sf_lineage.py`; dbt-edith itself never connects to
Snowflake: it has no HTTP client, no TLS and no credential handling, and
keeping it that way is what lets it ship as one dependency-free binary. The
script owns the connection and reads your dbt profile, so SSO, key-pair and
password targets all work unchanged. Picking Snowflake starts the script and
checks everything that needs no network: a Python with the connector, your
profile, its target and role, all named in the button's tooltip. Nothing
connects until you click a column, and the first click of a session may open a
sign-in tab.

**Collin runs when you need it.** Picking it runs collin if its cache is missing
or older than the manifest, and clicking a column while it is behind runs it
first; Regenerate, in Catalog > Columns, runs it at any time. collin reads the
compiled SQL, so run `dbt compile` before it for SQL that changed. It takes a
few seconds on a project of thousands of models and never runs at startup.
dbt-edith looks for `collin` beside itself, then on your `PATH`. Without it,
Collin's entry types the command that installs it into the terminal, for you to
run:

```
cargo install --locked --git https://github.com/IL-William/dbt-collin collin-cli
```

Where cargo is not available, copy a `collin.exe` built elsewhere beside
`dbt-edith.exe`.

**Other collin versions sit beside it.** "Install another collin version…",
under Collin in the menu, asks for a version and types the command that
installs it at its tag into a folder of its own, under dbt-edith's config
directory:

```
cargo install --locked --git https://github.com/IL-William/dbt-collin --tag v0.3.0 --root "<config>/collin/0.3.0" collin-cli
```

Once it is built, the menu lists every collin there is, the one beside
dbt-edith or on your `PATH` first, and the one picked runs for this project
from then on. Its cache is written again by it, since a cache written by
another version is treated as stale; the report says which collin wrote it.
Cargo's advice to add the folder to your `PATH` can be ignored. The update
line below applies to the default collin only: a version installed by tag
stays that version (0060).

**Two versions compare.** Each run's cache and report are copied to
`target/collin/<version>/`, the latest run of each version. Once two versions
have one, "Compare collin versions" under Collin opens a tab with B against A:
every figure of the two reports, the findings per kind, the edges by role and
by what they come from, the edges only one version has or reads with another
role, and the models where the most moved, each a link to it. A run older than
the manifest says so (0061).

Once collin is installed and Collin is the tool picked, a line under Collin in
the menu offers to update it. This dbt-edith expects collin 0.1.0 or later: an
older one, or one that gives no version, gets "Update collin", which types the
command above with `--force`. A recent one
gets "Check for a newer collin", the command as it is, which rebuilds collin
when GitHub has a newer commit and otherwise says it is already installed.

**Collin is alpha, and can be wrong.** Its button and its menu entry say so,
and so does Catalog > Columns, with the share of its edges that were matched
by column name rather than read from the SQL, which collin does when a compile
did not parse or cannot be trusted. Such an edge looks like any other on the
canvas, so check one against the compiled SQL before relying on it. collin's
report, `target/collin.report.json`, says how many models it read the SQL of
and why it read none of the others: the Columns tab shows the count, its
tooltip the reasons, and a model collin could not read says why above its
columns. Where the manifest has no compiled SQL, which a `dbt parse` writes,
collin reads the file a compile left in `target/compiled/`. Such a file
can be older than the manifest, or compiled for another target: collin sets
aside one that reads a table the manifest does not give its model, and matches
that model's edges by name. The tooltip counts both, the count leaves the set
aside ones out, and the file is named above a model collin listed. The report
also names columns, and each one it names is marked with a ! in the Lineage
column, why on hover: its lineage lost, absent from the warehouse or from the
compiled SQL, read from a CTE that lacks it, read downstream but missing here,
or matched by name to several parents. The note over the table names those the
table has no row for.

**collin's report, whole.** After a run, the Reports button in the top bar says
how many models and tables collin named, "12 to investigate", until you open
the report; "Open collin's report", under Collin in the menu, opens it at any
time. It is a section of [Reports](jobs-and-reports.md#reports): collin's figures first, then every
finding grouped by what would fix it. Rerun dbt: no compiled SQL, a compile for
another target, a catalog of another table, a table older than its code. Fix in
the project: a YAML behind the warehouse, a table read without a source or a
ref, a parent with no known columns, a column a child reads and this model's
list lacks. Check by hand: SQL collin could not parse, a column matched by name
to every parent that has it. And what is collin's to fix. A name opens the
model with its Columns tab, where the marks above say what was found on each
column. It reads a report from any collin since 0.1.0 (0059). From collin 0.5.0
the report carries collin's own findings, each with what it saw, what that
usually means and what to do, and the page shows those instead, in the same
groups; the badge then counts the models they name (0062).

Snowflake is in the menu only while Snowflake's features are on for the
project, see [snowflake.md](snowflake.md).

The script reads the dbt profile that the user and plug in the top bar open
(see [Explorer](explorer-and-search.md#explorer)), once, when it starts, so saving the profile there
restarts it. When Snowflake refuses the connection, the message points at that
file rather than leaving you with an error code.

A click asks for that column's upstream and downstream lineage, as deep as the
`up` and `down` boxes say and at most five levels, which is `GET_LINEAGE`'s
limit. What comes back is added to the Snowflake cache on screen, or to
`target/column_lineage.snowflake.json` when there is none, and drawn with
columns as nodes, so it is still there after a restart. Changing `up` or `down`
redraws what has been fetched; clicking a column asks Snowflake again.

**Which objects are asked about follows the environment pill** in the status
bar. On `manifest` that is the relation your last dbt run built, usually your
own schema. Choose a `.env` file and it is the relation that environment
resolves to, exactly as the Location table shows it. The cache itself names dbt
nodes rather than warehouse objects, so one file holds for every environment.

The Python that runs is a virtual environment of the project that has
`snowflake-connector-python` (dbt-snowflake brings it), else the one the status
bar names, else `python` on the `PATH`.

`GET_LINEAGE` needs Enterprise Edition and the `VIEW LINEAGE` privilege. When a
column comes back with nothing, check what the role is allowed to read:

```
python tools/sf_lineage.py probe \
  --relation my_database.my_schema.my_model --column my_column
```

The same script still fills the cache ahead of time, which is what to do before
working offline. One call covers one column, so a whole project is hundreds of
thousands of calls: always scope the dump.

```
python tools/sf_lineage.py dump --select model_a,model_b \
  --out target/column_lineage.snowflake.json
```

dbt-edith finds every `target/column_lineage*.json` on its own, the same way it
picks up `catalog.json`, and reloads the one on screen when it changes.
`--column-lineage <path>` overrides which file is loaded at startup. With edges
present, the Catalog > Columns table gains a Lineage column and the canvas gains
a Models / Columns switch. With no edges and Snowflake not picked, nothing
changes: the Columns table is exactly as it was and the Columns switch stays
disabled.
