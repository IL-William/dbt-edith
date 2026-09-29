# Changelog

What shipped, and when. This file is appended to, never rewritten, which is
what makes it the only place that keeps the order:
[docs/state.md](docs/state.md) says where the work stands today and is rewritten
as it moves, the [README](README.md) says what the tool does now, and
[docs/decisions/](docs/decisions/) says why it is built this way. A feature
belongs in the README too; only here does it carry a version.

One entry per change someone using dbt-edith would notice, written as a
sentence saying what it does, the way a commit subject is. A number in brackets
points at the decision behind it. A version's date is the day its tag was cut.

Versions up to 0.4.0 were reconstructed from git history and
[docs/state.md](docs/state.md) on 2026-09-22, after the fact, so they group a
release rather than follow each pull request. Everything from 0.6.0 on is
written as the work lands.

**There is no 0.3.0.** The version was bumped to it on a branch that went on
collecting work and merged as 0.4.0, so 0.3.0 never reached `main`, was never
tagged, and nothing was ever installed from it.

**There is no 0.5.0 either.** `Cargo.toml` was bumped to it early, then main
went on collecting work for eleven more pull requests and the tag was never
cut, so everything the bump was meant to carry ships as 0.6.0 instead.

## Unreleased

- A menu in the top bar picks where column lineage comes from, Fusion, Collin
  or Snowflake, each in its own colour, where a read-only chip used to name the
  source (0031). A tool with nothing to offer is greyed and says which file it
  lacks, and every other cache beside the manifest stays pickable. Picking a
  cache from the old menu in Catalog > Columns never worked, and now does.
- Picking Snowflake is what switches fetching on click on: the separate
  Snowflake switch is gone, and Snowflake picked before its first fetch shows
  no other tool's edges under its name (0031).
- A settings menu behind the gear in the top bar switches Snowflake's features
  on or off for the project. Until you choose, they follow the manifest's
  adapter; off, nothing of Snowflake is offered, its cache and the profile link
  included (0031).

## 0.6.0 - 2026-09-29

- `Cmd/Ctrl + F` finds inside the file being edited, the Compiled and Run SQL
  and either side of a diff, where it used to open the browser's own find,
  which missed every match not on screen (0030). The bar counts and paints
  every match, matches case, whole words or a regular expression on request,
  and steps with `Enter` and `Shift + Enter`; `Escape` leaves the cursor on the
  last match.
- In the Catalog, `Cmd/Ctrl + F` filters the Columns table by name, and the
  filter stays as you move from one model to the next.
- `?`, `F1` or the `?` button in the top bar lists every keyboard shortcut, in
  the keys of the machine it runs on, and the top bar says `Ctrl+K` on Windows
  where it said `⌘K` everywhere.
- `Alt + W` closes a tab on a Mac too, where Option turned the W into `∑`
  before the shortcut could see it.

- Seeds are drawn in a yellow-green of their own, on the lineage canvas, in its
  legend and in the sidebar's dots. Their green sat so close to a table's that
  every seed read as a table until its subtitle was read. The new colour stands
  apart from every other box and role colour, to a red-green colour-blind eye
  too.

- The Lineage tab can draw its canvas by folder: the **folders** checkbox,
  beside tests, gives each folder a band of columns, so numbered layers such as
  `10_raw`, `20_clean` and `30_vault` each read as a block, left to right
  (0029). The names stay pinned over the top of the canvas, and an exported
  file or copied image keeps the bands with their names. Folders go in the
  order of their numbers, then of the edges between them; an edge from a later
  folder to an earlier one is dashed, and the line under the canvas counts it.

- A macro call in the editor is a link: clicking `hub` in `{{ hub(...) }}`
  opens the file that defines it, at its `{% macro %}` line, and pausing on it
  says which package it came from (0028). A bare name reaches the project's own
  macro and never an installed package's, as in dbt, so `hub` and
  `automate_dv.hub` can open two different files. Calls in `dbt_project.yml`
  hooks and in a `macros:` properties entry are linked too. dbt's own macros, a
  package that is not installed and anything that is not a macro stay text.
