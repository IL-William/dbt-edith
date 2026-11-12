# Tests

Everything at once, which is what to run before calling a change done:

```
./scripts/check.sh
```

A part that needs something this machine lacks skips itself, and the run
still passes, but its last line then reads `passed, but skipped:` and names
each part left out, where a run of everything reads `all good`. The
comparison with dbt and the Chrome walks build dbt-edith and run it from
where cargo says it put it, which `CARGO_TARGET_DIR` moves, or run the binary
`DBT_EDITH` names.

The Rust side has unit tests for `.env` parsing, location resolution and the
settings store:

```
cargo test --locked
```

Clippy runs on every target with each lint an error, on stable (0171). It needs
the `clippy` component, `rustup component add clippy`, and `check.sh` skips it
without. Alone:

```
cargo clippy --all-targets --locked -- -D warnings
```

A lint that is wrong for one place is allowed on that item, with a
one-line reason; three are allowed for the whole crate, in `Cargo.toml`.

Only the Rust code is formatted, by `rustfmt.toml` (0172). It needs the
`rustfmt` component, `rustup component add rustfmt`, and `check.sh` skips it
without. Run `cargo fmt` to format, and this to list the files that differ:

```
cargo fmt --check -- -l
```

A conflict in a Rust file is never resolved by hand: take one side of the
file and run `cargo fmt` again.

`git blame` looks past the formatting commit once the clone is told where it is
listed: `git config blame.ignoreRevsFile .git-blame-ignore-revs`.

The browser code has small harnesses that run on macOS with the system
JavaScript engine, no install required, from the repository root:

