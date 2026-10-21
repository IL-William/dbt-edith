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
$JSC web/tests/diff.js        # diff ruler geometry, clamping and pane heights
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
$JSC web/tests/freshness.js   # the manifest freshness badge and its hover card
$JSC web/tests/testchips.js   # the Tests cell in Catalog > Columns, and its +N
$JSC web/tests/strategy.js    # an incremental model's strategy on its box and card, and the columns its key names
$JSC web/tests/export.js      # an exported graph: its header, its file, its safety, and none of the Halloween look
$JSC web/tests/layout.js      # where each box sits, and how a long edge gets there
$JSC web/tests/folders.js     # the canvas drawn by folder: which folder, in what order
$JSC web/tests/legendpick.js  # a legend entry clicked: what stays lit in each mode
$JSC web/tests/macros.js      # macro calls, and the names a properties file declares
$JSC web/tests/macrouse.js    # who calls a macro, on its card, its palette row and in Reports
$JSC web/tests/find.js        # find in a file: matches, steps, the count, the column filter
$JSC web/tests/graphfind.js   # find on the lineage: what a box is found by, the order, a folded test
$JSC web/tests/keys.js        # key names on each platform, and the shortcut list against wireKeys
$JSC web/tests/reports.js     # the Reports panel: its sections, the news on its button, where it opens
$JSC web/tests/panels.js      # every panel: one down at a time, what goes inert, where the focus goes back
$JSC web/tests/selpanel.js    # Custom selection's panel: what the dock gets back, its card, a line dropped
$JSC web/tests/settings.js    # the settings panel: its sections, what its rail says, where it opens, the gear's dot
$JSC web/tests/tour.js        # the guided tour's steps, which it leaves out, and where its ring and card go
$JSC web/tests/halloween.js   # the Halloween look: after dark, the wait for an apparition, the witch's way, its timers and saves, the box art's rules
```

The Snowflake script has tests of its own, against a fake connector and a fake
PyYAML, so they need no warehouse and nothing installed:

```
python3 tools/test_sf_lineage.py
```

The Custom selection box, `--select` and the lineage are compared with dbt
itself: every named selector and a list of typed expressions, asked of the box
and of the command line, and the nodes drawn around each model at one, two and
twenty levels, resolved by dbt-edith and by `dbt ls`, have to name the same
nodes. The Warnings report's config paths that apply to nothing are compared
with the warning dbt gives as it parses, path for path (0103). It runs on a
copy of dbt Labs' Jaffle Shop in `tests/fixtures/`, a second copy given such
paths, and Fivetran's Shopify package, cloned at the tag
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
be the exit code not being 0 (0110). The session also has to leave the project
and the manifest's folder as they were, write nothing to a configuration
folder, and, for `--run-collin` with a stand-in collin, change exactly the two
files collin writes. It needs the same dbt, parses with it, and skips itself
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

The lineage's up and down boxes are driven the same way: what keeps the depth
on the canvas, what starts it again at 1, and what Back and Forward bring back:

```
/tmp/dbt/bin/python scripts/walk_depth.py
```

Edith's menu is driven by a real pointer, the same way: what opens it under
the pointer, what closes it, and what a click on it holds open:

```
/tmp/dbt/bin/python scripts/walk_menu.py
```

An export of the lineage with the Halloween look on has to be the export of
the same canvas with it off, for each of the three box styles, art and cut
names included (0107):

```
/tmp/dbt/bin/python scripts/walk_export.py
```
