# Where the work stands

Rewritten as things change, unlike [decisions/](decisions/) and
[../CHANGELOG.md](../CHANGELOG.md), which are appended to. What shipped and in
which version belongs there; what is half done, deferred or waiting on someone
belongs here. Last updated 2026-10-30.

## Shipped

Editor with clickable `ref()`, `source()`, macro calls and the folder keys of
`dbt_project.yml`, and Jinja coloured by role,
lineage graph in model and column modes, column lineage from the tool picked in
the top bar, Snowflake's fetched when a column is clicked (0016, 0031),
terminal, file explorer with git and
unsaved colouring, search across nodes, macros, file names and file contents, Catalog with columns and
locations, the compiled and run SQL under target/ with freshness, git panel (status, branch switch, stage,
commit, push, pull, conflicts, side-by-side diff), Python environment in the
status bar, and environment-aware location resolution with a `.env` selector.

Hover cards: a lineage box or a `ref()` shows the model's
description, columns and counts; a `var()` or `env_var()` shows its value,
resolved under the selected environment. Project vars come from a hand-written
scanner over `dbt_project.yml` (0018), because the manifest does not carry them.
Showing a resolved value needed the .env boundary widened, which 0019 does,
under two guards. A click on a `var()` opens `dbt_project.yml` at its key.

Breadcrumb bar: the row under the tabs shows the file's path
and, inside a `.yml` or a `.md`, where the cursor sits in the document. Every
segment opens a menu, so a sibling file or a neighbouring model is one click
away without leaving the editor. The outline is scanned in the browser (0022);
`.sql` shows the path only, until string and comment masking exists.

Search across file contents: the Search tab reads the indexed
files rather than their names, so a column used in forty models is findable. It
never opens a `.env` (0020). A full pass over a 12 000 file project is under a
second in release, after an ASCII fast path and a per-file pre-check.

Selector expressions: the Lineage tab has a third mode where
you type a dbt selection expression and the canvas draws the set it matches,
laid out by longest path within the selection so disconnected pieces each start
at the left. `src/select.rs` re-implements dbt-core's selector methods over the
manifest rather than shelling out to `dbt ls` (0024), which answers in
milliseconds and needs no profile; the fidelity that costs is watched by
refusing an unsupported method by name and by a button that types the
equivalent `dbt ls` into the terminal for you to compare. Measured on a
109 MB manifest: one term with a `+` resolves 265 nodes in well under 10 ms.

Manifest freshness: a dot in the top bar's manifest pill, which ends in its
reload, says whether the lineage on screen still matches the files dbt would
parse, and clicking it runs `dbt parse` in the terminal. Freshness is measured
in file times rather than in commits (0025), because committing changes no file
and pulling an old commit changes several. git is asked one question only, per
file already known to be newer: is it dirty? That separates the user's own
unsaved work (amber) from a checkout or a pull they have not re-parsed since
(red). How far the branch trails the default one rides beside the dot in its
own segment and never colours it. `api::watch_remote` fetches every ten minutes
so that count means something, read only and deadlined like the rest of 0007.

Compiled and Run tabs: the two files dbt leaves per node
under `target/` get a tab each, where one tab used to probe `compiled/` then
`run/` and show whichever it found first. That silently answered the wrong
question whenever dbt had run a model without recompiling it. Each tab names the
date its file was written rather than only its age, because an age cannot be
compared with anything; the path beside it is a link that opens the file tree at
the file.

What turns that bar amber is measured the way 0025 measures the manifest, in
mtimes and never in a clock: the model, its schema file, `dbt_project.yml` and
the newest file under `macros/`, each named in the bar when it is the one that
moved, plus, for a run, a compiled sibling written after it. The first attempt
had an age threshold instead, an hour for compiled and a day for a run, and both
numbers were arbitrary: a file nothing has touched since is still what dbt would
write at any age, and a threshold only teaches the reader to ignore the colour.
Any macro counts rather than the ones the node declares, because the manifest's
`depends_on.macros` omits nested calls and is mostly `macro.dbt.*`, which has no
file; a false green is worse here than a false amber. The cost is that one macro
edit, or any `git pull` that touches `dbt_project.yml`, turns every model amber
until the next compile, which is true and is what dbt does about it too.
Measured on the 18 825 node project: 1.3 ms per tab load, macro walk included.
The precision left on the table is naming the macro a model actually uses, which
wants `manifest.macros` read and package paths resolved.

This is not the deferred **Run history** below: that one reads `run_results.json`
for status and timing, which no file under `target/run/` carries.

Every test in the Catalog: a column held the *names* of the
tests guarding it, and the pass that deduped them deduped on that name, so two
`relationships` pointing at different tables, or two `expression_is_true` with
different expressions, arrived as one chip. On the 18 825 node project that was
15 columns in the 600 models measured, each losing a test. A column now holds
graph indices of test nodes instead, which are 4 bytes where a name was 24 plus
its heap, and the payload carries dbt's generated name per test, since that is
the only thing telling two tests of one generic apart. The cell shows three
chips and a `+N`, capped in characters as well as in count because three
`dbt_expectations` names wrap a row to three lines where three of `not_null`,
`unique` and one more sit on one. The `+N` opens the shared hover card, and
clicks to expand in place: a hover is no affordance on a touch screen and no
route from a keyboard, and a second click cannot reopen a card that closes on
any outside mousedown.

Preview lists `n.tests` rather than only counting it, which is the only place a
test guarding the model and no column of it is ever named: 460 of those 600
models have at least one, and one hub has 48 singular tests that until
now appeared nowhere at all. The two kinds are listed apart rather than in one
list where the difference is a missing suffix, and the Columns toolbar names the
count it cannot show, because a table of columns can hold no test that names
none. What is still invisible, and deliberately: disabled
tests, unit tests, and a test whose `column_name` matches no declared column,
which is dropped silently with no owner to hang it on.

Every route sits behind the Host and Origin guard, added after a
security audit found the terminal reachable from any web page (0015). The same
pass confined `/api/git/diff` to the project and added `SECURITY.md`.

Compiled and Run for a test: both tabs were derived from a
node's `original_file_path`, which is the model's own file and, for a generic
test, the schema file that declares it. dbt writes that test somewhere else
again, under a name it truncates and hashes once the generated one runs long:
`relationships_fct_orders__cus_4a1f0c2e9b7d6a5c3e8f1b0d2c4a6e8f.sql`. Nothing
here could reconstruct that, so the manifest's own `compiled_path` is read
instead, which the module previously said Fusion does not write. It does: 15 911
of the 18 825 nodes carry one. Only the part inside the target directory is
kept, because `--manifest` elsewhere means the recorded directory and the one
being read need not agree, and `run/` is that same path under the other
directory, since dbt records no path for it and mirrors the layout into both.
Where no `compiled_path` exists the old derivation still runs, plus two guesses
at a generic test's file name, and every path tried is still reported.

Measured on that project: 148 of 148 generic tests sampled now show their
compiled SQL where none did, 60 of 60 models are unchanged, and only 7 of those
148 have a run file, which is true rather than a miss: dbt writes 26 250
compiled generic tests there and 472 run ones.

Export and Copy image: the canvas leaves as one HTML file for
a ticket, read in a browser with nothing installed (0026), or as a PNG on the
clipboard. The file pans and zooms its own viewBox, prints to one vector page,
which is the PDF, and forbids itself every fetch. Its header names the `dbt ls`
command for the same nodes and where the picture came from, in absolute dates.
Built from the payload that was drawn, never from `S`: a rejected selector
leaves the last picture on screen while `S.selectSub` is already null.

Measured on the 18 825 node project, driven in headless Chrome from `file://`:
a nine-model selection is a 27 KB file, a capped one of 400 boxes 524 KB. Each
file makes one request, its own, and logs nothing. Two things only a browser
showed. The app's own policy takes images from `data:` alone (0015), so the
PNG goes through a data URL; a blob URL failed without a word. And a clip counts
characters while a box holds pixels, so a name written out whole is measured on
the canvas, measured again once shrunk, since small sizes do not scale in
proportion, and pinned to that length for readers whose fonts run wider.

Columns from the edges: a box's column was the server's BFS
distance from the focus, so a model reached by a short path and a long one sat
at the short one, and a model it feeds could be drawn to its left (0027). The
browser now lays the canvas out from the edges it draws. Columns come from the
longest path, and a long edge gets a lane in each column it skips. Median sweeps
set the order. Brandes and Kopf sets the heights, unless it would spend more than
half again the tallest column's height; a compact isotonic step then takes over,
where it comes out shorter.
`depth` keeps its BFS meaning, because the export turns it into `N+model+M`.

Measured on a 112 model neighbourhood of the 18 825 node project, against the
old layout: edges pointing left 27 to none, crossings 316 to 57, passes behind a
box 199 to none, 14 to 21 columns, about a millisecond. With its tests ticked,
400 boxes: 359 edges pointing left to none, 1 700 crossings to 477, and 9 038 px
tall to 8 126. Brandes and Kopf alone drew that one at 15 615 px, found only by
running the real payload, which is why the switch exists. Seen in headless
Chrome on that payload.

Macro links, and the names a properties file declares: a
macro call inside Jinja opens the file that defines it at its `{% macro %}`
line, and in a `.yml` the name of each source table, model, seed, snapshot and
exposure moves the lineage onto its node. The macro table is the manifest's,
1 690 entries on the 18 825 node project, 171 of them the project's own, and a
call is resolved in dbt's order by `src/macros.rs` (0028), because 54 names are
defined twice there, some by the project wrapping a package macro under its
own name. A bare name reaches the project's, `pkg.name` the package's, and a
bare `star` nothing at all, as in dbt. The browser finds the names and the
definition line, the server only which macro and whether its file is on disk.

A declared name's click leaves the editor where it is, unlike a `ref()`, which
opens its target. In a properties file the name is the definition, and opening
the model's `.sql` as a preview would replace the file being edited. Returning
to that file used to move the lineage to whichever node `/api/node?file=` lists
first, rarely the table just clicked; `syncNode` now keeps the node already
shown when the file declares it.

Driven in headless Chrome on that project: `stage` in `automate_dv.stage(`
opened the package's `stage.sql` on its definition, the lineage unmoved; a
project macro opened on its line in a file named after something else; 22 of
22 tables of a sources file linked, the source itself not; a models file linked
its model and none of its columns; the four macro calls in `dbt_project.yml`,
hooks and the query comment, linked, `env_var` left to its card.
Not linked, deliberately: `adapter.dispatch` targets, generic tests named in
YAML, and packages under a custom `packages-install-path`.

`Cmd/Ctrl + K` finds a macro by its name, a package's as `package.name`, and
opens it on its definition, a generic test's and a materialization's included
(0084). Only the macros placed in the project are listed, ranked by how the
name matches before whose it is.

Folder bands: the by folder toggle beside tests gives each
folder its own run of columns (0029), at the first level where the drawn models
and sources stop sharing a path, so a canvas that sits in one folder gets its
sub-folders. The plan here was to refuse when two folders feed each other. The
18 825 node project has exactly one edge running from a later layer back to an
earlier one, and the canvas that draws it is the one most worth reading in
layers, so the edge is dashed instead, as a loop is in column mode, and counted
under the canvas. Ordered by the edges alone, that one edge put the later layer
first on that canvas and nowhere else, which is why a folder named with a
number keeps its number's place and the edges place the rest. Where they leave
a choice, a folder goes where its nodes sat without the bands: broken by names,
a seed nothing drawn reads came after every number, at the far right.

Measured on that project against the plain layout: a 64 model neighbourhood 8
to 13 columns, crossings 52 to 54, and the 194 pairs of boxes drawn in the
reverse of their folders' order to none; a 157 model one 13 to 19 columns,
crossings 1 191 to 1 752, 4 088 to 6 720 px tall; a 400 box canvas in about 6 ms.
The names sit in an HTML layer over the canvas, pinned to the top of the
window, because at the zoom that fits a graph the canvas's own text cannot be
read; a name gives way under 40 px of room, which on a tall graph is most bands
until you zoom in. Driven in headless Chrome on that project, in model and
selection mode, with the export and the image; column mode only by the harness,
the project's column cache holding two edges.

Sub-folder bands (0080): the levels box, opened under the by folder toggle by
the pointer or the keyboard and closed a moment after they leave, cuts each
band by its sub-folders, up to four levels down, each one a run of columns
inside its folder's, drawn as a rectangle within it in a hue per level, its name
on a row of its own. Asked for a layer split into sub-folders, which a
canvas across the layers drew as one block. Sub-folders
take 0029's order among themselves, inside a band that never moves for them,
and the models filed beside them get a run of their own, placed by their edges.
At one level the canvas draws as before. Driven in headless Chrome on a large
real project, in selection mode at one to three levels, with the export, and
the box's opening and closing by pointer, click and arrow key; column mode only
by the harness.

Find, the column filter and the shortcut list (0030):
`Cmd/Ctrl + F` used to open the browser's find, which sees only the thirty or so
lines CodeMirror keeps in the page, so a match below the screen was never found.
Each CodeMirror now has a find bar, through the `find`, `findNext` and
`findPrev` commands its keymaps already bind; the Catalog focuses a filter on
the Columns table; `?`, `F1` or a button lists every shortcut. The first idea
for that list was `Cmd + H`, which macOS takes before the page sees it.

Measured in headless Chrome on a 3.5 MB model of 34 198 lines, the largest the
editor opens being 4 MB: 6 to 14 ms per keystroke to count and repaint, whatever
the query, and half a millisecond per step. Past 10 000 matches the count stops
and says `10000+`; a step reads the text, not the list, so it still reaches the
next match, and typing stays near the line it started from instead of wrapping
to the top. Driven the same way on an invented project: the editor, Compiled,
Run, both sides of a diff, the column filter kept across models, and the list
by `?`, `F1` and the button. Then all of it again as Edge on Windows, with
`navigator.platform` at `Win32`, which is what turns CodeMirror to Ctrl: every
key with Ctrl, plus `F3`, `Ctrl + G`, `Alt + W` from a QWERTY and an AZERTY
keyboard, and AltGr+4 typing `{` rather than firing anything. That emulates
the page, not the browser.

Column lineage by tool (0031): the read-only chip in the top
bar is now a menu of Fusion, Collin and Snowflake, each in its own colour, and
the same menu sits in Catalog > Columns. The menu there had never worked: the
page never read the caches `/api/meta` listed, and picking one called an
`api.post` that did not exist, so a project with collin's cache could only ever
show it. A tool is its newest cache, by the producer the header names; older
files and other producers' stay under "other caches". Picking Snowflake is one
request that also starts the script, so the old switch, and `POST /api/sidecar`,
are gone. Snowflake's features sit behind a switch in a new menu, opened by a
snowflake in the top bar, which follows the manifest's adapter until the user
chooses; off, the server refuses the tool, its caches and a fetch. The switch and the Snowflake
entry are marked alpha until the first real answer from a warehouse, under
Waiting on a human below. The icon beside the switch is a snowflake drawn here,
not Snowflake's logo: their marks are licensed only for uses they approve in
writing, and this repository is public (0014).

Driven in headless Chrome on an invented project with a collin, a Snowflake and
a synthetic cache: the greyed Fusion entry and its tooltip, picking each entry,
the setting on and off with the Snowflake entry and the profile link going with
it, the script's `starting` then `failed` in both buttons, and Escape. Startup
checked with the setting unset on a `snowflake` and a `postgres` adapter, and a
Snowflake `--column-lineage` refused while off. The 18 825 node project holds
only a legacy `column_lineage.json` whose source is Snowflake, which the menu
files under Snowflake.

Switching tools no longer reads the manifest. Merging a cache adds the columns
it knows and the YAML does not, and never removes them, so switching used to
re-read everything to shed the last cache's columns. The server now keeps a
second graph, `base`, holding the manifest and the catalog alone, and merges the
chosen cache into a copy of it. The menu's list of caches reads a file's header
once per version of the file, where it read every cache in full each time the
menu opened. Measured in release against the build before:

| | before | after |
| --- | --- | --- |
| switching on the 18 825 node project | 438 ms | 25 ms |
| switching to a cache of 250 000 edges, 46 MB | 1 280 to 1 500 ms | 720 to 900 ms |
| opening the menu beside three such caches | 106 ms | under 1 ms |
| memory at startup, 18 825 node project | 135 MB | 181 MB |
| memory after switching, same project | 353 MB | 227 MB |

The 46 MB more at startup is the price of the second graph. Memory after
switching falls, because a switch no longer parses a 110 MB manifest to throw
the old one away. What a large cache still costs is parsing its own JSON.

Named selectors (0032): a menu beside the Selection box lists
the selectors of `selectors.yml`, and `--selector name` in the box draws what
`dbt ls --selector` lists. The manifest already holds every selector parsed, so
the YAML scanner this item was deferred on was never needed. Each criterion
brings its own tests, by its own `indirect_selection`, before the sets are
combined, as dbt-core's `select_nodes_recursively` does. Typed lines go through
the same engine now, so 0024's "applied once at the end" is gone, and `+` and
`@` walk into tests as dbt's graph does. For a named selector the tests eye
filters the drawing only. A test's models are drawn either way, dimmed when the
selector did not pick them; on, the test hangs off them, off, only it goes. The
first version drew nothing at all with the box off for a selector of tests
alone, and said why under the Selection bar, where the bar hid the sentence;
the hint now sits in the middle of the pane, and that canvas shows the models.

Measured on the 18 825 node project, parsed again from its current files with
dbt Fusion 2.0.6: all 85 selectors this build resolves answer what
`dbt ls --selector` prints, name for name, 31 of them with something in it, up
to 315 names; the other 54 are empty in dbt too. `ci` is refused for its
`state:`. The same manifest with `indirect_selection` stripped, which is how
2.0.0-preview.196 writes it, answers 30 of the 85 differently, 325 names too
many in all. It shows the note on 40, those 30 among them: the note is for a
selector the lost setting could change, found by resolving it with every
criterion at the fewest tests and at the most. A selector of tests alone never
shows it; the first version showed it on all 85, which taught nothing. All 86, 84 of them tests only, resolve over
HTTP in 2.5 s in a debug build, 26 ms for the median one and 144 ms for the
largest. Driven in headless Chrome on the preview.196 manifest: the menu and
its filter, tests off and on, a refused selector, a pasted `dbt ls --selector`.

Checked against dbt itself (0033): `scripts/compare_with_dbt.py`
resolves the 16 named selectors of a Jaffle Shop copy in `tests/fixtures/`, and
26 typed lines, with dbt-edith and with `dbt ls`, tests on. All 42 agree under
dbt-core 1.11.15 and 1.12.5, which CI installs, and under dbt Fusion 2.0.6 run
by hand; the 17th selector uses `state:` and is refused. The fixture gives each
`indirect_selection` mode an answer of its own, 13, 11, 10 and 1 nodes for one
model, because its first version had no test reading a model and its own
ancestor, and buildable answered what cautious did. A run takes about 6 s with
dbt in process.

Tests under their model (0034): with tests on, a test hangs
under the model whose YAML declares it, a one-line box on a stem, where it took
the column after its model's like any reader. The layout runs on everything
else with each host made taller by its stack, so with no test hanging, a
canvas comes out exactly as before, and the models keep their columns either
way. The server sends each test's `attached_node` as a position in the payload;
a singular test, which has none, hangs under the parent in the furthest column.
Driven in headless Chrome on the 18 825 node project: a model with 17 tests
draws them as a list under it, and a reconciliation selector hangs each test
under its model with the other models it reads coming in from the left. The
staircase graph in `layout.js` hung its sinks under their models once they
did, so its sinks are models now. Every line into a test is dotted and grey,
dash and dot when turned, `.edge.test` sitting before `.edge.hi` so a selected
one still turns accent.

Tests folded past five (0035): a hanging test costs 26 px, and
the most tested node of that project carries 134, about 3 500 px of pile. Under
a model the first five show and a chip opens the rest, per model, in place and
without moving the view; folded tests and their edges are not drawn, and the
status line counts them. The cap, which kept the canvas alive but emptied the
models reached last of their tests, now takes one test per model a turn, in
model mode and in Custom selection alike. In `layout.js`, 150 models of 20 tests
each lay out 1 978 px tall with five and a chip under each. Driven in headless
Chrome on that project: the node with 17 tests shows five and `+12 more tests`,
opens to seventeen by click and folds again by keyboard.

Folder keys in `dbt_project.yml` (0036): a key under
`models:`, or under its five path-valued siblings, opens the folder it
configures in the explorer, expanded, so a per-folder config and its folder are
one click apart. The chains are scanned in the browser and placed on disk by
the server, which reads `model-paths` and the rest for the first time in this
binary. A key that matches nothing is dashed and its card says dbt only warns
about that at parse time; a key that opens a folder with no resource under it
says that too, which is the manifest's one job here. Which keys are folders is
dbt's rule, as 0039 corrected it: a key holding a mapping is one unless it
starts with `+`, whatever its name. A package is recognised by the folder `dbt deps` wrote, not by a
node carrying its name, so a package of macros alone is recognised too, and a
key that is only a package name opens the package folder. On the 18 825 node project all 553 keys of its
project file resolve, in 160 ms for the whole round trip, 12 of them onto
folders the manifest has nothing under and one of them a package, which opens
under `dbt_packages/`. The card only calls a config dead when the freshness
badge says the manifest matches the project, since on a branch the folder is
usually newer than the parse. Driven in headless Chrome on that
project and on a copy of Jaffle Shop with no manifest at all, which is the case
the disk answers and the manifest could not.

A header above the tree: the project's name and two buttons,
one collapsing every folder and one reopening the tree down to the file being
edited. The collapse needed no new state, since what the tree knows about what
is open is `.kids.hidden` and `.row.open` in the DOM, and the children already
read stay there, held by `row.loading`, so reopening a folder fetches nothing.
The reveal reuses `revealInTree`, and both share `projectPath`, pulled out of
`activate`, since a tab's key is not a path for a diff or the profile.
`Alt + Shift + W` closes every tab, and `closeAll` now removes a diff tab's
MergeView host and hides the diff pane, which only `closeFile` did: closing a
diff that way left its two documents in the page. The tab bar also scrolls to
the active tab, which it never did while the bar overflowed.

A tab's own menu and a way back: right-click a tab for Close,
Close others, Close to the right and Close saved, the app's first context menu;
every other menu here is click-opened, but for Edith's, which opens under the
pointer too since 1.26.0 (0086). `Cmd + Alt + P` and `Cmd + Alt + N` walk
the two stacks of where following a link came from, with two chevrons before the
breadcrumbs doing the same. Not `Alt + Left` and `Alt + Right`: CodeMirror binds
those in both keymaps, and moving by word is worth more than the history. They
shipped as `Alt + B` and `Alt + N` and were wrong on a French keyboard, which is
the one this is used on: both wrote into the file rather than moving. Chasing
that turned up the wider fact (0037), that no `Alt + <letter>` reaches a French
Mac at all, for two reasons at once. `e.code` names positions, so the key marked
W arrives as `Alt + Z`; and the key at `KeyW`, marked Z, writes `Â` under Option,
a letter `keyCombo` leaves as typed because that is someone writing. So
`Alt + W`, shipped long before any of this, had never closed a tab there either,
and `Alt + Shift + W` inherited it. Cmd is what lets `keyCombo` fall back to
`e.code`, which `Mod + Alt + S` already relied on without anyone noticing, and
it only holds for a letter in the same place on both layouts, so not A, Q, Z, W
or M. Closing a tab now also answers to `Mod + Alt + X` and closing every tab to
`Mod + Alt + Shift + X`, `Alt + W` staying bound for the keyboards it reaches.
`Alt + Shift + W` is gone instead of kept: it was a modifier away from
`Shift + Cmd + W`, which closes the browser window, and it had shipped in no
release. Picking a key next to a destructive one is the mistake there, not the
layout.
What was tempting and wrong: dropping the Cmd escape in `keyCombo` so it would
only remap a non-letter. `Cmd + Option + S` reports `∑` on that keyboard, so
that would have broken `Mod + Alt + S` to fix nothing. Back
lands on the `ref()` that was followed, since the click moved the cursor there
before the jump, which is the call site you want again. The three close commands
now share `dropTab` and `settleTabs`, because two of them drifting apart is what
left a diff's MergeView in the page. Driven in headless Chrome on the 18 889 node
project: three palette jumps walk back and forward with the chevrons greying at
each end, a ref() click returns to its own line, Close to the right leaves
exactly the tabs to the left, and two edited files draw one confirmation naming
both.