```
JSC=/System/Library/Frameworks/JavaScriptCore.framework/Versions/A/Helpers/jsc
$JSC web/tests/tabs.js        # preview/pinned tab state machine
$JSC web/tests/disk.js        # the one stretch a tab replaces when its file changed on disk
$JSC web/tests/explorer.js    # git and unsaved colouring, including folders
$JSC web/tests/collineage.js  # composite column ids, node subtitles, and the column lineage menu
$JSC web/tests/conflicts.js   # conflict block detection, including half blocks
$JSC web/tests/colours.js     # materialization colours, custom ones, and seeds apart from tables
$JSC web/tests/colourvars.js  # every colour a variable: no literal in app.css below the theme blocks (the Halloween theme's art apart) nor in lineage.js
$JSC web/tests/diff.js        # diff ruler geometry, clamping and pane heights, the inline layout's lines and folds
$JSC web/tests/gittab.js      # the Git tab's words: ahead and behind, the glyphs' titles, the commit button
$JSC web/tests/palette.js     # search palette merging nodes and files
$JSC web/tests/location.js    # written, resolved and built locations
$JSC web/tests/snowsight.js   # links into Snowsight: a query, and a relation's object, schema and database
$JSC web/tests/clone.js       # Clone into my schema: what each node is copied as, from where, and the script
$JSC web/tests/jinja.js       # Jinja colouring, and SQL never shown the Jinja
$JSC web/tests/hovercard.js   # where a hover card lands beside its anchor
$JSC web/tests/vars.js        # var() / env_var() scanning, and where a value came from
$JSC web/tests/refs.js        # ref() / source() scanning, a version's included, and the Catalog's version chip
$JSC web/tests/grep.js        # what a search result says, and where the match falls
$JSC web/tests/selection.js   # what a resolved selector says: counts, warnings, dbt ls, named selectors
$JSC web/tests/breadcrumb.js  # the breadcrumb's path segments and document outline
$JSC web/tests/compiled.js    # what the Compiled and Run bars say about their file
$JSC web/tests/compiledenv.js # the Compiled and Run SQL for another environment: relations swapped, kept, and the words
$JSC web/tests/freshness.js   # the manifest pill and its card
$JSC web/tests/testchips.js   # the Tests cell in Catalog > Columns, and its +N
$JSC web/tests/strategy.js    # an incremental model's strategy on its box and card, the columns its key names, and the tag of a key
$JSC web/tests/export.js      # an exported graph: its header, its file, its safety, the theme it is drawn in, none of the Halloween art, and the export menu's words, counts and file names
$JSC web/tests/layout.js      # where each box sits, and how a long edge gets there
$JSC web/tests/folders.js     # the canvas drawn by folder: which folder, in what order
$JSC web/tests/legendpick.js  # a legend entry clicked: what stays lit in each mode
$JSC web/tests/macros.js      # macro calls, and the names a properties file declares
$JSC web/tests/macrouse.js    # who calls a macro, on its card, its palette row and in Reports
$JSC web/tests/find.js        # find in a file: matches, steps, the count, the column filter
$JSC web/tests/graphfind.js   # find on the lineage: what a box is found by, the order, a folded test
$JSC web/tests/keys.js        # key names on each platform, and the shortcut list against wireKeys
$JSC web/tests/reports.js     # the Reports panel: its sections, the news on its button, where it opens, its rail, the filter over its folds
$JSC web/tests/builds.js      # the last build per environment: where the tables are, the verdicts, the marks, the Reports lists
$JSC web/tests/joborder.js    # Job order in Reports: the groups, a pair's lines, the edges and the ten drawn, what the answer leaves out, and the pieces every section is drawn from, the model chips included
$JSC web/tests/jobbuilds.js   # builds by job: a cron read in its source's zone, the last start across a clock change, a job's models late, built since, not built
$JSC web/tests/panels.js      # every panel: one down at a time, what goes inert, where the focus goes back
$JSC web/tests/selpanel.js    # the Lineage space's panel: what the dock gets back, its card, a line dropped, what an empty box opens on
$JSC web/tests/slips.js       # a model's tests as slips: the two runs of a row, the bar's three shapes in 3 by 10, the room a box and a row have, the +N pill, the ring
$JSC web/tests/locationlines.js # the card's Location as a list: one value where the stages agree, a line per run where they differ
$JSC web/tests/spaces.js      # the top bar's tabs: the lit one, a click on it, Code from the settings opened over Reports, Escape
$JSC web/tests/colwait.js     # a column asked for during a collin run: the canvas waits cleared, the last click is drawn, a failure gives the model back
$JSC web/tests/working.js     # Edith's mark moving while she works: which requests count, its delay, its fade, two at once
$JSC web/tests/settings.js    # the settings panel: its sections, what its rail and its rows say (a .env file, a job source, the next look for a release), where it opens, the settings button's mark
$JSC web/tests/status.js      # the status bar's place for each space, its counts, and the order of the top bar's and the status bar's controls
$JSC web/tests/tour.js        # the guided tour's steps, which it leaves out, and where its ring and card go
$JSC web/tests/launchkey.js   # the launch key: taken from the link, kept per origin, dropped from the address, sent as a header
$JSC web/tests/halloween.js   # the Halloween theme: after dark, the wait for an apparition, the witch's way, its timers and saves, the box art's rules
$JSC web/tests/themes.js      # the four themes: their blocks, Light's contrast, Graphite's greys, the pick, the export's theme
$JSC web/tests/shapes.js      # one shape per meaning: no marker round, every state its pictogram
$JSC web/tests/catalog.js     # the Catalog: the words beside the name, its tabs, the filter's count, a Lineage cell's halves, a job chip, and the rules of its one look
$JSC web/tests/fileicons.js   # a file's icon: its shape by extension, its colour by what the manifest builds it as, the tree's git letters
$JSC web/tests/tabkinds.js    # the editor's tabs and Open editors: a model's tab draws its kind's bar, a row its file's icon, a late mark draws the tabs once more
$JSC web/tests/site.js        # the static site's answers: web/site.js against Rust's, from tests/fixtures/site_case.js, and the pieces its walks lean on
$JSC web/tests/sitepage.js    # a site's page: the header's facts and link, the tab's title, the node an address names
```

