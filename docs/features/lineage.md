# Lineage

The graph of the node in focus: where it comes from, how it follows the editor,
whether it is still true, how to read it, and how to share it.

## Following the editor

Opening a `.sql` or `.yml` file that belongs to a dbt node moves the lineage
onto that node, so the graph follows the editor. A properties file declares
several: when the lineage already shows one of them, it stays there, and
otherwise it moves onto the first of them by name, the same one every time. A
file with no node yet, usually a model added or pulled since the last parse,
says so on the canvas, naming the model still drawn there if there is one, and
the lineage moves onto it by itself once `dbt parse` rewrites the manifest,
which dbt-edith reloads on its own within seconds. A macro file says nothing:
it has no node however fresh the manifest, and the lineage stays on the model
you were reading.

The up and down boxes start at 1 and keep what you set while you walk the
canvas, double-clicking a box or pulling in a level with a `+N` badge. A model
reached any other way, from `Cmd/Ctrl + K`, a tab, a link in the editor or the
Catalog, brings both back to 1. Going back to a link you followed from another
tab, and forward again, brings back the depth the lineage had there.

The lineage graph itself comes entirely from `manifest.json` (`parent_map`).
No `.sql` file is ever parsed to build it: dbt already did that work.

`manifest.json` is polled every three seconds: run `dbt build` in the built-in
terminal and the lineage refreshes on its own when dbt rewrites the file.

## Is the lineage still true?

Beside **Reload manifest** sits a dot and the manifest's age. The dot answers
one question: does the manifest still match the files dbt would parse right
now?

| | |
| --- | --- |
| green | nothing dbt parses has changed since the manifest was written |
| amber | files are newer, and all of them are yours, saved but not committed |
| red | files are newer and already committed: a checkout, a pull or a merge moved them under this manifest |
| grey | there is no `manifest.json` yet |

Hovering says which files, how many, what the last commit was, and what to do
about it. Clicking runs `dbt parse` in the terminal, which is the command that
turns the dot green again. dbt runs there, in your shell and your environment,
never from the server.

Green does not mean recent. A manifest parsed three weeks ago on a branch
nobody has touched for three weeks is correct, and the age beside the dot is
there to say so. What makes a manifest wrong is a file moving under it, not
time passing.

A second segment appears when your branch has fallen behind the default one:
`main +7` means `origin/main` has seven commits you do not have. It never
changes the colour, because a branch behind `main` still has a manifest that is
true for the code in front of you; it is there so you know to pull before
trusting the lineage as a picture of production. That count is only as current
as your remote refs, so dbt-edith fetches every ten minutes in the background,
read only, and the card says when the refs were last refreshed.

## Reading the graph

Node colour is the materialization, not the resource type, because that is what
you reason about when reading a DAG: what exists in the warehouse and what gets
rebuilt. View, table, incremental and ephemeral each have their own colour,
sources, seeds and snapshots keep theirs, and **any materialization the tool
does not know is drawn in magenta** so a custom one stands out rather than
blending in. Ephemeral models are dashed, like disabled ones: nothing of them
exists in the warehouse. The legend in the corner lists only the
materializations actually on screen. The same colours are used for the dots in
the sidebar and the catalog, so a model looks the same everywhere.

**Click an entry of the legend** to keep what it names lit and fade the rest:
`incremental` leaves the incremental models and the edges between them. In
column mode the legend names what happened to each column instead, and
`transform` leaves the transform edges and the two columns each one joins;
`raw` and `mixed` leave the columns wearing that badge. Click it again to see
everything. The pick stays while you move around the graph, as long as the
legend still lists it, and an exported graph is drawn without it.

In column mode, **stop at** beside the depth boxes lists the roles the column
lineage holds: tick `window` and each window edge below the focused column is
still drawn, with the column it reaches, so you see how the column is used,
but nothing past that column is, unless the walk also reaches it some other
way within the depth asked.
Several roles can be ticked. Upstream is left whole. A column the walk
stopped at wears the +N badge for what goes on, and the status line, copied
into an export, says where the walk stopped. The button names what it stops
at, and keeps it while you refocus on other columns, until the page is
reloaded.

An incremental model's box names its strategy in place of the word
`incremental`: `merge · marts · 3 tests`. One that configures none shows the
adapter's default, read from the macros the manifest carries, the project's
and its packages' overrides included, and keeps `incremental` when they do not
say. Where those macros insert every row for want of a unique key, the box
reads `append` with the strategy struck through: on Snowflake, a merge with no
key is a plain `insert into`. The hover card says where the strategy comes
from and why an append is one (0076).

Each box sits to the right of all its parents on screen, so every edge runs
left to right and a model always comes after what it reads. A model read by more
models than it reads moves right, to just before its first reader, which keeps
its edges short. An edge that skips columns runs level through a lane between
the boxes of each column it crosses rather than behind them, and a model read
across many columns shares one lane among its readers. In column mode, where the
lineage can loop, the one edge of each loop drawn against its direction is
dashed. The columns are an order, not a distance: the up and down boxes, the +N
badges and the export's `dbt ls` line still count levels from the focus, as dbt
does.

The **tests** eye beside the depth boxes draws the data tests on the canvas too.
It is off to start with and turns amber when on, because tests can outnumber the
models several times over: hundreds of boxes slow the canvas and bury what it
was drawn for. Its tooltip says so.