The keys reaching a file (0038): a model, seed, snapshot,
analysis or singular test has `dbt_project.yml` at the end of its breadcrumb,
which lists every key of that file whose chain begins the file's fqn, broadest
first, with what each sets, and opens the file at the one picked. One key per
folder was never the shape of the answer: on the 18 825 node project, 362 of
its 3 825 models have a key on their own folder, 3 185 are configured from the
folder above, 276 from two to four folders up, and every one sits under three
to eleven nested keys. The fqn is read off the path, and matched the
manifest's for all 3 341 models, 8 seeds and 609 singular tests it holds. The
menu says where, never what wins, since `dbt_project.yml` comes last of dbt's
three places for a config.

Driven in headless Chrome on that project: a model three folders below its
deepest key lists five levels and opens `dbt_project.yml` on the fifth; a
`dbt_artifacts` model lists `models:` and the key naming the package; a
singular test lists the `data_tests:` levels and never a model's. 6 to 17 ms
from the click to the menu, in a debug build. On a copy of Jaffle Shop with no
manifest, the same.

A way back out of a preview tab: following a link out of a
preview tab pins it, where the link's target, opened as a preview, used to take
its slot and close it, leaving Back an entry `nextJump` skips as closed. So a
file opened by a single click in the tree, the commonest way in, had no way
back from any link since the history shipped. Pinning the tab left is
what VS Code does by default (`workbench.editor.enablePreviewFromCodeNavigation`
off). Reopening a closed tab from Back was the other way, and it would also
reopen tabs closed on purpose, and could not for a diff or the profile. A chain
of links now leaves a pinned tab per step, the target alone staying a preview.
Driven in headless Chrome on the 18 825 node project: two `ref()` followed out
of a preview tab, then Back twice, lands on the first model at the line of its
`ref()`.

Trying that, the user asked for the lineage to behave as the tree does: one
click on a box opens its file as the preview, leaving the canvas where it is,
and a double click pins it and centres the lineage, which it already did. One
click is browsing, not following a link, so it records the move without pinning
the tab it leaves (`pushJump({ pin: false })`): walking the graph box by box
replaces one preview, and Back returns to the last pinned tab. The double click
records nothing more, since its first click already did. Driven in headless
Chrome on the same project: two single clicks left one pinned tab and one
preview, the canvas unchanged; a double click on a third box pinned it and
moved the canvas; Back returned to the pinned tab.

A key holding a mapping is a folder whatever its name
(0039). 0036 read a key named like a config as that config, through
`DBT_CONFIGS`, and called that dbt's rule. dbt-core's rule is the shape alone,
a mapping without `+` being a path, and Fusion's is the same, checked by
parsing one invented project with dbt-core 1.11.11 and Fusion 2.0.6. On the
18 825 node project the old reading hid ten `contract:` folder keys, whose tags
the manifest shows dbt applying, and fourteen folders below them: all 571 keys
now resolve, in 150 ms, and the levels of the breadcrumb list agree with an
independent walk of the file for all 3 825 models, where 31 disagreed. The list
now only names the mistake: a `docs:` or `meta:` written without its `+` is
dashed, and its card says dbt reads it as a folder and applies nothing under
it. Driven in headless Chrome on that project and on the invented one.

The lineage follows a file opened before its model was parsed, fixed
after it was reported from the VM: a model pulled since the last
parse opened onto "Select a model to see its lineage", and stayed there after
`dbt parse` and after Reload manifest, until its tab was clicked again. Two gaps
made that. `syncNode` missed in silence, and nothing ever looked again: the
button's `rerender` redraws only what is focused, and the page never heard of
the watcher's reloads at all, so the model counts and the manifest's date went
stale with it. A missed tab is now remembered in `S.unsynced`, the canvas says
the manifest has no node for the file (and names the stale manifest when the
badge does), and `followActive` looks again after either kind of reload. Said
in place of the empty hint, or, over a model left on screen, in an amber line
naming that model: the first version said nothing there, and the report that
followed it was a model named like the open file, read as its lineage. A file
whose buffer defines a macro, a generic test or a materialization
(`definesMacros`) says nothing, so opening a macro from the tree leaves the
model being read alone, as it always did.
The freshness poll is how the page learns of the watcher's: its `manifest_at`
passing `S.meta` fetches `/api/meta` again. Rejected: redrawing the canvas on
every background reload, since `dbt run` rewrites the manifest too and the
graph would reset under the pointer at every run. Driven in headless Chrome on
the 18 889 node project with a copy of its manifest missing one model: the
canvas named the file and the stale manifest, then drew it 6.6 s after the full
manifest was written back, and 2 s after Reload manifest. Over a kept canvas:
the line named the model drawn, a macro file opened after it hid the line, and
the file's own lineage replaced both 8 s after the parse.

Tests grouped by generic (0040): a hanging test said dbt's
generated name, about 50 characters clipped to 25, in the grey every test had.
The lineage payload now sends each test's generic, column, package and warn
severity, all four skipped when empty, and the canvas groups a model's tests
by generic, `not_null  105 cols`, opening into a row per column, with a test
alone reading `unique · id`. Tests on the whole model come first, with a
near-white bar doubled by a thin one, then singular tests with a light grey bar
in three pieces and a dashed outline, then the column tests in the grey every
test had, the bar's shape being what tells the three apart at a glance; a warn
test has an amber triangle. The first version gave the two new kinds indigo and
lavender, which cleared every box colour but read as kinds of model; greys keep
a row a test, and are held apart from each other only, since no light grey
clears a materialized view's cyan to a red-green colour-blind eye. Grouping by column was measured first and rejected: a column
carries one or two tests, so it would have drawn as many rows. Driven in
headless Chrome on the 18 825 node project: the most tested model, 120 tests,
draws five rows and `+8 more tests`; its not_null row opens to 105 by click and
closes by Enter, keyboard focus staying on the row; a group of six checks on
the whole model, each named in its YAML, lists them by their own names; a model
with three tests opens itself; a test picked from the Catalog's Preview shows
alone under its closed group; the legend names three kinds of test; an exported
picture holds the same labels. In jsc, 150 models of 30 tests each lay out in
a few milliseconds. The character budget a row gets was measured there too, at
5.1 to 5.4 px a character for an identifier, against the 6.8 the old clip at 25
assumed.

A test opens at its line (0041): opening a generic test
opened its properties file at the top. The manifest records no line for a
test, so the server now sends each test's `host`, the node whose YAML declares
it, and `testLine` finds the item over the buffer with `yamlOutline`, the
scanner the breadcrumb already used: no new parsing. Measured in jsc over the
13 308 generic tests of the 18 825 node project, whose files are CRLF, which
CodeMirror reads as lines like any other: 13 299 land on their own item, 9 on
their model's tests, their column written in the YAML otherwise than dbt
recorded it, none at the top; 269 ms for all of them. 2 483 of those tests hang
on a disabled model, which the graph keeps, so they find their host too.
Driven in headless Chrome on that project: one click on a test row lands on
`- accepted_values:` with the keyboard left on the canvas, a second test of the
same file moves the cursor and Back returns to the first, a double click pins
and focuses the editor, a Catalog chip and a Preview double click land on their
line, and a singular test opens its file at the top. A model's tests are often
spread over several files there; one click on a test of another file replaces
the preview tab, as one click on a box does, so Back skips it.

collin run from dbt-edith (0042): Collin's cache used to be
written by hand, `collin generate` in a terminal and a pick afterwards. Picking
Collin now runs collin when its cache is missing or older than the manifest, a
column clicked while it is behind waits for one run first, and Catalog >
Columns has Regenerate. collin is found beside the binary, then on the PATH,
and nothing of it is built into dbt-edith or runs at startup. Without it,
Collin's entry types `cargo install --git ...` into the terminal, which needs
no account since collin's repository is public. Collin is marked alpha in the
button and the menu, and Catalog > Columns gives the share of its edges matched
by column name rather than read from the SQL, or, from collin's report, how
many models it read the SQL of, with the reason over each model it could not
read. The menu offers "Update collin" to a collin that gives no version, or one
below 0.10.0, which reads a seed's header and lists the columns a literal sets
(0096, 0098, 0109), and
"Check for a newer collin" to any other (0043); a version installed by its tag
gets neither (0060). Both type
the install command, which cargo answers. On the 18 825 node project that
share was 100% while its manifest, written by Fusion's `parse`, carried no
`compiled_code`, before collin read `target/compiled/` (below).

A column's box opens its model's file: one click on
a model's box gave the file's preview and two clicks the pinned tab, for model boxes only. A
column's box only refilled the Catalog, which shares the dock with the lineage
and is hidden behind it while a column is followed, so walking a column from
model to model showed nothing. Its box now does what a model's box does, with
its model's file, which `/api/collineage` already sends on every box. Driven in
headless Chrome on the 18 825 node project: from `as_of_date` in a model's
Columns tab, one click on the same column in a downstream model opened that
model's file as the preview, the canvas unchanged, and a double click pinned it
and centred the lineage on that column.

The terminal on Windows, fixed from a report on the VM: `dbt run` in
the terminal printed "UNC paths are not supported. Defaulting to Windows
directory" and died on `C:\Windows\logs\dbt.log`. `canonicalize` spells the
project root `\\?\C:\...`, the shell started there, and pyenv-win's `dbt.bat`
shim hands it to cmd.exe, which refuses it. `files::plain` drops the prefix
once, where `main` resolves the root, so the shell, git, the Python sidecar and
every path the page shows get the plain spelling; the settings key was already
normalised without it, so no saved setting moves. The same report said nothing
could be copied or pasted there (0044). Driven in headless Chrome posing as
Windows: a double-clicked word reaches the clipboard on Ctrl+C with no `^C` to
the shell, a second Ctrl+C interrupts, and Ctrl+V types the clipboard, where
main's `app.js` sent `^C` and `^V`. The `.exe` cross-compiles.

SQL from `target/compiled/`: collin reads
a model's compiled file when the manifest has no `compiled_code`, and sets the
file aside when it reads a table the manifest does not give the model, then
matches that model's edges by name. Its report says so per model,
`sql_file` and `sql_file_set_aside`, and in its totals, which dbt-edith did not
read: a set aside model counts as parsed there, so the Columns tab said "SQL read
for 3470 of 3472" of a run that matched most edges by name. The count now leaves
the set aside out, the tooltip says how many models came from files and how
many were set aside, and the note over a model collin listed names its file.
A model read from a file with nothing else wrong is not in collin's report, so
only the tooltip's count covers it. Measured on two copies of the 18 825 node
project, the manifest stripped of `compiled_code` beside its own
`target/compiled/`: as it is, collin read all 3472 files and wrote the same
edges as from the full manifest; with every model's database renamed, as a
manifest for another target would have it, it set 1997 aside, and the Columns
tab read "SQL read for 1473 of 3472 models", where main said 3470. Driven in
headless Chrome on the second copy, the note shows over a set aside model.

collin's column notes: collin's report lists, per model,
columns whose lineage it lost, columns the warehouse and the compiled SQL
disagree on, names read from a CTE that lacks them, columns a child reads that
the model's list lacks, and columns matched by name to several parents. dbt-edith
read none of it, so a column with no edge looked like one with no parent. Each
is now a ! in the Lineage cell of its row, its reason on hover, and the note
over the table counts them and names, grouped by reason, the ones with no row:
a column only the compile produces, or a name only the SQL reads. The lists are
read field by field, so a later collin reshaping one loses that list and not
the report. A compile collin could not parse marks no column as missing from
it: it produced none collin knows of. Driven in headless Chrome on the 18 825
node project's own report: 6 columns marked on a model with lost columns, 16 on
one whose table is behind its compile with 5 compiled columns named as having
no row, and 15 matched by name to two parents on one whose SQL did not parse.

collin's report page (0059): the report was read for a summary and for the
model on screen, never whole. The server now files it by kind, from the lists
collin has written since 0.1.0, and the page showed it as an editor tab, a
section of Reports since 1.10.0 (0064):
collin's figures, then the findings grouped by who acts, each model a link to
its Columns tab. A run leaves a badge with the count of models and tables named
until the report is opened, now news on the Reports button, told apart from the previous run by a counter on
collin's status, and a run renames its report into place with its cache
rather than writing it in place. Driven in headless Chrome on Fivetran's
Shopify package with collin at 0.4.0's code: picking Collin ran it and left
"107 to investigate", the badge and the menu both opened the tab, a name opened
the model's Columns tab, and Regenerate brought the badge back.

collin versions side by side (0060): one collin at a time, the one beside the
binary or on the PATH. Another version now installs at its tag into
`<config>/collin/<X.Y.Z>/`, by a cargo command the menu types into the
terminal, and the project picks the one that runs among those found. A cache
whose `producer` names another version is stale, so a switch runs the new one.
Driven in headless Chrome on Fivetran's Shopify package, the release v0.4.0 as
default: the menu typed the command for 0.1.0, which built in about a minute;
picking 0.1.0 ran it (4875 edges, 117 to investigate, 23 compiles set aside
for two CTEs of one name), and picking the default back ran 0.4.0 (5548
edges, 107).

Two collin versions compared (0061): each run's cache and report are copied to
`target/collin/<X.Y.Z>/`, and two kept runs compare on the server, figures,
findings, edges by role and origin, the edges only one has, and the models
where the most moved. Driven in headless Chrome on Fivetran's Shopify package,
0.1.0 against 0.4.0: 703 edges gained, 30 lost, 600 with another role, the 630
inferred edges gone and 23 models read from their SQL, 16 of them inferred
and 7 unresolved before, columns with an edge from 4479 to 4996, which is what
collin's own pull request for that change measured.

The terminal's environment on Windows, fixed from a report on the
VM: with dbt-edith started from a shell where the project's venv was active,
`dbt ls` in the terminal ran a dbt-core installed elsewhere on the machine and
failed with "Could not find adapter type snowflake", while the project's
activate script answered that the venv was already active. `portable-pty`
builds the child's environment itself, and on Windows overwrites what was
inherited with the registry's variables, `PATH` included: the venv's `Scripts`
folder was gone, and `VIRTUAL_ENV`, which the registry does not hold, stayed.
`pty::command` now clears that environment and copies dbt-edith's own. Its test
compares the two on every platform but can only fail on Windows, which CI does
not run. On the VM it was not enough: the venv reached the shell,
second in `type -a dbt`, and `~/.bashrc` put pyenv-win's shims before it, while
VS Code's terminal, which activates the venv after the startup files, had it
first. So each new terminal now types the activate script of the venv
dbt-edith was started in (0050), once the line editor reads on Unix and at once
on Windows. Measured on macOS against `zsh -l` and its real `~/.zshrc`: the
line showed once, after the prompt, and ran within 0.6 s. The `.exe`
cross-compiles.

The profile on every project: the top bar showed
`profiles.yml` only once the Snowflake script had run with Snowflake's
features on, so a Snowflake project with column lineage on `none` offered no
profile at all, and the user asked where it was. dbt-edith now finds it where
dbt looks, `DBT_PROFILES_DIR`, the project, then `~/.dbt` (0049), behind a
violet button beside the snowflake, a user and a plug, grey with an amber dot
when there is no file there. Driven in headless Chrome on the 18 825 node
project with an invented home: the button opens the profile in a tab with
Snowflake's features off and still shows once they are on, and
`DBT_PROFILES_DIR` pointed at an empty directory greys it.

The config button: the `dbt_project.yml` segment at the end
of a model's breadcrumb (0038) became a round button in the same violet, a file
with an arrow coming into it, its name in the tooltip. Drawing it showed the
Catalog's `.crumb` rule giving every breadcrumb segment a margin that lifted it
3.5px and cut the top off a hover; the bar now scopes that away. Driven in
headless Chrome on a model of the 18 825 node project: the button sits whole in
the bar and opens the menu of keys.

Query history (0048): with Snowflake's features on, a dock
tab lists the queries the connected Snowflake user ran in the last seven days,
twenty at a time with Load 20 more, each linked to its page in Snowsight. It is
read by the same script as the column lineage, now asked by op, from
`INFORMATION_SCHEMA.QUERY_HISTORY_BY_USER`, which needs no privilege. That
function applies its limit before any `WHERE`, so the script scans its most,
10 000, and pages after the role filter, by start in epoch nanoseconds and then
query id, since two queries can start together. The role menu offers the
connected target's role, the roles of the other targets of the profile that
sign in as the same user and account, and every role; the `ready` event now
lists the targets, with their names and roles only, and a target signing in as
someone else is greyed with that reason. The script tags its own sessions
`dbt-edith` and leaves them out. Once read, the history keeps the script
running when another column lineage tool is picked. The link is built from
`current_organization_name()` and `current_account_name()`; without an
organization there is none, and the row copies the id instead. Nothing of it is
written or logged. Driven in headless Chrome on an invented project, through a
fake connector answering 45 invented queries: the tab only with the features
on, 20 rows then 32 under the target's role, all roles, the greyed targets and
their reasons, a statement cut at 10 000 characters, a refused connection
naming `profiles.yml`, and no link without an organization. Not the deferred
**Run history** below, which reads `run_results.json`. Not yet against any
warehouse.

Snowsight from the Catalog (0051): with Snowflake's features on, each relation
in the Location table, resolved and built, has a Snowsight menu beside its Copy
button, listing the object, its schema and its database as links that open in a
new tab. The account comes from a new `session` op of the script, which
connects when nothing has, so the first menu of a page says it is asking
Snowflake and may bring a sign-in tab; the page then keeps the answer until the
profile is saved or the features go off. The flag that kept the script running
for the history now keeps it for the menu too. Each part of the address is the
name as Snowflake keeps it, an unquoted one upper-cased, and the object's kind
comes from the materialization, a source being asked for as a table. Driven in
headless Chrome on an invented project through a fake connector that takes two
seconds to connect: the button only with the features on, the waiting note then
the three links, a quoted source keeping its case, the arrow keys and Escape,
and a refused connection naming `profiles.yml`. The addresses of a database, a
schema, a table and a view match the ones a real account's Snowsight shows,
organization and account in lower case. The menu itself has not yet run
against a warehouse.

Security pass of 2026-10-10 (0052, 0053, 0054). An audit of `main` found no
traversal, injection or leak, and six places to tighten, all done here. The API
and the terminal want the key each launch prints, turned into a cookie by the
link, since `Host` and `Origin` are headers any local program writes: one
`curl` with both used to get a shell. The profile reaches the page with its
passwords, tokens and keys replaced by a placeholder that saving puts back.
Opening a project no longer runs `python --version` and `dbt --version` from
its venvs; their versions come from `pyvenv.cfg` and the `.dist-info` names,
and a venv git tracks is never run. Replies are `no-store`, carry CORP, COOP and
a Permissions-Policy, allow the terminal's socket on this port alone, and a
non-read the browser marks as cross-site is refused. The profile's temporary
file is born 0600. `scripts/build_windows.sh` writes the `.exe`'s SHA-256
beside it. Checked by the Rust tests, against a real server over TCP: without
the key, `.env` and the terminal answer `401`. Driven in headless Chrome on an
invented project with an invented home: the link lands on `/` with the key
gone and the cookie out of the page's reach, the terminal's socket opens, the
profile shows two placeholders and saving a role keeps both secrets and the
file's 0600; a second browser context gets the bar, a refused socket and a
`401` on `.env`, and a tab reloaded after a restart gets the bar too. A
committed venv's `python` that leaves a mark when run left none. Compared with
`main` on the 18 825 node project, every read route and the main screens in
Chrome agree once ordering is set aside, as two runs of `main` do, and every
write agrees on an invented project. That comparison caught two regressions
before merging, both fixed: the venv tooltip named `dbt-autofix` for a Fusion
venv, and the temporary file's name, one per process, failed two writes at
once.

Supply-chain pass of 2026-10-15 (0085). The README's `cargo install --path .`
built the newest versions cargo could pick, not the lockfile CI audits, twenty
crates apart on the day; every install command now carries `--locked`, the
`.exe` build too. The vendored audit compared version strings and now compares
each file's SHA-256 with the upstream release it came from, all thirteen found
identical, and fails on a file it does not list. Dependabot proposes a version
a week after it is published. `cargo vet` was measured against the public
registries, 24 of 129 crates covered at these versions, and not adopted yet.

Security pass of 2026-10-15 (1.25.1). A review of `main` with a server up on
an invented project found six gaps, each needing the launch key or a hostile
file in the project, none a way past the key, all closed here. The compiled
SQL route joined whatever path the manifest recorded onto `target/`, and
`/etc/hosts` came back: candidates are relative now and confined to the target
directory, as the browser's paths are to the project. The profile's redaction
missed a value written on the line after its key, and a placeholder saved at
a deeper indentation got its block put back level with the key, where the
next showing sent it in clear: the line below is judged, and the block moves
with its key. A link in the project to a file not there yet passed as a new
file, and the write created it outside. The hover card's vocabulary lacked
`PASS`, `DSN` and connection URLs, and a password inside a URL is hidden now
whatever the name. The release look ran git from the temporary directory,
shared on Linux, under a name the pid made predictable: it runs from the
config directory, and makes its repository only where nothing was. A selector
name from a job file was typed with its control characters. Each has a test
beside it, the link ones on Unix (0090).

The update cleans after itself (0120, 1.54.1), amending 0095 and 0111. For a
binary cargo installed, the command runs `cargo clean --release` on the clone's
own `target/` once the install worked, so the folder no longer grows with every
dependency bump and every new rustc; a failed clean changes neither the install
nor the closing line, and a binary built in the clone keeps its folder. Each
update compiles the dependencies again, a little longer. Its limits: a
`build.target` or `CARGO_BUILD_TARGET` leaves `target/<triple>/release` as it
was, a `target/` cargo did not create is refused for want of its `CACHEDIR.TAG`,
and the update to 1.54.1 itself, typed by 1.54.0, has no clean.

Security pass of 2026-10-27 (0121, 0122, 1.55.0). An audit of `main` found
nothing critical and three things to close. A project copied with its `.git`
brought a configuration git acts on from the status poll alone: a file-system
monitor, a hook, a filter, a credential helper, an upload-pack program. Every
call now switches the monitor off, what runs on a timer switches hooks off
too, and a repository whose own configuration holds a key outside a list of
data-only ones is left alone, the Git panel and the freshness card naming the
key. The launch key was kept in a cookie, which a browser sends to every
server on 127.0.0.1 whatever its port; the page now keeps it for its own port
and sends it as a header, the terminal's socket taking it in its address. A
`.env` that is a link out of the project is no longer listed. Each has a test
beside it; the update command's branch was settled by 0129, and a collin
install in a shared folder is refused on macOS and Linux (0128) and in
Deferred for Windows, item 31.

The update says when it is installed (0095, 1.28.1). The first update run from
Edith's menu on the VM, 1.25.0 to 1.28.0, worked, and read wrong to the person
running it. The update is now a button named after the release, `Update to
1.28.0`, with what to do right under it: it types the command into the
terminal, Enter there runs it, and dbt-edith then has to be stopped where it
was started and started again. Once another binary is in place, the menu says
`Installed: restart dbt-edith to run it` where the button was, and the pill
`restart to update`. No version is named for it, since the file's time says
another binary is there, not which. The command fetches the tags in a step of
their own before the pull, whose failure stops nothing, so the build stamp
names the release it built.

The update ends on a line and a card (0111, 1.45.0), amending 0095. The command
ends on `dbt-edith is installed. Stop dbt-edith where you started it (Ctrl+C)
and start it again to run it.`, printed only when the install worked. A card in
the middle of the page says the same once the button has typed the command
(Press Enter, and the restart, gone after fifteen seconds) and when the menu
first turns to installed, closed by its button or by Escape, over every
backdrop; it replaces the toast for the update alone. A failed build in Git
Bash on Windows, the binary put back, now ends in failure, where it ended as if
it had worked; in PowerShell the line is skipped but the status is still a
success.