The Snowflake script has tests of its own, against a fake connector and a fake
PyYAML, so they need no warehouse and nothing installed:

```
python3 tools/test_sf_lineage.py
```

The linters run alone with `python3 scripts/lint.py` (`--github` for CI's
annotations): Ruff on the Python and ShellCheck on the shell scripts git
tracks or would add, Biome on `web/app.js`, `web/lineage.js` and `web/site.js`, with the rules
of `ruff.toml` and `biome.jsonc` (0173). Each is downloaded once, from its
upstream release and against a SHA-256 pinned in the script, into
`target/lint-tools/<tool>-<version>-<hash>/`, and run from there; delete that
folder to fetch them again. It exits 2, and `check.sh` skips the part, offline
with nothing cached or on a platform no build is pinned for. Needs Python 3.10
or later and nothing else.

The selection box of the Lineage space, `--select` and the lineage are compared with dbt
itself: every named selector and a list of typed expressions, asked of the box
and of the command line, and the nodes drawn around each model at one, two and
twenty levels, resolved by dbt-edith and by `dbt ls`, have to name the same
nodes. The Warnings report's config paths that apply to nothing are compared
with the warning dbt gives as it parses, path for path (0103), and the four
lists of the Checks report with the same lists worked out in Python from the
manifest dbt wrote (0116). It runs on a
copy of dbt Labs' Jaffle Shop in `tests/fixtures/`, a second copy given such
paths, a third given sources, models and tests of every shape the checks tell
apart, and Fivetran's Shopify package, cloned at the tag
`tests/fixtures/shopify/` names. It needs dbt-core and dbt-duckdb, and skips
itself without them; without git or the network it skips Shopify:

```
python3 -m venv /tmp/dbt && /tmp/dbt/bin/pip install dbt-core dbt-duckdb
/tmp/dbt/bin/python scripts/compare_with_dbt.py                 # every eighth Shopify model's lineage
/tmp/dbt/bin/python scripts/compare_with_dbt.py --every-model   # all of them, as CI does
DBT=~/.local/bin/dbt python3 scripts/compare_with_dbt.py   # or any dbt executable, Fusion included
```

`dbt-edith --mcp` is driven over its stdin and stdout on a parsed copy of the
same Jaffle Shop, the way an MCP client drives it, and every question it
answers is asked of the command line too: the two have to give the same
stdout and stderr, bar the ages and the time collin took, and `isError` has to
be the exit code not being 0 (0110). The copy is parsed and then compiled
but for `stg_payments`, so `--compiled-sql` is compared on a file that is
there, a seed and a model dbt did not compile (0132). The copy gets a `.dbt-edith/jobs.yml`
over a copy of the invented job fixtures, so `--describe`, `--job`, `--job-order` and `--job-coverage`
are compared on jobs too (0115, 0125, 0131). The session also has to leave the project
and the manifest's folder as they were, write nothing to a configuration
folder, and, for `--run-collin` with a stand-in collin, change exactly the two
files collin writes. It needs the same dbt, parses and compiles with it, and skips itself
without it; CI runs it beside the comparison above:

```
/tmp/dbt/bin/python scripts/walk_mcp.py
```

`--measure` times the same kinds of question through the command line and
through one server process, on Jaffle Shop and on Fivetran's Shopify, and
prints tables of milliseconds, bytes and memory. It builds the release binary,
compares nothing, and neither check.sh nor CI runs it. Shopify needs git and
the network; `SHOPIFY_FIXTURE` names a folder with `target/manifest.json`,
`target/catalog.json` and `expected/cache.json` to stand in for `dbt parse` and
for a column cache, else the `--column` lines are left out:

```
/tmp/dbt/bin/python scripts/walk_mcp.py --measure [--work /tmp/measure]
```

