# Where the work stands

Rewritten as things change, unlike [decisions/](decisions/) and
[../CHANGELOG.md](../CHANGELOG.md), which are appended to. What shipped and in
which version belongs there; what is half done, deferred or waiting on someone
belongs here. Last updated 2026-10-01.

## Shipped

Editor with clickable `ref()`, `source()`, macro calls and the folder keys of
`dbt_project.yml`, and Jinja coloured by role,
lineage graph in model and column modes, column lineage from the tool picked in
the top bar, Snowflake's fetched when a column is clicked (0016, 0031),
terminal, file explorer with git and
unsaved colouring, search across nodes, file names and file contents, Catalog with columns and
locations, the compiled and run SQL under target/ with freshness, git panel (status, branch switch, stage,
commit, push, pull, conflicts, side-by-side diff), Python environment in the
status bar, and environment-aware location resolution with a `.env` selector.

Hover cards, added 2026-09-18: a lineage box or a `ref()` shows the model's
description, columns and counts; a `var()` or `env_var()` shows its value,
resolved under the selected environment. Project vars come from a hand-written
scanner over `dbt_project.yml` (0018), because the manifest does not carry them.
Showing a resolved value needed the .env boundary widened, which 0019 does,
under two guards.

Breadcrumb bar, added 2026-09-18: the row under the tabs shows the file's path
and, inside a `.yml` or a `.md`, where the cursor sits in the document. Every
segment opens a menu, so a sibling file or a neighbouring model is one click
away without leaving the editor. The outline is scanned in the browser (0022);
`.sql` shows the path only, until string and comment masking exists.

Search across file contents, added 2026-09-18: the Search tab reads the indexed
files rather than their names, so a column used in forty models is findable. It
never opens a `.env` (0020). A full pass over a 12 000 file project is under a
second in release, after an ASCII fast path and a per-file pre-check.

Selector expressions, added 2026-09-19: the Lineage tab has a third mode where
you type a dbt selection expression and the canvas draws the set it matches,
laid out by longest path within the selection so disconnected pieces each start
at the left. `src/select.rs` re-implements dbt-core's selector methods over the
manifest rather than shelling out to `dbt ls` (0024), which answers in
milliseconds and needs no profile; the fidelity that costs is watched by
refusing an unsupported method by name and by a button that types the
equivalent `dbt ls` into the terminal for you to compare. Measured on a
109 MB manifest: one term with a `+` resolves 265 nodes in well under 10 ms.

Manifest freshness, added 2026-09-21: a dot beside Reload manifest says whether
the lineage on screen still matches the files dbt would parse, and clicking it
runs `dbt parse` in the terminal. Freshness is measured in file times rather
than in commits (0025), because committing changes no file and pulling an old
commit changes several. git is asked one question only, per file already known
to be newer: is it dirty? That separates the user's own unsaved work (amber)
from a checkout or a pull they have not re-parsed since (red). How far the
branch trails the default one rides beside the dot in its own segment and never
colours it. `api::watch_remote` fetches every ten minutes so that count means
something, read only and deadlined like the rest of 0007.

Compiled and Run tabs, added 2026-09-22: the two files dbt leaves per node
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

Every test in the Catalog, added 2026-09-22: a column held the *names* of the
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

Every route sits behind the Host and Origin guard added on 2026-09-17 after a
security audit found the terminal reachable from any web page (0015). The same
pass confined `/api/git/diff` to the project and added `SECURITY.md`.

Compiled and Run for a test, added 2026-09-23: both tabs were derived from a
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

Export and Copy image, added 2026-09-23: the canvas leaves as one HTML file for
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
Not yet opened in Firefox, Safari, or Edge on the VM.

Columns from the edges, added 2026-09-23: a box's column was the server's BFS
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
Chrome on that payload; not yet on the VM.

Macro links, and the names a properties file declares, added 2026-09-23: a
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
YAML, and packages under a custom `packages-install-path`. Not yet opened on
the VM.