The Windows `.exe` is built by CI and the update downloads it (0123, 1.56.0),
amending 0068, 0093, 0095 and 0111. The release workflow builds an MSVC
`dbt-edith.exe` (x64, on `windows-latest`) and `dbt-edith-arm64.exe` (on
`windows-11-arm`), with `DBT_EDITH_RELEASE_ASSET` set, and `release` attaches
both and their `.sha256` files when it creates the release, then checks the URL
the update downloads from; a pull request touching the workflow, `Cargo.toml`,
`Cargo.lock`, `build.rs` or `src/` builds them without publishing. A binary
built so asks `update::download` for its command: the latest release's file of
its own architecture, checked against its SHA-256, the running `.exe` renamed to
`.old` and put back if the swap fails, in Git Bash and PowerShell. On Windows
the menu offers the download first and, for a binary cargo installed, the cargo
update beside it (`update::hows`, the `hows` list of `/api/update`); a build
anywhere under the clone's `target/` gets neither. Checked with `cargo test`
(both shells' text, both asset names, the choice's order and refusals, and the
real `sh` and `curl` on a `file://` folder: a matching hash swaps and prints the
closing line, a wrong or missing one replaces nothing, and so does a running
file that cannot be moved aside) and the Update menu's sentence by
`web/tests/update.js`. The real Git Bash and PowerShell 5.1 are driven against a
local server by a test that only compiles here and runs in both legs of the
release workflow, which also starts each `.exe`. Not checked: the workflow's
runs on GitHub until a pull request touching the crate has made them.

A test in the Catalog reads as a test (1.29.0). It was drawn as a model: Mat.
type `test`, no columns, a location table for a relation dbt never builds for
it, and "No description in the YAML." It has Preview alone now, which opens on
what it checks, as its hover card says it: the generic or `singular`, the node
whose YAML declares it, its column, its severity, `warn` in amber, and the
models it reads. A config section lists the keys it moved off dbt's defaults,
among `store_failures`, `store_failures_as`, `where`, `limit`, `fail_calc`,
`warn_if` and `error_if`, and the relation its failing rows are stored in when
it keeps them.

What a repository brings is held to it (0094, 1.29.1). Three findings of that
review waited a day. A collin git tracks is passed over, beside the binary or
on `PATH`, and the column lineage menu says which one and why: a
`dbt-edith.exe` copied into a project had run the `collin.exe` committed beside
it, its `--version` at every page load. A save from the page writes through a
`profiles.yml` that is a link, rather than replacing the link with a plain
file. A `profiles.yml` in the project that links outside it is refused by every
reader, its button says why, and the Snowflake script refuses it too. A `..`
after a folder that is not there, and on Windows a segment holding a colon,
are refused as paths that leave the project. Its review added four: git is
asked case aside, so a committed `Collin` counts, and the answer is kept
against the index; a path not there yet is walked link by link, a folder's
included; and git, `cmd` and a bare `python` are found on the `PATH` alone,
never beside the binary, where Windows looks first.

A model thousands of columns wide loads at once (1.29.2). Its column
lineage, its catalog and its column tests were each matched to its columns
one name at a time, over every column; one lower-cased index of the node's
columns per merge now serves the page and `--column` (0088) alike.

Custom selection in a panel (0091, 0092, 1.30.0). What the Reports panel
decided on its own, what goes inert under it, where the focus goes back, what
Escape closes, is one mechanism now, `PANELS` with `openPanel` and
`closePanel`, one panel down at a time: what follows the file in front stays
in the dock, what speaks of the whole project or wants the window's height
opens in a panel (0091). Custom selection is the first to use it. Its button
opens a panel over the editor and the dock, the sidebar beside it, the canvas
at the window's height, and a card beside the canvas, the Catalog's, which a
click on a box fills without opening its file. Escape, the × and a file
opened from the sidebar roll the panel up and give the dock back the lineage
it had; a double-click opens the box's file, rolls the panel up, and the dock
draws that model's lineage. The dock's canvas is moved into
the panel rather than copied, so its find, folded tests and legend pick come
along (0092). A model's lineage and git are to open the same way (Deferred,
items 17 and 18).

The settings open in a panel (0100, 1.36.0). One gear in the top bar, in place
of the column lineage menu, the snowflake and the profile's button, rolls the
settings down, a section at a time from a rail, as the reports: Project, with
the profile, the environments and Python, Column lineage, Snowflake,
Orchestration, Updates, and Help and tour. The rail names the column lineage on
screen, says whether Snowflake is on and whether a newer release is out, and
the gear and the Project row carry the amber dot the profile's button had when
dbt would find no profile. The status bar's `env` and venv pills open Project,
the update pill Updates; a venv is activated from its button there, the click
on the pill no longer typing the line. The environments' table and the job
sources are forms in their section rather than dialogs, kept as they were left
until the panel rolls up, and a clean read of the sources goes on to the
coverage, or to the Jobs tab under the panel. The sections draw the menus' own
rows, so the Columns tab's column lineage menu, the Catalog's environment menu
and Edith's menu stay and offer the same. The tour's Column lineage step rings
the section. On Jaffle Shop the top bar now breaks under about 860px, where it
broke under about 1050px.

A cut canvas says when its starts fill it (1.31.0). When what a selection
names takes half the canvas or more, as `source:*+` can, the warning under
the box counts them and says that Draw all, or a narrower selection, draws
the rest of their lineage; the order 0087 chose stays.

The Windows `.exe` names its publisher (0093, 1.33.0). A Windows build
writes a VERSIONINFO resource naming Datadorelix, dbt-edith and the version,
through mingw-w64's `windres`, which antivirus heuristics read on an unsigned
file. Without `windres` the build warns and goes on;
`scripts/build_windows.sh`, which makes the VM's `.exe`, refuses one without
it. MSVC gets none.

Indirect edges hidden by default (0097, 1.34.0). collin's `--indirect` writes
`filter`, `join_key` and `dedup_key` edges, from the column that decided which
rows a model keeps to every column of that model, and the column lineage drew
them as value edges and read them for the badge: on a Jaffle Shop copy that
filters `stg_orders` and sets a literal, a solid line ran from
`raw_orders.status`, the column of `where status is not null`, into
`stg_orders.loaded_at`, which the literal sets. The walk now crosses none of
them, upstream or down, and the status line, copied into an export, says
`indirect edges hidden`. An `indirect` button beside stop at, shown only when
the cache holds such edges, walks them and draws them thin, in short dashes
and a teal of their own, under one `indirect` entry in the legend. No badge
reads them, and a column's card says what keeps its rows apart from where its
value comes from, `rows kept by: raw_orders.status (filter)`, button on or
off. Stop at, the Catalog's Lineage cell, which reads `indirect only` for a
column only they reach or leave, and `--column`, unless `--indirect` asks,
leave them out by default. A cache without them answers as before, byte for
byte.

Warnings in Reports (0103). The config paths of `dbt_project.yml` that apply
to no resource, which dbt warns of once as it parses, are worked out from the
manifest with dbt's own rule and listed in a Warnings section, each at its
line and with why; the editor marks each key in a gutter of its own. dbt-core
agrees path for path on Jaffle Shop, Shopify and a Jaffle Shop copy given
such paths, in `scripts/compare_with_dbt.py`, and Fusion on that copy.

Each version of a model is named apart (0104, 1.41.0). The graph read a
node's `name` alone, which every version of a model shares, so v1 and v2 drew
two boxes of one name and the Catalog copied a name `--select` reads as every
version. A version is now named as `dbt ls` names it, `orders.v2`, on its box,
in the Catalog, search, `--select`, `--describe` and `--column`, and the
Catalog says beside it whether it is the latest, an old one or a prerelease.
The bare name reaches the latest, as a `ref()` naming no version does, in the
editor's links and on the command line; `ref('orders', v=1)` links to v1. The
implicit method matches a version by `orders.v2` or `orders_v2` and never by
`v2` alone, and `version:` is read. A Jaffle Shop copy with a model in three
versions agrees with `dbt ls` on each, where main disagreed on every
selection that reached one.

Who calls a macro (0105, 1.42.0). A macro's card says how many nodes reach it,
by kind, `used by 9 models, 40 tests`, through the macros calling it too,
since dbt records each one's own calls only, and from each node's SQL and
hooks where dbt recorded no call; the palette row says the total, and a click
on the card's line opens Reports' Macros section on that macro, every node
reaching it listed. That section lists the project's macros no node reaches,
never as unused: each is grouped under why it may still run, from a
run-operation or another package's call to a dispatch variant for another
adapter, a materialization or strategy macro dbt runs, or a call dbt makes
itself, walked down from what it runs. Only what nothing explains lights the
Reports button.

An exported lineage zooms as close as the canvas does (1.42.1). The wheel of
the `.html` export stopped when the view was 120 units wide, which on a tall
graph left the boxes too small to read, since the scale on screen follows the
height. `zoomViewBox` now stops at the closer of that floor and the canvas's own
scale of 3, read once per wheel step; a wide graph stops where it did. In
headless Chrome, a frame 600 wide and 20 000 high stopped at 0.17 px per unit on
`main` and goes to 3.0; one 1 000 by 560 stops at 10.2 on both.

The Halloween look (0106 to 0108, 1.43.0). A pumpkin in the top bar dresses the
page in orange and black, off until pressed and kept per machine in
`look.json` (0106). The Theme section of the settings picks the look, the
random apparitions, the sound and the style of the boxes, a tombstone, a coffin
or a grimoire. Their art is drawn up to 400 boxes, a plainer version of the
style past it (0107); the edges become silk with dewdrops and the selected box
grows a web (0108). A ghost, a witch, the zombie switches and the apparitions
come with it, their sounds made in the browser and muted by default. An export
or a copied image is the same with the look on or off, which
`scripts/walk_export.py` checks in Chrome for the three styles.

collin is given the project for a manifest lying in it (0109, 1.43.1).
`--project` used to go to every run, and collin 0.10.0 reads a seed's header
through it, so `dbt-edith --manifest /artifacts/manifest.json` started in
another project's folder would have given that manifest this folder's seeds.
`collin_run::manifest_in_project` compares the two paths, resolved and a
component at a time, in the page's Regenerate and in `--run-collin` alike, and
the flag goes only when the manifest is under the folder. `WANTS` is 0.10.0.

Same answers on every start (0055): the manifest's sections are read sorted by
unique_id, so a node's index, and every list and first-of-several that follows
it, no longer changes between starts. Where the pick is one a person reads, it
goes by name: the node a properties file shows, equal search hits, the names
Copy keeps past 5 000. A dbt-core source test that also reads a model now
hangs on the source's column. On the 18 825 node project, now 19 233, three
starts of 1.2.0, asked the same 4 682 read requests, disagreed on 99: 21
properties files, 39 selections with tests past the canvas cap, 9 capped
lineages and 30 searches at their limit. Three starts of this build agree on
all of them to the byte, and differ from 1.2.0 only in the version they report
and where an answer was capped or had several candidates to pick from. In
headless Chrome, two starts showed the same search list, the same node for
each properties file and the same boxes and edges for each selection, where
two starts of 1.2.0 differed on all three.

Checked against dbt on Shopify too (0056): `scripts/compare_with_dbt.py` clones
Fivetran's Shopify package at the tag dbt-collin pins, installs its locked
dependencies and parses its integration tests, then compares 29 typed lines
and the lineage of its models, at one, two and twenty levels each way and at
two with tests, with `dbt ls -s N+model+M`. The lineage agreed from the first
run, on all 240 models. The typed lines did not: the fqn method reached
sources, so `*` listed 87 sources dbt does not, and a bare name selected a
source table of that name; it no longer does, and a term only sources answer
says to use `source:`. Under dbt-core 1.12.5, 1052 answers agree in 96 s with
`--every-model`, as CI runs it, and 212 in 23 s in check.sh, which compares
every eighth Shopify model, against 6 s for Jaffle Shop alone before. dbt
Fusion 2.0.6, run by hand, agrees on all but `file:shopify.yml`: it lists none
of the 20 tests that file declares, where dbt-core lists them all.

Selections on the command line (0057): `--select LINE` and
`--selector NAME` resolve, print and exit, with no port, key, browser,
settings or catalog, so an agent skill can ask for a lineage with one command.
They go through the box's own entry points, in the route's order, with dbt's
defaults rather than the box's: tests in, every name, no cap. `--output json`
adds the parents of each node, which `dbt ls --output name` cannot give and a
question about the edges between two layers needs. stderr names the manifest,
its date and dbt version, then the freshness badge's verdict, written after
the answer because the walk and git are most of the time. On the 19 233 node
project, everything downstream of a staging model, tests excluded, came out as
the same 355 names dbt Fusion 2.0.6's `dbt ls` printed, in 0.9 s, 0.16 s of it
the manifest, where each `dbt ls` took 14 to 20 s. `scripts/compare_with_dbt.py`
asks the command line every selection it asks the box, and checks it wrote no
settings: 1 124 of 1 124 agree with `--every-model` under dbt-core 1.11.15
and 1.12.5, for about 3 s more on check.sh's run. No skill is written here;
the one on that project still runs `dbt ls`.

A column on the command line (0088): `--column MODEL.COLUMN` with
`--up` and `--down`, a number of levels or `all`, and `--stop-at`, walks the
canvas's own column walk over a cache already beside the manifest, prints and
exits, with no port, key, settings or catalog. `--output json` gives one line
per model reached, with `original_file_path`, `materialized`, and only the
columns reached, each with its depth, the columns it is read from and the role
of each edge, its tests, and `more_up` or `more_down` where the answer stops
short. Without a cache named, collin's or Fusion's newest answers before
Snowflake's, which holds only the columns fetched. Nothing produces lineage
there: a cache older than the manifest still
answers, and stderr gives the steps that write one, the target left as
`<TARGET>` for the user to name, with the profile's target names, the `.env`
files and the command of the dbt that wrote the manifest, sent to the
manifest's folder when it is not `target/`; Fusion's
`docs generate` writes no catalog without `--write-catalog`, checked with
Fusion 2.0.8 and dbt-core 1.11.16 on Jaffle Shop. collin's report from the
same run says whether it set files aside as compiled for another target. On
Jaffle Shop, `dbt build`, `dbt docs generate` and collin 0.7.0 gave a cache
whose answers follow `raw_payments.amount` through its rename into
`stg_payments` and its aggregates in `orders` and `customers`. On a large real
project the command line and the page gave the same columns, edges and roles
for one column, upstream, downstream and three levels each way. Discovery
now asks a cache for `edges`: the report `collin generate` names
`column_lineage.report.json` beside its cache had been offered as a cache of
no edges, and answered in its place when collin's cache was missing.

Column counts that say what they count (0089, 1.26.1). The stop at menu counts
each role's edges on the canvas, from the answer last drawn, with a line for
the edges that have no role, so that it adds up to the status line; the
project's count is on hover. The Catalog's Lineage cell counts the whole chain
upstream and downstream, as "in total" does for a model, and its hover adds
the one level the canvas opens on. `/api/colreach` walks it, asked by the
Columns table alone: each side is one pass for all of a node's columns, cut
into strongly connected parts and crossed once in topological order, where a
walk per column took hundreds of milliseconds a table on a dense synthetic
cache, and seconds on one shaped as a long chain.

A capped selection, nearest its start (0087, 1.27.0). A selection draws up to
3 000 nodes, where it stopped at 400, and past that keeps what is nearest what
its criteria matched before their operators grew it, breadth first, where the
cut used to keep the first names: `source:stripe+1` no longer draws sources
read by nothing. A warning under the box says so, and `Draw all N` draws the
rest of that line, up to 20 000 nodes. Model mode keeps 400. A selection
1.26.1 drew whole answers as it did, its new `most` field aside.

Clone into my schema (0058): with Snowflake's features on, a button beside a
model's, a seed's or a snapshot's Location heading writes the statements that
copy it and its direct parents from an environment into the developer's
schema, which the user copies and runs. It ports an agent skill written for
the 19 233 node project, from the relations the Catalog already resolves: a new
route lists the nodes, walking through ephemerals, with every `.env` file's
resolution, and the page writes the SQL. The destination comes from the
profile's active target, read by the scanner `redact` already used, now in
`profiles`, and what the user types instead is kept per project. The settings
menu now names every Snowflake feature, the Snowsight menu included, which it
had missed. On that project, 3 516 nodes asked of the route answer 200, in
under 11 ms each, and the same on two starts to the byte; for 40 models drawn
by kind, with UAT, all 156 statements are the ones the skill's own script
prints, character for character, ephemerals included. Every read request of
the replayed sample answers as on `main`, the version aside. Driven in
headless Chrome on an invented project with every materialization and on that
project. Never run against a warehouse.

A lighter status poll and a faster content search (1.8.1): measured on `main`
against the 19 233 node project, every click answered within 72 ms, but the
poll every 5 s cost the server about 0.8 s, and a content search 550 ms. Most
of the poll was git's helper napping 30 ms before looking whether git had
finished, which commands of 5 ms paid in full, ten times per round. It now
naps 1 ms first, doubling to 30. The status takes two git calls, the branch,
its upstream and how far apart they are coming from `git status
--porcelain=v2 --branch`, and the freshness badge reuses the git directory the
status read. A hidden tab stops polling and catches up when shown. The search
reads its files on up to eight threads, in batches whose results are counted
in index order, so the ceilings fall where one thread put them. On that
project the poll went from 405 to 250 ms for the status, git's own status
being 190 of it, and from 360 to 230 ms for the badge, the walk being most of
it; the search from 550 to 165 ms. Every replayed read request, 60 searches
and the git answers of 14 repository states, from an unborn branch and a
deleted upstream to a rebase stopped on a conflict and a linked worktree,
answer as on `main`. Rejected: cancelling a search a newer one replaces, since
a second tab would then be shown a partial answer as a whole one, and keeping
the files' contents in memory, which still pays a stat per file to know they
are current.

The Jobs tab (1.9.0, 0063): which Airflow DAGs and dbt Cloud jobs run the node
in focus, read from YAML in folders named in the settings, outside the project,
which is what the deferred orchestration coverage was waiting on. dbt Cloud is
read in dbt-jobs-as-code's schema; Airflow in a proposed shape whose every key
is a dotted path a source can rename, so the 19 233 node project's DAG factory,
whose files are laid out its own way, reads with four renamed keys and no code
of its own. A YAML reader
written here, `src/yaml.rs`, builds the tree that jobs-as-code's anchors and
`<<` merges need, and refuses a file whole at a line. `src/command.rs` reads a
command line flag by known flag and refuses an unknown one rather than guess
whether it takes a value. On that project's orchestration repository: 48 DAG
files and 116 tasks, none refused or unresolved; 34 DAGs scheduled, 33 on a
cron and one on an asset five test tasks update; six steps that select nothing,
three on tags no model carries and three on tags only disabled models carry;
and 3 213 of the project's own 3 474 models, seeds and snapshots run by a
scheduled DAG, 260 only by manual ones, one by none, the 34 of an installed
package being left out of the count. Opening an installed package's node, from
those lists, the palette, the Models list or the lineage, used to fail with no
such file, since the manifest gives its path inside the package; it is now
looked for in `dbt_packages` as a package macro is, and asked for there by
every answer that names a file, while selection keeps dbt's path. The 34
package nodes answer as on `main` but for that path. The first read takes 1.2 s in release, a node
9 ms at the median and 55 ms at most. Ten distinct step shapes were compared
with dbt Fusion 2.0.6's `dbt ls`: nine agree, and the tenth, a `test --selector`
whose selector the VM's manifest wrote without its `indirect_selection: empty`,
is the loss 0032 already names, 181 tests against 127, which the step now says.
On Jaffle Shop, the invented jobs under `tests/fixtures/jobs/` agree with
dbt-core's `dbt ls` step for step in `scripts/compare_with_dbt.py`. The walk
first skipped any folder named `target` or `logs`, which on that repository is
a layer of eight DAGs, without a word; only `node_modules` and `__pycache__` are
skipped now. Every replayed read request answers as on `main`. Driven in
headless Chrome on that project. Not yet on a real dbt-jobs-as-code repository.
Since 1.49.0 (0115) the sources may be kept in the project's own
`.dbt-edith/jobs.yml` instead of the settings, which the command line and MCP
read too: see the paragraph below the agent skill's.

Reports (1.10.0, 0064): what speaks of the whole project sat in two places,
collin's report in an editor tab behind a badge, and Coverage in a view of the
Jobs tab. A Reports button in the top bar now rolls a panel down over `#main`,
the editor and the dock both, with a rail of sections: collin's report once a
run wrote one, the job coverage once a source is named. The sidebar stays.
Anything that shows a file or a dock tab rolls it up: `activate()`, a link in a
report through `openFromReport`, the palette, the Models list, the terminal's
key and anything typed into it, Escape and the close keys. Shown again, a
section keeps the folds and the scroll it was left with, so a list is gone
through one model at a time. The coverage now groups a model under the
deepest folder it shares with another, its path below that said on its row:
on a project with a folder per model, a list of hundreds of models sat in as
many folders of one, and now sits in a handful of shared folders. collin's badge became news on the button, under
0059's rule, and the Jobs tab keeps the node in focus with a button to the
coverage. `GET /api/reports` says which sections there are from file times and
the settings alone, and `/api/freshness` carries `reports_at`, the newest
report file, which is how the page hears that a run wrote one. Driven in
headless Chrome on Jaffle Shop with an invented report and a DAG over the
staging models: 32 checks, the editor and the dock under the panel and the
sidebar not, no key reaching the editor, Escape and `⌥⌘R`, the focus given
back, a report's name and a coverage row each opening the model with its dock
tab, and no roll under reduced motion; then collin 0.2.0 picked from the menu,
its "5 to investigate" on the button, the panel opening on it, and the news
gone once shown. On a large real project, against `main`, every replayed
read request answers the same bytes but where two starts of one build differ
as well, both sections are drawn in under a second, and the coverage gives
the figures it gave in 1.9.0.

Last run (1.11.0, 0065): whether the last build went through was the
terminal's to say. A section of Reports now reads `target/run_results.json`
as dbt left it: the command and its selection, a tile per kind with what ran
against what the manifest holds, the errors and failed tests with dbt's
message, the warnings, the skipped nodes and the slowest. A model opens with
its Run tab, a test at its line, a unit test at its YAML, read from the
manifest's own section of them. What the command did not reach is counted,
never listed, an ephemeral model aside since it never runs on its own; and the
section says when the manifest was written since, when the run names tables
the manifest names otherwise, and when `compile`, `show`, `docs generate` or
`run-operation` wrote the file over a run. A macro run by `run-operation` or
`show --inline`'s query is no node gone from the manifest. Errors and failed tests ride on the Reports button, the
index parsing the file once per write through `files::Cached`, which the
section reads too. Statuses read in dbt-core's spelling and in Fusion's.
Driven in headless Chrome on Jaffle Shop after a real `dbt build` with a model
in error, its child skipped and a failed singular test: 13 checks, then a
`dbt test` run while the page was open, its "1 failed" on the button within a
poll and gone once shown. On a large real project, against `main`, every
replayed read request answers the same bytes but where two starts of one
build differ as well, and its Fusion `compile` reads as one that ran nothing.
The job coverage has news too, how many more models no scheduled job runs than
when it was last shown (0066), and with several reports to look at, each chip
on the button gives its count alone. Driven with the three at once on Jaffle
Shop: the coverage seen, then a collin run, a model added that no job selects
and a failing test, the button reading "1", "5" and "+1" at 115 px, each chip
going once its section is shown.

