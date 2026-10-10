# Working on dbt-edith

A browser IDE for dbt projects: editor, terminal and lineage, served from one
binary that contains its own frontend. It reads a dbt project, it never runs
dbt.

**The constraint behind most of this codebase:** it has to run on a locked-down
Windows VM where nothing can be installed. No runtime, no npm, no installer.
That is why there is no build step, no framework, and a short dependency list.

## Verify

```
./scripts/check.sh          # Rust tests, both audits, the Snowflake script, dbt itself, browser tests, syntax
```

Run it before saying a change works. It is the only answer to "how do I check
this", and it needs nothing installed beyond Rust and macOS. The audit step
needs `cargo install cargo-audit --locked` and skips itself without it, like
the browser half without a JavaScript shell. The second audit asks OSV about
`web/vendor/` and skips itself offline. GitHub Actions runs the Rust tests and
both audits on every push and every Monday, never the browser half, which wants
macOS (0013). The Snowflake script is tested against a fake connector, so it
needs no warehouse, only Python 3.10 or later, and skips itself without one.
The selection engine and the lineage are compared with `dbt ls` on
`tests/fixtures/jaffle_shop` and on Fivetran's Shopify, cloned at a pinned tag,
which needs dbt-core and dbt-duckdb: set `DBT_PYTHON` to a Python that has
them, or `DBT` to a dbt executable. It skips itself without either, and skips
Shopify without git or the network; CI runs it on every model for two versions
of dbt-core (0033, 0056).
Updating a vendored library means changing its version in
`scripts/audit_vendored.py` and `THIRD_PARTY_NOTICES.md` too: the audit fails
when they disagree.

## Run

```
cargo run -- /path/to/dbt-project --port 4399 --no-open    # debug: serves web/ from disk
cargo build --release                                      # release: embeds web/
./scripts/build_windows.sh                                 # the VM's .exe, and its SHA-256
```

Debug reads `web/` from disk, so edit and refresh. Release embeds it, so a
frontend fix does not exist in a release binary until it is rebuilt (see 0005).

## Rules

- **No new dependency** without a decision record saying why. The absence of a
  regex, YAML or HTTP crate is deliberate (0003).
- **No build step for the frontend.** What ships is what is in `web/` (0004).
- **Never run dbt.** The binary never talks to a warehouse either:
  `tools/sf_lineage.py` does, started by the server only once the user picks
  Snowflake as the column lineage tool, opens the query history or a
  relation's Snowsight menu, with Snowflake's features on for the project,
  under their own credentials (0002, 0016, 0031, 0048, 0051). collin, which
  parses the compiled SQL and reaches nothing, is started only once Collin is
  picked, a column is clicked or Regenerate pressed, never at startup (0042).
- **Never return or log a `.env` value**, beyond the three things allowed to
  leave: resolved locations, `DBT_TARGET` (0012, 0017), and a resolved
  `env_var()` in the hover card, which passes two guards first (0019).
- **Nothing outside the project is read or written**, except the dbt profile,
  found where dbt looks for it, through its own route (0017, 0049), and that
  route never sends its passwords, tokens or keys (0054).
- **Treat this repository as public.** Fixtures and examples are invented, never
  taken from a real project (0014).
- **The browser is not trusted.** Every route sits behind the Host and Origin
  guard in `src/api.rs`, and no CORS header is ever added (0015). Every
  `/api/` and `/ws/` route also wants the key the launch printed (0052).
- **Nothing in the project runs on its own.** Learn about a venv from its files,
  never by running it, and never run one git tracks (0053).
- **Bump the version in `Cargo.toml`** for anything anyone installs, and turn
  `## Unreleased` into `## <version> - <date>` in the same pull request: CI
  refuses one without the other. Merging it is the whole release: the release
  workflow tags that commit and publishes the section as its notes (0045).
  Between tags the build stamp tells builds apart; the version is what says a
  release happened.
- **Read the deferred list before starting a feature**, in
  [docs/state.md](docs/state.md). The work is usually already there, in the
  order it was chosen, with the trap that deferred it written down: that trap is
  the reason it is not built, and it has not gone away. When the feature ships,
  strike its entry from that list in the same change. It is the only list of
  what is missing, and the README points at it rather than repeating it, because
  two such lists drift apart and it is the stale one that gets believed.
- **Every change someone using the tool would notice adds a line to
  [CHANGELOG.md](CHANGELOG.md)**, under `## Unreleased`, in the same sentence
  form as a commit subject. CI refuses a pull request that touches `src/`,
  `web/` or `tools/` without one, unless it carries the `no changelog` label,
  which is how a refactor or a dependency bump opts out. That file is appended
  to and never rewritten, unlike `docs/state.md`.