Folder bands, added 2026-09-28: the folders checkbox beside tests gives each
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
the project's column cache holding two edges. Not yet opened on the VM.
Find, the column filter and the shortcut list, added 2026-09-28 (0030):
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
the page, not the browser: not yet opened on the VM, where Edge itself has to
leave `Ctrl + F`, `F3` and `F1` to the page, as Chromium does for any key it
does not reserve.

Column lineage by tool, added 2026-09-29 (0031): the read-only chip in the top
bar is now a menu of Fusion, Collin and Snowflake, each in its own colour, and
the same menu sits in Catalog > Columns. The menu there had never worked: the
page never read the caches `/api/meta` listed, and picking one called an
`api.post` that did not exist, so a project with collin's cache could only ever
show it. A tool is its newest cache, by the producer the header names; older
files and other producers' stay under "other caches". Picking Snowflake is one
request that also starts the script, so the old switch, and `POST /api/sidecar`,
are gone. Snowflake's features sit behind a switch in a new menu, opened by a
snowflake in the top bar, which follows the manifest's adapter until the user
chooses; off, the server refuses the tool, its caches, a fetch and the profile. The switch and the Snowflake
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
files under Snowflake. Not yet opened on the VM.

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

Named selectors, added 2026-09-29 (0032): a menu beside the Selection box lists
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
Not yet opened on the VM.

Checked against dbt itself, added 2026-09-30 (0033): `scripts/compare_with_dbt.py`
resolves the 16 named selectors of a Jaffle Shop copy in `tests/fixtures/`, and
26 typed lines, with dbt-edith and with `dbt ls`, tests on. All 42 agree under
dbt-core 1.11.15 and 1.12.5, which CI installs, and under dbt Fusion 2.0.6 run
by hand; the 17th selector uses `state:` and is refused. The fixture gives each
`indirect_selection` mode an answer of its own, 13, 11, 10 and 1 nodes for one
model, because its first version had no test reading a model and its own
ancestor, and buildable answered what cautious did. A run takes about 6 s with
dbt in process.

Tests under their model, added 2026-09-30 (0034): with tests on, a test hangs
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
one still turns accent. Not yet opened on the VM.

Tests folded past five, added 2026-09-30 (0035): a hanging test costs 26 px, and
the most tested node of that project carries 134, about 3 500 px of pile. Under
a model the first five show and a chip opens the rest, per model, in place and
without moving the view; folded tests and their edges are not drawn, and the
status line counts them. The cap, which kept the canvas alive but emptied the
models reached last of their tests, now takes one test per model a turn, in
model mode and in Custom selection alike. In `layout.js`, 150 models of 20 tests
each lay out 1 978 px tall with five and a chip under each. Driven in headless
Chrome on that project: the node with 17 tests shows five and `+12 more tests`,
opens to seventeen by click and folds again by keyboard. Not yet on the VM.

Folder keys in `dbt_project.yml`, added 2026-10-01 (0036): a key under
`models:`, or under its five path-valued siblings, opens the folder it
configures in the explorer, expanded, so a per-folder config and its folder are
one click apart. The chains are scanned in the browser and placed on disk by
the server, which reads `model-paths` and the rest for the first time in this
binary. A key that matches nothing is dashed and its card says dbt only warns
about that at parse time; a key that opens a folder with no resource under it
says that too, which is the manifest's one job here. What keeps the dashed mark
honest is dbt's own rule for this file, a key being a config when it starts with
`+` or carries one of dbt's config names and a path segment otherwise, with the
shape of the key as a second guard: `DBT_CONFIGS` in `web/app.js` holds those
names, read off the `config` of a model, a seed and a test node of that
project's manifest and completed from the documentation for the resources it
has none of. A package is recognised by the folder `dbt deps` wrote, not by a
node carrying its name, so a package of macros alone is recognised too, and a
key that is only a package name opens the package folder. On the 18 825 node project all 553 keys of its
project file resolve, in 160 ms for the whole round trip, 12 of them onto
folders the manifest has nothing under and one of them a package, which opens
under `dbt_packages/`. The card only calls a config dead when the freshness
badge says the manifest matches the project, since on a branch the folder is
usually newer than the parse. Driven in headless Chrome on that
project and on a copy of Jaffle Shop with no manifest at all, which is the case
the disk answers and the manifest could not. Not yet on the VM.

