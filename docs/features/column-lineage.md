# Column lineage

Edges between columns, from one of three tools picked in the settings.

## Column lineage

Column-level edges come from one of three tools, picked in the settings'
Column lineage section, behind the gear in the top bar, which says above the
list which one is on screen. The same choices sit in a menu in Catalog >
Columns, and the settings' rail names the tool on screen beside the section.

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
`dbt-edith.exe`, in a folder git does not track.

**A collin git tracks does not run.** One the repository commits came with it,
not from you, and a `dbt-edith.exe` copied into the project would otherwise run
it the moment Collin is picked. dbt-edith asks git about the collin it finds,
beside itself or on the `PATH`, whatever the case of its name, passes over one
that git tracks and goes on to the next. When another one is found, the column
lineage menu says under Collin `Using <path>. Skipped <path>: git tracks
<path>, so it came with a repository.` With none, it says `Not run: git tracks
<path>, so it came with a repository`, the Columns tab says the same, and
Collin's entry offers the install. In a repository git will not read, one
another account owns say, it cannot tell whose collin it is, so the collin is
passed over all the same and the note gives git's reason; `git config --global
--add safe.directory <folder>` lets git read it. Your own copy
beside `dbt-edith.exe`, which git does not track, runs as before; one copied
over a file git tracks is still that tracked file, and is passed over too
([0094](../decisions/0094-what-a-repository-brings-is-held-to-it-what-you-bring-is-followed.md)).

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
the menu offers to update it. This dbt-edith expects collin 0.10.0 or later,
the first to read a seed's columns from its CSV header, after 0.8.0, the first
to list the roots the `generated` tag below is read from: an older
one, the old repository's 0.2.0 included, or one that gives no version, gets
"Update collin", which types the command above with `--force`, and the
Columns tab says so first. A recent one gets "Check for a newer collin", the
command as it is, which rebuilds collin when GitHub has a newer commit and
otherwise says it is already installed. While a version installed by tag
runs, there is neither line, and nothing calls it older, whatever its number:
no update reaches it, and the default one is picked back in the same menu
(0098, 0109).

collin is handed the project folder, where it looks for a seed's CSV, only when
the manifest in use lies under it: a manifest named from another project's
folder is read with its own seeds or none, never this folder's (0109).

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

The script reads the dbt profile that the settings' Project section opens
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

dbt-edith finds every `target/column_lineage*.json` that holds edges on its own,
the same way it picks up `catalog.json`, and reloads the one on screen when it
changes. The report `collin generate` writes beside its cache, under the same
prefix, is not offered.
`--column-lineage <path>` overrides which file is loaded at startup. With edges
present, the Catalog > Columns table gains a Lineage column and the canvas gains
a Models / Columns switch. The Lineage cell counts the columns on each side of
the column, the whole chain in the lineage loaded: `←0 10→` is nothing
upstream and ten columns downstream, however many levels down. With Snowflake
picked, that is what the clicks have fetched so far. Hovering it adds how many
of them sit one level away, which is what the canvas opens on at a depth of 1.
The canvas draws 1000 boxes at most, so past that it says `truncated at 1000`
and draws fewer than the cell counts, with a button to draw up to 3000 (see
the indirect edges below). With no edges and Snowflake not picked,
nothing changes: the Columns table is exactly as it was and the Columns switch
stays disabled.

**The tag above a column box says what produced it**, read off the edges into
it: their role, or `mixed` when two roles meet there. A column nothing feeds
in view is tagged by where it starts. With a cache that lists collin 0.8.0's
roots ([0096](../decisions/0096-a-column-a-literal-sets-is-generated-not-raw.md)):

| Tag | The column |
| --- | --- |
| `raw` | a source's or a seed's, or one a model reads from a table no node of the project owns |
| `generated` | set by its model's SQL from no column, `current_timestamp() as loaded_at`: one of the roots collin lists in its cache |
| none | a model's or a snapshot's column that is no root: collin lost its lineage, which a ! in the Catalog may say |

A filter or a join key on a table no node owns reads no column of it, so it
makes nothing `raw`. A cache without roots, Fusion's, Snowflake's or an older
collin's, cannot tell a literal from a source: every column nothing feeds is
`raw` there.

A `generated` column keeps its tag when filter or join key edges reach it, as
they reach every column of a model under collin's `--indirect`. Hovering its
box says what the SQL sets it to, and so does its Lineage cell in Catalog >
Columns, before the counts, clipped and whole on hover: one value, one per
branch of a union that sets it differently, `'store' | 'web'`, or `generated`
alone when collin could not tell all of it, as for a value computed from a
literal or a union it read in part. Its row opens the column even when
nothing reads it, drawn alone. The legend lists each tag on the canvas, with
or without a role on an edge, and a click on it lights its columns.

**Indirect edges are hidden unless asked for, and end on their model's box.**
A `filter`, a `join_key` or a `dedup_key` edge, which collin writes under
`--indirect`, runs from a column that decided which rows a model keeps to every
column of that model. It says nothing of a value, so the canvas leaves it out
until asked: the walk does not cross it, upstream or down, and neither the +N
nor the status line counts it; the status line, and an export, say `indirect
edges hidden`
([0097](../decisions/0097-an-indirect-edge-is-hidden-by-default-drawn-dashed-when-shown.md)).
The page always has them to show: Regenerate runs collin with `--indirect`
every time, so a cache written by an older version needs one Regenerate
([0118](../decisions/0118-an-indirect-edge-ends-on-its-models-box-and-names-its-clause.md)).

The **indirect** button beside stop at, an eye like the tests one, shown only
when the cache holds such edges, draws them, thin and in short dashes, each
role in a hue of its own, which stays when the edge is lit. Each is named by
the SQL clause it comes from: `QUALIFY` for a dedup key, `JOIN` for a join key,
`WHERE / HAVING` for a filter. Upstream, an edge runs from column to column,
as any other, and carries no label: neighbouring columns are too close for one,
and the legend and the card say the clause. Downstream it runs to one box for
the whole model, drawn like a model's own box and reading `table  ·  whole
model`, not to each of its
columns: `stg_orders.status` filtered on in `orders` has one dashed `WHERE /
HAVING` edge to an `orders` box, however wide `orders` is, with its clause
written along it. A box ends the walk.
Where a column reaches a model by several roles the edge says the strongest,
`QUALIFY` before `JOIN` before `WHERE / HAVING`. A click on a box previews its
file, and a double-click draws it in Models mode. The status line counts
columns and models apart: `status · 3 columns · 1 model · 4 edges`.

The legend lists `indirect` and each clause on the canvas after the other
entries, and a click on `indirect` lights all three. The button stays as set
while you move from column to column, and is off again after a reload. No badge
reads an indirect edge: a column only they reach is tagged as one nothing
feeds.

Hovering a column says what keeps its rows, apart from where its value comes
from, and what it keeps the rows of, with the button on or off:

```
rows kept by: raw_orders.status (WHERE / HAVING)
keeps the rows of: orders (QUALIFY), customers (JOIN)
```

Eight names at most on each line, and how many more. Stop at offers none of
the three roles, on or off: an edge that ends on a box has nothing past it to
stop. The Lineage cell's counts, and its one level on hover, leave the indirect
edges out and say so; a column only they reach or leave reads `indirect only`
there, and still opens from its row, onto its box, with its card when they
reach it.

**The column canvas draws 1000 boxes**, a box for a column or a model. A
canvas cut there says so in its status line, and a **Draw up to 3000** button
draws that many for that column, depth, stops and indirect button alone,
as the selection's does for its own cap. It cannot say how many there were:
the walk stops when it is full. Past the cap every step of zoom and pan is
slower, which is why the cap is not the default.

## From the command line

`--column` answers one column's lineage without the browser, as
[`--select`](selection.md#from-the-command-line) answers a selection, so a
script or an agent can ask what a column reaches with one command
([0088](../decisions/0088-a-column-is-answered-on-the-command-line-from-the-cache-already-written.md)):

```
dbt-edith path/to/project --column stg_payments.amount --down all --output json
dbt-edith path/to/project --column orders.coupon_amount --up all
dbt-edith path/to/project --column stg_orders.order_id --down 2 --stop-at window,aggregate
```

The column is `model.column`, `source_name.table.column`, or a unique_id and
its column. The model is named as `--describe` names a node
([command-line.md](command-line.md#which-node)), among models, seeds,
snapshots and sources. `--up` and `--down` take a number of levels or `all`;
with neither, only the column itself is printed. `--stop-at` stops the walk
downstream at the roles named, as the stop at menu does on the canvas.
`--indirect` crosses collin's indirect edges too, which the answer leaves out
by default, as the canvas does with its indirect button off (0097). The walk is
the canvas's own, with no cap, except that an indirect edge is crossed to each
column of its model as it always was, where the canvas draws one box for the
model (0118). An answer without `--indirect` over a cache holding indirect
edges says so on stderr, which is every cache the page or `--run-collin`
writes.

`--output name` prints `model.column`, one per line, upstream first, then the
column, then downstream. `--output json` prints one line per model reached,
with dbt's key names (`unique_id`, `package_name`, `original_file_path`,
`materialized`) and only the columns reached. This line is the `orders` of
`--column raw_payments.amount --down all` on Jaffle Shop:

```json
{"unique_id":"model.jaffle_shop.orders","name":"orders","resource_type":"model","package_name":"jaffle_shop","original_file_path":"models/orders.sql","materialized":"table","depth":2,"columns":{"coupon_amount":{"depth":2,"from":{"stg_payments.amount":"aggregate"},"tests":["not_null_orders_coupon_amount"]}}}
```

| Key | What it holds |
| --- | --- |
| `depth` | levels from the column asked, negative upstream; the model's is its nearest column's |
| `from` | the columns of the answer this one is read from, with the role of each edge: a rename shows here, under its new name |
| `generated` | on a column its model's SQL sets from no column, the values it is set to, `["current_timestamp()"]`, one per branch of a union, `[]` when collin could not tell (0096) |
| `tests` | the data tests on the column |
| `more_up`, `more_down` | edges the answer leaves out on the side walked, where the depth or a stop ran out |

`--output compact` prints a tab-separated line per column reached, under a
header, without these last two keys: see
[command-line.md](command-line.md#a-column).

Without `--run-collin` it reads a cache already written and writes none:
collin is never started and Snowflake never asked. `--column-tool collin`,
`fusion` or `snowflake` picks a tool's newest cache, `--column-lineage path` a
file. Otherwise collin's or Fusion's newest cache answers, since Snowflake's
holds only the columns fetched so far from the page; without either, the cache
the page loads with nothing picked.

stderr says what the answer is worth, a refusal included: the manifest and the
freshness badge's verdict, as for `--select`, then the cache, who wrote it, for
which target and when, how many of its edge ends name nothing the manifest
holds, and, for Snowflake's, that it may be partial. A cache older than the
manifest still answers, and stderr then gives the
steps that write a new one, with the target left as `<TARGET>`: the manifest,
the catalog and collin's run must all come from the target you choose, with
its `.env` loaded, and only you know which one the answer should describe. It
lists the profile's targets and the `.env` files, and the command for the dbt
that wrote the manifest:

```
dbt docs generate --target <TARGET>                 # dbt-core, which compiles too
dbt compile --target <TARGET> --write-catalog       # dbt Fusion, which writes no catalog unless told
collin generate --project . --target <TARGET> --out target/column_lineage.collin.json --report target/collin.report.json
```

collin is told the names the page's own run writes, so the page then loads
this cache and reads this report rather than older ones it remembers.

With `--manifest` in another folder than `target/`, the steps send dbt
(`--target-path`) and collin (`--manifest`, `--catalog`, `--out`) there. collin's
report from the same run says whether it set compiled files aside as compiled
for another target, or found catalog entries describing another table: either
means the three did not come from one target.

### From a manifest alone

`--run-collin` runs collin over the manifest first, then answers from the
cache it wrote, so the lineage of any project can be asked from what dbt left
of it, a CI run's artifacts included, with nothing set up in the page and no
project around them
([0099](../decisions/0099-a-column-may-run-collin-first-when-asked.md)):

```
dbt-edith --manifest artifacts/manifest.json --column stg_payments.amount --down all --output json --run-collin
dbt-edith --manifest artifacts/manifest.json --column orders.amount --up all --run-collin --compiled sql/compiled
```

collin reads the manifest as it stands, for whichever target it was compiled
for, with `--catalog` else the `catalog.json` beside the manifest, and the
compiled SQL of the models the manifest has none for from `--compiled`, the
folder dbt compiled into, one folder per package (`target/compiled`), else
from `compiled/` beside the manifest. `--compiled` needs collin 0.9.0 or
later, and a folder that is not there is refused before collin runs, as is a
`--catalog` named and not there. collin is always told to write its
indirect edges too, which it does only when asked, so that the cache has the
ones the page draws and `--indirect` has some to cross; `--indirect` still
decides whether the answer crosses them, column by column. The cache and the
report go beside the manifest, under the names the page's Regenerate writes,
so the page shows that run too; a failed run leaves the cache that was there.
A model the manifest does not have is refused before collin runs, so a
mistyped name never rewrites the cache.

collin runs each time the flag is given. Ask once with it, then without it:
the cache it wrote answers from then on, until the manifest changes. stdout
carries the answer alone; stderr says which collin wrote which file, in how
long and how many models it read the SQL of, and collin's own lines when it
fails. A collin older than the version this dbt-edith expects still runs, and
stderr says so with the update command. When collin found no compiled file for
some of the models the manifest has no SQL for, stderr says how many and where
it looked: their edges are matched by name, and the cache the page reads is
now that one. collin is the one found beside dbt-edith or on the `PATH`, never
one git tracks, and not the version picked in the page, whose settings the
command line does not read.

Model lineage needs no cache: `--select` answers from the same manifest
([selection.md](selection.md#from-the-command-line)), and `--output json`
gives each node's parents.

| Exit | Meaning |
| --- | --- |
| 0 | answered, a stale cache included, which stderr says |
| 1 | refused: no node of that name, two of it, which wants the unique_id, or no such column, with the columns that have edges |
| 2 | nothing was asked: no manifest, no cache, which stderr says how to write, an unreadable one, a file that is not a cache, a wrong flag, or `--run-collin` with no collin to run, one too old for `--compiled`, a `--compiled` folder or a `--catalog` that is not there, or a run that wrote no cache |
