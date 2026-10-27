# Working on dbt-edith

A browser IDE for dbt projects: editor, terminal and lineage, served from one
binary that contains its own frontend. It reads a dbt project, it never runs
dbt.

**The constraint behind most of this codebase:** it has to run on a locked-down
Windows VM where nothing can be installed. No runtime, no npm, no installer.
That is why there is no build step, no framework, and a short dependency list.

## Verify

```
./scripts/check.sh          # Rust tests, both audits, the Snowflake script, dbt itself, the MCP server, the tour, the depth boxes and Edith's menu in Chrome, browser tests, syntax
```

Run it before saying a change works. It is the only answer to "how do I check
this", and it needs nothing installed beyond Rust and macOS. The audit step
needs `cargo install cargo-audit --locked` and skips itself without it, like
the browser half without a JavaScript shell. A passing run that skipped
anything ends on `passed, but skipped:` and the parts it left out, never on
`all good`, so say which when reporting it. The second audit compares
`web/vendor/` with its upstream releases by hash, offline too, and asks OSV
about it when online. GitHub Actions runs the Rust tests and both audits on
every push and every Monday, never the browser half, which wants
macOS (0013). The Snowflake script is tested against a fake connector, so it
needs no warehouse, only Python 3.10 or later, and skips itself without one.
The selection engine and the lineage are compared with `dbt ls`, and the unused
config paths with dbt's warning (0103) and the Checks lists with what the
manifest says (0116), on `tests/fixtures/jaffle_shop` and on Fivetran's Shopify,
cloned at a pinned tag, which needs dbt-core and dbt-duckdb: set `DBT_PYTHON` to
a Python that has them, or `DBT` to a dbt executable. It skips itself without
either, and skips Shopify without git or the network; CI runs it on every model
for two versions of dbt-core (0033, 0056). The MCP server is driven over its
stdio on the same Jaffle Shop by `scripts/walk_mcp.py` and compared with the
command line's answers; it needs the same dbt, skips itself without it, and CI
runs it (0110).
The guided tour is walked step by step in headless Chrome on the same Jaffle
Shop by `scripts/walk_tour.py`, which needs that dbt and Chrome and skips
itself without either; CI does not run it, having no Chrome to drive (0074).
`scripts/walk_depth.py` drives the lineage's depth boxes there the same way,
`scripts/walk_menu.py` Edith's menu under a real pointer, and
`scripts/walk_export.py` an export of the lineage with the Halloween look on.
[docs/testing.md](docs/testing.md) lists each part and how to run it alone.
Updating a vendored library means replacing its files and changing its
version and each file's SHA-256 in `scripts/audit_vendored.py`, and its
version in `THIRD_PARTY_NOTICES.md`: the audit fails when they disagree. A
hash is the one of the file in the upstream npm tarball of that version, which
the audit fetches to check, never one taken on the copied file.

## Run

