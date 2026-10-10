# Where the work stands

Rewritten as things change, unlike [decisions/](decisions/) and
[../CHANGELOG.md](../CHANGELOG.md), which are appended to. What shipped and in
which version belongs there; what is half done, deferred or waiting on someone
belongs here. Last updated 2026-10-10.

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

Hover cards: a lineage box or a `ref()` shows the model's
description, columns and counts; a `var()` or `env_var()` shows its value,
resolved under the selected environment. Project vars come from a hand-written
scanner over `dbt_project.yml` (0018), because the manifest does not carry them.
Showing a resolved value needed the .env boundary widened, which 0019 does,
under two guards.

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

Manifest freshness: a dot beside Reload manifest says whether
the lineage on screen still matches the files dbt would parse, and clicking it
runs `dbt parse` in the terminal. Freshness is measured in file times rather
than in commits (0025), because committing changes no file and pulling an old
commit changes several. git is asked one question only, per file already known
to be newer: is it dirty? That separates the user's own unsaved work (amber)
from a checkout or a pull they have not re-parsed since (red). How far the
branch trails the default one rides beside the dot in its own segment and never
colours it. `api::watch_remote` fetches every ten minutes so that count means
something, read only and deadlined like the rest of 0007.

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
Not yet opened in Firefox, Safari, or Edge on the VM.

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
Chrome on that payload; not yet on the VM.

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
YAML, and packages under a custom `packages-install-path`. Not yet opened on
the VM.

Folder bands: the folders checkbox beside tests gives each
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
the page, not the browser: not yet opened on the VM, where Edge itself has to
leave `Ctrl + F`, `F3` and `F1` to the page, as Chromium does for any key it
does not reserve.

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
Not yet opened on the VM.

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
one still turns accent. Not yet opened on the VM.

Tests folded past five (0035): a hanging test costs 26 px, and
the most tested node of that project carries 134, about 3 500 px of pile. Under
a model the first five show and a chip opens the rest, per model, in place and
without moving the view; folded tests and their edges are not drawn, and the
status line counts them. The cap, which kept the canvas alive but emptied the
models reached last of their tests, now takes one test per model a turn, in
model mode and in Custom selection alike. In `layout.js`, 150 models of 20 tests
each lay out 1 978 px tall with five and a chip under each. Driven in headless
Chrome on that project: the node with 17 tests shows five and `+12 more tests`,
opens to seventeen by click and folds again by keyboard. Not yet on the VM.

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
the disk answers and the manifest could not. Not yet on the VM.

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
the active tab, which it never did while the bar overflowed. Not yet on the VM.

A tab's own menu and a way back: right-click a tab for Close,
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
manifest, the same. Not yet on the VM.

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
`ref()`. Not yet on the VM.

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
the file's own lineage replaced both 8 s after the parse. Not yet on the VM.

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
assumed. Not yet on the VM, whose system font is Segoe UI.

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
Not yet on the VM.

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
below 0.1.0, where collin's numbers restarted (0047), and "Check for a newer
collin" otherwise (0043); both type
the install command, which cargo answers. On the 18 825 node project that
share was 100% while its manifest, written by Fusion's `parse`, carried no
`compiled_code`, before collin read `target/compiled/` (below). Not yet tried on the VM, where cargo may not
reach GitHub and copying `collin.exe` beside the binary is the fallback.

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
main's `app.js` sent `^C` and `^V`. The `.exe` cross-compiles; neither fix has
run on the VM yet.

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
cross-compiles; the activation has not run on the VM yet.

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
Not yet on the VM.

The config button: the `dbt_project.yml` segment at the end
of a model's breadcrumb (0038) became a round button in the same violet, a file
with an arrow coming into it, its name in the tooltip. Drawing it showed the
Catalog's `.crumb` rule giving every breadcrumb segment a margin that lifted it
3.5px and cut the top off a hover; the bar now scopes that away. Driven in
headless Chrome on a model of the 18 825 node project: the button sits whole in
the bar and opens the menu of keys. Not yet on the VM.

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
**Run history** below, which reads `run_results.json`. Not yet on the VM, nor
against any warehouse.

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
committed venv's `python` that leaves a mark when run left none. Not yet on
the VM, where a restart now means opening the new link.

## Deferred, in the order they were chosen

1. **A used-by count per macro.** The links have shipped (0028); the
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
3. **Orchestration coverage.** Named selectors have shipped (0032);
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
yet" and nowhere else for a while, which is how that section came to
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

1.0.0 is the first release of this repository, published on 2026-10-10 under
FSL-1.1-MIT with Datadorelix as the licensor (0046). The 0.x versions before
it were published under MIT from an earlier repository, and their history is
kept outside this one. Merging a version bump is the release, tagged and
published with its CHANGELOG.md section by CI (0045). No binary is attached, so
installing means building from source, as
[the README](../README.md#getting-started) describes. Attaching binaries is a
deliberate later step: an unsigned executable download brings its own friction
on a managed Windows machine.

The binary says which build it is, so `--version` and the status bar
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
  fake connector. The query history (0048) waits on the same answer, with its
  own questions: whether the Snowsight URL built from the organization and
  account names opens the query, whether a role sees its user's queries under
  other roles, as the documentation implies, and whether reading the history
  wakes the target's warehouse. The Snowsight menu (0051) adds its own: whether
  a dynamic table lives under `dynamic-table` and a materialized view under
  `view`, since Snowflake publishes no scheme for them, what Snowsight does
  with a view asked for as a table, and whether a quoted lower-case name opens
  as written.
- **Two checks on Windows**: `.env` files with CRLF endings read correctly, and
  the time a node click takes there. The plan was to cache the per-node
  environment resolution only if it exceeded 10 ms, and it measures well under
  that on a Mac.
- **A binary built before 0015 has no guard.** Anyone running one
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
- **The page hears of a manifest reload only through the freshness poll.**
  The watcher reloads `manifest.json` server-side whenever dbt rewrites it, and
  nothing is pushed: `refreshFreshness` comparing `manifest_at` with `S.meta` is
  the one place the page catches up. Anything that has to follow a parse hangs
  off that, or off the Reload manifest button, and needs both.
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
- **Reaching the server by any name other than `127.0.0.1` or `localhost`
  gets a 403** (0015). A tunnel or a proxy in front of it is not a supported
  setup, and the symptom is every request refused, not a blank page.

## Automated checks

GitHub Actions runs `cargo test`, a RustSec audit of the lockfile, an OSV audit
of `web/vendor/`, the Snowflake script's tests and the comparison with dbt-core
1.11 and 1.12 (0033) on every push and every Monday, and Dependabot opens
weekly lockfile bumps. A pull request touching `src/`, `web/` or `tools/` also has to touch `CHANGELOG.md`, or carry the
`no changelog` label, and its branch and title have to read the way AGENTS.md
says, Dependabot's own branches excepted. CodeMirror is at 5.65.21, which does
not fix CVE-2025-6493 (SECURITY.md). The browser harnesses are not
in CI: they need `jsc`, which ships with macOS (0013).