- **Comments say why, not what.** The code already says what it does.
- **No em dash** in code, comments or documentation.

## Conventions

- **`web/app.js`** is one IIFE with a single state object `S`. Pure logic goes
  in named `function` declarations, because the test harnesses slice the file
  between two function names: renaming one breaks its harness (0013).
- **Rust tests** live beside the code in `#[cfg(test)]` modules.
- **Rust modules** are one concern each, with a header comment that states the
  concern and any invariant. `src/git.rs` is the example to follow.
- **API payloads** are `serde` structs in `src/api.rs`, skipping empty fields.
- **Branches** are `<kind>/<what-it-does>`, kebab-case: `feature/` for a new
  capability, `fix/` for a correction, `deps/` for a dependency bump, `docs/`
  for documentation alone, and `archive/<area>/` for history kept but never
  merged. The kind comes first because that is what the branch list gets read
  for. CI checks the four that open a pull request; a `dependabot/` branch is
  exempt, since Dependabot names its own.
- **Commit subjects** say what the commit does, as a sentence and not a label:
  "Fetch a column's Snowflake lineage on click, behind a switch". The body is
  where the why goes, and what was rejected, which no diff can show.
- **Pull request titles** are that same sentence, because a pull request is
  squashed into one commit and its title becomes that commit's subject. So:
  three words or more, capitalised, no full stop at the end, no branch name in
  it, and short enough to read in a list. "Feature/breadcrumb bar" is the shape
  to avoid, and the only one that ever reached `main`.

## Where to read next