Tabs follow their files (1.11.1, 0067), fixed after it was reported from the
VM: a model open in a tab kept its old text after a `git pull` in the
terminal, until the page was reloaded, and saving it would have written that
text back over the pull. Each read of a file now carries a stamp, its size
and modification time; `POST /api/file/stamps` gives the open tabs' stamps,
asked with the git poll, when a tab is shown and after a git action. A tab
with nothing unsaved takes the new text as one replace of the stretch that
differs, under the origin `disk`, which the editor's change handler does not
count as an edit, and clears the undo history, since one Ctrl+Z would
otherwise take the pull back out for the next save to write. A tab with edits
keeps them and turns amber, and its save sends the stamp it started from,
which the server refuses with 409 once the file moved, so the page asks
first. Saves are checked and written one at a time; a tab's second save waits
for its first, and what is typed while one is out stays unsaved
(`changeGeneration`). A review before merging found those last three, and a
struck-through tab that could be neither saved back nor closed with a
question. Driven in headless Chrome on a copy of Jaffle Shop with a remote and
a second clone pushing to it: 44 checks, a pull shown in the tab in front
within the poll with the cursor kept on its line, one in a tab behind, a tab
with edits keeping them through a pull and its save asking, a plain save
asking nothing, a deleted file struck through, written back and asking before
closing, undo after a reload changing nothing, typing during a slowed save
kept, a double save asking nothing, Save all saying a declined file was left
rather than failed, a file turned binary marked and read once, and a CRLF file
neither reloaded by a new stamp over the same text nor marked edited by a real
change.

Update offer (1.12.0, 0068): a release was found by looking at GitHub. Once a
day, in the background, git now lists dbt-edith's tags (`ls-remote`), the
answer kept in the config directory for every start of that day, and a newer
release is a pill after the version in the status bar. Its menu links the
release's notes and, for a binary installed by cargo or built in the
`target/release/` of a clone still on the machine, types the pull of `main` and
the rebuild into the terminal; on Windows the running `.exe` steps aside as
`.exe.old` first and comes back if the build fails, and a build in the clone
gets no command, since cargo writes it through a file the rename does not
free. A copy carried from another machine gets the notes, and every binary
without a command is told why. Driven in headless Chrome on Jaffle Shop
with a release build: a start with an empty config directory asked GitHub,
found 1.11.0 and showed nothing; with 1.12.0 kept for the day, the startup
banner named it, the pill and its menu showed, and the command landed in the
terminal unrun.

Update looks through the Paris night (1.18.0, 0079), from a report on the VM: a
start showed no update with several out, an older answer still standing under
0068's 24 hours. An answer now stands until the next turn, every hour on the
hour from midnight to 09:00 in Paris and then midnight, worked out from the
EU's summer-time rule; a look that got no answer is tried again an hour later,
and Edith at the top left opens a menu, as the pill does: the version and
build, every release since the running one by its pull request's title (fetched
once per release through git, the tagged commits alone, and kept with the
answer), when GitHub last answered or what git said when it did not, when it
looks next in the machine's time, and Check for updates. Driven in headless
Chrome on Jaffle Shop with a debug build: a start with an empty config
directory asked GitHub and said 1.17.0 was the newest, a kept answer naming
newer releases put a dot on the name, the pill in the status bar and one row
per release in the menu, and a proxy git could not reach put git's line in the
menu beside the older answer. Pull request titles are held to fifty characters
from here, one idea in the user's words, so each fits its row (AGENTS.md, and
CI's naming check).

The venv the terminal activated (1.12.1, 0069), from a report on the VM: once
a project's own script had switched Git Bash to its Fusion venv, the prompt
named that venv and the status bar still said `venv (inactive)` with the
project's other one, as it had at startup. bash is now handed a
`PROMPT_COMMAND` that prints its `VIRTUAL_ENV` as OSC 6973 before each prompt,
and the page names that venv, with the versions `/api/meta` now sends for every
venv found (`found`), and goes back to the startup answer when the shell exits.
A venv is matched by its folder or by the one its activate script sets, which
is how bash spells it behind a mapped drive or a symlink, and a folder matching
nothing has `GET /api/venv` read the project's venvs again.
Checked by a real bash on a terminal in the Rust tests, and by headless Chrome
on a large real project with `/bin/bash` as the shell: activate, deactivate,
activate the other, exit, each shown at the next prompt; started inside a venv,
the typed activation reported as the terminal's; zsh unchanged.

Picking a job source (1.13.0, 0070): a source's path was typed by hand, and a
typo was found only by saving. **Browse…** beside each path opens a picker
under it, already open on a source being added, from the form's buttons or
from Add a source. `GET /api/jobs/browse` lists one folder, located as a
source's path is, with what the walk would read there: its folders and its
YAML files by name, never a hidden entry, a symlink or a junction, folders
first and at most 2000; a path that leads nowhere, into a hidden folder or
onto a link lists the nearest folder above it, and a folder the system will
not list is shown empty with the reason. The picker drops an answer to a
listing asked for before the last one, and the second click of a double click
counts only on the row the first one hit, since a folder opened by the first
can put a file under the pointer. A pick is written from the project within
two folders above it, in full otherwise. The job coverage is now listed in
Reports before any source is named, as where one is added: hollow in the rail,
it is not opened on while another section has something. Driven in headless Chrome on a large real project with its
orchestration repository beside it: Reports to Add a source, the picker on the
project, up one folder, three folders down, Escape closing the picker and not
the form, a path typed into the picker, and Use writing
`../<repository>/dags/...`.

A pan from a box opens nothing (1.13.2), after panning was reported from the
VM as going wrong with the folder bands on. Dragging the canvas from a box
ended with the browser's click on that box, since the box follows the
pointer, and the box's file opened in place of the one being read. A press
now becomes a pan past 4 px either way (`pastSlop`), Windows' own drag box,
which the exported viewer uses too, and only then shows the grabbing cursor.
The pan moves the view by each move since the last, so a wheel zoom, a fit or
a redraw in the middle of one stands. Only the left release ends it, and it
puts a one-shot click listener on the window in the capture phase, which
swallows the click that release brings; the next press, or a timer when the
release brings none, removes it. A context menu or the window losing focus
stops a pan, and the click its left release may still bring is swallowed
too. A double click is dropped while fewer than two presses came since the
last pan (`panDouble`), since the system counts the pan's release as a click.
The bands themselves never stopped a pan: a drag from a band, from the gutter
between two, or with the CPU slowed, moved the canvas as without them. Three
reviews before merging shaped it: a flag that eats the next click left armed,
then pointer events with capture that brought a hover card after every pan
from a box and a pan a right click no longer ended, then a pan ended by any
button's release and an offset from the press that undid a fit. Driven in
headless Chrome on Jaffle Shop and on a large real project, with and without
the bands, 22 checks each: a pan from a box, with a drag lock too, opens
nothing and brings no hover card; a click, and one that trembles 3 px, still
opens the box, with no grabbing cursor; a pan from the background moves; a
release over the editor ends a pan; a right click stops it, a middle click
leaves it going, and the left release after either opens nothing; a Ctrl+click
whose release a context menu takes leaves nothing following the pointer; two
quick pans fit nothing; a click right after a pan re-centres nothing, while a
double click of its own does; a zoom or a fit during a pan stands. Ten of
them fail on `main`.

The venv in the status bar, coloured and clickable (1.14.0, 0072), asked for on
the VM, where a `deactivate` left the bar's grey text barely changed. The bar
is now a tinted pill: green with a full dot when the venv is active, amber with
an empty one when the venv found is not, grey and outlined with none. While it
is amber, a click types the line that activates the venv it names and leaves
Enter to the user (0071). Only that venv's line is typed at once: with another
offered too, or none for the named one, the click lists them, the named one
first, and a venv the server read twice, as `VIRTUAL_ENV` and as the project
spells its folder, is listed once. The project's venvs are read again on each
click. The line is made by the server with `pty::activation`, the function
behind the line a terminal gets at startup (0050), and each venv in `/api/meta`
and `/api/venv` carries it as `activation`. A venv with no script for the
terminal's shell carries none and is not offered, nor is one whose path holds a
control character, which would reach the line editor as a key. The pill is a
button, reached by Tab only while it offers something. Checked in headless
Chrome on Jaffle Shop with `/bin/bash` as the shell: amber at startup; two
venvs, the menu, with nothing marked in it; the line typed unrun, green after
Enter, amber again after `deactivate`; a folder named with a Ctrl+U and a
carriage return read with no line and not offered; a venv made after startup in
the menu on the next click; the named venv without a script, the other listed
and not typed; one venv, typed at once; Enter on the focused pill opens the
menu, and Escape gives it the focus back.

The terminal's line keys (1.14.1), after Ctrl+Left, Ctrl+Right and Home were
reported from the VM as doing nothing in Git Bash, where VS Code's terminal
moves by word and to the start of the command (0073). To Git Bash on
Windows, `termLineKey` sends the keys readline binds by itself in emacs mode:
Ctrl+A and Ctrl+E for Home and End, ESC b and ESC f for Ctrl+Left and
Ctrl+Right, Ctrl+W and ESC d for Ctrl+Backspace and Ctrl+Delete. On a Mac,
in any shell, Cmd+Left, Cmd+Right and Cmd+Backspace send VS Code's Ctrl+A,
Ctrl+E and Ctrl+U, where xterm.js sent nothing, and Option+Delete sends ESC d.
Only the shell's own line gets them: bash's prompt hook (0069) sets
`S.termPrompt`, Enter clears it (`promptAfter`), and on Windows, when bash
never said, a program asking for application cursor keys counts as one
reading the keys. Not on a Mac, where oh-my-zsh asks for them at every
prompt. The shortcut list names the keys of the platform it is read on.
Driven in headless Chrome posing as Windows, with bash as the shell: at the
prompt each key sent its bytes and bash moved or deleted as meant; in
`less -X`, still on the normal screen, and in `cat`, the keys stayed
xterm.js's; back at the prompt they were bash's again. With a zsh and its
user's own startup files on a Mac, Cmd+Left reaches the start of the line, as
under a zsh asking for application cursor keys at its prompt, as oh-my-zsh
does. Why the VM lost xterm.js's sequences was not seen: every step from
xterm.js through ConPTY and the MSYS2 runtime to Git's `inputrc` reads right
on paper.

A node's jobs by how they start, and in the Catalog (1.15.0). The Jobs tab
listed a node's jobs in the files' order, so a manual DAG could come before
the one that runs the model every night, and only the schedule's colour told
them apart. Its cards, the steps not resolved and the coverage's Jobs read now
come scheduled first, then what something other than a clock starts (CI, a
merge, an asset only a manual DAG updates, `@once`), then manual, each class in
the files' order (`jobsStart`, `jobsByStart`), sorted in the page so the
server's answer stays the one 0055 settles. The schedule became a pill: green
and filled, **Scheduled** and the schedule; grey, **Manual**; for the class
between, its words alone, since it is neither. The coverage's *Manual only*
became *Only in unscheduled jobs*, which is what it always listed. The
Catalog's Preview has a Jobs section under Location: one chip per step that
runs the node, `DAG › task` or `job › step n/m`, green or grey as its job's
pill and in the tab's order, the schedule and the command in its tooltip. A
click opens the Jobs tab on that node, which a single click on a box puts in
the Catalog without moving the focus (`S.jobsFor`, held while the Catalog
shows that node and cleared by the next `focusNode`); the tab scrolls its own
body to the card and lights the step, as a jump in the editor does. The step is
found by its job and its task id, or its place in a dbt Cloud job
(`jobsStepKey`), since the tab's opening may read a changed file again and give
its index to another step; the index only tells apart two sources of one label. Dashed chips count
the steps not resolved and the files refused that could run the node, and open
the tab on them. A node no job runs says so, and without a source the section
is not drawn. The Catalog asks `/api/jobs/node` without `look`, and only while
it is seen, its tab forward and Reports rolled up: one drawn behind either is
filled in by `showDock` or as Reports rolls up, so no tab nobody looks at reads the
job files (0063). Each answer is kept per node (`S.catJobs`), so the Catalog
drawn again for an env, a sidecar or a profile asks nothing, until a look, a
save of the sources or a manifest that moved forgets them all
(`forgetCatalogJobs`), never a switch of column lineage or a collin poll;
what it shows then stays until the next answer. A review before merging found
the tab opening on the focus rather than the Catalog's node, a request on every
redraw, a section that ignored a save of the sources or a refused file, and a
first read under Reports. Driven in headless Chrome on Jaffle Shop with
invented jobs: the order, a chip lighting its step, the tab on a box clicked
once, the unresolved and refused chips, a node no job runs, nothing asked while
the Catalog is hidden or under Reports or drawn again, and one request after a
look or a save. On a large real project with its orchestration repository,
before the review: the order, a chip lighting its step, a node no job runs,
and nothing asked while the Catalog is hidden.

A guided tour, Give me a tour at the top of the shortcut list, and offered once
per machine by a card under the `?` button (0074), by the first page with a
model to show: a page with none writes nothing and asks again when a manifest
lands, so a `dbt parse` after opening brings the offer (0078). Fourteen steps
on a project with a model, each ringing one part of the page, the rest dimmed,
with what it is for: the freshness badge, right after the welcome, saying that
everything is read from `manifest.json` and as true as the last `dbt parse`,
what its age and its dot's colours mean, and that a click types `dbt parse`;
the model's file, the `dbt_project.yml` button with its menu open, the
lineage, a Custom selection of `+model+`, Export, Copy image and Copy names,
the Catalog, the column lineage menu, the Jobs tab, the Reports button, the
Terminal tab, Search and the `?` button. The model is the server's pick
(`src/tour.rs`): one of the project's own with parents and children, then one a
job runs when the Jobs tab has a source (`jobs::run_by_a_step`), the most on
its thinner side up to three, then the neighbourhood closest to eight, then by
name; `stg_orders` on Jaffle Shop, with the invented jobs or without. A step
about a model is left out without one, and a feature that is off says how to
turn it on: no job source, no column lineage tool, Snowflake's features off, a
Reports button dimmed until a report has something. It runs nothing and writes
no setting, and never asks to look at the job files: its Jobs step shows the
jobs as last read, read once by the pick if nothing had, and a Reports panel it
rolled up comes back as it was left, unread. It points at the Terminal tab
without opening it, and at Export without pressing it. Ended at any step, it
puts back the tabs in their order, a preview its own took the place of
included, the tab in front, the canvas and its mode, the node Jobs, Compiled
and Run speak of, the selection box and its history, the Catalog, the dock and
a Reports panel that was down; a page with no tab open keeps the model as a
place to start. The steps set the page up one at a time, and one passed by a
held key or an end stops at its next look, so none finishes over the next or
after the end; the overlay stays, saying so, until the page is back, so nothing
done meanwhile is undone by it and no second tour starts on a page half
restored. The page under it is inert, the `dbt_project.yml` menu it opens too,
the app's keys wait, the browser's answers to them as well, and the arrows,
Enter and Escape walk it. `tour.json` beside `update.json` keeps when it was
offered and the newest version that served a page, which the tour of what is
new (below) will start from. `web/tests/tour.js` checks the steps and their
wording per feature, and `scripts/walk_tour.py`, in `check.sh`, walks every
step in headless Chrome on Jaffle Shop: no offer before `dbt parse` and the
short tour saying there is no manifest, the ring around each target, the page
under it, Back, the offer once the parse lands, at the end of a tour on screen
when it lands, and not after a reload, the terminal never started, no file in
the config directory but `tour.json`, and a page in use put back after Escape.
A welcome without a model says whether there is no manifest or a manifest with
none, ringing the `?` button the tour is taken again from, and one on a
package's model, the project having none of its own, names the package.

The Reports button is in view from the first paint, dimmed while no section
has anything, which a first start after a `dbt parse` never has, with no chip,
its tooltip saying what fills it: the last run, collin's report, the job
coverage once a source is named (0075). It keeps 3:1 against the bar at 70%
opacity, which half did not. A click opens the empty coverage, where a source
is added, as `Mod+Alt+R` did. It was hidden until then (0064, 0070), so nobody
new to the tool saw that Reports exist. The tour's Reports step points at it
as it is, and waits for the reports' index, so it says what the button shows.

Incremental strategy: an incremental model's box names its strategy in place
of the word `incremental`, the adapter's default when none is configured, and
`append` with the strategy struck through when the macros dbt runs insert
every row for want of a unique key. The default and the insert are read from
the manifest's `macro_sql` in `src/incremental.rs`, the way dbt reaches each macro:
by name, through `adapter.dispatch` in the order `dispatch:` gives, and from
the materialization's fallback (0076). The insert is claimed on three known
macros only, each by its own test of the key, so DuckDB's, Jaffle Shop's
adapter, claims nothing. The hover card says where the strategy comes from
and why an append is one; the Catalog adds Strategy and Unique key to
"context", one key column per line with a link to its row, and the Columns tab
tags the key's columns and sorts by them. `config.unique_key:` now tests a
list for membership, as `dbt ls` does, which `compare_with_dbt.py` checks on
two incremental models it adds to its copy of Jaffle Shop. A link to a column
row now scrolls to it, clears a filter hiding it, and ignores case, whichever
of a report, a column box or Snowflake's lineage set it. Left out unless asked:
a snapshot's own `strategy`, a microbatch model's `batch_size` and
`event_time`, and a default picked by a condition on the key, the form
Postgres's takes, which stays unnamed.

Schedules not read (0077), the known limit #36 left: a job whose clock this
tool could not read counted as one nothing starts, and its models as *Only
in unscheduled jobs*, with news on the Reports button. `link` in
`src/jobs.rs` now gives each job one of three answers (`Start`): scheduled,
not, or not read, a chain taking the best of what pulls it and a DAG the
worst of its assets. A dbt Cloud job whose schedule trigger is on is
scheduled, its cron read or not. A DAG file with no dbt task, which left
nothing, now leaves a `Feeder`, its schedule and its outlets, so a DAG on what
an ingestion DAG loads is scheduled with it; a task with no command in a dbt
DAG counts as well. A schedule or a trigger in Jinja, a schedule or an asset
written as a mapping, an empty list of assets, an asset no file read updates,
or a job no file defines is not read: an amber **Not read** pill, ranked after the scheduled jobs, the
Catalog's chip in amber, and the coverage's own list, *Only in jobs whose
schedule is not read*, which the news leaves out. The Rust tests cover each
case, the coverage's three lists, and the same answers from files read in the
reverse order; driven in headless Chrome on Jaffle Shop with invented jobs,
one of each. Left out unless asked: a job not read only through its chain,
behind a feeder whose clock is templated for instance, names no link that is
not read, its pill saying `on copied` under the general tooltip.

Find on the lineage: `Cmd/Ctrl + F` over the canvas opens a find bar of its
own, under the selector bar when that is open, and brings the box it lands on
to the middle of what the bars and the rows of folder names leave, zoomed in to
its drawn size when the graph was fitted smaller (0081). It searches what the
canvas was sent, a test folded under a chip or a closed group's row included:
typing brings that chip or row to the middle and draws nothing again, and a
step opens it, two turns at most. A name typed whole comes before the names it
starts and those it is in, then reading order, left to right a column at a time
and down each, a model's tests after it, which `Enter`, `Shift + Enter`,
`Cmd/Ctrl + G` and `F3` step through. That order is the one the tests are drawn
in once opened, so a step that opens a group skips none of them. It is kept,
with the names, until the canvas is drawn again. `Escape` closes the bar from
the canvas too, and the focus goes back to what had it; in model mode, the find
bar comes under the note over a canvas not of the file in front. 0030 had left
the lineage to the browser's find, which read names clipped to their box and
never moved the view. The canvas CSS for the paint is under `#graph`, out of
`exportCanvasCss`, and `snapshot()` strips its classes.
`web/tests/graphfind.js` checks the pure half and the bar's routing, keys and
focus; driven in headless Chrome on Jaffle Shop with invented tests added, so
that some fold under a chip and a closed group: model mode, tests off and on,
Custom selection drawn by folder, bands cut two and three levels deep on a copy
moved into invented sub-folders, Escape, Cmd+F again, Cmd+G from the canvas,
and the Catalog's filter still answering there. Column mode only by the
harness, Jaffle Shop having no column lineage.

Pictures in the README: the editor with a hover card, the lineage of the
whole project by folder, the Catalog's columns, one column's lineage from
collin, the Jobs tab and collin's report, all on Jaffle Shop with invented
jobs. `scripts/take_shots.py` builds the copy on duckdb, commits it, names the
job sources and takes them through headless Chrome with walk_tour.py's
client, into `docs/shots/`, committed with the README so a tag shows its own
interface (0082). It is not in `check.sh`, since it writes into the
repository: retake them when a change alters what one shows.

A `generated` badge from collin's roots (0096, 1.32.0): collin 0.8.0 lists
the columns a literal sets, with the value, and the column lineage now badges
such a column `generated`, whatever filter or join key edge reaches it, with
the value on a hover card on its box, in its label on one line, in the
Catalog's Lineage cell, whose row opens it even when nothing reads it, and in
`--column`'s JSON. With roots in the cache (`roots_known`), `raw` is kept for
sources, seeds and a column a table no node owns feeds by value, and a
model's column with nothing feeding it that is no root has no badge; a cache
without roots badges as before and answers as main did, byte for byte. The
legend lists every badge drawn, with or without a role on an edge. Checked
against main on Jaffle Shop and on Shopify v1.10.0, and in headless Chrome on
a Jaffle copy that sets literals and reads tables no node owns. Since
1.34.1, `collin_run::WANTS` asks for collin 0.8.0, so "Update collin" brings
roots to an older one, and since 1.43.1 for 0.10.0, which reads a seed's
header (0109).