```
cargo run -- /path/to/dbt-project --port 4399 --no-open    # debug: serves web/ from disk
cargo build --release --locked                             # release: embeds web/
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
  relation's Snowsight menu, or reads the last builds, with Snowflake's
  features on for the project, under their own credentials (0002, 0016, 0031,
  0048, 0051, 0117). collin, which
  parses the compiled SQL and reaches nothing, is started only once Collin is
  picked, a column is clicked or Regenerate pressed, never at startup (0042),
  and from the command line only when `--column` is asked with `--run-collin`
  (0099), typed or in the MCP tool's line (0110).
- **Never return or log a `.env` value**, beyond the three things allowed to
  leave: resolved locations, `DBT_TARGET` (0012, 0017), and a resolved
  `env_var()` in the hover card, which passes two guards first (0019).
- **Nothing outside the project is read or written**, except the dbt profile,
  found where dbt looks for it, through its own route (0017, 0049), and that
  route never sends its passwords, tokens or keys (0054). The clone route reads
  it too, and sends only the active target's name, database and schema (0058).
  The job folders the user names are read as well, in the settings or in the
  project's own `.dbt-edith/jobs.yml`, which the command line and MCP read too
  and which is written on the person's click only (0115). Their `.yml` and
  `.yaml` files only, and only what was parsed from them leaves, never a
  `--vars` value (0063). To pick one, the sources form lists any folder, by the
  names of its folders and YAML files only (0070). Whether this binary was installed by
  cargo or built in a clone is told from which files exist beside it, never
  from inside them (0068), and whether another was installed in its place from
  the time its own file was written (0095). A path the manifest records for a
  compiled file is
  held to the target directory, as a path the browser sends is to the project.
- **Treat this repository as public.** Fixtures and examples are invented, never
  taken from a real project (0014).
- **The browser is not trusted.** Every route sits behind the Host and Origin
  guard in `src/api.rs`, and no CORS header is ever added (0015). Every
  `/api/` and `/ws/` route also wants the key the launch printed (0052).
- **Nothing in the project runs on its own.** Learn about a venv from its files,
  never by running it, and never run one git tracks (0053), nor a collin git
  tracks, beside the binary or on the PATH (0094). A link the repository
  commits is held to the project, a `profiles.yml` included; one the user made
  outside it, a dotfiles one, is followed and kept (0094).
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
- **What a feature does is written in its page under
  [docs/features/](docs/features/)**, changed in the same pull request as the
  feature. The README is not that place: it is what a developer reads before
  installing dbt-edith, a line per area, how to install and run it, the options
  and the common fixes, and it changes only when one of those does. Nor is
  either page a record of what went wrong on the way: a trap a change ran into
  goes in its commit body, or in a decision when it shapes the code. A change
  that alters what one of the README's pictures shows retakes them all with
  `scripts/take_shots.py` (0082).
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
  it. "Feature/breadcrumb bar" is the shape to avoid, and the only one that
  ever reached `main`. The title is also what someone a release behind reads
  in Edith's menu, one line per release (0079), so:
  - **50 characters at most**, which that line holds;
  - **one idea**, the one someone using it would notice; the rest goes in the
    CHANGELOG entry and the body, not after a comma;
  - **in their words**: what they see (the lineage, the Catalog, the terminal,
    the status bar), not the mechanism behind it;
  - **a verb first**, in the imperative: "Show the incremental strategy on the
    lineage", not "Incremental strategy on the lineage and its key in the
    Catalog".

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
| change which `profiles.yml` is opened, or when it can be, or how it is saved | [0049](docs/decisions/0049-the-profile-is-found-where-dbt-looks.md), [0094](docs/decisions/0094-what-a-repository-brings-is-held-to-it-what-you-bring-is-followed.md) |
| change what the profile route sends or accepts, or which keys are secret | [0054](docs/decisions/0054-the-profile-is-edited-without-its-secrets.md), [0090](docs/decisions/0090-a-credential-is-hidden-by-its-value-as-well-as-its-name.md) |
| change what a tab does when its file changes on disk, or what a save checks first | [0067](docs/decisions/0067-an-open-tab-follows-its-file-on-disk.md) |
| rename a function in `web/app.js`, or add a test | [0013](docs/decisions/0013-tests-without-a-toolchain.md) |
| add a route, change the port logic, or add a CORS header | [0015](docs/decisions/0015-the-browser-is-not-trusted.md), [0052](docs/decisions/0052-the-api-wants-the-key-this-launch-printed.md) |
| run a program found in the project, or ask a venv what it holds | [0053](docs/decisions/0053-a-project-is-read-before-anything-in-it-runs.md), [0094](docs/decisions/0094-what-a-repository-brings-is-held-to-it-what-you-bring-is-followed.md) |
| start a process from the server, or touch `src/sidecar.rs` | [0016](docs/decisions/0016-column-lineage-on-demand.md), [0031](docs/decisions/0031-column-lineage-is-picked-by-tool.md) |
| add a column lineage tool, or anything that only works on Snowflake | [0031](docs/decisions/0031-column-lineage-is-picked-by-tool.md) |
| show anything read from Snowflake besides lineage, link to Snowsight, or change when the script runs | [0048](docs/decisions/0048-the-query-history-is-read-live-and-never-kept.md), [0051](docs/decisions/0051-a-relation-opens-in-snowsight-by-its-name.md) |
| show when a model last ran per environment, change what the builds read selects or sends, or put a status on a lineage box | [0117](docs/decisions/0117-the-last-build-per-environment-is-read-from-dbt-artifacts.md), [0048](docs/decisions/0048-the-query-history-is-read-live-and-never-kept.md), [0113](docs/decisions/0113-the-card-resolves-a-location-per-env-file.md) |
| start collin, change when it runs or where it is looked for, or bundle a lineage producer | [0042](docs/decisions/0042-collin-runs-on-demand-beside-the-binary.md), [0099](docs/decisions/0099-a-column-may-run-collin-first-when-asked.md), [0094](docs/decisions/0094-what-a-repository-brings-is-held-to-it-what-you-bring-is-followed.md), [0043](docs/decisions/0043-collin-is-updated-by-the-command-that-installs-it.md), [0047](docs/decisions/0047-collin-numbers-restart-at-0-1-0.md), [0060](docs/decisions/0060-collin-versions-sit-side-by-side-and-one-is-picked.md), [0098](docs/decisions/0098-collin-is-wanted-at-0-8-0-and-a-version-installed-by-tag-is-never-called-older.md), [0109](docs/decisions/0109-collin-is-given-the-project-only-for-a-manifest-lying-in-it.md) |
| change collin's report, its news on the Reports button, or what a run leaves of the report | [0059](docs/decisions/0059-collins-report-is-filed-here-from-the-lists-every-collin-writes.md), [0062](docs/decisions/0062-the-report-page-shows-collins-own-findings-when-it-writes-them.md) |
| show a warning dbt gives as it parses, or judge whether a key of `dbt_project.yml` applies to anything | [0103](docs/decisions/0103-a-config-path-is-unused-when-no-fqn-of-its-kind-begins-it.md), [0039](docs/decisions/0039-a-mapping-without-plus-is-a-folder.md) |
| add a report, or change what opens or closes the Reports panel | [0064](docs/decisions/0064-project-wide-reports-open-in-a-panel-over-the-editor-and-the-dock.md), [0070](docs/decisions/0070-a-source-is-picked-from-folders-the-server-lists.md) |
| add a panel, or move a view between the dock and a panel | [0091](docs/decisions/0091-what-speaks-of-the-whole-project-opens-in-a-panel.md) |
| add a setting, or change what the gear or a status bar pill opens | [0100](docs/decisions/0100-the-settings-open-in-a-panel-of-their-own.md), [0091](docs/decisions/0091-what-speaks-of-the-whole-project-opens-in-a-panel.md) |
| hide the Reports button, or change what it says with no report | [0075](docs/decisions/0075-the-reports-button-stays-in-view-dimmed-while-it-has-nothing.md) |
| give a report news on the Reports button, or change what its chips say | [0066](docs/decisions/0066-each-report-s-news-is-a-short-chip-on-the-reports-button.md) |
| read `run_results.json`, or change what the Last run section says or sends | [0065](docs/decisions/0065-the-last-run-is-read-from-run-results-as-dbt-left-it.md), [0012](docs/decisions/0012-secrets-and-boundaries.md) |
| change what a collin run keeps under `target/collin/`, or the comparison of two versions | [0061](docs/decisions/0061-each-collin-version-keeps-its-last-run-to-compare.md) |
| read a `.yml` file from the server, or reach for a YAML parser | [0018](docs/decisions/0018-project-vars-by-scanner.md), [0076](docs/decisions/0076-an-incremental-strategy-is-read-from-the-macros-dbt-runs.md), [0103](docs/decisions/0103-a-config-path-is-unused-when-no-fqn-of-its-kind-begins-it.md) |
| read the orchestrator's jobs, change what leaves them, or resolve a job's dbt command | [0063](docs/decisions/0063-jobs-are-read-from-folders-you-name.md) |
| change where the job sources are kept, what `.dbt-edith/jobs.yml` holds or who writes it, or what the command line and MCP read of the jobs | [0115](docs/decisions/0115-job-sources-may-live-in-the-project.md), [0063](docs/decisions/0063-jobs-are-read-from-folders-you-name.md) |
| change which jobs count as scheduled, or what the job coverage calls unscheduled | [0077](docs/decisions/0077-a-start-no-file-says-is-neither-scheduled-nor-unscheduled.md), [0066](docs/decisions/0066-each-report-s-news-is-a-short-chip-on-the-reports-button.md) |
| judge a job's builds, read a cron or a time zone, or give a job source a key | [0119](docs/decisions/0119-a-jobs-builds-are-judged-against-its-own-clock-in-its-sources-zone.md), [0117](docs/decisions/0117-the-last-build-per-environment-is-read-from-dbt-artifacts.md), [0063](docs/decisions/0063-jobs-are-read-from-folders-you-name.md), [0115](docs/decisions/0115-job-sources-may-live-in-the-project.md) |
| list a folder outside the project, or change what the sources form's picker shows or writes | [0070](docs/decisions/0070-a-source-is-picked-from-folders-the-server-lists.md) |
| link anything in `dbt_project.yml`, or read `model-paths` | [0036](docs/decisions/0036-a-config-key-links-to-its-folder.md), [0039](docs/decisions/0039-a-mapping-without-plus-is-a-folder.md) |
| show where a node's config comes from, or derive an fqn from a path | [0038](docs/decisions/0038-a-file-lists-the-keys-that-reach-it.md) |
| return any value derived from a `.env` file | [0019](docs/decisions/0019-a-resolved-value-may-be-shown.md), [0090](docs/decisions/0090-a-credential-is-hidden-by-its-value-as-well-as-its-name.md) |
| make the server read files it was not asked for by name | [0020](docs/decisions/0020-search-never-opens-an-env-file.md) |
| show the structure inside a file, or parse SQL for it | [0022](docs/decisions/0022-the-outline-is-scanned-in-the-browser.md) |
| read a `.sql` file to work out anything at all | [0002](docs/decisions/0002-lineage-from-the-manifest.md), [0023](docs/decisions/0023-column-lineage-may-be-parsed.md) |
| resolve a dbt selector, or add a selector method | [0024](docs/decisions/0024-selectors-resolved-from-the-manifest.md) |
| change what the freshness badge claims, or compare the manifest with a commit | [0025](docs/decisions/0025-freshness-is-mtimes-not-commits.md) |
| export the graph, or change what an exported file carries | [0026](docs/decisions/0026-the-lineage-exports-as-one-html-file.md) |
| move a box on the lineage canvas, or read `n.depth` | [0027](docs/decisions/0027-the-canvas-lays-out-from-the-edges.md) |
| change where the canvas draws a test, or what `attached` means | [0034](docs/decisions/0034-a-test-hangs-under-its-model.md) |
| change how many tests show under a model, or what a capped canvas keeps | [0035](docs/decisions/0035-tests-fold-past-five.md), [0087](docs/decisions/0087-a-capped-selection-draws-what-is-nearest-its-start.md) |
| change how many nodes a selection draws, or where a capped one starts | [0087](docs/decisions/0087-a-capped-selection-draws-what-is-nearest-its-start.md) |
| change where a selection is drawn, or what a click on a box does in its panel | [0092](docs/decisions/0092-custom-selection-opens-in-a-panel.md) |
| group, label or colour the tests under a model, or add a field a test carries in a payload | [0040](docs/decisions/0040-tests-hang-grouped-by-generic.md) |
| change where opening a data test lands, or find a line in a properties file | [0041](docs/decisions/0041-a-test-opens-at-its-line.md) |
| handle a key inside the terminal, or change what Ctrl + C does there | [0044](docs/decisions/0044-the-terminal-copies-like-windows-terminal.md), [0073](docs/decisions/0073-the-terminal-sends-the-line-keys-the-shell-reads-everywhere.md) |
| make the terminal type anything on its own, or change the environment its shell starts with | [0050](docs/decisions/0050-the-terminal-activates-the-venv-it-was-handed.md), [0069](docs/decisions/0069-the-status-bar-names-the-venv-the-terminal-reports.md) |
| make a button type a command in the terminal, or press Enter there | [0071](docs/decisions/0071-a-click-types-a-dbt-command-and-leaves-enter-to-you.md), [0025](docs/decisions/0025-freshness-is-mtimes-not-commits.md) |
| change what the status bar says about the venv, or what a click on it types | [0069](docs/decisions/0069-the-status-bar-names-the-venv-the-terminal-reports.md), [0072](docs/decisions/0072-a-click-on-an-inactive-venv-types-its-activate-line.md), [0100](docs/decisions/0100-the-settings-open-in-a-panel-of-their-own.md), [0053](docs/decisions/0053-a-project-is-read-before-anything-in-it-runs.md) |
| change the folder bands, how deep they go, or the order the folders go in | [0029](docs/decisions/0029-the-canvas-may-be-drawn-by-folder.md), [0080](docs/decisions/0080-a-band-may-be-cut-by-its-sub-folders.md) |
| link something to a macro, or change which one a call reaches | [0028](docs/decisions/0028-a-macro-call-resolves-the-way-dbt-resolves-it.md) |
| change what `Cmd/Ctrl + K` lists, how it ranks a macro, or where opening one lands | [0084](docs/decisions/0084-a-macro-is-searched-by-its-name-and-opens-on-its-line.md) |
| say who calls a macro, or that nothing does, or read `depends_on.macros` | [0105](docs/decisions/0105-a-macro-is-counted-by-the-nodes-reaching-it-and-never-called-unused.md) |
| add a rule to the Checks section of Reports, or read a source's `freshness` | [0116](docs/decisions/0116-the-tools-own-checks-are-listed-apart-from-dbts-warnings.md), [0103](docs/decisions/0103-a-config-path-is-unused-when-no-fqn-of-its-kind-begins-it.md) |
| show what an incremental model does on a run, follow `adapter.dispatch`, or keep a macro's body | [0076](docs/decisions/0076-an-incremental-strategy-is-read-from-the-macros-dbt-runs.md) |
| hide part of a column graph by role, or make the column walk skip an edge | [0083](docs/decisions/0083-a-role-stops-the-column-walk-downstream-on-the-server.md) |
| change what a column box's badge says, or read a field of collin's cache other than its edges | [0096](docs/decisions/0096-a-column-a-literal-sets-is-generated-not-raw.md), [0097](docs/decisions/0097-an-indirect-edge-is-hidden-by-default-drawn-dashed-when-shown.md) |
| change how an edge of a role is drawn or walked, what the legend groups, what the indirect button hides, or how many boxes a column's canvas draws | [0097](docs/decisions/0097-an-indirect-edge-is-hidden-by-default-drawn-dashed-when-shown.md), [0118](docs/decisions/0118-an-indirect-edge-ends-on-its-models-box-and-names-its-clause.md) |
| change what the stop at menu or a column's Lineage cell counts | [0089](docs/decisions/0089-a-column-count-says-what-it-counts.md), [0083](docs/decisions/0083-a-role-stops-the-column-walk-downstream-on-the-server.md) |
| add a keyboard shortcut, or change what Cmd/Ctrl + F does | [0030](docs/decisions/0030-find-is-the-apps-own.md), [0037](docs/decisions/0037-option-alone-cannot-carry-a-shortcut.md) |
| change what find on the lineage matches, the order it steps in, or where it moves the view | [0081](docs/decisions/0081-find-on-the-lineage-centres-the-box-it-finds.md), [0035](docs/decisions/0035-tests-fold-past-five.md) |
| resolve a named selector, or change which tests a criterion brings along | [0032](docs/decisions/0032-named-selectors-from-the-manifest.md) |
| change what a node is named, or what a versioned model's bare name reaches | [0104](docs/decisions/0104-a-versioned-model-is-named-by-its-version.md), [0024](docs/decisions/0024-selectors-resolved-from-the-manifest.md) |
| change how a selection or the lineage resolves, or touch the Jaffle Shop or Shopify fixture | [0033](docs/decisions/0033-selections-are-checked-against-dbt.md), [0056](docs/decisions/0056-selections-and-the-lineage-are-checked-on-shopify-too.md) |
| iterate a HashMap into anything a route returns, or change the order nodes are numbered in | [0055](docs/decisions/0055-the-same-manifest-answers-the-same-on-every-start.md) |
| change what `--select` prints or exits with, or add a flag that answers without serving | [0057](docs/decisions/0057-a-selection-is-answered-on-the-command-line.md) |
| change what `--column` reads, or let the command line write lineage | [0088](docs/decisions/0088-a-column-is-answered-on-the-command-line-from-the-cache-already-written.md), [0099](docs/decisions/0099-a-column-may-run-collin-first-when-asked.md) |
| answer over MCP, change the `edith` tool, what its line accepts, or what `--mcp` reads and writes | [0110](docs/decisions/0110-an-agent-may-ask-over-mcp-through-one-tool.md), [0057](docs/decisions/0057-a-selection-is-answered-on-the-command-line.md), [0099](docs/decisions/0099-a-column-may-run-collin-first-when-asked.md) |
| add a key to `--count-by`, or let another mode count instead of list | [0112](docs/decisions/0112-a-selection-may-be-counted-instead-of-listed.md), [0057](docs/decisions/0057-a-selection-is-answered-on-the-command-line.md) |
| change what `--describe` or `--overview` prints, what `--output compact` leaves out, or let the command line read a file of the project | [0102](docs/decisions/0102-a-node-and-the-project-are-described-on-the-command-line.md) |
| print a location derived from a `.env` file on the command line or over MCP, or change the card's per-file lines | [0113](docs/decisions/0113-the-card-resolves-a-location-per-env-file.md), [0012](docs/decisions/0012-secrets-and-boundaries.md), [0090](docs/decisions/0090-a-credential-is-hidden-by-its-value-as-well-as-its-name.md) |
| change the agent skill, what `--skill` prints, or let the binary write anything for an agent client | [0114](docs/decisions/0114-a-skill-is-shipped-and-printed-never-installed.md), [0110](docs/decisions/0110-an-agent-may-ask-over-mcp-through-one-tool.md) |
| write SQL for the user to run, change what a clone copies or where it lands, or read anything else out of the profile | [0058](docs/decisions/0058-a-model-is-cloned-into-your-schema-by-statements-you-run.md) |
| set this up for someone, rather than change it | [README, Getting started](README.md#getting-started) |
| build the Windows `.exe`, or touch `scripts/build_windows.sh` | [docs/windows.md](docs/windows.md) |
| change what the `.exe`'s properties say, or what a Windows build does without `windres` | [0093](docs/decisions/0093-the-exe-names-its-publisher-when-windres-is-there.md) |
| change what a feature does, or check what it does now | its page in [docs/features/](docs/features/) |
| add a picture to the README, or retake them | [0082](docs/decisions/0082-the-readmes-pictures-are-committed-and-retaken-by-a-script.md) |
| pick up the next piece of work | [docs/state.md](docs/state.md) |
| find out when something shipped, or in which version | [CHANGELOG.md](CHANGELOG.md) |
| release a version, or change how one is tagged and published | [0045](docs/decisions/0045-merging-a-version-bump-releases-it.md) |
| make dbt-edith reach the network on its own, or change the update offer | [0068](docs/decisions/0068-a-newer-dbt-edith-is-looked-for-once-a-day-through-git.md) |
| change when a newer release is looked for, or what Edith's menu shows | [0079](docs/decisions/0079-an-update-is-looked-for-hourly-through-the-paris-night-and-on-demand.md), [0068](docs/decisions/0068-a-newer-dbt-edith-is-looked-for-once-a-day-through-git.md) |
| change what the update types, or what the menu offers once it has run | [0095](docs/decisions/0095-the-update-says-when-it-is-installed-and-brings-its-tags.md), [0111](docs/decisions/0111-the-update-ends-on-a-line-and-a-card-and-a-failed-build-is-a-failure.md), [0120](docs/decisions/0120-the-update-empties-the-clones-target-once-installed.md) |
| open a menu under the pointer, or change what closes or holds one | [0086](docs/decisions/0086-edith-s-menu-opens-under-the-pointer.md) |
| change the Halloween look, its switch or its sounds | [0106](docs/decisions/0106-the-halloween-look-is-kept-per-machine-and-never-exported.md), [docs/features/look.md](docs/features/look.md) |
| change what the Halloween look draws on a lineage box, how many boxes, or what an export measures | [0107](docs/decisions/0107-the-box-art-is-drawn-under-a-count-and-never-exported.md), [0106](docs/decisions/0106-the-halloween-look-is-kept-per-machine-and-never-exported.md), [docs/features/look.md](docs/features/look.md) |
| change the dewdrops on the edges under the Halloween look, or the web on the selected box | [0108](docs/decisions/0108-the-silk-has-dewdrops-and-the-selected-box-grows-a-web.md), [0107](docs/decisions/0107-the-box-art-is-drawn-under-a-count-and-never-exported.md) |
| add a step to the guided tour, or change when it is offered or what it touches | [0074](docs/decisions/0074-the-tour-is-offered-once-per-machine-and-runs-nothing.md), [0078](docs/decisions/0078-the-tour-is-offered-once-there-is-a-model-to-show.md) |
| change the license or the copyright line, or bundle code under another license | [0046](docs/decisions/0046-free-to-use-not-to-resell.md) |

Every decision, with what was rejected each time, is indexed in
[docs/decisions/](docs/decisions/). [docs/features/](docs/features/) says what
each part of the tool does, in full, and the [README](README.md) what a
developer needs to install and run it. Rationale lives in the decisions, never
in both.

## Layout

```
src/main.rs           the command line, startup, and what it prints
src/manifest.rs       manifest.json -> raw structs (only the fields the UI needs)
src/graph.rs          compact node vector, adjacency, search, lineage BFS, selection layering
src/api.rs            HTTP + WebSocket handlers
src/collin.rs         the column lineage cache, merged like catalog.json
src/collin_run.rs     collin, found beside the binary or on PATH and run only on request
src/collin_report.rs  collin's report read whole, filed by kind for the report page
src/collin_compare.rs what two collin versions read in one project, side by side
src/run_results.rs    the last dbt command's results, read from run_results.json as dbt left it
src/select.rs         dbt selector expressions, parsed and resolved against the graph
src/selectors.rs      the named selectors of selectors.yml, as the manifest recorded them
src/ls.rs             --select and --selector: one selection printed, and no server
src/describe.rs       --describe and --overview: a node's card and the project's shape printed, and no server
src/job.rs            --job: one DAG or dbt Cloud job printed step by step, with the nodes each runs, and no server
src/mcp.rs            --mcp: the stdio JSON-RPC loop and its one tool, over the same answers
src/mcp_line.rs       the tool's command line: split, parsed by clap, accepted by an allow-list of flags
src/mcp_graphs.rs     the manifest and the base graph the server keeps, reloaded when their files change
src/skill.rs          --skill: the agent skill of docs/agent-skill/ printed for this version, and held to the flags this binary accepts
src/jobs.rs           the orchestrator's jobs, read from the folders named (in the settings or `.dbt-edith/jobs.yml`), and the nodes each step acts on
src/command.rs        a dbt command line, read into what it runs and what it selects
src/yaml.rs           a subset of YAML, anchors and merges included, read into a tree
src/clone.rs          which nodes a clone into your schema copies: the model, its direct parents, through ephemerals
src/sidecar.rs        the Snowflake script: started once Snowflake is picked, or the history or the last builds read, one request at a time
src/compiled.rs       the compiled/ and run/ SQL under target/, and how fresh each is
src/macros.rs         macros from the manifest, and which one a call in the editor reaches
src/macro_use.rs      which nodes reach each macro, through the macros calling it, and why one no node reaches may still run
src/incremental.rs    which incremental strategy dbt runs for a model, read from those macros
src/freshness.rs      whether manifest.json still matches the files dbt would parse
src/envs.rs           .env parsing and location resolution per environment
src/project.rs        the vars: and dispatch: blocks of dbt_project.yml, read by hand
src/unused_paths.rs   the config paths of dbt_project.yml no fqn of the manifest begins, as dbt warns of them
src/checks.rs         the tool's own checks: sources without freshness, models with no test or description, sources no node reads
src/profiles.rs       where dbt looks for profiles.yml, the one file outside the project opened, and its active target
src/redact.rs         the profile's secrets, replaced before it is sent and put back when it is saved
src/git.rs            working tree status and the git commands the UI can run
src/update.rs         whether a newer release is out, asked through git hourly through the Paris night or on demand, and the command that installs it
src/tour.rs           the guided tour's model, and whether it was offered on this machine
src/look.rs           the Halloween look's choices (look, sound, apparitions, box style), kept per machine in look.json
src/settings.rs       per-project settings, kept outside the project
src/venv.rs           which Python environment is in play
src/files.rs          filesystem access, confined to the project root
src/pty.rs            one PTY per terminal connection
src/program.rs        a program started by its name, found on the PATH and never beside the binary
build.rs              stamps the binary with git describe, so two builds of one release can be told apart, and names its publisher in a Windows .exe
web/                  UI: no framework, CodeMirror 5 and xterm.js are vendored
web/app.js            the shell, including the document outline behind the breadcrumbs and the exported page
web/lineage.js        layered graph layout and SVG renderer, model, column and selection modes, and the snapshot an export is made of
web/vendor/           CodeMirror, xterm, the merge addon and diff-match-patch
tools/                sf_lineage.py, the only piece that talks to a warehouse
```
