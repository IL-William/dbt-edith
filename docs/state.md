# Where the work stands

Rewritten as things change, unlike [decisions/](decisions/), which is appended
to. Last updated 2026-09-19.

## Shipped

Editor with clickable `ref()` and `source()` and Jinja coloured by role,
lineage graph in model and column modes, column lineage fetched from Snowflake
when a column is clicked and the switch in Catalog > Columns is on (0016),
terminal, file explorer with git and
unsaved colouring, search across nodes, file names and file contents, Catalog with columns and
locations, compiled SQL with freshness, git panel (status, branch switch, stage,
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

Every route sits behind the Host and Origin guard added on 2026-09-17 after a
security audit found the terminal reachable from any web page (0015). The same
pass confined `/api/git/diff` to the project and added `SECURITY.md`.

## Deferred, in the order they were chosen

1. **Macro layer.** Links on `{{ macro() }}` calls, and a used-by count per
   macro. The interesting part is reporting macros with no inbound reference
   without claiming they are dead: a macro can be called from YAML, from a
   selector, or by another package.
2. **Run history.** `run_results.json` gives status and timing per node. Status
   as the box stroke in the graph, plus staleness against the manifest. Watch
   for partial runs: a node absent from the file was not run, which is not the
   same as not tested.
3. **Named selectors, then orchestration coverage.** `src/select.rs` resolves a
   typed expression; what is left is reading the project's `selectors.yml` and
   offering those by name, which needs a hand-written YAML scanner (0018). Only
   then scan the orchestrator's jobs to show which models no schedule covers.
4. **A var's definition line, clickable.** The card names
   `dbt_project.yml:<line>`. The scanner this was waiting for now exists:
   `yamlOutline` plus `gotoPos` in `web/app.js` is most of the work.
5. **Symbols in SQL.** CTE names and `{% macro %}` blocks in the breadcrumb,
   which needs SQL strings and comments masked first, for the reason 0022 gives.

Sketched but not started: a second column-lineage source using dbt Fusion's
local index (`dbt compile --static-analysis strict --write-index
--write-lineage`), which needs no warehouse privileges and covers uncommitted
SQL. It fills the same cache file (0008).

0.4.0 ships the breadcrumb bar, the selector mode in the lineage tab and the
rename to Edith, which reached main together. 0.2.0 added the hover cards;
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
- **Release binaries embed the frontend** (0005). A frontend fix that appears to
  do nothing usually means the release binary was not rebuilt. The build stamp
  in the status bar settles it: compare it with `git describe` in the clone.
- **Switching Snowflake lineage on proves nothing about Snowflake.** It checks
  Python, the profile and the connector, all local. The first click is what
  reaches the warehouse, and what may open a sign-in tab.
- **A manifest carries the separator of the machine that parsed it.** A project
  parsed on the Windows VM gives every node an `original_file_path` full of
  backslashes, which a macOS or Linux dbt-edith then has to read. `Graph::build`
  normalises it once, at the boundary, so nothing downstream has to ask. If
  paths ever look doubled, unmatched in the tree, or open twice as two tabs,
  that normalisation is the first thing to check.
- **The test harnesses slice `web/app.js` by function name** (0013). Renaming a
  sliced function breaks its harness; `./scripts/check.sh` catches it.
  `web/tests/selection.js` slices from `selectKindCounts` to
  `async function loadSidecar`, so anything new between those two has to be pure
  or it dies at eval time rather than at an assertion.
- **A selector answer is this tool's, not dbt's** (0024). When one looks wrong,
  the dbt ls button types the command that settles it; the usual answer is the
  tests checkbox, which dbt has no equivalent of in `dbt ls`.
- **`openFile` sits inside the slice `web/tests/tabs.js` evaluates.** Anything
  new it calls has to be stubbed there, or the harness dies with no output at
  all rather than a failed assertion.
  This is why the breadcrumb hooks hang off `activate`, which that harness
  already stubs, and not off `openFile`.
- **Reaching the server by any name other than `127.0.0.1` or `localhost`
  gets a 403** (0015). A tunnel or a proxy in front of it is not a supported
  setup, and the symptom is every request refused, not a blank page.

## Automated checks

GitHub Actions runs `cargo test`, a RustSec audit of the lockfile, an OSV audit
of `web/vendor/` and the Snowflake script's tests on every push and every
Monday, and Dependabot opens weekly lockfile bumps. CodeMirror is at 5.65.21 since 2026-09-17, which does
not fix CVE-2025-6493 (SECURITY.md). The browser harnesses are not
in CI: they need `jsc`, which ships with macOS (0013).