A column from a manifest alone (0099, 1.35.0): `--run-collin` runs collin over
the manifest before `--column` answers, and `--compiled DIR` hands it the
folder dbt compiled into, so the column lineage of a project can be asked from
its artifacts, with nothing set up in the page and no project around them.
collin writes beside the manifest under the names the page's Regenerate uses,
through the page's own runner function, with every path made whole since
collin runs in the project. collin 0.9.0 reads `--compiled`, and its own
defaults now follow the manifest too (dbt-collin's 0046); an older collin is
never handed the folder. On collin's Jaffle Shop manifest with its
`compiled_code` taken out, as a `parse` leaves it, copied alone into a folder,
its compiled SQL in another, asked from a third:
`--column stg_payments.amount --down all --run-collin --compiled ../sql/compiled`
ran collin 0.9.0 in 50 ms, read the SQL of 5 of 5 models and gave 31 edges,
all parsed, then three models and seven columns with their roles; the same
question without the flag answered from that cache. A model the manifest does
not have, or a `--compiled` folder that is not there, is refused before collin
runs, and a run that found no compiled file for some models says how many on
stderr. On a copy of a large project's artifacts, asked from another
folder: the cache it wrote equals the one collin 0.8.0 writes with the page's
own arguments, and so does the answer; with the `compiled_code` taken out,
`--compiled` read every model's file, and without it stderr said none was
found. Not driven in the page.

A node's location per `.env` file on the command line (0113, 1.47.0):
`--describe`, and the MCP tool through it, adds to a built node's card a
`configured` line (the database, schema and alias as the project writes them,
when one differs from the built one) and a line per `.env` file the Catalog
shows by default, each the configured value with that file's variables filled
in, before dbt's `generate_*_name` macros (`relation` stays dbt's own, for this
parse's target), with `configured` and `by_env` in the JSON. The values come
from `envs::resolve_place`, so `DBT_ENV_SECRET_*` is never substituted; the
card also hides a variable named like a credential and a value carrying a
password, and a written default that reads one. A database or schema nothing
writes is `<target>`, a value taken from one branch of an `if` ends in `~`.
Checked by `cargo test` on an invented graph with temporary `.env` files (the
rows, the JSON, a secret, a credential-shaped value, a template, a placeholder,
an ephemeral model) and by running `--describe` and `--mcp` on a small
invented manifest. The macros `generate_*_name` are not evaluated, so a line
is not where that environment builds the node.

An MCP server (0110, 1.44.0): `dbt-edith --mcp` answers an MCP client over
stdio through one tool, `edith`, taking the command line's flags as a line:
allowed by clap argument id, no shell, the project, manifest and catalog fixed
when it starts. It keeps the manifest and the base graph between calls and
reloads them when their files change, answers the command line's text in two
blocks, and lets `--run-collin` run per call, stopped by a cancellation or the
end of input. Opt-in: nothing configures a client. Only the legacy
handshake is served. Checked by `cargo test` and by `scripts/walk_mcp.py`,
which drives it over stdio on Jaffle Shop and compares every answer with the
command line's, bytes and exit codes, and by `--measure` on Jaffle Shop and
Shopify v1.10.0: a warm call 4.9 ms against 9.8 for the command line on Jaffle
Shop, 34.9 against 53.0 on Shopify, of which the freshness check with git is
almost all.

A node and the project on the command line (0102, 1.37.0): `--describe`
prints a node's card, what the Catalog shows of it, with `--sql` for its own
file; `--overview` prints what the project holds, by kind, materialization and
folder, with the caches beside the manifest; `--output compact` prints a
selection or a column as tab-separated lines under a header, without what the
name and the path already say. Both new modes read the graph the Catalog is
painted from, the catalog merged in, and `--describe` names its nodes the way
`--column` does. `docs/features/command-line.md` is the page, with the recipes
an agent follows. `--compiled-sql` (0132, 1.65.0) adds the SQL dbt compiled
for the node, read from its file under the target folder through the lookup the
compiled tab uses and confined to it, never from the manifest's `compiled_code`;
it goes with `--describe` over MCP too, and a node dbt has not compiled is said
on stderr with `dbt compile -s name`, the card and the exit code unchanged.
Checked with `cargo test` on an invented graph (every card, the JSON
keys, the rules of a name, `--sql` against a temporary project, the overview at
two depths) and by running them on Jaffle Shop parsed, compiled, seeded and
documented with dbt 1.11, with collin's cache beside the manifest, the flags'
conflicts and an unknown name; `--compiled-sql` also by `scripts/walk_mcp.py` on
Jaffle Shop compiled but for one model. On Jaffle Shop a compact answer is about a
quarter of the JSON's bytes.

A selection counted instead of listed (0112, 1.46.0): `--count-by KEY` on
`--select` or `--selector`, with `kind`, `materialized`, `source`, `folder`,
`tag` or `package`, prints one row per value under a `count` header and the
nodes selected as `(all)`, or one JSON object; `--depth` cuts the folders, as
the overview's does. The MCP line allows it, so a count no longer needs a pipe
that line refuses (0110). A source is counted by the name the manifest gives
it, now kept on the node. Checked with `cargo test` on an invented graph (each
key, the depth cut and its order, a package's source, a tag written twice, the
JSON shape, an empty selection, the flags' refusals) and by `walk_mcp.py`
comparing the line with the command line on Jaffle Shop.

An agent skill, shipped and printed (0114, 1.48.0): `docs/agent-skill/SKILL.md`
holds the command-line page's recipes, its three habits and the freshness rule
under a description that names the questions it answers, and `dbt-edith --skill`
prints it for the installed version, with nothing beside it and nothing read,
written or started. It is refused over MCP. The binary never writes a client's
folder: the caller redirects the output. Checked with `cargo test`: the output is
the file with the version filled, the frontmatter is valid and its description is
at most 1000 characters, every long flag written anywhere in the file and every
`--count-by` key is one this binary has (a test misspells several to prove it),
and every command the file cites parses, passes the mode checks and is accepted
by the MCP line. Whether an agent loads it was measured with agents before it
was written, not by a test.

Job sources in the project, and jobs on the command line (0115, 1.49.0): the
sources form offers to keep the sources in `.dbt-edith/jobs.yml`, shared and
committed, or for the person only, in the settings, as before. The file is
written by a save of the form and by nothing else, refuses a version it was not
shown, wins over the settings while it exists, and gives no source and says why
when it does not read, at its line, never the settings' in its place. The
command line and MCP read it and never the settings: `--describe` gains a
`jobs` line (and `jobs` in its JSON), `--overview` a `jobs` line and key, and
`--job NAME` prints a DAG or a dbt Cloud job step by step with the nodes each
runs. MCP keeps the read between calls while the graph, the file and the job
files are as they were. Checked with `cargo test` (the file read, refused at a
line, winning, a link out of the project, the emitter round-tripped, the PUT
writing or not, a changed file refused, the cards, `--job`, the MCP slot) and by
`walk_mcp.py`, which writes the file over the job fixtures in its copy of Jaffle
Shop and compares the tool with the command line on `--describe orders` and
`--job`. The form's states were driven in headless Chrome on a Jaffle Shop
copy: no file, saved into the project, reopened on the file, and a file that
does not read. Not driven: the reason shown under the choice for a path the
file may not hold, and Save held while a save is on its way.

The tool's own checks (0116, 1.51.0): a Checks section in Reports, apart from
Warnings, listing the root project's own enabled nodes that four rules pick
out: sources with no freshness threshold as dbt-core counts one (read from the
source's `freshness`, else its config, and given no verdict where the manifest
has neither), models no test counts for (a generic test declared on one, found
by dbt's `attached_node` or, for a source's test, its `file_key_name`; a
singular test selecting from one; a unit test of one), models with no
description, and sources no node reads. Each rule is a fold with its count,
its rows grouped by folder, the first hundred drawn and a button for the
rest, and the server sends at most a thousand rows of each with its total. The
section is quiet: listed in the rail, never lighting the button, carrying a
chip or opening the panel, so 0075 holds. Checked with `cargo test` on an
invented graph (each rule, a relationships target, a source's test with a
package prefix or a name of the user's, a versioned model, a disabled node, a
package's, a key absent, the cap and the total, the route and the index) and by
`scripts/compare_with_dbt.py`, which works the four lists out again in Python
from the manifest dbt-core 1.12.6 wrote, on Jaffle Shop, Shopify and a copy of
Jaffle Shop given sources, models and tests of every shape. Not checked: dbt
Fusion's manifest for `freshness` and for `file_key_name`, and the Reports
panel with the lists of a project of many thousands of nodes.

Builds by job (0119, 1.54.0): a second Reports section from the same read as
Builds by environment, one row per DAG or dbt Cloud job with a column per
environment, judging each job's own models, errored, late, not built, against
the job's cron in the time zone its source names, and counting those built
since the last start. An Airflow source gained a `timezone` (UTC when
absent; refused on dbt Cloud, whose crons are UTC), in the form,
`.dbt-edith/jobs.yml` and the settings; the server holds its shape and the
page checks the name with `Intl`. `GET /api/jobs/models` sends
what each job runs. Checked with `cargo test` (`models_by_job` on an invented
graph: own models only, ephemerals and packages out, step numbers, seeds
counted apart, the same bytes twice; the zone checked, written and read back;
the route behind the key) and `web/tests/jobbuilds.js` (the cron, the last
fire across the clocks changing in London, the verdicts). Its limits: a
schedule on one task that only a DAG factory knows is not seen; a zone set per
DAG (dag-factory's `timezone` or `default_args.timezone`, or a zoned
`start_date` in Python) is not read, one zone holds per source; a job started
by an asset or another job, a manual one, and a cron this cannot read are
judged on the global threshold, not a clock; the DAG of an invocation is
inferred from the job's selections, not read, since `dbt_artifacts` does not
record it; and a long list of assets is cut short in the schedule pill, whole
only in its tooltip. Seeds, snapshots and tests wait on item 26. Since 1.60.0 a
row opens one environment at a time: a click on a cell lists that environment's
errored, late and not built models, another cell of the row switches, the same
cell closes, and a click elsewhere on the row opens the environment picked last,
else the job's worst. Since 1.59.1 an environment Snowflake refused shows the
whole error, its cause included, where it used to stop at `SQL compilation
error:`.

Job order (0124 to 0126, 1.57.0 to 1.59.0): the order the job files declare,
said three ways. `--job`, and MCP's `--job` line, carry it (0124): under a DAG
each sensor and what it waits for (a task, the tasks of an `external_task_ids`,
a task group whose tasks are not read, or the whole run), and under each step
what it follows and precedes, the sensors of any DAG awaiting it and the assets
it updates. `--job-order` takes every dbt dependency between steps of jobs that
may start on their own, the nearest ancestor another step builds, walking
through views, ephemerals and dynamic tables, and says whether a `depends_on`
chain, a sensor, an asset or a dbt Cloud trigger orders it; the pairs nothing
orders, those only the clock orders and those ordered the other way are grouped
apart (0125). Reports gained a Job order section from the same answer, which
never lights the button (0126). Checked with `cargo test` (DAGs written in the
tests: sensors of every form, assets with several updaters, DAGs with no dbt
task, crons across zones, cycles written in any order, the same answer twice;
the route behind the key, its body the command's) and `web/tests/joborder.js`.
Its limits are in the list left by these pull requests, below.

Keys (0127, 1.61.0): the Columns tab tags `KEY` on a column a `unique` test, a
`dbt_utils.unique_combination_of_columns` test or an incremental `unique_key`
names, one tag per column list whatever the order or case, `1/2` on each column
of a key of several, letters once a node has two keys, and the whole key dashed
when one of its columns has no `not_null` test. A test written under the model,
which dbt gives no `column_name`, counts by its kwarg. Checks gained three
rules: keys with a column not tested `not_null`, an incremental `unique_key` no
uniqueness test checks, and models with no key test; a `unique_key` the strategy
ignores (`append`, `insert_overwrite`, `replace_where`, and `microbatch` except
on Postgres) is left out of the first two unless a test also declares it.
Checked with `cargo test` (`keys::`, `checks::`), `web/tests/strategy.js` and
`scripts/compare_with_dbt.py` against dbt's own manifest. Its limits are in the
list left by it, below.

An install others can write to is not run (0128, 1.61.1): on macOS and Linux a
version under `<config>/collin/` has to be owned by the account, with its
binary and each folder up to that one writable by nobody else, and a link below
it is refused. The menu names the path, the owner or who may write it, and the
`chmod` or `chown`. Checked with `cargo test` (`collin_run::`, the version route
in `api::`) and `web/tests/collineage.js`. The Windows part is Deferred item 31.

## Deferred, in the order they were chosen

Item numbers move as items are struck. Decisions 0022, 0065, 0100, 0101 and
0103 name items by the numbers they had when written, and decisions are never
edited, so find an item by its title.

1. **Run status on the canvas.** The last run is read since 1.11.0, in
   Reports (0065), and a dot per box shows the last build of one environment
   since 1.52.0 (0117); what is left is the last run's status as the box
   stroke in the graph, which waits on seeing whether the section is used, and has to say when the
   run is for another target or older than the manifest, as the section does.
   A history across commands is not planned: dbt rewrites the file each time,
   and this tool keeps nothing of the project's. Per-node freshness would
   read the same mtimes 0025 already walks.
2. **Symbols in SQL.** CTE names and `{% macro %}` blocks in the breadcrumb,
   which needs SQL strings and comments masked first, for the reason 0022 gives.
3. **Which jobs test a model.** The Jobs tab resolves test steps with tests in
   the universe, as dbt does, and lists them on a test node; what is left is
   showing, on a model, the steps that run its tests, through
   `graph.host_of`. Left out of 1.9.0 because a model with 48 tests would list
   one step 48 times unless they are grouped by step first.
4. **A dbt section in Reports.** What the project holds, counted:
   - models by materialization;
   - tests, generic and singular, and how many are `severity: warn`;
   - sources, seeds, snapshots and exposures;
   - macros, the project's apart from each package's;
   - disabled nodes.

   Most of it is in the graph already. `Meta.counts` has the counts by kind
   that the top bar shows, and `graph.macros` has the macros. So the section
   reads no file and costs the index nothing (0064). The section opens the
   lists behind each count, as the coverage does.
5. **dbt's own warnings, from `logs/dbt.log`.** No artifact carries them:
   `manifest.json` has no field for them, `run_results.json` only says how each
   node's run or test ended, and a dbt Fusion 2.0 preview `run_results.json` of
   a compile lists no result at all.
   - **Where they are.** Both dbt-core and Fusion write the warnings to
     `logs/dbt.log`, one event per line, with its name and code. For example,
     Fusion writes `[warn ]: [UnusedResourceConfigPath (dbt1097)]: …` and puts
     the details on the following lines. dbt-core logs the same event under the
     code `I051`, and writes JSON lines with `--log-format-file json`.
   - **The plan.** Show the last command's warnings, grouped by event, saying
     how old the log is next to the manifest.
   - **What reaches the page.** Only warn and error events leave, never a
     debug line, which can carry SQL and connection details.
   - **The traps.**
     - The log holds the last commands only, appended to and rotated at
       10 MB (`dbt.log.1`, `dbt.log.2`).
     - Its folder can be moved with `--log-path` or `DBT_LOG_PATH`.
     - A manifest copied from another machine comes without its log.
6. **More warnings worked out here, without a run.** They hold for dbt-core
   and Fusion alike, and when no log is there. The Warnings section of Reports
   lists the first, the config paths of `dbt_project.yml` that apply to no
   resource (0103), and the Checks section the tool's own rules (0116); each
   of these would join the one that fits it, checked against dbt the same way
   in `scripts/compare_with_dbt.py`.
   - **Others worth ranking:**
     - ~~sources no model reads;~~ Settled in 1.51.0 (0116), in Checks.
     - ~~sources without a freshness config, which `dbt source freshness`
       skips;~~ Settled in 1.51.0 (0116), in Checks.
     - ~~models with no test or no description;~~ Settled in 1.51.0 (0116), in
       Checks.
     - vars `dbt_project.yml` defines that no `var()` call reads, from 0018's
       scanner;
     - the `tests:` key, which dbt 1.8 renamed `data_tests:`;
     - a package in `packages.yml` that `dbt_packages/` does not hold.
   - ~~**A "Checks" section in Reports, apart from Warnings:** sources without
     freshness, project models with no test, no description, and sources no
     model reads. dbt warns of none of these. It needs `freshness` read from
     the manifest, and its own decision.~~ Settled in 1.51.0 (0116). Left
     over: dbt Fusion's manifest shape for `freshness` was not looked at, and
     the checks hold only the root project's own nodes.
7. **A Run button on a node's file.** It sits at the end of the breadcrumb
    bar, after the config button. One click opens a menu of the commands the
    node takes. A command opens its selections and types nothing itself:
    only a selection types its command, so looking never touches the
    terminal.
    - **Named Run, after the command it proposes.** The menu opens on
      `run`, with its selections unfolded and the node alone first.
      `dbt run --select name` is therefore the command proposed, and the
      others are one pick away. For a seed, a snapshot or a singular test,
      the first command is the node's own: `seed`, `snapshot` or `test`.
      A label saying that the button types, such as "Type a dbt command",
      was weighed and dropped. The name says what is proposed, and the
      strip from `sendToTerminal` says that nothing has run.
    - **The commands:**
      - a model takes `run`, `build` and `test`;
      - a seed takes `seed`, `build` and `test`;
      - a snapshot takes `snapshot`, `build` and `test`;
      - a singular test takes `test` on itself alone. `+name` would also
        bring in the tests of every model it reads, through dbt's eager
        indirect selection.

      `dbt run` selects no seed or snapshot, so it is never offered for one.
    - **The selections:** the node alone, which is the default, then
      `name+1`, `name+`, `1+name`, `+name` and `+name+`. An entry that would
      run the same nodes as one above it is left out. So `seed` keeps only
      the seed alone, and `snapshot` keeps a wider entry only where it
      reaches another snapshot, which one snapshot reading another, directly
      or through models, does.
    - **Which nodes a file has.** A file can hold several: a snapshot file
      with two `{% snapshot %}` blocks, or a `.yml` defining snapshots,
      which dbt recommends since 1.9. The menu then starts with one entry
      per node. `node_of_file` picks a single node by name, so the menu
      cannot rely on it. `CONFIGURABLE` leaves `.yml` out today, so a
      `.yml` gets the button for the snapshots it defines, not for its
      tests or sources.
    - **The counts:** each entry says how many nodes the command itself
      runs, which dbt decides by resource type:
      - under `run`, the models;
      - under `seed` and `snapshot`, the seeds or the snapshots;
      - under `test`, the data tests and the unit tests;
      - under `build`, all of these together.

      An ephemeral model never runs, and neither does an analysis, a source
      or an exposure. An entry that runs nothing says so, as an ephemeral
      model alone does.
    - **The engine work they need.**
      - `/api/select` counts by kind only, through `tally`. It cannot leave
        out an ephemeral model or an analysis, which `dbt ls` lists. So
        `src/select.rs` takes a filter per command, as dbt's tasks filter by
        resource type, and the box, the menu and `--select` (0057) then
        give one answer.
      - Unit tests are not nodes (`graph.unit_tests`), so no selection
        reaches them today. `dbt test` and `dbt build` run those of every
        selected model, which the engine has to count.
      - One request answers the whole menu, counts only. Six `/api/select`
        calls would each send a lineage and up to 5000 names for one number.
        A new field or route is read against 0012 first.
    - **How the counts are checked.** On a fixture holding what Jaffle Shop
      lacks, invented (0014):
      - a snapshot that reads another;
      - an ephemeral model;
      - an analysis;
      - a unit test.

      Each command and selection is compared with what dbt itself ran,
      which `run_results.json` lists. `dbt ls` cannot check them on its own:
      it lists ephemeral models, and `--resource-type test` leaves out unit
      tests, which dbt types `unit_test`.
    - **When the counts may be wrong.** dbt parses the project again when it
      runs, so a `ref()` added since the last parse moves them. When the
      freshness badge says the files are newer than the manifest, the menu
      says so, as the Last run section does.
    - **The target.** The menu names the target a new terminal would use,
      which `profiles.rs` already resolves: `DBT_TARGET` as dbt-edith
      inherited it, which the terminal gets too, then the profile's
      `target:`. It is the same name the clone route sends (0058). The menu
      adds that a shell may have changed it since.
    - **What a click does: it types the command, and Enter is the user's
      (0071).** Picking an entry types the command into the terminal without
      running it, so "Never run dbt" and 0002 hold as written. The freshness
      badge stays the only button that presses Enter, because `dbt parse`
      reaches no warehouse, while `build --select +name+` writes to one.
    - **`sendToTerminal` changes for every caller.** Run again, Recompile,
      dbt ls, the selector command, the update offer and collin's install
      line have the same two problems as this menu, so the fix goes there,
      not into the menu alone.
      - **The line already typed.** A command lands after whatever is on the
        line. It is cleared first only where a sequence is known to clear the
        whole line in that shell and its edit mode, each checked in a real
        shell first. In bash, zsh and fish in their default emacs mode, that
        is Ctrl-E then Ctrl-U, since bash's Ctrl-U alone keeps what follows
        the cursor. In cmd, and in PowerShell's Windows mode, it is Escape.
        In PowerShell's Emacs mode, the default off Windows, Escape starts a
        key chord, so it is not that. A shell set to vi mode reads these keys
        otherwise, and nothing here can see that setting. Where no sequence
        is known, nothing is cleared, and the strip below says the command
        was typed after what was there.
      - **Saying that nothing has run.** The toast goes after 3.2 s, often
        while the eyes are on the terminal. A strip above the terminal says it
        instead. It shortens the terminal rather than covering its last rows,
        where the typed command sits. The strip:
        - names the command;
        - says that nothing has run and that Enter runs it;
        - names the target a new terminal would use, with the caveat that
          the shell may have changed it.

        It shows once the command has reached the terminal, never when the
        send gave up. The next key or paste in the terminal closes it,
        read from `onKey` and the paste event. `onData` would close it at
        once, since it also carries the terminal's own replies, such as a
        focus report or a cursor position. The next send replaces it, and a
        reconnection removes it.
      - **Before the click,** the button's tooltip and the menu's foot say
        that an entry types its command.
    - **What holds the terminal takes the keys.** A pager or an editor
      running there reads them as its own commands, Enter or not. That is
      true of every caller of `sendToTerminal` today, and nothing is built
      here to wait for the prompt.
    - **Also left.**
      - A name shared by two packages selects both. The command then needs
        a narrower selector, checked against dbt. A version's own name,
        `orders.v2`, already selects it alone (0104).
      - A disabled node is selected by nothing, so its button is greyed,
        with the reason as its tooltip in place of what a click types. It is
        greyed through `aria-disabled` rather than `disabled`, which some
        browsers show no tooltip on.
      - The Run tab's "Run again" types `dbt run` for every node but a test.
        For a seed or a snapshot that runs nothing, so the tab takes this
        menu's commands too.
8. **The dock's bar on a narrow window.** The dock's tabs and the lineage's
    tools share one row, 30px high. The tabs give way first: each carries a
    glyph beside its name, and once the row cannot hold every label on one
    line they keep the glyph alone, named by its tooltip (`foldDockTabs`).
    If that is not enough, Fit, Export and Copy image keep theirs alone too,
    the same way. Past that, nothing in the tools is set to give way: the
    browser shrinks each one to its narrowest, so every label that can break
    does ("Custom selection", "up", "down") and the row spills over the
    canvas.
    - **When it happens:** the room left depends on several things at once:
      - the window's width, and the sidebar's;
      - the Query history tab, shown with Snowflake's features on;
      - the eye's count, which counts the tests the canvas is sent. Past
        five on a model it draws fewer of them (0035), so the label can be
        longer than the boxes on screen suggest;
      - the column chip in Columns mode, which carries a column's name and
        has no width limit;
      - the indirect button in Columns mode, there when the cache holds
        indirect edges (0097): with it, Fit, Export and Copy image fold to
        their glyphs at 1440px.

      On Jaffle Shop, with Snowflake's features and the tests on and the
      sidebar at its default width, the tools broke from a 1470px window
      before the tabs could fold, and broke from 1240px once they did. The
      glyphs made the labelled tabs wider, so the tabs folded from 1590px: in
      a window the size of a laptop's screen, they show their glyphs alone.
      The glyphs before Fit, Export and Copy image cost the row 60px more:
      the tabs fold from 1650px. Those three then fold to their glyphs from
      1300px, and the tools break from 1180px.
    - **Why the tabs fold that way.** It is measured, rather than set at a
      width through a container query, because the room moves with
      everything listed above, and any width would be wrong for some of it.
      A `ResizeObserver` watches the row's width, and a `MutationObserver`
      watches what the tools hold. The tools change while another tab is
      open, when they are hidden, and a hidden element reports no resize.
      An observer on the tools' size also looped, since folding the tabs
      resizes the tools. The tools are measured shown whichever tab is open,
      the open tab folds with the others, and a tab change never measures,
      so a click on a tab never moves the tabs under the pointer. A first
      attempt kept the open tab's label: a click then shifted every tab
      after it. Only the tools are asked whether they broke: a tab cannot,
      and under a large minimum font a tab taller than the row kept the
      labels folded at any width. A measure costs a few hundredths of a
      millisecond on Jaffle Shop. Cutting Query history alone with an
      ellipsis, the other answer for the tabs, frees nothing without
      Snowflake's features.
    - **What any answer for the tools needs:** no label ever breaks inside
      its own item. The two toggles, `.eyebtn`, the three actions after
      them, `.icobtn`, and the tabs are `nowrap` already. These are not:
      - the segmented modes;
      - the `up` and `down` labels, whose input drops under the word;
      - the column chip.
    - **Still open: what gives way after the tabs and the three actions.**
      - The tools that matter least, Export and Copy image first, fold behind
        a `…` menu at the end of the row once they stop fitting. This is the
        way VS Code's panels go. `foldDockTabs` already measures the row, and
        would go on to this once the actions are folded.
      - The rest of the tools give way to their icons alone, as the tabs and
        the three actions do. The two toggles have theirs; the segmented
        modes and the `up` and `down` inputs have no obvious one.
      - The row scrolls sideways, with no scrollbar showing. This is the
        simplest, but a tool past the edge is a tool nobody finds.
      - The tools take a second row, and the bar grows to 60px. Nothing is
        hidden, but the pane under it loses that height, all of it with the
        dock at its 60px minimum, and the dock jumps when a mode change adds
        a chip.

      The first, with the column chip capped at a width and its full name in
      its tooltip, is the likeliest.
    - **How it is checked:** headless Chrome on Jaffle Shop, with Snowflake's
      features and the tests on, at several window widths and in each mode.
      Two measures, since each misses what the other catches:
      - no item of the row is taller than the row, which catches a label
        breaking;
      - the row's `scrollWidth` is no more than its `clientWidth`, and every
        item's right edge is inside the row, which catches the row running
        past the dock's edge once nothing breaks. `body` hides its overflow,
        so a tool pushed past the edge would vanish without a scrollbar.

      `foldDockTabs` decides on the first measure, for the tools only, and
      the `scrollWidth` half of the second, once for the tabs and once again,
      with the tabs folded, for the three actions. #28 added up its items' widths
      against the row's own, which predicts both but proves neither. Columns
      mode needs a column lineage, which Jaffle Shop comes without
      (`/api/collineage` answers 404): run collin on it first, or put a long
      name in the chip from the page, since its text is all its width depends
      on.
9. **What is new since the version you had.** An update usually skips
    several releases, and the update pill's What changed (0068) opens one
    release's notes, on GitHub, which the VM cannot always reach.
    - **Where it starts from:** `tour.json` keeps the newest version that
      served a page on the machine, from 1.16.0 on (0074). A page of a later
      version shows what came since.
    - **The list:** `CHANGELOG.md` built into the binary with `include_str!`,
      its sections between the two versions, grouped by version and folded,
      since a release can carry a dozen lines. Read offline, unlike the pill.
    - **The tour of it:** each step of the tour carries the version that
      brought it, and the tour of what is new plays the steps after the
      version kept. A feature with no step is in the list alone, so a pull
      request adding something visible adds its step, as it adds its
      CHANGELOG line.
    - **Where it is offered:** the card under `?` that offers the tour, once
      per update, and What's new in the shortcut list at any time.
    - **The traps:**
      - a machine that ran nothing since 1.16.0 has no version kept, and gets
        the first tour instead, which is new to it anyway;
      - several dbt-edith on one machine: the version kept only rises, so an
        older one started after a newer one offers nothing;
      - the first tour's steps predate the field, and stay out of every tour
        of what is new.

10. **The top bar on a narrow window.** Under about 1000px its chips break
    onto two or three lines ("5 / models", "Search / ⌘K", "Reload /
    manifest"), and under about 850px the bar runs past the window. The
    GitHub mark (1.25.0) brings both about 26px sooner. The status bar does
    not break, since `#statusbar > *` is `nowrap`; nothing in `#topbar` is.
    - **The likely answer:** `nowrap` on the bar's items, and the project's
      name shrinking with an ellipsis, its full name in the tooltip. The
      settings panel (0100) took three buttons out of the bar, and Reload
      manifest became the arrows at the end of the manifest's pill: on Jaffle
      Shop it broke under about 780px rather than 1050px, where the chips,
      Search and the manifest's pill take a second line. The Custom selection
      button (0101) brings that about 50px sooner, to about 830px, and to
      about 930px while its panel is down, its name unfolded beside its
      glyph, as Reports' and the gear's are while theirs are.
    - **How to check it:** item 8's two measures, at several widths, with
      Snowflake's features on and off.
11. **The README's options, held to `--help`.** The README writes its options
    by hand, and they have drifted once already: `--help` says `--shell`
    starts `$SHELL`, while `src/pty.rs` and the README say `$SHELL -l`. Fix
    the help text, then add a test that compares the README's block with what
    clap prints, so neither can move alone.
12. **An issue template.** The GitHub menu opens an issue whose body is a
    version and a system, in English, with no template. A report then often
    comes without the steps that lead to the bug. Templates under
    `.github/ISSUE_TEMPLATE/` would ask for the steps, what was expected and
    what showed.
13. **The levels box, over what is under it.** By folder's levels box (0080)
    opens while the pointer rests on the toggle, so a touch screen cannot
    reach it at all.
    - **What it covers while open:** the count beside the Custom selection
      field, and the left of the lineage's find bar (0081).
    - **Possible answers:** open it on a click too, or place it where it
      covers no tool.
14. **`cargo vet` on the lockfile.** `cargo audit` says nobody has filed a
    problem against these versions; `cargo vet` would say someone read them,
    importing the attestations Mozilla, Google and others publish. Measured
    on 2026-10-15 with six public registries imported, 24 of the 129 crates
    were covered at these versions and 105 were not, mostly because an
    attestation names a version and this lockfile's are newer. The trap is
    the exemption list those 105 would need on day one: the audit in name
    only, and every Dependabot bump failing until someone reads the diff.
    Worth adopting once the registries reach these versions, with trusted
    publishers for the crates one author maintains, which covers serde, syn,
    anyhow and tokio's family (0085).
15. **A target for Regenerate.** The page and `--run-collin` (0099) run collin
    without `--target`, so their caches say "target not recorded", and
    `--column` (0088) cannot say which target its answer describes. The
    manifest does not record the target it was compiled for, so the page
    cannot fill it in: Regenerate would offer the profile's targets and pass
    the one picked, which the user then has to have compiled and generated
    the catalog under.
16. ~~**A typed command and an empty entry of `PATH`.**~~ Settled in 1.62.0
    (0130): the startup banner and Edith's menu say when the terminal's shell
    reads the current folder, by a relative entry of the `PATH` (named as
    written), an empty one that survives, or cmd.exe, and the freshness badge
    then types `dbt parse` without pressing Enter. An empty entry is dropped
    before the first prompt by a macOS login shell (`path_helper`) and by Git
    Bash (msys2), so it warns on Linux and on a macOS shell that is not a login
    one. The check reads the `PATH` dbt-edith hands the shell, which the
    shell's startup files may still change.

Items 17 to 19 are the rest of the plan chosen with the panels' mechanism
(0091), Custom selection (0092) and the settings (0100). Each is a pull request
of its own, and each panel updates the tour, its page under
[features/](features/) and the README's pictures (0082):

17. **A model's lineage seen whole.** The dock's lineage of the model in
    front, opened in a panel at the window's height, as Custom selection is.
18. **git seen whole.** The side-by-side diff and the commit in a panel, the
    git panel of the sidebar staying the way in.
19. **Boxes without their text far out, then the cap again.** Past 3 000
    boxes zoom and pan slow down (0087). Drawing a box without its text when
    it is too small to read would let the selection draw more before it is
    capped.
20. **From the command line, for an agent.** Ideas for `--describe`,
    `--overview`, `--column` and `--sql` (0102), unranked:
    - one compact line per column of every selected node;
    - a card of the columns alone;
    - a shorter `--overview` JSON, with `--depth 1` first;
    - `--jobs KIND:PATH`, to name job sources on a line without the project's
      file (0115), which MCP could not take without a path in a line (0110).
21. **Where the sources file goes after a save.** The form already offers
    `.dbt-edith/jobs.yml` (shared, committed) beside "For me only" (0115),
    but the command line and MCP read only the file, never the settings, so
    someone who wants an agent to see the sources without sharing them must
    write the file and then keep it out of git. After a save that leaves the
    file untracked (`??` in `git status --porcelain -- .dbt-edith/jobs.yml`;
    nothing is asked when it is tracked or ignored) the form would ask:
    - **keep it for me**, which adds `/<prefix>.dbt-edith/` to the
      repository's `info/exclude`, the untracked ignore list, so the file never
      shows in `git status` and never reaches a commit; the entry is anchored
      to the project's folder by the `--show-prefix` that `src/git.rs` already
      reads, since a bare `.dbt-edith/` would match at any depth;
    - **share it with the team**, which does nothing more: the file is there to
      commit.

    Either way it says that a relative path such as `../orchestration/dags`
    only works where that folder is cloned at the same place, so a teammate
    without it reads no source. The traps: the exclude file is found by
    `git rev-parse --git-path info/exclude`, not by `.git/info`, because in a
    worktree `.git` is a file and the list is shared by the main checkout; it is
    the first file dbt-edith writes there itself (0007 goes through the git
    CLI), and the write cannot go through `files::resolve` (0094), which
    refuses paths outside the project, where the git folder lies in a worktree
    or when the dbt project sits in a subfolder; and nothing may ever add to a
    tracked ignore file, which is the repository's.
22. ~~**Job coverage from the command line.**~~ Settled in 1.63.0 (0131).
23. **A cap on the size of an answer.** Over MCP a client cuts a long answer (a
    compact list of a whole folder of a large project) and the file it saves
    cannot be opened. The server could cut itself at a stated size and say how
    to narrow, with `--count-by` or a narrower selection. Trap: the answers of
    the command line and of MCP are byte-identical (0110), which
    `walk_mcp.py` compares.
24. **A skill for a client that defers tools.** Print the MCP variant of the
    skill (`--skill mcp`?), which tells the agent to load the `edith` tool and
    call it. 0114 says the shipped skill is printed, never installed, and
    `--skill` is refused over MCP. The variant was tried only with Claude Code
    and has to be tried with other clients, Copilot among them.
25. **When a table last changed, from Snowflake's metadata.** The last
    builds (0117) say when dbt ran a model, which is the package's word. What
    Snowflake says of the table itself, `LAST_ALTERED` in the schema's
    `INFORMATION_SCHEMA.TABLES`, is a different question (the table changed,
    not dbt ran) and would answer it for a project without the package. It
    needs its own read per environment and its own privilege, and would show
    beside the package's answer, not instead of it, since the two disagree on
    a model changed by hand.
26. **Seeds, snapshots and tests in the builds.** The package keeps
    `seed_executions`, `snapshot_executions` and `test_executions` beside
    `model_executions` (0117), with the same `node_id` and the same invocation.
    The read joins the models' table alone, so a seed or a snapshot box carries
    no dot and a test's last result is not in the section. Each is one more
    query per environment, and a test's status words (`pass`, `fail`) are
    already in the verdict.
27. **The package's schema is the config before `generate_schema_name`.**
    The builds place the package's tables with the relation the Location
    table resolves (0113), which is the config as written. A project whose
    `generate_schema_name` rewrites the package's schema sends the query to a
    table that is not there, and the section shows Snowflake's error for the
    relation it asked. Fixing it means evaluating that macro, which this tool
    does not do (the `generate_*_name` note below).
28. **Runs of one target by several people.** An `.env` file that sets
    `DBT_TARGET` keeps that target's runs (0117), and every run under one
    target is one, whoever ran it: two developers building `dev` into their
    own schemas share one last attempt. Telling them apart needs the model's
    `schema` column to leave the warehouse (who built where) and the schema
    each environment really builds into, which `generate_schema_name` decides
    and this tool does not evaluate (0113).
29. ~~**The update pulls `main`, not the tag it announced.**~~ Settled in 1.61.2 (0129).
30. ~~**The `.exe`'s hash travels with the `.exe`.**~~ Settled in releases
    after 1.61.0: releases from 1.57.0 on are immutable, so `gh release
    verify-asset` checks a file against GitHub's record of the release, and
    the release job attests each `.exe`'s build provenance (`gh attestation
    verify`), both in docs/windows.md. The update's own check still reads the
    `.sha256` of the same release, `gh` not being on the machines it runs on
    (0085, 0123).
31. **A collin install in a folder others can write to, on Windows.** On
    macOS and Linux a version installed under `<config>/collin/<X.Y.Z>/bin/`
    is run only when its binary and each folder up to `<config>/collin/` belong
    to the account and are writable by nobody else (0128, 1.61.1). Windows is
    left as it was: ownership there is a SID and an ACL, which std does not
    read, so the same check needs a call into the Windows security API or
    `icacls` parsed, and a folder Administrators own is the user's own on many
    machines (0042, 0060, 0094).
32. **A version resource for the MSVC `.exe` (0123).** The release's file has
    none, since `rc.exe` is off `PATH` outside a developer prompt (0093). The
    runner has it; the work is a resource script `build.rs` hands to it, or a
    step in the workflow, and a check that the properties read as 0093 says.
33. **Signing the release's `.exe`.** The certificate options are in the
    release paragraph below: Azure Artifact Signing wants an organisation in
    Europe and its certificates last three days, SignPath's free plan an OSI
    licence, and an endpoint's certificate exclusion pins one certificate.
34. **`collin.exe` as a release asset.** collin is still built with cargo on
    Windows (0043); its own repository would carry the asset, and the update
    that installs it (0043, 0060) would download as 0123 does.
35. **Downloads off Windows.** A macOS or Linux build is installed with cargo
    from a clone; a release asset for them would need its own build, hash and
    command, and 0123 gives them none.

Left by the pull requests of 2026-10-14, each a known limit written in
its pull request, unranked:

- **Toasts (1.18.2):** a toast covers the bottom of a tall dialog, a menu by
  the status bar or the hover card for three seconds, and its text, an
  error's included, cannot be selected.
- **The tour (0078):** a page left open on a project that never gets a model
  never offers it; it stays in the shortcut list.
- **Schedules not read (0077):**
  - an asset updated from outside Airflow, through its API or another tool,
    reads as not read even when it comes every hour;
  - an asset condition written as a mapping is not read, even when a
    scheduled DAG updates every asset in it.
- **Find on the lineage (0081):**
  - the bar floats over the canvas's top right corner and can hide a box
    there;
  - a test's name carries its model's, so a model's tests are found with it;
  - what a step opened stays open after Escape.
- **The column stops (0083):**
  - they last until the page is reloaded;
  - a copied image, the canvas alone, does not say where the walk stopped;
  - the +N of a stopped column deepens the walk, which stops at the same
    role again;
  - the menu is checked through its source only, since `jsc` has no DOM.
- **Macros in Cmd/Ctrl + K (0084):**
  - a search cut short by a stale manifest does not say what it left out;
  - a materialization naming its adapter by anything but a quoted literal
    opens at the top of its file.
- **The update looks (0079):**
  - a release published during the Paris day is seen at the next midnight;
  - two dbt-edith on one machine that draw nearly the same spread can both
    ask at a turn;
  - behind a proxy that takes the connection and never answers, a look shows
    `Looking…` for about five minutes, until curl gives up.
- **`scripts/take_shots.py` (0082):** a collin that dbt-edith started keeps
  running once dbt-edith itself is stopped, since `src/collin_run.rs` stops
  it only at its deadline. A run aborted while collin works leaves collin
  writing into a deleted folder.

Left by the pull requests of 2026-10-15 and 2026-10-16, each a known limit
written in its pull request or left by its review, unranked:

- **The supply chain (0085):**
  - the cooldown holds what Dependabot proposes, not a `cargo update` run by
    hand nor a crate a bump resolves on the way;
  - `--deny yanked` fails Monday's audit on a yank with no fix out yet, and
    the answer that day is a bump or a pin;
  - the vendored files are compared with upstream only online, and
    diff-match-patch, which has no release, answers for itself by its pin;
  - a compromised npm release copied in as a new version, with its own
    tarball's hashes, passes: OSV answers for that, a week late at most;
  - a hidden file under `web/vendor/` is not audited but is embedded, and
    `scripts/build_windows.sh` builds from the working tree;
  - `--locked` holds the crates, not the compiler: there is no
    `rust-toolchain.toml`, and the scripts' own `cargo build` runs without
    `--locked`;
  - a Windows clone made before `.gitattributes` keeps its vendored files in
    CRLF until the one-time fix in [windows.md](windows.md).
- **The security pass (0090):**
  - a credential whose name says nothing, and whose value is neither a URL
    with a password nor a `password=` parameter, still shows in the hover
    card: `DBT_ENV_SECRET_*` is the way to mark one;
  - `CI_PASS`, `TESTS_PASS` and `EXASOL_DSN` are hidden by their name, while
    the profile key `dsn`, a host and a port to dbt-exasol, is not;
  - a flow mapping holding a secret key is one placeholder, so changing its
    other keys in the page means typing the mapping over;
  - the link tests run on Unix only, and the `create_dir` race in
    `git::tag_subjects` has no test;
  - a NEL inside a `--select` splits the term, where `--selector` refuses it;
  - the review did not cover `web/lineage.js` beyond a search for sinks,
    Windows' ConPTY and `cmd /C start` other than by reading, nor any real
    warehouse; `.git/config` reads through `/api/file` as the rest of the
    project does (SECURITY.md).
- **The column counts (0089):**
  - with Snowflake picked, a cell counts what the clicks have fetched, and
    grows with each click;
  - past 1000 boxes the canvas draws fewer than the cell counts, and the
    cell counts the whole chain whatever stop is ticked;
  - on a cycle of two columns each counts the other on both sides, where the
    canvas draws it on one;
  - the count runs again with each node payload the Columns table shows, and
    a walk is never cancelled when the user moves on;
  - its hover is a native tooltip, shown after the browser's delay.
- **The selection cap (0087):**
  - each pause in typing redraws up to 3 000 boxes, even when the answer did
    not change;
  - past 3 000 boxes zoom and pan slow down (Deferred, item 19);
  - Copy names lists the first 5 000 names, where Draw all goes to 20 000;
  - a tall canvas, fitted, is a thin column at the smallest zoom, as it was
    before;
  - the starts are worked out for every caller, the Jobs tab and `--select`
    included, at the cost of one insertion per criterion.
- **A column on the command line (0088):**
  - it answers from collin's or Fusion's newest cache, not from the one the
    page's menu picked, and reads every cache whole to choose;
  - a test on a column the YAML does not declare is not listed, as in the
    Catalog, and the cache's age is judged by time alone (0042);
  - stderr prints a `DBT_TARGET` value shaped like a name, not only the
    profile's target names;
  - the startup line and the Catalog's notes give one dbt's catalog command,
    Fusion's `dbt compile --write-catalog` in some and `dbt docs generate` in
    others, each wrong for the other dbt;
  - the printed steps pass `--out` and `--report` to write the names the
    page's own run writes, which collin before 0.9.0 does not, its `--help`
    naming `column_lineage.json` besides;
  - `--run-collin` (0099) runs the collin beside dbt-edith or on the `PATH`,
    not the version the page picked for the project (0060), and runs it every
    time it is given: nothing says a cache it wrote earlier read another
    catalog or another folder of SQL than the one in front of the caller;
  - a report `--run-collin --compiled` wrote is summed up in the page as read
    from `target/compiled/`, until the page runs collin again; a model's own
    note names the file where it is.
- **Edith's menu under the pointer (0086):**
  - the status bar's pill opens it on a click only, and a menu the pointer
    opened takes no arrow keys;
  - Escape on a menu a click held sends the focus to the name, not back to
    the editor or the terminal that had it;
  - `closePopup(anchor)` holds a menu the pointer opened rather than closing
    it, which a later caller passing an anchor would trip on;
  - its rest and grace, 150 and 300 ms, are a pair of its own beside the
    hover cards' 350 and 180 ms, in code of its own too.
- **The lineage's buttons (1.25.2):**
  - the Custom selection row is about 75px wider at its narrowest, and
    spills under a dock of about 500px;
  - the Jobs tab's `dbt ls` has no glyph, where the selection's has one;
  - [the lineage page](features/lineage.md) describes Fit under "Sharing the
    graph".
- **`scripts/take_shots.py` (0082):** without `COLLIN`, which wins, it takes
  the first collin on `PATH` and prints no version, so a stale one puts an
  older report in `collin-report.png`, which happened twice on 2026-10-15.
  `COLLIN` is not checked either. The script should refuse a collin older
  than `collin_run::WANTS`, or at least print the version it takes.

Left by the ten pull requests merged on 2026-10-18, each a known limit
written in its pull request, unranked:

- **The update (0095):**
  - the first update from a release before 1.28.1 still shows the old menu,
    and a stamp naming the old release, since the running one types the
    command; a `git fetch --tags --force` in the clone puts the stamp right;
  - an update left half done, the `.exe` renamed and cargo stopped, still
    shows the button, and the command run again stops on the rename.
- **A test in the Catalog (1.29.0):**
  - dbt fills `store_failures_as: table` once `store_failures` is true, so
    that line shows though nobody wrote it;
  - a `--store-failures` given on the command line is not in the manifest,
    and is not shown.
- **What a repository brings (0094):**
  - the link tests run on Unix only, and the `C:x` refusal is tested through
    `resolve` on Windows only;
  - a collin copied over one git tracks is still the tracked path, and is
    skipped;
  - your own collin in a repository git refuses to read is passed over until
    `safe.directory` allows it;
  - two menu requests while git is slow each start their own git, at most
    once a minute;
  - a relative entry of `PATH` is skipped, so a git reachable only through
    one is not used;
  - a `profiles.yml` linking to a monorepo's shared one needs
    `DBT_PROFILES_DIR` before the project opens here;
  - a save still breaks a hard link, and a link swapped between the check and
    the write is not caught.
- **Wide models (1.29.2):** the cache merge still crosses its edges once per
  role, and the Catalog's table of a model thousands of columns wide was not
  measured.
- **The cut canvas's note (1.31.0):**
  - the line is cut on a narrow bar, after "draws the re…" at 1400px, its
    whole text in the tooltip;
  - the count says how many starts are drawn, not how many stand alone.
- **The `generated` badge (0096):**
  - an exported page has no hover card, so the value is in the box's label
    only;
  - collin gives no value for a literal selected beside a table no node owns,
    and the card says it gave none;
  - a filter on a table no node owns, into a column whose lineage is lost, is
    checked by the Rust test only.
- **The `.exe`'s properties (0093):**
  - the MSVC route builds without the resource and warns on every build;
  - the file is still unsigned, with no icon and no application manifest, and
    the release's MSVC one has no resource at all (0123);
  - the fourth version number is always 0, and the build stamp is not in the
    properties;
  - the copyright's year is written in `build.rs`;
  - until a Windows build finds `windres`, every change of `PATH` rebuilds the
    crate.
- **Indirect edges (0097):**
  - ~~shown, stop at lists the three indirect roles one by one;~~ Settled in
    1.53.0 (0118): it lists none, and an indirect edge ends on its model's box;
  - `--column --indirect` lists an indirect edge in `from` with its role, and
    `--output name` prints that column with none;
  - a card opens on nearly every column, button on or off, since join keys
    reach every column of a model that joins;
  - collin draws value edges from the right side of an `EXCEPT`, which only
    takes rows away, and the button cannot hide them: collin's to fix.

Left by the pull requests of 2026-10-19 and 2026-10-20 (#79 to #87, 1.36.0 to
1.42.0), each a known limit, unranked:

- **Custom selection over the whole window (0101):**
  - once the panel is under about 600px wide the canvas gets less than 320px,
    the card's 280px minimum winning over the canvas's;
  - the wait for a double-click is a fixed 500 ms (`DOUBLE_CLICK_MS`), so a
    slower system double-click can still get the card between its two clicks;
  - a redraw while a card waits drops it, even when its box is still drawn;
  - a find (Cmd/Ctrl + F) or a fit during the wait does not count as a move,
    which only a pan or a wheel zoom does, so the card's catch-up can still
    shift the canvas;
  - `S.cardAsk` guards `catalogFor` only, not the `renderCatalog` of
    `focusNode`;
  - the card's width lasts the session only.
- **The command line (0102):**
  - a catalog older than the manifest is never flagged: only the manifest's
    freshness is checked;
  - every `--describe` walks `place_packages` over every package node, for the
    one node it prints;
  - `--output compact` on `--column` carries no column type.
- **The build stamp (1.37.2):** a worktree or a submodule is not stamped again
  by an incremental build after a commit or a tag, since `build.rs` watches
  `.git/HEAD` and `.git/refs` only when `.git` is a folder. `cargo install`
  always stamps again.
- **The Warnings (0103):**
  - if the one index read after a parse fails, the chip stays hidden and the
    marks grey until the next index read; asking again in `refreshFreshness`
    while the freshness answer's `manifest_at` is newer than the warnings'
    would close it;
  - a key inside a multi-line flow mapping is marked on the line
    `yamlOutline` gives it, which takes the place of the server's
    (`unusedLines`);
  - `dbt deps` adding a package is not seen by the freshness walk, which
    leaves `dbt_packages/` out;
  - `src/yaml.rs` now gives the items of a multi-line flow list a line of
    their own, which `src/jobs.rs` reports for a step, and 0063 does not say
    so.
- **Versioned models (0104):**
  - a package-qualified versioned name such as `shop.dimv` is not looked up
    by `--describe` or `--column`, and `shop.totals` answers a source of that
    name beside package shop's versioned `totals`, which it does not reach;
  - `--column totals.c1` resolves to `totals.v2` with no note on stderr;
  - the editor and `/api/resolve` pick a versioned name by the order of the
    manifest across packages (`graph.by_name`), where the command line
    refuses several.
- **Who calls a macro (0105):**
  - materializations tied between packages all count as run, where dbt runs
    one of them, the later in `manifest.macros` order;
  - a `dbt.x(` call and a method call `a.y(` in dbt's own macros
    (`string_literal`, `escape_single_quotes`, `replace`, `get`, `split`)
    make a project macro of that name read as called by dbt;
  - dbt Fusion records a macro reached through another as a direct call, so
    the count "through other macros" differs between the two engines.
- **The selection engine against dbt-core (0024, 0056):** dbt-core 1.12.6
  added a rule to `is_selected_node`: a wildcard with no dot, such as
  `stg_*`, also matches the leaf alone (`fqn[-2]` for a versioned model).
  1.12.5 and 1.11 lack it, and `src/select.rs` matches nothing there. CI's
  1.12 leg resolves to 1.12.6, so a wildcard expression added to
  `scripts/compare_with_dbt.py` needs an expectation that depends on the
  version.
- **Reports:** the first opening after Snowflake's features are switched on
  waits noticeably on `/api/reports`, on `main` before #79 as well. The cause
  was not looked for.
- **Plurals:** the hover card, the lineage's footer and the Catalog write "1
  columns", "1 tests", "1 edges", "1 nodes" and "1 models upstream".

Left by the pull requests of 2026-10-20 (#89 to #91, 1.42.1 to 1.43.1), each a
known limit, unranked:

- **The export's zoom (1.42.1):**
  - `web/tests/export.js` tests `zoomViewBox` directly, not the call site
    (`m.a, 3`) nor `lineage.js`'s `Math.min(3,` cap, so the two 3s can drift
    apart;
  - the CHANGELOG says "tall graph", but the cap also bites on a squarish graph
    in a short window;
  - a wide export still zooms closer than the canvas, on purpose, down to the
    120-unit floor;
  - only the wheel zooms an export: no key, and a pinch is not checked.
- **The Halloween look (0106 to 0108):**
  - `web/tests/halloween.js` is mostly source-text checks; behaviour is covered
    by `scripts/walk_export.py`, which needs a dbt venv;
  - the audio unlock on a key or a click is tested through the source only;
  - the box art stops at 400 boxes, a number measured in headless Chrome only;
  - the clipped corners of a tombstone or a coffin take no click;
  - in find, a match that is also selected or focused shows the match ring
    only;
  - in column mode the AGGREGATE badges nearly touch the box above, with the
    look off as well, and the hover card's anchor was not checked on a column
    box with a role tag;
  - a tab left open from before the upgrade sends three fields to `/api/look`,
    gets a 422 and loses that save without a word.
- **collin's project (0109):**
  - a relative `--manifest` or `--catalog`, given when dbt-edith is started
    outside the project folder, is resolved by collin from the project folder,
    so the page's Regenerate fails (`src/main.rs:244-245`,
    `src/api.rs:1269-1273`); `--run-collin` makes its paths absolute first;
  - a refinement could pass `--project` also when the manifest's
    `metadata.project_name` equals the project's name: a CI manifest of the
    same project kept outside the checkout loses its seed headers today;
  - the `collin_run` test module is unix-only and the CI has no Windows job, so
    the Windows spelling of `files::plain` is not run there.

Left by the MCP server (0110, 1.44.0), unranked:

- **The modern handshake (2026-07-28)** is not served: `server/discover` gets
  -32601, which Claude Code answers by falling back to `initialize`, and VS
  Code sends `initialize` anyway. Serving it means a version tracker, the
  per-request `_meta`, `ttlMs`/`cacheScope` and its own errors.
- ~~**The skill or the server.**~~ Settled in 1.48.0 (0114): the docs prefer the
  skill where there is a terminal, and the tool where a client has none. The
  tool is about 153 tokens, plus at most 384 for `help`; the skill's
  description about 190 and its body about 800 more once loaded (bytes over
  four). What is left: which clients defer MCP tools, and whether any can list
  this one up front; deferral was seen on Claude Code only.
- **Windows has no check:** the `.exe` spawned by each client, a
  `TerminateProcess` leaving collin running, a CRLF answer in VS Code. No CI
  job runs Windows, and the check's stand-in collin is a shell script.
- **A project-scoped `.mcp.json`** with `"."` for the project: the folder
  Claude Code starts a server in is undocumented, and was not checked. Only
  `claude mcp add` with the path written out is documented.
- **Clients cut long answers**: Claude Code stops at 25 000 tokens. The server
  does not cap, so that the answer stays the command line's.
- **Freshness is checked on every call**, git included, and is the whole of
  the warm call on a project in a repository. Keeping one git answer for a
  second or two would take it well under a millisecond on Shopify, and make
  the line late.
- ~~**A test to watch:**
  `the_end_of_input_kills_a_run_of_collin_and_the_server_ends` (`src/mcp.rs`)
  failed once in a first run and passed alone and on a rerun; possibly
  flaky.~~ Closed in 1.49.1: a race that could explain it (the stand-in wrote
  its pid after the file the test waits on), reproduced by pausing between the
  two writes; the original failure was never observed.

Left by the card's per-file locations (0113, 1.47.0), unranked:

- **A `.env` symlink pointing outside the project** is followed by
  `envs::discover`, as it has been for the page; the command line and the MCP
  tool now print the locations resolved from it too. Holding `discover` to the
  project would change the Catalog's files, so it was left.
- **`generate_*_name` macros are not evaluated**, so a project that overrides
  one gets the input to it on the card, not what dbt builds, and dbt's default
  prefixes a custom schema with the target's.
- **The card's `.env` files follow the default hide and show rules**, not the
  Catalog settings' own choices.
- **The word "lands"** survives in the section headings and some test names of
  `src/describe.rs`, after the wording moved to "config before the macros".

Left by the update card (0111, 1.45.0), unranked:

- **PowerShell is read, not run:** the update commands are tested for their
  structure only, and a failed build there still ends with a success status,
  only the closing line is skipped.
- **The card's Escape and focus rules** are covered by source-level tests and
  by a check in headless Chrome, not by a test that presses a key or clicks.
- **Escape in the editor, a find bar, the tour or a menu** stays theirs, so a
  card that stays there closes with its button alone.
- **With the Halloween look on**, the witch sprite sits on the dock tab row
  and the tabs are shifted about 14px down; not caused by the card, and not
  investigated.

Left by the count (0112, 1.46.0), unranked:

- **One key per call:** `--count-by` with two keys is refused, and a node's tags
  can add up to more than `(all)`, which is documented.
- ~~**Over MCP**, an invalid `--count-by` value is refused without clap's
  `[possible values]` line.~~ Settled in 1.49.1: the refusal keeps that line for
  `--count-by`, `--output` and `--column-tool`.
- **Over MCP the `[possible values: ...]` line** is kept whichever flag the
  error names, not checked against the allowed flags. Harmless today, but a
  terminal-only enum flag would list its values before being refused; clap's
  `ContextKind::InvalidArg` and `ValidValue` would be sturdier than parsing the
  text (`src/mcp_line.rs`).
- **Count mode** still sorts the selection by name before counting; linear and
  harmless.

Left by the job sources in the project (0115, 1.49.0), unranked:

- **The file names folders outside the project**, up to two folders above it
  and never a hidden one, so whoever opens a project that commits it reads
  them; the settings keep taking any path.
- **A hand-written file with one mistake is refused whole**, with its line; the
  form then shows an empty list until it is saved over.
- **`SKILL.md` is at its 4096 bytes now**, after cuts to make room for the jobs
  row; its description does not mention jobs, so an agent finds the row only
  once the skill is loaded.
- **A step that compiles** is not listed under `--job`'s nodes, and a job's
  `nodes N` counts what the steps that run, build, test or check act on.
- **With the sources file present**, every `--describe` of a command-line run
  reads the jobs, once per run and once per MCP session; a flag or a lazy read
  would make it opt-in.
- **Saving a path that does not exist** says "nothing at ..." rather than the
  precise refusal (hidden, absolute, too far), because the existence check runs
  first.
- **The `git pull` race** between the atomic rename and the next save is closed
  by the stamp the save returns, but no test covers it, for lack of a seam.
- **Two form states** were not driven in headless Chrome: the reason under the
  place choice for a path the file may not hold, and Save held during a save.
- **The Windows hidden and system attribute branch** is compiled and never run;
  the macOS `UF_HIDDEN` branch is tested.

Left by the agent skill (0114, 1.48.0), unranked:

- **The test checks that every flag and command in `SKILL.md` parses**, not that
  a recipe means what its row says.
- **`SKILL.md` is close to the 4096 bytes its test allows**, so a recipe added
  needs another cut; `scripts/walk_mcp.py --measure` and the figures of 0110
  still weigh the one-line skill.
- **Skill against MCP tool** was tried with Claude Code only, whose deferral of
  MCP tools is the client's setting; Copilot and the others were not tried.

Left by the pull requests of 2026-10-22 (#103 to #105, 1.49.1 to 1.50.0), each
a known limit, unranked:

- **`SKILL.md` is 4082 of its 4096 bytes** (the test's limit), so the next
  recipe needs a cut. The warning to say which steps were not resolved was
  dropped for lack of room.
- **The MCP variant of the skill** (a `SKILL.md` telling an agent to load the
  edith tool and call it) is documented, not shipped, and was tried with Claude
  Code only.
- **Show all and the zero-count Job coverage groups are untested**, since no
  harness slices `runSection` or `renderCoverage` (0013).
- **"Read again" folds Slowest back to five rows**, while a redraw from the
  panel keeps the full list.
- **A kind tile whose statuses include an unknown one is amber**, like the dot.
- **A new status word goes in two places:** the index run entry carries error,
  fail, warn, partial, ok and other (`counts()` in `src/run_results.rs`), and
  the page head recomputes the same from `run.statuses` (`runEndCounts` in
  `web/app.js`).
- **README pictures must be retaken** with `COLLIN` set to a 0.10 build and a
  dbt 1.12 venv, never the collin found on PATH when it is older;
  `scripts/take_shots.py` uses whichever it finds.

Left by the Checks section (0116, 1.51.0), unranked:

- **Fusion's manifest shape for `freshness` is unverified.** A source whose
  manifest has no key gets no verdict; where Fusion writes `null` for no
  threshold, the source reads as having none.
- **A source's test with a custom `name:`** in a manifest with neither
  `file_key_name` nor `attached_node` still falls to the file rule (0116).
- **Show all inside an open check** redraws every folder with the check fold's
  open state, so folders closed before it reopen. That is intended, as the
  comment in `web/app.js` says.
- **Two tables of one source in one file, with a relationships test between
  them,** both match `sources.<name>`, the first by id wins, and the column
  chip can hang on the wrong table. This is older than Checks and the same on
  main; Checks counts models only, so it is unaffected.
- **dbt-project-evaluator's definition of a tested model was not read.** The
  rule names are borrowed from its documentation.
- **Sources group under their YAML file's folder,** which reads "models" when
  they are kept there.
- **A new section that must not light the button or open the panel** sets
  `quiet` through `lights(sec)` in `web/app.js`.

Left by the indirect edges' boxes (0118, 1.53.0), unranked:

- **WHERE and HAVING share one label,** `WHERE / HAVING`, since collin merges
  them into `filter`. Telling them apart needs a fourth role from collin.
- **A column in two clauses of one model shows one,** the strongest by
  collin's order (`QUALIFY`, then `JOIN`, then `WHERE / HAVING`); the card's
  `keeps the rows of` has the same single role.
- **The command line answers per column where the page draws per model,** so
  `--column --indirect` lists every column an indirect edge reaches, as it
  always did, and `--output name` is not what the canvas shows.
- **A cache written before 0118 needs one Regenerate** to hold indirect edges
  at all: its modified time says nothing of a missing flag, so staleness does
  not offer it.
- **The MCP server merges the cache again for every column question,** and
  `--column` on every call (the figures are in 0118). Keeping the merged graph
  per cache path, size and modified time in `src/mcp_graphs.rs` would spare the
  MCP server that.
- **Every `--column` without `--indirect` now says on stderr** that the
  indirect edges were left out, since every cache the page or `--run-collin`
  writes holds them.
- **`Draw up to 3000` does not know the total.** The walk stops when it is full
  and counts nothing it turned away, so the button cannot say how much is left,
  or that 3000 will be enough.
- **A box's own +N is never drawn,** since nothing is walked from it: the
  column of its model an indirect edge reaches is opened from Models mode.
- **An indirect edge into a column the value walk reached too** is drawn by
  the model's box alone, and the column's +N leaves it out. Upstream, the
  edges stay column to column and can still fill a cap on a model with many
  keys.

Left by the pull requests of 2026-10-26 (#111 and #112, 1.52.0 and 1.53.0),
each a known limit, unranked:

- **A `DBT_TARGET` left unexpanded** (`${X}` in an `.env` file) is taken as
  written, since `src/envs.rs` keeps interpolation as text and warns about it.
  The environment is named with that value and no run matches it. When the table
  holds other targets' runs, the note names them. Expanding the reference, or
  treating such a value as unset, would change `src/envs.rs` (0117).
- **A run with no invocation row** has no target after the join, so an
  environment kept to its target drops it and its note does not count it. The
  package uploads both tables at the end of the same run, so this is rare.
- **The lineage dots are small,** 7 px across at zoom 1 and less when zoomed
  out, and the hollow ring of a model never built is easy to miss. Whether they
  should grow is open.
- **The hover card can cover the right end of the status bar,** as it already
  did before 1.52.0.
- **The pictures of the builds** need a Jaffle Shop copy with dbt_artifacts
  installed (`+schema: dbt_artifacts`, its database from an `.env` file) and
  `/api/snowflake/builds` answered by a stand-in. Nothing in the repository
  builds either, unlike `scripts/take_shots.py` for the README.
- **The export list's heading** ("Columns and models") is checked by a test on
  the source text only: no harness runs `exportLineage`.
- **`rows_by` merges two columns whose model and column names match in different
  packages,** which is older than 0118, and `rows_of` now lists two models of
  the same name twice.

Left by the pull requests of 2026-10-27 (#115 to #118, 1.54.1 to 1.56.0),
each a known limit, unranked:

- **A PowerShell run of the update command.** The `$?` chains (swap, restore,
  clean, closing line) are checked as composed strings only (0111, 0120); pwsh
  is preinstalled on the Ubuntu CI runners, so a test could run them there, as
  `sh` already runs the Posix ones.
- **The clean under a build target.** With `build.target` or
  `CARGO_BUILD_TARGET` set, the install builds into `target/<triple>/release`,
  which `cargo clean --release` leaves even with the same setting in force
  (0120); cleaning it needs the triple the install used, which only the
  terminal's shell and cargo's config know, not the server.
- **A `target/` cargo did not create.** It has no `CACHEDIR.TAG`, so every clean
  refuses it (exit 101) and it keeps growing, with cargo's error in the terminal
  as the only sign (0120); the menu could point it out.
- **Releases before 1.57.0 are not immutable.** The repository's setting has
  been on since 2026-10-27 and holds for releases published since, as
  `release.yml`'s error line and 0123 say; the files of 1.56.0 and older can
  still be edited in place.
- **No test makes a PowerShell `Move-Item` fail under `Stop`.** The real-shells
  test sets `$ErrorActionPreference='Stop'` on the swap, mismatch and 404 paths,
  but only the sh test covers a rename that fails.
- **Artifacts of the `windows` job last one day on pull requests and the
  repository default on releases.** Re-running a failed release job after that
  finds none, so it needs the whole workflow (workflow_dispatch).
- **VirusTotal is not an endpoint's on-device engine.** The release files scan
  clean there (0123), but an on-device model and its threshold are not on
  VirusTotal, so it cannot show what such an engine says of the MSVC file.
- **The ARM64 `.exe` has only run in CI** (`windows-11-arm`: the shells test and
  `--version`), never on Windows on ARM in someone's hands.
- **A build outside `<clone>/target` is offered the download, which overwrites
  it.** This covers a custom `CARGO_TARGET_DIR` or a clone moved after building
  (0123).
- **PowerShell's redirects are not held to HTTPS, as curl's are.** The release's
  `.sha256` comes from the same server, so the update's check catches a
  truncated download, not a swapped one.
- **Git Bash's `curl` reads `HTTPS_PROXY` only, not Windows' proxy settings.**
  Behind a proxy, the PowerShell terminal or a manual download is the way.
  Reading the system proxy is not done.
- **A build before 1.56.0 offers cargo alone.** Moving to the release's file
  takes one more cargo update, or a manual download plus `cargo uninstall
  dbt-edith`.
- **The git config gate and the launch key (0121, 0122):**
  - a repository git refuses for an owner it calls dubious is paused as
    unreadable, and its venvs are set aside with the rest;
  - a missed verdict costs two processes (`rev-parse` and `config`), and a
    repository where the search by eye and `rev-parse` disagree is never cached,
    so it pays them on every call;
  - a click still runs the hooks the repository holds, copied ones included
    (0053);
  - git older than 2.38 ignores `safe.bareRepository`, so the refusal of a
    bare-looking folder rests on `locate` alone;
  - git older than 2.26 cannot list scopes, so every repository is paused as
    unreadable and asked again at each poll, with no fallback to
    `--show-origin`;
  - a key a tool writes into the repository's own config (`git lfs install
    --local`, a credential helper set per repository) or an `include` pauses git
    until it moves to the global config;
  - the gate's read and git's own are not one step: a config changed in between
    is the one git uses;
  - a venv the user made in a paused repository is set aside until the pause
    lifts, its note saying git cannot tell;
  - `tracks()` still matches "not a git repository" anywhere in git's message,
    where the gate now matches its start.

Left by the job order pull requests of 2026-10-28 (#120 to #122, 1.57.0 to
1.59.0), each a known limit, unranked:

- **A task's group is not read.** dag-factory names it on each task
  (`task_group_name`), so a sensor on an `external_task_group_id` gates none of
  its tasks and its line says so (0124, 0125).
- **`--job-order` has no `--select`** to narrow the readers; an agent filters
  the JSON.
- **A DAG with no dbt task is one point.** Its own sensors and `depends_on` are
  not read, and one that neither updates an asset nor runs on one is not kept,
  so a sensor on it orders nothing (0125).
- **Not read:** a `TriggerDagRunOperator` (the DAG it starts reads as manual),
  an Airflow task starting a dbt Cloud job, and `trigger_rule`.
- **The Job order section** forgets the edges a "+N more" opened once the user
  comes back to it, and names a dbt Cloud step without its `[[ident]]`, which
  the answer does not carry (0126).
- **`waited_by` holds `(whole run)` inside its string**, as 0124 chose, so an
  agent reads the flag from the text; and `answer()` works out the steps'
  sensors a second time where `step_view` already has them.

Left by the keys from tests (0127, 1.61.0), each a known limit, unranked:

- **Not counted as a key:** a test with a `where:`, a kwarg that is not a list
  of strings or a `column_name` holding `{{` (Jinja left unrendered), a
  combination test named through a namespace other than `dbt_utils`, contract
  `constraints`, `dbt_constraints`,
  `dbt_expectations.expect_compound_columns_to_be_unique`, and not-null checks
  other than dbt's `not_null`.
- **Keys are not in `--describe` or MCP,** only on `/api/node`.
- **The three rules hold models only,** like the other four, not sources, seeds
  or snapshots, though a snapshot's `unique_key` is tagged in the Columns tab.
- **A `unique_key` expression** stays an entry (counted in `1/2`), is not
  checked for `not_null`, and merges only with a `unique` test written on the
  same text.
- **A `not_null` with `severity: warn`** covers its column, though it fails
  nothing.

Left by the pull requests of 2026-10-29 (#124 and #126, 1.59.1 and 1.60.0),
each a known limit, unranked:

- **A long Snowflake error in the column lineage** now shows whole in the
  status at the canvas's bottom right, which has no width limit, so on a
  narrow window it can reach the legend.
- **Builds by job's open row closes on a Refresh,** as before 1.60.0; only the
  environment picked last is kept, and the picks last as long as the page, not
  across a reload.
- **Builds by job's row wiring** (switching, closing on the same cell, opening
  by the row, the rows put back) is checked in headless Chrome only;
  `web/tests/jobbuilds.js` holds the order of the pick.
- **Every listable cell of Builds by job is a tab stop,** so a long table takes
  many presses of Tab.
- **Reports reopens on a section with news,** Job coverage for one, rather than
  on Builds by job.

Also unranked, set aside on 2026-10-29: picking the Snowflake role the builds
read connects with. One connection, with the active target's role, reads every
environment (0117).

Wanted but never ranked, because nobody has needed either enough to place it:
persisting the open tabs between sessions, which `src/settings.rs` already has
the store for, and filtering the column lineage by any edge kind, beyond stop
at's roles (0083) and the indirect button (0097), which waits on seeing how
dense a real graph is. Both were listed in the README's "Not there
yet" and nowhere else for a while, which is how that section came to
disagree with this one.

Also unranked, from `--job-order` (0125): reading a sensor on every DAG that
carries a tag, for which a job source would need a key naming the tag and the
DAG's tags key; and counting tests as readers of the models they test, which
the first needs, since a DAG that gates its tests on a tag is one the mode now
sees none of. Not started either: the per-task schedules a factory interprets, a
sensor's lookback or `execution_delta`, and which of two crons runs first. The
Job order section of Reports (0126) has no chip: one for the pairs the files
order the other way alone, which are rarely meant, would be the first to earn
a place on the button.

Also unranked, since nobody has asked for it: the terminal's venv in the status
bar under zsh, fish or PowerShell. They take prompt hooks from their startup
files alone, and 0069 rejected both ways in, a generated `ZDOTDIR` and a typed
line.

Left over from the layout (0027), also unranked: Fit centred on the focus with
a zoom floor, now that a neighbourhood can be 21 columns wide; the entry points
of a wide fan-in spread along the box's left side, where 27 parents now converge
on one pixel; network simplex for the columns, which would move the marts to
the end; and the Kahn layering in `Graph::selection`, which places nothing any
more and could simply send depth 0.

Sketched but not started: a column-lineage source using dbt Fusion's local
index (`dbt compile --static-analysis strict --write-index --write-lineage`),
which needs no warehouse privileges and covers uncommitted SQL. The index is
parquet under `target/index/`, so it needs a converter outside this binary
(0003). It would write `column_lineage.fusion.json`, which the menu's greyed
Fusion entry already waits for (0031).

1.0.0 is the first release of this repository, published on 2026-10-10 under
FSL-1.1-MIT with Datadorelix as the licensor (0046). The 0.x versions before
it were published under MIT from an earlier repository, and their history is
kept outside this one. Merging a version bump is the release, tagged and
published with its CHANGELOG.md section by CI (0045). Since 1.56.0 a release
carries a Windows `.exe` for x64 and one for ARM64 (0123); elsewhere installing
means building from source, as [the README](../README.md#getting-started)
describes. The files are unsigned, a certificate 0085 leaves to a decision of
its own. Looked at on 2026-10-15: Azure Artifact Signing takes an organisation
in Europe, and SignPath's free plan wants an OSI licence, which FSL-1.1-MIT is
not (0046). A VERSIONINFO resource came first (0093, 1.33.0).

The binary says which build it is, so `--version` and the status bar
tell two installs of one release apart. Updating a machine is `git pull` then
`cargo install --path . --locked`, which the status bar offers once a release
is out (0068), and comparing the stamp with `git describe` in the clone is how
you check it took.

A locked-down Windows machine can also build its own binary: `cargo install
--path . --locked` works there without administrator rights, with Rust's GNU
toolchain and a mingw-w64 installed through winget, once the assembler that
`raw-dylib` linking needs is on `PATH`. [docs/windows.md](windows.md) gives
both that route and the cross-compile.

## Waiting on a human

- **A real Snowflake answer.** Neither `probe` nor a column click has ever
  reached a warehouse, so the permissions story is unverified: Enterprise
  Edition, `VIEW LINEAGE`, and whether the objects of the chosen environment
  carry lineage at all. Everything up to the connection is tested against a
  fake connector. The query history (0048) waits on the same answer, with its
  own questions: whether the Snowsight URL built from the organization and
  account names opens the query, whether a role sees its user's queries under
  other roles, as the documentation implies, and whether reading the history
  wakes the target's warehouse. The Snowsight menu (0051) adds its own: whether
  a dynamic table lives under `dynamic-table` and a materialized view under
  `view`, since Snowflake publishes no scheme for them, what Snowsight does
  with a view asked for as a table, and whether a quoted lower-case name opens
  as written. Clone into my schema (0058) has written statements nobody has
  run yet: whether `drop view if exists` refuses on a second run once the copy
  is a table, a create as select from a dynamic table or a materialized view,
  and which grants the developer's role needs on the source databases.
- **Two checks on Windows**: `.env` files with CRLF endings read correctly, and
  the time a node click takes there. Also worth timing there: a content search
  for a word no file holds, and `/api/git` and `/api/freshness` asked 3 s
  apart, which take 165, 250 and 230 ms on a Mac. The plan was to cache the per-node
  environment resolution only if it exceeded 10 ms, and it measures well under
  that on a Mac.
- **The update on Windows (0068).** Whether the running `dbt-edith.exe`
  renames to `.exe.old` while it runs, under Git Bash and under PowerShell,
  and whether `cargo install` then puts the new one in place. The PowerShell
  form has run on no machine yet: the Mac has none.
- **Three choices left open on 2026-10-14.**
  - The Catalog's upstream and downstream lists put the depth back to 1, as
    any move from outside the canvas does, though they list the canvas's own
    neighbours. Keeping the depth there is one `keepDepth`.
  - The hues of the sub-folder bands per level (blue, teal, violet) were a
    first proposal (0080).
  - A source's copied name is `source.table`, as its title shows it, where
    `--select` wants `source:` in front and `source()` two arguments.
- **The Halloween look's sounds (1.43.0).** The ghost's wail and the eyes'
  whisper, made in the browser, have never been listened to: the checks ran the
  apparitions, not a speaker.
- **The GitHub menu's issue from someone else's account.** Pull requests
  are for collaborators only and issues open to all. The settings read right
  through the API, but nobody has yet opened an issue from Raise an issue
  with an account that is not a collaborator.
- **Choices the ten pull requests of 2026-10-18 left open.**
  - The `.exe`'s FileDescription, "dbt-edith, a browser IDE for dbt
    projects", is the name Task Manager shows for the process (#65); it
    could be `dbt-edith` alone. And the MSVC route's warning on every build
    could become a search for `rc.exe`.
  - A `collin.exe` copied over a file git tracks is refused too (0094), so
    the VM's collin has to sit in a folder git does not track.
  - The cut canvas's note says "the rest of their lineage" rather than
    "their children", a selection growing upstream too, and shows once the
    starts take half of what is drawn (1.31.0).
- **A binary built before 0015 has no guard.** Anyone running one
  needs `cargo build --release` again, the old one being vulnerable to the
  three attacks 0015 describes.

## Traps worth knowing

- **A clone's statements are written by `cloneShape` to `cloneScript`**, the
  pure block `web/tests/clone.js` slices out up to `openClonePlan`: a helper
  added after that name is outside the harness. A legacy snapshot configured
  with `target_schema` is refused under an environment, because that key is
  not read, and a package's files are never checked for changes since the
  parse, since their paths are the package's (0058).
- **A HashMap's order is new on every start**, and between two maps of one
  run. Iterating one into a payload, a list, a cap or a first-of-several gives
  each start its own answer, which a test building its graph once rarely
  shows: `every_answer_is_the_same_on_every_start` builds one manifest nine
  times for that reason. Read into a BTreeMap or sort by name, never by
  whatever came out first (0055).
- **A synthetic column-lineage cache looks exactly like a real one** apart from
  its `source` field. If the column graph looks suspiciously complete, check
  what produced the cache before trusting a screenshot of it.
- **The graph is held twice**, as `base` (manifest and catalog) and `graph`
  (`base` with the chosen cache merged into a copy). Anything that reads the
  manifest or the catalog again replaces both, through `load_all`; replacing
  `graph` alone brings the old manifest back on the next switch, which reads
  `base`. A test that swaps in its own graph sets both for the same reason.
- **A tab's text changes without the user when its file changes on disk**
  (0067), as a change whose origin is `disk`. A `change` handler that means
  "the user edited this" checks the origin, as the dirty flag does, or a pull
  marks every open tab edited.
- **A pan's click is swallowed on the window, in the capture phase.** Its
  listener is put there by the left release that ends a pan, so it runs
  before any other listener, capture-phase ones on the document included, and
  nothing in the page hears that click. A listener that must see every click,
  a pan's too, has to be on the window in the capture phase and added before
  the pan ends (`init` in `web/lineage.js`).
- **The page hears of a manifest reload only through the freshness poll.**
  The watcher reloads `manifest.json` server-side whenever dbt rewrites it, and
  nothing is pushed: `refreshFreshness` comparing `manifest_at` with `S.meta` is
  the one place the page catches up. Anything that has to follow a parse hangs
  off that, or off the manifest's reload, the arrows ending its pill, and
  needs both.
- **Release binaries embed the frontend** (0005). A frontend fix that appears to
  do nothing usually means the release binary was not rebuilt. The build stamp
  in the status bar settles it: compare it with `git describe` in the clone.
- **Picking Snowflake proves nothing about Snowflake.** It checks Python, the
  profile and the connector, all local. The first click is what reaches the
  warehouse, and what may open a sign-in tab.
- **A manifest carries the separator of the machine that parsed it.** A project
  parsed on the Windows VM gives every node an `original_file_path` full of
  backslashes, which a macOS or Linux dbt-edith then has to read. `Graph::build`
  normalises it once, at the boundary, so nothing downstream has to ask. If
  paths ever look doubled, unmatched in the tree, or open twice as two tabs,
  that normalisation is the first thing to check.
- **`model-paths` and its five siblings are read in one place**,
  `project::paths_in`, and only the folder links use them (0036).
  `src/freshness.rs` still assumes `macros/`, so the gap 0025 records is
  narrower than it was but not closed.
- **A key in `dbt_project.yml` that matches no folder stays dashed on purpose**,
  and a key that matches nothing at all in a block whose paths could not be read
  gets no link. Neither is a bug: refusing to guess a root is 0018's rule, and
  0036 says what each mark means.
- **The test harnesses slice `web/app.js` by function name** (0013). Renaming a
  sliced function breaks its harness; `./scripts/check.sh` catches it.
  `web/tests/selection.js` slices from `selectKindCounts` to
  `async function loadSidecar`, and `folders.js` and `export.js` read the same
  range, so anything new between those two has to be pure or it dies at eval
  time rather than at an assertion. The named selector helpers live there,
  `selectorCommand` to `selectEmptyText`, and the export slice calls them.
  `web/tests/collineage.js` slices from `function sidecarLabel` to
  `function profileLink`, which holds the column lineage menu: only
  declarations there, and the tool table is `lineageToolDefs()` rather than a
  constant for that reason. `humanAge` is now the
  start of two slices, `compiled.js` up to `freshnessBadge` and `freshness.js`
  up to `sendToTerminal`, so `artifactBar` between them is read by both and has
  to stay pure. `web/tests/testchips.js` slices from `testChips` to
  `function catalogColumns`, which makes `catalogColumns` an end marker as well
  as a function: `fillTestsCard` and `paintTests` sit inside that slice and are
  only ever read as declarations, so nothing between the two may run at eval
  time. A `const` there does not survive either, which is why the chip caps are
  arguments of `testChips` and not a constant beside the cell.
- **The comparison with dbt fails in CI when it skips itself**, since exit 2 is
  not 0: a dbt that did not install reads as a failure, never as a pass. The
  Jaffle Shop copy is upstream's with the additions its README lists; anything
  added there goes in that list, and a case the engine should meet goes in the
  fixture or in the script's `EXPRESSIONS`. Shopify is cloned at a pinned tag
  and commit, so a case for it goes in `SHOPIFY_EXPRESSIONS`, and a tag moved
  upstream fails the run rather than changing what it reads (0056).
- **A selector answer is this tool's, not dbt's** (0024). When one looks wrong,
  the dbt ls button types the command that settles it; the usual answer is the
  tests eye, which dbt has no equivalent of in `dbt ls`. Not for a named
  selector, where the box only filters the drawing: there the usual answer is
  the manifest, when a dbt Fusion that drops `indirect_selection` wrote it, and
  the amber note says so (0032).
- **`function renderTabs` is an end marker now**, for
  `web/tests/closetabs.js`, which slices `web/app.js` from `function
  closeTargets` to it. Only `closeTargets` may sit in that range, and it stays
  pure: anything else put between the two has to be pure or the harness dies at
  eval time. `web/tests/jumps.js` slices `function nextJump` to
  `function jump(`, which is the same arrangement for the back stack, so
  `nextJump` may not reach for `S` or the DOM.
- **`openFile` sits inside the slice `web/tests/tabs.js` evaluates.** Anything
  new it calls has to be stubbed there, or the harness dies with no output at
  all rather than a failed assertion.
  This is why the breadcrumb hooks hang off `activate`, which that harness
  already stubs, and not off `openFile`.
- **`web/tests/macros.js` slices** `web/app.js` three times: from
  `function maskJinjaComments` to `/* Explicit ref()`, which `vars.js` reads
  too and which holds `jinjaBlockEnd`, the comment mask's own dependency; from
  `function yamlIndent` to `/* ATX headings`; and from `const DECLARING_LISTS`
  to `async function markRefs`. Everything in those ranges stays pure.
  `markMacros` is called from `openFile`, which is why `tabs.js` stubs it, and
  `markProjectDirs` is called from there too and stubbed beside it.
  `web/tests/projectdirs.js` reads both of those ranges as well, so the folder
  key scanner sits in the second one and stays pure like everything around it.
- **`web/tests/find.js` slices** `web/app.js` from `function escapeRegExp` to
  `const findPrefs`, which is every pure piece of find, and from
  `function columnMatches` to `function focusColumnFilter`. Both of those sit
  inside the `testchips.js` slice as well, as declarations only.
  `web/tests/keys.js` slices from `function keyCombo` to
  `function toggleShortcuts`, and reads the `case '...'` labels of `wireKeys`
  as text, up to the `boot` banner: a global key bound any other way escapes
  the check that it is in the shortcut list. `web/tests/graphfind.js` slices
  `web/lineage.js` from `function findNames` to `function pastSlop`, where
  `pan.js` starts, as well as the layout's own slice, and `web/app.js` from
  `function findLabel` to `function findOverlay`, from `function findButton`
  to the comment over `findBar`, and from `function findInPane` to
  `const base = (path)`.
- **`exportViewer` runs in the exported file, not in the app.** It is written
  into the page as its own source text, so a name from `web/app.js` inside it
  passes every check here and fails only in a downloaded file.
  `web/tests/export.js` looks for the usual ones.
- **The canvas CSS exists twice**, in `web/app.css` and in `exportCanvasCss`.
  Editing a `.nd`, `.edge`, `.role`, `.band` or `#graph text` rule in one
  fails the export harness until the other matches.
- **A band's name on the canvas is not in the SVG.** It sits in
  `.folder-heads`, an HTML layer `Lineage.init` puts after the svg, moved by
  `placeHeads` on every pan and zoom, so a name stays readable at any zoom.
  `snapshot()` writes the names into an export itself and cuts the bands,
  drawn far taller than the graph on the canvas, to the picture.
- **The export harness slices** `web/app.js` from `function exportTitle` to
  `function exportLineage`, and from `function legendEntries` to
  `function paintLegend`. Everything in those ranges stays pure, and
  `exportLineage` must never become `async`: the slice would end on a bare
  `async` and stop parsing. The same harness evaluates `frameOf`, which sits
  inside the `web/lineage.js` slice that `colours.js` reads.
- **The layout lives in that same slice**, from `const MAT = {` to
  `let svg, root`: `dagLayout` and every `dag*`, `bk*` and `iso*` function it
  calls, read by `colours.js`, `export.js`, `layout.js` and `folders.js`. `W`, `H`, `HGAP` and
  `VGAP` sit above the slice, which is why sizes arrive as a `dim` argument, and
  nothing in it may touch the DOM or the module's `data`, `place` or `bbox`.
  `RIDE`, the sizes of a test hanging under its model, sits inside it, beside
  the `dagRiders` to `dagMount` functions that hang them (0034) and
  `foldLabel`, the chip's words (0035). So do `TEST`, `testLevel`, `dagRows`,
  `groupKey`, `rowLabel`, `memberName` and `rowClip`, which make a model's
  tests into rows (0040). What is opened lives outside it, in `unfolded`, and
  reaches `dagLayout` as its `open` argument: a model's id for the rows past
  the fold, a group's key for a group turned from how it started. A `const`
  in the slice is invisible to the harness that evaluates it, so a harness
  reaches `TEST` through `nodeColor` and builds no key but through `groupKey`.
- **`testLine` sits in the `const DECLARING_LISTS` to `async function markRefs`
  slice**, beside `macroDefLine`, which `web/tests/testline.js` evaluates with
  the `yamlIndent` one, so it stays pure; the size guard, `OUTLINE_MAX`, is
  `openTest`'s, outside both. `openTest` decides from the cursor whether a click
  is a new place for Back: a second click on the test already shown, half of a
  double click, records none.
- **`testLevel` reads a test with no `test_name` as singular.** Every payload
  that carries tests carries `test_name` and `column` for that reason, the
  search hits included; a new one that leaves them out paints every generic
  pale and dashed. `testFacts`, the hover card's lines about a test, sits in
  the `hovercard.js` slice, from `function placeFloating` to
  `function hoverCardBody`, and takes the level as an argument rather than
  reaching for `Lineage`, so it stays pure.
- **`st.root` is not a canonical path on Windows.** It is `canonicalize`
  without the `\\?\` prefix (`files::plain`), so comparing it with a path
  `canonicalize` returned fails there and nowhere else: canonicalize both, as
  `files::resolve` does.
- **`portable-pty` does not simply inherit the environment.** Its
  `CommandBuilder` starts from a copy of its own, which on Windows it
  overwrites with the registry's variables, so an activated venv's `PATH` is
  lost there and nowhere else. Build the terminal's command through
  `pty::command`, which clears that copy and takes this process's.
- **The shell's startup files reorder `PATH` after it is inherited.** A
  `~/.bashrc` that puts pyenv's shims or conda first hides an inherited venv's
  `dbt`, so getting the environment right is not enough: the venv's activate
  script is typed after them (0050). Typing anything into a new terminal on
  Unix before the line editor reads shows it twice; wait as
  `pty::line_editor_ready` does.
- **A job's terms go through `Expr::from_terms`, never `Expr::parse`.**
  `parse` strips a pasted `dbt ls -s` off the front, so a model called `test`,
  `source` or `run` would lose its term, and it reads `-e` as `--exclude`,
  which dbt does not.
- **The jobs cache holds the graph by a `Weak`.** `load_all` and a switch of
  column lineage replace the `Arc`, which reads every job again; the
  `Arc::make_mut` of a click's merge does not, and must never add nodes, since
  each step's answer is a list of node indices. The job files themselves are
  looked at only when a request says `look=true`, which the page sends when the
  Jobs tab, the sources form or the Reports panel's job coverage opens; the
  user asked for nothing to be checked while nothing moved, so a click in the
  tab or the panel never looks, and a coverage row brings the Jobs tab forward
  with `showDock('jobs', { look: false })`. The guided tour never looks
  either: its Jobs step and the dock it puts back pass `look: false`, and a
  panel it rolled up comes back with `openReports(key, { reread: false })`,
  unread (0075). The Catalog's Jobs section asks
  without `look` too, and only while the Catalog is seen (`fillCatalogJobs`,
  from `catalogPreview`, `showDock` and the Reports panel rolling up). It keeps each answer
  per node, so every request that looks calls `forgetCatalogJobs` too, as do a
  save of the sources and a new manifest: a new path that reads jobs again
  has to, or the Catalog keeps the old chips.
- **A feeder is never a job (0077).** `Read::feeders` holds the DAGs with no
  dbt task only for `link` to number after the jobs; nothing that lists,
  counts or groups jobs may read it, or every ingestion DAG would show under
  Jobs read and in the sources form's counts. A feeder file still counts as
  skipped, and is never refused: a file that is not about dbt has no line to
  be wrong at. `Start` is ordered `No < Unread < Yes`, which `link` and
  `coverage` compare with `max` and `min`; a fourth state goes in that order
  or breaks both.
- **A step's `dbt ls` is written by the server** (`Invocation::ls`), so the
  page, `web/tests/jobs.js` and `compare_with_dbt.py` type the same line. That
  harness slices `web/app.js` from `function jobsScheduleText` to
  `async function loadJobs`; the DOM half sits after it.
- **A panel rolls up and down by its `.open` class, never `.hidden`** (0091).
  No transition leaves `display: none`, and the editor, the terminal and the
  canvas under it must never be measured at nothing. Closed, it is clipped
  and `visibility: hidden`; open, what is under it is `inert`. Every panel
  goes through `openPanel` and `closePanel`, and is listed in `PANELS` with
  all five of its hooks: what paints its button, what it does once rolled
  up, what the tour keeps of it (`save`) and how it puts it back (`reopen`);
  `S.panel` names the one down. A check for "something covers the editor and
  the dock" reads `S.panel`, one for a given panel `panelOpen(name)`. When one
  panel takes another's place, `closePanel({ next })` runs the first one's
  hooks with `S.panel` already naming the second: a hook asking whether
  something under them is seen, `catalogSeen()` say, must hear no.
- **The selection's panel borrows the dock's canvas (0092).** `SELECT_MOVES`
  lists what `selectPanelDown` moves up, `#dock-lineage` with the selector bar,
  five of the canvas's tools and `#catalog`, and `selectPanelUp` puts each back
  before the sibling it was taken from, the last first. Code that asks whether
  the canvas or the Catalog is on screen reads `lineageSeen()` or
  `catalogSeen()`, never `S.dock` alone: with the panel down, `S.dock` may be
  the terminal while the canvas is in front. `showDock` leaves `#dock-lineage`
  shown while it is up there, `paintMode` rolls the panel up once the mode is
  no longer `select`, and `focusNode` draws its answer only while the canvas
  still asks for that model, since the panel rolling up and a file opened can
  each ask for one, in either order. `runSelection` calls `selectPanelDown`
  before it sets the mode, so `S.selectBack` holds the mode, the column and
  the card the dock had; `selectPanelUp` clears `selectTimer` and bumps
  `S.selectAsk`, so neither a keystroke's run nor an answer on its way lands
  after it.
- **The settings draw from the page's state, and a form is kept (0100).** A
  block of a section with `data-live` is drawn again by `paintSettingsLive`,
  which `repaintSettings` calls once for every change made in one go:
  `paintSourceButton`, `applyLineage`, `paintUpdate`, `paintVenv` and
  `paintEnvSelect` call it, so a new piece of state a section shows needs its
  painter to call it too. `redrawIfChanged` draws a block, and the rail, into
  a detached copy first and replaces it only when the markup differs: a
  state a row's listener reads must show in the row, as `S.featuresBusy`
  does in Snowflake's `aria-busy`, or a redraw that skips keeps a listener on
  the old state. The focus comes back by `data-key`, so a control in a live
  block, and each rail row, carries one. Project and Orchestration hold forms: their body
  is kept in `S.settingsKept` while the panel is down, and dropped by the
  panel's `closed` hook, but for the tour's, which `save` and `reopen` carry
  across; a form that failed to read is not kept. Their live blocks are drawn
  again on every showing, their inputs never. The `closed` hook is told
  whether the close gives the focus back, which is when the settings give
  Reports back if they came down over it. A section draws the menus' own rows, `fillSourceMenu`, `envRow` and
  `paintUpdateMenu`, then `unmenu` takes the menu roles off them.
- **`activate()` rolls the panel up, and `settleTabs` passes `keepPanel`**,
  so closing a tab from the sidebar while one is down leaves it there.
  `showDock` leaves it alone: a column's lineage brings the canvas forward
  when its answer lands, seconds later with Snowflake, and must not roll up a
  panel opened meanwhile, so the terminal's key and `sendToTerminal` close
  it themselves.
  `focusNode` cannot close it, since a collin run's end calls it in the
  background: every way in from a report goes through `openFromReport`, and
  the Models list and the palette call `closePanel` themselves.
- **The index counts the job coverage only once the jobs were read**
  (`jobs::Cache::read_once`), and with `look=false`: it never reads the job
  files first, and a new manifest reads them again as the Jobs tab would
  (0066). The page keeps what the coverage held when last shown, and lowers it
  when the count falls, so news is a rise from there.
- **`run_results.json` is parsed by the index too**, once per write, through
  the cache `/api/reports/run` reads (`files::Cached`): the button counts its
  errors and failed tests. A file dbt is still writing fails the parse, and the
  index then lists the run by its time alone; the cache keeps that failure for
  the same size and time, and the page asks the index again on every poll
  until the counts are there, since dbt can finish within the second the
  stamp already holds. Of `args`, only `which` or
  `command` and the three selection flags are declared: `invocation_command`
  and `vars` hold what the user typed, secrets included (0065).
- **The page hears of a report written outside it through `reports_at`** in
  the freshness poll, which is compared before anything else is repainted. A
  new report's file belongs in `reports_at` and in the index, never in a timer
  of its own. `web/tests/reports.js` slices `web/app.js` from
  `function reportsSections` to `async function loadReportsIndex`.
- **Reaching the server by any name other than `127.0.0.1` or `localhost`
  gets a 403** (0015). A tunnel or a proxy in front of it is not a supported
  setup, and the symptom is every request refused, not a blank page.

## Automated checks

GitHub Actions runs `cargo test --locked`, a RustSec audit of the lockfile that
refuses a yanked version, an audit of `web/vendor/` by SHA-256 against the
upstream tarballs and against OSV, the Snowflake script's tests and the
comparison with dbt-core 1.11 and 1.12 on Jaffle Shop and Shopify, every
model's lineage included (0033, 0056), the MCP server against the command
line on Jaffle Shop (0110), on every push and every Monday, and
Dependabot opens weekly lockfile bumps. A pull request touching `src/`, `web/` or `tools/` also has to touch `CHANGELOG.md`, or carry the
`no changelog` label, and its branch and title have to read the way AGENTS.md
says, Dependabot's own branches excepted. CodeMirror is at 5.65.21, which does
not fix CVE-2025-6493 (SECURITY.md). The browser harnesses are not
in CI: they need `jsc`, which ships with macOS (0013). Nor are the Chrome
walks `scripts/check.sh` runs, the tour, the depth boxes and Edith's menu.

Several checks depend on timing, and can fail on a loaded machine, then pass
when run again alone:

- the 250 ms layout in `web/tests/layout.js`, where 3150 nodes are laid out
  (in the group of tests folding under a chip);
- `git::a_quick_command_is_not_held_back`;
- `scripts/walk_depth.py`, which times out on a wait for the lineage to settle;
- since 1.25.0, the ring around `#keys-btn` in `scripts/walk_tour.py`, drawn
  2.6px right of the button, and under load the one around `#palette-btn`.
  The ring is placed while the ? button still shifts. The likely cause is
  that the age on the manifest's pill gains a digit during the measure, and
  moves the buttons to its left. It fails on `main` too, so it is not a
  branch's doing.

`scripts/walk_menu.py` fails now and then in its cleanup, not in a check: the
temporary Chrome profile it removes is still written to, and the removal
stops on `OSError` 66, directory not empty. Run it again.

Another is skipped rather than failed: since 1.26.0, "crossing the name opens
nothing" in `scripts/walk_menu.py` needs two pointer moves within 100 ms, and
after three slow tries it is not made, with a NOTE under "menu ok", and
check.sh passes, its last line naming "a timing check of Edith's menu".
