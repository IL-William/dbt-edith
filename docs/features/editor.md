# Editor

What the editor links, colours and shows on hover, and how its tabs, find
and breadcrumbs behave.

## Links

The editor links are a separate mechanism. The manifest knows *what* a model
depends on, but not reliably *where* in the file that name is written, and its
positions freeze at parse time anyway. So the vocabulary comes from the
manifest (a node's real parents) and the positions come from the open buffer,
which keeps the links correct while you type.

A name is linked when it is quoted, is a YAML key, or is a YAML value, which
covers `ref()`, `source()` and every automate_dv shape (`source_model` quoted or
bare, lists, `satellites:`, `stage_tables:`) without linking every column that
happens to share a model name. Jinja comments are skipped. Disabled models are
amber and still open; a name in `ref()` that no manifest node matches is red,
which makes it a genuine dangling-reference signal. Alt-click places the cursor
instead of navigating.

In a properties file the names that declare nodes are links as well: each
table under `sources:`, and each entry under `models:`, `seeds:`,
`snapshots:`, `analyses:` and `exposures:`, but never a column. Clicking one
moves the lineage onto that node and leaves you in the file, since the line you
clicked is the definition; from the Terminal tab, the Lineage tab comes
forward. A name the manifest does not have yet is red, like a dangling `ref()`,
until the next `dbt parse`.

A macro call inside `{{ }}` or `{% %}` links to the macro's file, and the click
lands on its `{% macro %}` line: `hub` in `{{ hub(...) }}`, `stage` in
`{{ automate_dv.stage(...) }}`, a hook in `dbt_project.yml`, an entry under
`macros:`. Which macro a name reaches follows dbt's own order, from the
manifest: the package's own macro when you write `automate_dv.hub`, and for a
bare `hub` the project's before anything else, so a project that wraps a
package macro under the same name opens its wrapper. A bare name never reaches
an installed package, because in dbt it does not either. Package macros open
from `dbt_packages/`, and only once `dbt deps` has installed them. dbt's own
macros (`is_incremental`, `run_query`) live outside the project and stay text,
as do Jinja's builtins and a name that is no macro.

In `dbt_project.yml` the keys under `models:`, `seeds:`, `snapshots:`,
`analyses:`, `macros:` and `data_tests:` name folders, and each one opens its
folder in the explorer, expanded, so a per-folder config and the folder it
configures are one click apart. The level naming the project is dropped the way
dbt drops it, a package key opens under `dbt_packages/`, and a last key naming
one model opens that file instead. Where a folder starts comes from
`model-paths` and its siblings, so a project that moved them is followed. A key
that matches no folder is dashed rather than linked, and its card says so: dbt
only mentions a config that configures nothing once, at parse time. A key is
told apart from a config the way dbt-core and Fusion tell it apart: one holding
a mapping is a folder unless it starts with `+`, whatever its name, so a folder
named `contract` links like any other. A `docs:` or `meta:` written without its
`+` is therefore a folder to dbt, which applies nothing under it, and its card
says so (0039). A package that keeps its models somewhere unconventional opens at its
own folder rather than at a guess. Hovering a key that did link names the folder, says when no
resource in the manifest sits under it, and names the other roots it exists
under when a project has several.

The other way round, a model, seed, snapshot, analysis or singular test has a
round violet button at the end of its breadcrumb, a file with an arrow coming
into it, for `dbt_project.yml`. It lists every key of that file
reaching it, broadest first and indented as the file nests them, each with what
it sets, and picking one opens `dbt_project.yml` at that key. A key reaches a
file when its chain begins the file's fqn, which is dbt's own rule, so a model
three folders below the only key written is listed under that key and those
above it, not under a key for its own folder that does not exist. A package's
models are reached by the key naming the package and by the configs set
directly under `models:`. The list says where `dbt_project.yml` configures a
file, never the config it ends up with: a `config()` in the file and its
properties YAML both come first, and `tags`, `meta` and the hooks add up
across levels (0038).

Jinja in a model is coloured by role: delimiters and keywords in orange, what
dbt itself provides (`ref`, `source`, `this`, `adapter`, `is_incremental`...) in
yellow, any other macro or filter in blue, named arguments in red. The SQL
colouring is never shown the Jinja, so an apostrophe in a Jinja comment cannot
turn the rest of the file into a string.

## Hover cards

Pausing on a lineage box, or on a `ref()` / `source()` in the editor, opens a
small card with the model's description, its first columns and their types, the
upstream, downstream and test counts, and its tags. It is the Catalog Preview in
passing, without leaving the file or the graph. Panning, zooming, scrolling,
clicking or typing dismisses it, and a click on a `ref()` still navigates.
On a macro, the card names the package it came from and its file, which is how
to tell the project's `hub` from automate_dv's, with the description and
arguments its YAML documents.

Pausing on a `var('x')` or an `env_var('X')` shows what that variable is worth.
This is the one thing dbt's own artifacts cannot tell you: the manifest holds no
`vars` at all, so the values are read straight from the `vars:` block of
`dbt_project.yml`, and the card names the line they came from. A var whose value
is itself `{{ env_var(...) }}` is resolved under the environment selected in the
status bar, and the card says whether the value came from that file or from the
default written in the call. Any line of the block it could not read is reported
with its line number rather than quietly skipped. A click on a `var('x')` opens
`dbt_project.yml` at its key, lit for a moment, the way a click on a `ref()`
opens the model; a var the block does not define says so instead. With unsaved
edits in `dbt_project.yml`, the click follows the tab rather than the disk, and
a `ref()` written as a var's default still opens its model. An
`env_var('X')` keeps the help cursor: its value comes from a `.env` or the
environment, and there is no line of the project to open.

Two things are never shown, both refused on the server rather than hidden in the
page: a `DBT_ENV_SECRET_*`, which dbt itself marks as never renderable, and any
variable whose name reads as a credential (`SNOWFLAKE_PASSWORD`, `DBT_API_KEY`,
`SF_PW`, `DATABASE_URL`), or whose value carries a password inside a URL,
whatever it is called. Those still show their name, whether the file defines
them, and which of the two gave them away, which is usually the question
anyway. Ordinary config keeps working: `PARTITION_KEY`
and `DBT_UNIQUE_KEY` are not credentials.

## Find

`Cmd/Ctrl + F` finds inside what you are reading: the file in the editor, the
SQL in the Compiled or Run tab, either side of a diff. The browser's own find
cannot do this, because the editor keeps only the lines on screen in the page,
so a match further down is not there for it to find. The bar counts every match
in the file and paints them all; `Enter` and `Shift + Enter` step through them,
and `Escape` closes it with the cursor left on the last one, ready to type.

Beside the box, `Aa` matches case, `ab` whole words, and `.*` takes a regular
expression, where `^` and `$` are the ends of a line. A selection on one line
becomes the query, and a bar opened elsewhere starts from the last query, so a
column found in the model is one `Cmd/Ctrl + F` away in its compiled SQL.
`Cmd/Ctrl + G` and `F3` step from the text itself, with `Shift` to go back.
Typing searches from where the cursor was when you started, so in a long file
the first letters never drag the view to the top. Past 10 000 matches the count
stops and says so; stepping still reaches every one.

In the Catalog, `Cmd/Ctrl + F` goes to the Columns tab and into its filter,
which keeps the columns whose name contains what you type, in any case. The
filter stays as you move to another node, so one column can be followed through
the lineage a model at a time. `Escape` empties it.

Over the lineage, `Cmd/Ctrl + F` finds a node on the canvas by its name and
brings it to the middle, zoomed in to its drawn size when the graph was fitted
smaller. It looks through every source, model and seed drawn, and every test
once the tests box is on, a test folded under `+N more tests` or a closed
group's row included: the view goes to that chip or row, and `Enter` opens it.
A source is found by its table alone too, and a test by its generic or its
column. A name typed whole comes first, then the names it starts, then those it
is in; `Enter` and `Shift + Enter` step through the rest, left to right a column
at a time and down each, a model's tests after it. Every match is outlined in
amber, and `Escape` closes the bar with the view left where it is. Only what the
canvas draws is searched: `Cmd/Ctrl + K` searches the whole project.

In the terminal, `Cmd/Ctrl + F` is the browser's own find. `?` outside a text
box, `F1` from anywhere, or the `?` button in the top bar lists every shortcut,
with the keys of the machine you are on.

## Tabs

A single click in the file tree opens a **preview** tab, shown in italics and
reused by the next single click, so browsing the project does not pile up tabs.
Double-clicking, or editing the file, pins it, and so does following a link out
of it, a `ref()`, a macro call, a `var()` or a `Cmd/Ctrl + K` result, so that
Back has it to return to; the link's target opens as the preview in its turn.
The **Open editors** panel at the top of the sidebar lists every open file
with its own close and save buttons, plus save-all and close-all in its
header. Closing a tab never touches the file on disk.

A tab follows its file when something else rewrites it: a `git pull` in the
terminal, a checkout, dbt, another editor. With nothing unsaved, it shows the
new text within a few seconds, or as soon as it is clicked, and the cursor
stays on its line. With unsaved edits, it keeps them and its name turns amber;
saving it then asks before writing over what the disk now holds, rather than
quietly putting back what the pull just changed. A file deleted on disk leaves
its tab as it was, its name struck through: saving the tab writes the file
back, after asking, and closing it asks first.

## Breadcrumbs

The row under the tabs says where you are twice over: the file's path through
the project, then, inside a `.yml` or a `.md`, where the cursor sits in the
document, as `models > 0 > data_tests`. Every segment is a button. A path
segment opens a menu of the folder it sits in, where a folder drills one level
down and a file opens in a preview tab; a document segment lists the keys or the
list entries beside it and jumps the cursor to the one you pick. Arrow keys move
in the menu, Enter picks, Escape closes.

A `.sql` file shows its path and stops there. Finding a CTE name honestly means
masking SQL strings and comments first, and a bar that is occasionally wrong is
worse than one that is short.