The guided tour is walked in headless Chrome on the same Jaffle Shop, every
step, through the DevTools protocol and the standard library alone. It needs
dbt to parse the copy, and Chrome; it skips itself without either:

```
/tmp/dbt/bin/python scripts/walk_tour.py                  # every step, and what the tour leaves behind
/tmp/dbt/bin/python scripts/walk_tour.py --shots /tmp/t   # and a picture of each
```

The lineage's depth control is driven the same way: what keeps the depth on the
canvas, what starts it again at 1, what Back and Forward bring back, and the
control itself by the keyboard on a copy of Jaffle Shop that has a model of 40
characters with no children: Enter opens a menu on the level drawn, an arrow and
Enter draw another with one request, Escape gives the focus back, Enter on the
copy button writes the selector as shown, a model with no children shows `+0`
while the hidden box holds 1, the long name is cut in the middle with its menu's
rows fitting, and the name gives way to the bar of its kind in a narrow dock:

```
/tmp/dbt/bin/python scripts/walk_depth.py
```

Edith's menu is driven by a real pointer, the same way: what opens it under
the pointer, what closes it, and what a click on it holds open, where the
theme menu sits and what its keys do, and that the manifest pill's card, held
open, has a poll rewrite its age in place and keeps the focus on its Run button
when a poll changes the answer (0157):

```
/tmp/dbt/bin/python scripts/walk_menu.py
```

An export of the lineage with the Halloween theme on has to be the export of
the same canvas with the usual look, for each of the three box styles, art and
cut names included (0107), and its colours the theme on screen's, Night under
Halloween and Light under Light (0150), and the Export menu works from the
keyboard, each of its rows saving what it says (0153):

```
/tmp/dbt/bin/python scripts/walk_export.py
```

The static site (0177) is `dbt-edith PROJECT --site`, a folder that opens from
`file://` and from any static host. Its answers are Rust's, held in two places.
The first is a golden file: `tests/fixtures/site_case.js` is written by a Rust
test from an invented graph (a diamond, a test guarding two models, a disabled
node, a versioned model, an operation, an exposure, a chain 22 levels deep, a
fan-out, accents and emoji, one name in two packages), with what `/api/lineage`,
`/api/search` and `/api/node` answer, and `web/tests/site.js` makes the same
requests of `web/site.js` and compares them key for key. A change to a walk in
`src/graph.rs` or `src/api.rs` that moves an answer fails the Rust test until
the file is written again, and the jsc test until `web/site.js` follows:

```
DBT_EDITH_BLESS=1 cargo test site_case     # writes the file; without it the test compares
/System/Library/Frameworks/JavaScriptCore.framework/Versions/A/Helpers/jsc web/tests/site.js
```

The second is a walk in Chrome. It writes the site on Jaffle Shop, built with
its catalog, and on Fivetran's Shopify at the pinned tag (0056), parsed only,
each a git repository with an invented github.com origin and a `.env` that holds
a canary. It compares every node's lineage at 1+1, 2+2 and 20+20 with the tests
on and off, and at 2+2 under a cap of 5, every card but its location resolved
against a `.env`, a set of searches, and every folder of the project tree the page
lists against the server's `/api/dir`, cut to the files that carry a kind and the
folders that hold one, between the server and the page opened
from the folder; reads the page as a person does (the header and its commit
link, the Models list, a click, a box, the Catalog and its Columns, Cmd+K, an
address that names a node, a click on what the site hides); and fails on a
request outside the folder, an exception, a console error, a violation of the
page's policy, an absolute path or the canary in what was written. Then it does
the same over `http.server` under a sub-path, counting what the host was asked
for. It needs dbt and Chrome, and skips without either, exit 2; without git or
the network Shopify alone is skipped, and the exit is 2 even when Jaffle Shop
agreed:

```
/tmp/dbt/bin/python scripts/walk_site.py
```