- In a properties file, the name of each source table, model, seed, snapshot and
  exposure links to its node: a click moves the lineage onto it and leaves the
  editor in the file, and a pause shows the node's card.
- Coming back to a properties file keeps the lineage on the node it shows when
  that file declares it, instead of moving to whichever of its nodes the server
  lists first.

- The lineage canvas puts every box to the right of all the parents drawn, so
  every edge runs left to right and an intermediate model sits between what it
  reads and what reads it, instead of after a model that reads it (0027). A
  model read by more models than it reads moves right, to just before its first
  reader. The columns used to be each model's distance from the focus, which on
  a 112 model graph sent 27 of its 152 edges leftwards.
- An edge that skips columns runs level through a lane between the boxes of each
  column it crosses, instead of passing behind them, and a model read across
  many columns shares one lane among its readers. On the same graph, edges
  passing behind a box went from 199 to none, and crossings from 316 to 57.
  Where level lanes would cost more than half again the height of the tallest
  column, as with the tests ticked, some lanes bend instead when that makes the
  graph shorter: 8 126 px rather than 15 615 on that graph with its tests.
- In column mode, an edge the canvas has to draw against its direction, where
  the column lineage loops, is dashed, and a column feeding itself gets a small
  loop on the right of its box.

- The Lineage tab exports what it shows: Export saves one HTML file that opens
  in any browser with nothing installed and no network, zooms without blurring
  and prints to a one-page PDF, for a ticket that says which models a release
  rebuilds (0026). Its header names the `dbt ls` command that lists the same
  nodes, the warnings of a mistyped selector, how much of a capped selection was
  drawn, and when and from which manifest, branch and commit the picture came.
- Copy image, beside Export, puts the same picture on the clipboard as a PNG to
  paste into a ticket's description, and saves the PNG instead when the browser
  refuses the clipboard.

- The Compiled and Run tabs answer for a test, not only for a model. A generic
  test's file is named by dbt, which truncates a long generated name and appends
  a hash, so it is found by reading the `compiled_path` the manifest records
  rather than by deriving a name nothing could reconstruct. Measured on an
  18 825 node project: 148 of 148 generic tests sampled now show their compiled
  SQL, where none did.
- The Run tab offers `dbt test --select` for a test, where it offered
  `dbt run --select`, which selects nothing and would have left the tab saying
  what it said before.

- Catalog > Columns shows every test guarding a column rather than one chip per
  kind: two `relationships` on one column, or two `expression_is_true`, used to
  collapse into a single chip and the second test was gone before the browser
  saw it. A column guarded by more than the cell can hold shows a `+N` that
  lists the rest on hover, on click and from the keyboard, each with the name
  dbt generated for it.
- Catalog > Preview lists the tests that guard the whole model instead of only
  counting them, under their own heading and apart from the ones guarding a
  column. A singular test under `tests/` and a generic with no column, such as
  `unique_combination_of_columns`, name no column and so had no row in the
  Columns table: until now they appeared nowhere at all. The Columns toolbar
  says how many of them there are, and clicking it goes to that list.

- A dot beside Reload manifest says whether the lineage on screen still matches
  the files dbt would parse right now, and clicking it runs `dbt parse` in the
  terminal. Freshness is measured in file times rather than in commits (0025),
  which is what separates your own unsaved work from a checkout or a pull you
  have not re-parsed since.
- A second segment beside that dot says how far the branch trails the default
  one, as `main +7`. It never colours the dot, and a background fetch every ten
  minutes, read only, keeps the count meaning something.
- A Run tab sits beside Compiled, over `target/run/`: the statement dbt actually
  sent to the warehouse, where Compiled is the model with its Jinja rendered.
  Until now one tab probed both directories and showed whichever it found first,
  so a model dbt had run but not recompiled showed its `create table` under
  Compiled.