| If you are about to | Read |
| --- | --- |
| add a crate, or wonder why some parsing is hand-written | [0003](docs/decisions/0003-minimal-dependencies.md) |
| add a framework, a bundler, or upgrade CodeMirror | [0004](docs/decisions/0004-frontend-without-a-build-step.md) |
| touch a frontend change that seems to have no effect | [0005](docs/decisions/0005-embedded-in-release-disk-in-debug.md) |
| touch `src/git.rs` | [0007](docs/decisions/0007-git-through-the-cli.md) |
| touch `src/envs.rs`, or evaluate Jinja | [0009](docs/decisions/0009-env-resolution-by-scanner.md) |
| change the location table or the environment selector | [0010](docs/decisions/0010-moved-compares-parsed-with-built.md) |
| persist anything, or add a write endpoint | [0011](docs/decisions/0011-settings-outside-the-project.md) |
| add a field to a payload, a log line or a route | [0012](docs/decisions/0012-secrets-and-boundaries.md), [0017](docs/decisions/0017-the-profile-is-reachable.md) |
| change which `profiles.yml` is opened, or when it can be | [0049](docs/decisions/0049-the-profile-is-found-where-dbt-looks.md) |
| change what the profile route sends or accepts, or which keys are secret | [0054](docs/decisions/0054-the-profile-is-edited-without-its-secrets.md) |
| rename a function in `web/app.js`, or add a test | [0013](docs/decisions/0013-tests-without-a-toolchain.md) |
| add a route, change the port logic, or add a CORS header | [0015](docs/decisions/0015-the-browser-is-not-trusted.md), [0052](docs/decisions/0052-the-api-wants-the-key-this-launch-printed.md) |
| run a program found in the project, or ask a venv what it holds | [0053](docs/decisions/0053-a-project-is-read-before-anything-in-it-runs.md) |
| start a process from the server, or touch `src/sidecar.rs` | [0016](docs/decisions/0016-column-lineage-on-demand.md), [0031](docs/decisions/0031-column-lineage-is-picked-by-tool.md) |
| add a column lineage tool, or anything that only works on Snowflake | [0031](docs/decisions/0031-column-lineage-is-picked-by-tool.md) |
| show anything read from Snowflake besides lineage, link to Snowsight, or change when the script runs | [0048](docs/decisions/0048-the-query-history-is-read-live-and-never-kept.md), [0051](docs/decisions/0051-a-relation-opens-in-snowsight-by-its-name.md) |
| start collin, change when it runs, or bundle a lineage producer | [0042](docs/decisions/0042-collin-runs-on-demand-beside-the-binary.md), [0043](docs/decisions/0043-collin-is-updated-by-the-command-that-installs-it.md), [0047](docs/decisions/0047-collin-numbers-restart-at-0-1-0.md) |
| read a `.yml` file from the server, or reach for a YAML parser | [0018](docs/decisions/0018-project-vars-by-scanner.md) |
| link anything in `dbt_project.yml`, or read `model-paths` | [0036](docs/decisions/0036-a-config-key-links-to-its-folder.md), [0039](docs/decisions/0039-a-mapping-without-plus-is-a-folder.md) |
| show where a node's config comes from, or derive an fqn from a path | [0038](docs/decisions/0038-a-file-lists-the-keys-that-reach-it.md) |
| return any value derived from a `.env` file | [0019](docs/decisions/0019-a-resolved-value-may-be-shown.md) |
| make the server read files it was not asked for by name | [0020](docs/decisions/0020-search-never-opens-an-env-file.md) |
| show the structure inside a file, or parse SQL for it | [0022](docs/decisions/0022-the-outline-is-scanned-in-the-browser.md) |
| read a `.sql` file to work out anything at all | [0002](docs/decisions/0002-lineage-from-the-manifest.md), [0023](docs/decisions/0023-column-lineage-may-be-parsed.md) |
| resolve a dbt selector, or add a selector method | [0024](docs/decisions/0024-selectors-resolved-from-the-manifest.md) |
| change what the freshness badge claims, or compare the manifest with a commit | [0025](docs/decisions/0025-freshness-is-mtimes-not-commits.md) |
| export the graph, or change what an exported file carries | [0026](docs/decisions/0026-the-lineage-exports-as-one-html-file.md) |
| move a box on the lineage canvas, or read `n.depth` | [0027](docs/decisions/0027-the-canvas-lays-out-from-the-edges.md) |
| change where the canvas draws a test, or what `attached` means | [0034](docs/decisions/0034-a-test-hangs-under-its-model.md) |
| change how many tests show under a model, or what a capped canvas keeps | [0035](docs/decisions/0035-tests-fold-past-five.md) |
| group, label or colour the tests under a model, or add a field a test carries in a payload | [0040](docs/decisions/0040-tests-hang-grouped-by-generic.md) |
| change where opening a data test lands, or find a line in a properties file | [0041](docs/decisions/0041-a-test-opens-at-its-line.md) |
| handle a key inside the terminal, or change what Ctrl + C does there | [0044](docs/decisions/0044-the-terminal-copies-like-windows-terminal.md) |
| make the terminal type anything on its own, or change the environment its shell starts with | [0050](docs/decisions/0050-the-terminal-activates-the-venv-it-was-handed.md) |
| change the folder bands, or the order the folders go in | [0029](docs/decisions/0029-the-canvas-may-be-drawn-by-folder.md) |
| link something to a macro, or change which one a call reaches | [0028](docs/decisions/0028-a-macro-call-resolves-the-way-dbt-resolves-it.md) |
| add a keyboard shortcut, or change what Cmd/Ctrl + F does | [0030](docs/decisions/0030-find-is-the-apps-own.md), [0037](docs/decisions/0037-option-alone-cannot-carry-a-shortcut.md) |
| resolve a named selector, or change which tests a criterion brings along | [0032](docs/decisions/0032-named-selectors-from-the-manifest.md) |
| change how a selection or the lineage resolves, or touch the Jaffle Shop or Shopify fixture | [0033](docs/decisions/0033-selections-are-checked-against-dbt.md), [0056](docs/decisions/0056-selections-and-the-lineage-are-checked-on-shopify-too.md) |
| iterate a HashMap into anything a route returns, or change the order nodes are numbered in | [0055](docs/decisions/0055-the-same-manifest-answers-the-same-on-every-start.md) |
| set this up for someone, rather than change it | [README, Getting started](README.md#getting-started) |
| pick up the next piece of work | [docs/state.md](docs/state.md) |
| find out when something shipped, or in which version | [CHANGELOG.md](CHANGELOG.md) |
| release a version, or change how one is tagged and published | [0045](docs/decisions/0045-merging-a-version-bump-releases-it.md) |
| change the license or the copyright line, or bundle code under another license | [0046](docs/decisions/0046-free-to-use-not-to-resell.md) |

Every decision, with what was rejected each time, is indexed in
[docs/decisions/](docs/decisions/). The [README](README.md) is the user-facing
documentation: what the tool does and how to use it. Rationale lives here, never
in both.

## Layout

`src/manifest.rs` reads the manifest, `src/graph.rs` holds the compact graph,
`src/api.rs` serves HTTP and WebSocket, and the remaining modules take one
concern each: `envs`, `project`, `profiles`, `redact`, `settings`, `git`, `collin`, `collin_run`, `select`,
`selectors`, `sidecar`, `compiled`, `freshness`, `macros`, `venv`, `files`, `pty`. `build.rs` stamps
the binary with `git describe`, so two builds of one release can be told apart. `web/` is the
frontend, `web/vendor/` the vendored libraries, `tools/sf_lineage.py` the only
piece that talks to a warehouse. The README has the annotated version.