A header above the tree, added 2026-10-01: the project's name and two buttons,
one collapsing every folder and one reopening the tree down to the file being
edited. The collapse needed no new state, since what the tree knows about what
is open is `.kids.hidden` and `.row.open` in the DOM, and the children already
read stay there, held by `row.loading`, so reopening a folder fetches nothing.
The reveal reuses `revealInTree`, and both share `projectPath`, pulled out of
`activate`, since a tab's key is not a path for a diff or the profile.
`Alt + Shift + W` closes every tab, and `closeAll` now removes a diff tab's
MergeView host and hides the diff pane, which only `closeFile` did: closing a
diff that way left its two documents in the page. The tab bar also scrolls to
the active tab, which it never did while the bar overflowed. Not yet on the VM.

A tab's own menu and a way back, added 2026-10-01: right-click a tab for Close,
Close others, Close to the right and Close saved, the app's first context menu;
every other menu here is click-opened. `Cmd + Alt + P` and `Cmd + Alt + N` walk
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
both. Not yet on the VM.

## Deferred, in the order they were chosen

1. **A used-by count per macro.** The links shipped on 2026-09-23 (0028); the
   count is what is left of the macro layer. The interesting part is reporting
   macros with no inbound reference without claiming they are dead: a macro can
   be called from YAML, from a selector, or by another package. Most of the
   count is already in the manifest: every node and every macro carries
   `depends_on.macros`, which named one project macro for all 300 models
   calling it (0028). Hooks are the gap: their operation nodes list what they
   call under dbt-core and nothing under Fusion.
2. **Run history.** `run_results.json` gives status and timing per node. Status
   as the box stroke in the graph, plus staleness against the manifest. Watch
   for partial runs: a node absent from the file was not run, which is not the
   same as not tested. Per-node freshness would read the same mtimes 0025
   already walks, so the walk is the piece to reuse rather than repeat.
3. **Orchestration coverage.** Named selectors shipped on 2026-09-29 (0032);
   what is left is scanning the orchestrator's jobs to show which models no
   schedule covers. On the project this was built for, each job is a YAML file
   in another repository, a dbt command (`run --select tag:x`, or
   `test --selector name`) beside a cron schedule, so resolving one is the
   engine that now exists. The open questions are where that repository is,
   which is outside the project (0017), and an input format that is not one
   client's layout (0014).
4. **A var's definition line, clickable.** The card names
   `dbt_project.yml:<line>`. The scanner this was waiting for now exists:
   `yamlOutline` plus `gotoPos` in `web/app.js` is most of the work.
5. **Symbols in SQL.** CTE names and `{% macro %}` blocks in the breadcrumb,
   which needs SQL strings and comments masked first, for the reason 0022 gives.

Wanted but never ranked, because nobody has needed either enough to place it:
persisting the open tabs between sessions, which `src/settings.rs` already has
the store for, and filtering the column lineage by edge kind, which waits on
seeing how dense a real graph is. Both were listed in the README's "Not there
yet" and nowhere else until 2026-09-22, which is how that section came to
disagree with this one.

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

