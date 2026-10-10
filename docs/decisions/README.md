# Decisions

Why this codebase is shaped the way it is, and what was rejected along the way.
The README says what dbt-edith does for the person using it; these files say why
it is built this way, for whoever changes it next.

| | Decision | Read it before |
| --- | --- | --- |
| [0001](0001-one-self-contained-binary.md) | One self-contained binary, written in Rust | adding anything the user would have to install |
| [0002](0002-lineage-from-the-manifest.md) | Lineage comes from manifest.json, never from parsing SQL | reading `.sql` files to work out dependencies |
| [0003](0003-minimal-dependencies.md) | A deliberately small dependency list | adding a crate, or wondering why there is no regex |
| [0004](0004-frontend-without-a-build-step.md) | A frontend with no build step | reaching for a framework, a bundler or CodeMirror 6 |
| [0005](0005-embedded-in-release-disk-in-debug.md) | Disk in debug, embedded in release | a frontend change that seems to have no effect |
| [0006](0006-hand-written-graph-layout.md) | The graph is laid out and drawn by hand | adding a graph library, or editing `web/lineage.js` |
| [0007](0007-git-through-the-cli.md) | git runs as a subprocess, never as a library | touching `src/git.rs` |
| [0008](0008-column-lineage-from-an-offline-cache.md) | Column lineage arrives as a cache file | making the server talk to a warehouse |
| [0009](0009-env-resolution-by-scanner.md) | Environments are resolved by a scanner | touching `src/envs.rs`, or adding a template engine |
| [0010](0010-moved-compares-parsed-with-built.md) | "Moved" always compares parsed with built | changing the location table or the environment selector |
| [0011](0011-settings-outside-the-project.md) | Settings live outside the project | persisting anything |
| [0012](0012-secrets-and-boundaries.md) | What may leave the server, and what may not | adding a field to a payload, a log line, or a route |
| [0013](0013-tests-without-a-toolchain.md) | Tests that need nothing installed | renaming a function in `web/app.js` |
| [0014](0014-public-repository-hygiene.md) | Treat this repository as public | writing a fixture, an example or a commit message |
| [0015](0015-the-browser-is-not-trusted.md) | The browser is not trusted: Host and Origin are checked | adding a route, changing the port logic, or adding a CORS header |
| [0016](0016-column-lineage-on-demand.md) | Column lineage on demand, behind a switch | starting a process from the server, or changing what clicking a column does |
| [0017](0017-the-profile-is-reachable.md) | The dbt profile is reachable, and it alone | opening, reading or writing anything outside the project |
| [0018](0018-project-vars-by-scanner.md) | The `vars:` block is read by a scanner, not a YAML parser | reading a `.yml` file from the server, or reaching for a YAML parser |
| [0019](0019-a-resolved-value-may-be-shown.md) | A resolved value may reach the browser, under two guards | returning any value derived from a `.env` file |
| [0020](0020-search-never-opens-an-env-file.md) | A content search never opens a `.env` file | making the server read files it was not asked for by name |
| [0021](0021-one-cache-file-per-producer.md) | One column-lineage cache per producer, and the user picks | adding a source of column lineage, or changing where one writes |
| [0022](0022-the-outline-is-scanned-in-the-browser.md) | The breadcrumb's outline is scanned in the browser, and SQL has none | adding structure inside a file to the UI, or reaching for a YAML parser |
| [0023](0023-column-lineage-may-be-parsed.md) | Column lineage may be parsed out of SQL, and 0002 still holds | reading a `.sql` file to work out anything, or wondering how 0002 allows collin |
| [0024](0024-selectors-resolved-from-the-manifest.md) | A selector expression is resolved from the manifest, never by running dbt | changing how the Selection box matches, adding a selector method, or reaching for `dbt ls` |
| [0025](0025-freshness-is-mtimes-not-commits.md) | Manifest freshness is measured in file times, not in commits | changing what the freshness badge claims, or comparing the manifest against a commit |
| [0026](0026-the-lineage-exports-as-one-html-file.md) | The lineage exports as one HTML file, built in the browser | changing what an exported graph carries, or adding a way to export one |
| [0027](0027-the-canvas-lays-out-from-the-edges.md) | The canvas lays out from the edges, and depth stays a distance | moving a box on the lineage canvas, or reading `n.depth` |
| [0028](0028-a-macro-call-resolves-the-way-dbt-resolves-it.md) | A macro call resolves by name, the way dbt resolves it | linking anything to a macro, or changing which macro a call reaches |
| [0029](0029-the-canvas-may-be-drawn-by-folder.md) | The canvas may be drawn by folder, and an edge against the folders is dashed | changing which folder a node is filed under, or the order the folders go in |
| [0030](0030-find-is-the-apps-own.md) | Find is the app's own, and the shortcut list opens on ? | adding a keyboard shortcut, or changing what Cmd/Ctrl + F does |
| [0031](0031-column-lineage-is-picked-by-tool.md) | Column lineage is picked by tool, and Snowflake sits behind its own setting | adding a source of column lineage, or anything that only works on Snowflake |
| [0032](0032-named-selectors-from-the-manifest.md) | A named selector is read from the manifest and resolved whole | changing how `--selector` resolves, how a criterion brings its tests, or what the tests checkbox does in Selection |
| [0033](0033-selections-are-checked-against-dbt.md) | Selections are checked against dbt itself, on a copy of Jaffle Shop | changing how a selection resolves, touching `tests/fixtures/jaffle_shop/`, or a failing `dbt` job |
| [0034](0034-a-test-hangs-under-its-model.md) | A test hangs under the model whose YAML declares it | changing where the canvas draws a test, or what `attached` in a lineage payload means |
| [0035](0035-tests-fold-past-five.md) | A model's tests fold past five, and a capped canvas shares its room | changing how many tests show under a model, or which tests the canvas keeps when capped |
| [0036](0036-a-config-key-links-to-its-folder.md) | A config key in `dbt_project.yml` links to its folder on disk | linking anything in `dbt_project.yml`, or reading `model-paths` and its siblings |
| [0037](0037-option-alone-cannot-carry-a-shortcut.md) | Option alone cannot carry a shortcut, so the bound ones ask for Cmd | binding `Alt + <letter>` on a Mac, or changing what `keyCombo` does with `e.code` |
| [0038](0038-a-file-lists-the-keys-that-reach-it.md) | A file lists the keys of `dbt_project.yml` that reach it, by its fqn | showing where a node's config comes from, or deriving an fqn from a path |
| [0039](0039-a-mapping-without-plus-is-a-folder.md) | A key holding a mapping is a folder, whatever its name | deciding whether a key of `dbt_project.yml` is a config or a path, or touching `DBT_CONFIGS` |
| [0040](0040-tests-hang-grouped-by-generic.md) | A model's tests hang grouped by their generic | changing how the tests under a model are grouped, labelled or coloured, or what a payload says about a test |
| [0041](0041-a-test-opens-at-its-line.md) | A test opens at the line that declares it | changing where opening a data test lands, or what `host` in a payload means |
| [0042](0042-collin-runs-on-demand-beside-the-binary.md) | collin runs on demand, as an executable found beside the binary | starting collin from the server, changing when it runs, or bundling a column lineage producer |
| [0043](0043-collin-is-updated-by-the-command-that-installs-it.md) | collin is updated by the command that installs it, and its version decides how | raising the collin version dbt-edith accepts, or changing what the menu offers once collin is installed |
| [0044](0044-the-terminal-copies-like-windows-terminal.md) | The terminal copies and pastes the way Windows Terminal does | handling a key inside the terminal, or changing what Ctrl + C does there |
| [0045](0045-merging-a-version-bump-releases-it.md) | Merging a version bump releases it | changing how a release is started, what makes the tag, what a release's notes say, or the headings of CHANGELOG.md |
| [0048](0048-the-query-history-is-read-live-and-never-kept.md) | The query history is read live, twenty at a time, and never kept | showing anything read from Snowflake other than column lineage, linking to Snowsight, or changing when the Snowflake script starts or stops |
| [0049](0049-the-profile-is-found-where-dbt-looks.md) | The profile is found where dbt looks, whatever the warehouse | changing which `profiles.yml` dbt-edith opens, or what decides whether it can be opened |
| [0050](0050-the-terminal-activates-the-venv-it-was-handed.md) | The terminal activates the venv dbt-edith was started in | changing what the terminal types on its own, or which Python environment its shell starts with |
| [0051](0051-a-relation-opens-in-snowsight-by-its-name.md) | A relation opens in Snowsight by its name, on the session's account | linking a dbt object to Snowsight, or changing how a Snowsight address is spelled |
| [0052](0052-the-api-wants-the-key-this-launch-printed.md) | The API wants the key this launch printed | adding a route, changing what the printed link holds, or letting anything reach the API without the key |
| [0053](0053-a-project-is-read-before-anything-in-it-runs.md) | A project is read before anything in it runs | running a program found in the project, or changing how a venv's Python and dbt versions are learned |
| [0054](0054-the-profile-is-edited-without-its-secrets.md) | The profile is edited without its secrets | changing what `/api/profiles` sends or accepts, or which keys of a profile are secret |

## Keeping these honest

- **One decision per file**, numbered in order, dated, with what was rejected.
  The rejected options are the part worth writing: the decision itself is
  usually visible in the code, the discarded alternatives never are.
- **Never edit a decision to change its meaning.** Reversing one means adding a
  new file that says which number it supersedes, and marking the old one
  `Status: superseded by NNNN`. The reasoning then reads in order, including the
  mistakes.
- **Keep them short**, under about forty lines. A file too long to read fully is
  a file that gets skimmed.
- **Write only what can be checked**, in this repository or by running
  something. Measurements taken elsewhere are dated and given as orders of
  magnitude, not as current facts.
- **Anything that changes week to week** belongs in [../state.md](../state.md),
  which is rewritten rather than appended.
