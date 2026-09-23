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
release rather than follow each pull request. Everything from 0.5.0 on is
written as the work lands.

**There is no 0.3.0.** The version was bumped to it on a branch that went on
collecting work and merged as 0.4.0, so 0.3.0 never reached `main`, was never
tagged, and nothing was ever installed from it.

## Unreleased

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