A test hangs under the model whose YAML declares it, as a one-line box joined
to it by a stem. A singular test, which has no YAML, hangs under the last built
of the models it reads. An edge from another model the test reads comes in from
the left, loops out on the right when that model sits in the same column, and
comes back dashed when it is built later. Every line into a test, the stem
included, is dotted and grey, where the lines between models are solid: it links
a check to what it checks rather than data to where it goes.

A row says what the test checks rather than the name dbt generated for it. A
test alone on a column reads `unique · order_id`. Two or more tests of one
generic share a row, `not_null  12 cols ▸`, which a click or Enter opens into a
row per column, and closes again; a generic that guards the whole model several
times reads `expect_table_row_count_to_be_between ×3 ▸` and opens into its
tests, named by what is left of dbt's name once the generic and the model are
taken off. Generics on the whole model come first, then singular tests, then
generics on a column, each by name. A model whose rows all fit in five, every
group opened, starts with its groups open, so a model with three tests shows
three.

Each kind of test has a grey and a shape, lighter in the order the rows go, and
the legend names them: a generic on the whole model a near-white bar doubled by
a thin one, a singular test a light grey bar in three pieces and a dashed
outline, a generic on a column the grey bar every test had. A test at
`severity: warn` carries an amber triangle, and a group's row counts how many
of its tests do. The hover card on a test says which generic it is and on which
column, the package it was named through when dbt recorded one, and the
severity when it is warn.

One click on a test row opens its properties file as the preview tab, at the
item that declares the test, `- not_null` under its column or
`- dbt_utils.unique_combination_of_columns:` under the model, and moves there
too when that file is already the one shown; the keyboard stays on the canvas.
The test's lines, its arguments included, light up and fade, so the eye finds
it among the others.
A double click pins the tab. dbt records no line for a test, so the line is
read off the file: of two tests of one generic on one column, which differ
only by their arguments, the one named in the YAML as dbt names the test is
found, else the first. A singular test opens its own file at the top.

Past five rows, a model shows its first five and a chip, `+12 more tests`,
counting the tests in the rest; opened, it reads `▴ hide 12 tests` and folds
them again. The test the lineage is centred on is never folded away: in a
closed group it shows under the group's row on its own. The line under the
canvas counts what a chip hides, and the eye, while open, counts the tests on
the canvas. When the canvas is capped
for room, every model keeps some of its tests: the cap takes one test per model
a turn.

The **by folder** toggle, beside the eye, draws the same canvas by folder: each
folder gets a band of columns of its own, at the first level where the models
drawn stop sharing a path, so numbered layers such as `10_raw`, `20_clean` and
`30_vault` each read as a block. Their names stay pinned over the top of the
canvas while you pan, and pausing on a band's background names its folder in
full. The folders go left to right in the order of their numbers, then of the
edges between them, so `staging`, `intermediate` and `marts` need no numbers.
An edge that runs from a later folder back to an earlier one, a layer read by
one before it, is dashed, and the line under the canvas counts them. Seeds,
snapshots and each installed package get a band of their own, and a test sits
with the model it tests. It costs width, since a band starts only where the one
before it ends: a way of reading the graph, not a better layout of it
([0029](../decisions/0029-the-canvas-may-be-drawn-by-folder.md)). An exported
file and a copied image keep the bands, with each name written at the top.

While the toggle is on, pausing on it opens a **levels** box under it, which
stays open while you type in it and closes a moment after you leave. It cuts
each band by its sub-folders: at 2, a `marts` holding `core` and `finance`
draws each of them as a rectangle inside its band, in the hue of its level, with
their names on a second row; 3 and 4 go one and two levels further. Sub-folders
keep the order folders have, numbers first and then edges, inside their band,
which never moves for them. The models filed in a folder beside its
sub-folders get columns of their own there, with no rectangle round them. A
level is the same depth on every band, so a folder holding a single sub-folder
still draws it, and each level costs columns
([0080](../decisions/0080-a-band-may-be-cut-by-its-sub-folders.md)).

## Sharing the graph

**Export**, beside Fit, saves what the canvas shows as one HTML file named
after it, `lineage-<what it shows>-<date>.html`, for a ticket or a message.
Anyone opens it in a browser with nothing installed and no network. The wheel
zooms without blurring, a drag pans, `0` fits it back, and a click on a box
lights its edges. **Nodes** lists every box with its full name and file, and a
name too long for its box is written whole in the box too, smaller, rather than
cut short as it is on screen.

The header says what the picture is. It names the `dbt ls` command that lists
the same nodes, with `--exclude "resource_type:test"` when no test is drawn,
since dbt would list them. It repeats the warnings of a mistyped selector, and
says how much of a capped selection was drawn, the rest being listed by name.
It says where the picture came from, the project, the dbt version, the
manifest's date, the branch and commit, and whether the manifest still matched
the files at that moment. Every date carries its offset from UTC: the reader may
be elsewhere, and reads it later.

**Ctrl+P** then *Save as PDF* in that file prints the whole graph on one page,
as vectors, so the PDF zooms too. **Copy image** puts the same picture on the
clipboard as a PNG to paste into a ticket's description, and saves the PNG when
the browser refuses the clipboard. In a ticket, the PDF and the pasted image
show in place; the HTML file is downloaded and opened, and it is the one that
pans and lists names.

The file holds what the canvas shows and that header, nothing else: no absolute
path and no value from a `.env` file
([0026](../decisions/0026-the-lineage-exports-as-one-html-file.md)).