- Both tabs name the date their file was written, not only its age, so it can be
  compared with the manifest, the nightly build or the edit still open.
- What turns those tabs amber is now what has moved, never the clock (0025). The
  bar names it: the model, its schema file, `dbt_project.yml` or a macro, and on
  the Run tab, a compile that happened after the last run. Age alone no longer
  colours anything, because a file nothing has touched since is still what dbt
  would write, and the hour-old rule it replaces only taught you to ignore the
  colour.
- The path in either tab is a link. Clicking it opens the file tree at the file
  dbt wrote, under `target/`.

## 0.4.0 - 2026-09-21

- The Search tab in the sidebar looks inside every indexed file rather than at
  their names, so a column used in forty models is findable. It never opens a
  `.env` (0020).
- A breadcrumb bar under the tabs says where you are twice over: the file's path
  through the project, and, inside a `.yml` or a `.md`, where the cursor sits in
  the document. Every segment opens a menu. The outline is scanned in the
  browser, and `.sql` shows the path alone (0022).
- A third lineage mode draws whatever a dbt selection expression matches. The
  expression is resolved against `manifest.json` rather than by running `dbt ls`
  (0024), so it answers in milliseconds and needs no profile; a button types the
  equivalent `dbt ls` into the terminal when you want dbt's own answer.
- The column lineage cache names which producer wrote each edge and what role
  the edge plays (0021), and 0023 records how parsing SQL for it sits with 0002.
- A manifest parsed on Windows now reads correctly on macOS and Linux: its
  backslash paths are normalised once, at the boundary, in `Graph::build`.
- Writing a file atomically keeps that file's mode. It stopped mattering only
  for settings when 0017 sent `~/.dbt/profiles.yml` through the same helper.
- The tool is renamed from dbt-lens to dbt-edith.

## 0.2.0 - 2026-09-18

- Pausing on a lineage box or on a `ref()` opens a card with the model's
  description, its first columns and types, its upstream, downstream and test
  counts, and its tags. Pausing on a `var()` or an `env_var()` shows what that
  variable is worth, resolved under the selected environment, with the line it
  came from. Project vars are read by a hand-written scanner over
  `dbt_project.yml` (0018), because the manifest carries none; showing a
  resolved value widened the `.env` boundary, under the two guards 0019 sets.
- Clicking a column in Catalog > Columns fetches its Snowflake lineage, behind a
  switch remembered per project (0016). `tools/sf_lineage.py` owns the
  connection and reads your dbt profile, so the binary still has no HTTP client,
  no TLS and no credential handling (0008).
- The top bar names the `profiles.yml` the script read and opens it in the
  editor. It is the one file outside the project dbt-edith opens, and only
  because the script says which one it is (0017).
- The binary carries a build stamp from `git describe`, shown by `--version`, by
  the startup banner and in the status bar, because until then two installs of
  one release were indistinguishable and reinstalling looked like it had done
  nothing.
- `web/vendor/` is audited against OSV, which neither cargo audit nor Dependabot
  reads, and CodeMirror moved to 5.65.21.

## 0.1.0 - 2026-09-17

First release: one self-contained binary serving a browser IDE for a dbt project
on `127.0.0.1`.

- An editor with clickable `ref()` and `source()` and Jinja coloured by role,
  preview and pinned tabs, and a file explorer coloured by git and unsaved
  state.
- A lineage graph in model and column modes, laid out and drawn by hand (0006),
  read from `manifest.json` and never from parsing SQL (0002).
- A terminal, one PTY per connection, and a git panel that stages, commits,
  pulls, pushes and resolves conflicts through the CLI (0007), where nothing
  destroys work.
- A Catalog showing each node's columns and where it lives, resolved per
  environment from the project's `.env` files by a scanner (0009, 0010), with
  compiled SQL and its freshness, search over nodes and file names, and the
  Python environment in the status bar.
- Every route sits behind a Host and Origin guard, added after a security audit
  found the terminal reachable from any web page (0015). A release binary built
  before that has no guard and needs rebuilding.