0.6.0 ships everything since 0.4.0: the manifest freshness badge, the Compiled
and Run tabs, the tests in the Catalog, the lineage export and its folder bands,
the macro and YAML links, the seed colour, find and the shortcut list. The
version was bumped to 0.5.0 early and the tag never cut, so that number never
shipped. 0.4.0 shipped the breadcrumb bar, the
selector mode in the lineage tab and the rename to Edith, which reached main
together. 0.2.0 added the hover cards;
since 0.2.0 the binary also carries a build stamp (`git describe`, or a build
date without a `.git`), shown by `--version`, by the startup banner and in the
status bar, because until then two installs of the same release were
indistinguishable and reinstalling on the VM looked like it had done nothing.

0.4.0 is tagged, as 0.2.0 and 0.1.0 were, and released on GitHub as source only.
**There is no 0.3.0.** The version was bumped to it on the breadcrumb branch,
that branch went on collecting work, and it merged as 0.4.0, so 0.3.0 never
reached main and was never tagged. An earlier version of this file said it was
tagged and released. It was not, and nothing was ever installed from it. No binary
is attached, so installing means building from source, as
[the README](../README.md#getting-started) describes. Attaching binaries is a
deliberate later step: an unsigned executable download brings its own friction
on a managed Windows machine.

Since 0.2.0 the binary says which build it is, so `--version` and the status bar
tell two installs of one release apart. Updating a machine is `git pull` then
`cargo install --path . --locked`, and comparing the stamp with `git describe`
in the clone is how you check it took.

A locked-down Windows machine can also build its own binary: `cargo install
--path .` works there without administrator rights, with Rust's GNU toolchain
and a mingw-w64 installed through winget, once the assembler that `raw-dylib`
linking needs is on `PATH`. The README gives both that route and the
cross-compile.

## Waiting on a human

- **A real Snowflake answer.** Neither `probe` nor a column click has ever
  reached a warehouse, so the permissions story is unverified: Enterprise
  Edition, `VIEW LINEAGE`, and whether the objects of the chosen environment
  carry lineage at all. Everything up to the connection is tested against a
  fake connector.
- **Two checks on Windows**: `.env` files with CRLF endings read correctly, and
  the time a node click takes there. The plan was to cache the per-node
  environment resolution only if it exceeded 10 ms, and it measures well under
  that on a Mac.
- **A release binary predating 2026-09-17 has no guard.** Anyone running one
  needs `cargo build --release` again, the old one being vulnerable to the
  three attacks 0015 describes.

## Traps worth knowing

- **A synthetic column-lineage cache looks exactly like a real one** apart from
  its `source` field. If the column graph looks suspiciously complete, check
  what produced the cache before trusting a screenshot of it.
- **The graph is held twice**, as `base` (manifest and catalog) and `graph`
  (`base` with the chosen cache merged into a copy). Anything that reads the
  manifest or the catalog again replaces both, through `load_all`; replacing
  `graph` alone brings the old manifest back on the next switch, which reads
  `base`. A test that swaps in its own graph sets both for the same reason.
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
  fixture or in the script's `EXPRESSIONS`.
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
  the check that it is in the shortcut list.
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
  `foldLabel`, the chip's words (0035). Which models are opened lives outside
  it, in `unfolded`, and reaches `dagLayout` as its `open` argument.
- **Reaching the server by any name other than `127.0.0.1` or `localhost`
  gets a 403** (0015). A tunnel or a proxy in front of it is not a supported
  setup, and the symptom is every request refused, not a blank page.

## Automated checks

GitHub Actions runs `cargo test`, a RustSec audit of the lockfile, an OSV audit
of `web/vendor/`, the Snowflake script's tests and the comparison with dbt-core
1.11 and 1.12 (0033) on every push and every Monday, and Dependabot opens
weekly lockfile bumps. A pull request touching `src/`, `web/` or `tools/` also has to touch `CHANGELOG.md`, or carry the
`no changelog` label, and its branch and title have to read the way AGENTS.md
says, Dependabot's own branches excepted. CodeMirror is at 5.65.21 since 2026-09-17, which does
not fix CVE-2025-6493 (SECURITY.md). The browser harnesses are not
in CI: they need `jsc`, which ships with macOS (0013).
