# Changelog

What shipped, and when. This file is appended to, never rewritten, which is
what makes it the only place that keeps the order:
[docs/state.md](docs/state.md) says where the work stands today and is rewritten
as it moves, [docs/features/](docs/features/) says what the tool does now, and
[docs/decisions/](docs/decisions/) says why it is built this way. A feature
belongs in docs/features/ too; only here does it carry a version.

One entry per change someone using dbt-edith would notice, written as a
sentence saying what it does, the way a commit subject is. A number in brackets
points at the decision behind it. A version's date is the day its bump merged,
which tags and releases it with its section here as the notes (0045): the
newest dated heading has to be the version `Cargo.toml` names.

## 1.59.0 - 2026-10-28

- Show the dependencies the jobs do not order in a Job order section of
  Reports, beside the job coverage: the pairs `--job-order` lists, in their
  three groups, each model a link that opens it with the Jobs tab, never
  lighting the Reports button (0126). `--job-order --output json` now also
  carries `checked`, the steps that were looked at, and the text of an empty
  answer says `no pair in what was read` while a source was not read.

## 1.58.0 - 2026-10-28

- Add `--job-order`, which lists the dbt dependencies between the steps of the
  jobs that start on their own which the job files do not order: a step that
  builds a model whose parent another step builds, with the views and
  ephemerals between them named, in text and JSON and over MCP. It tells apart
  what nothing orders, what only the clock may, and what the files order the
  other way, and notes a sensor that no task depends on (0125).

## 1.57.0 - 2026-10-28

- Print under each step of `--job` what it runs after and before, the sensors
  that await it, and the assets it updates with the DAGs they start, and under
  a DAG its sensors by id with what they wait for, in the text and the JSON
  (0124). A sensor on several tasks (`external_task_ids`) or on a task group
  (`external_task_group_id`) is read as such and no longer as one on the whole
  run, and a sensor written without an id no longer shows a dangling `›` in the
  Jobs tab.

## 1.56.0 - 2026-10-27

- Attach to every release a Windows `dbt-edith.exe`, and `dbt-edith-arm64.exe`
  for Windows on ARM, each with its SHA-256 and built by CI on Windows, so a
  Windows machine downloads the file instead of building it (0123).
- Offer on Windows a button that downloads the release's `.exe` of this build's
  architecture, checks its SHA-256 and swaps it in, first in Edith's menu and
  the settings, beside the cargo update for a binary cargo installed, which
  compiles on this machine. A binary run from its clone's `target` folder is
  offered neither. The running one is put back if the swap fails.

## 1.55.0 - 2026-10-27

- Keep a repository's own configuration from naming a program edith runs:
  every git call switches the file-system monitor off, what runs on a timer
  switches hooks off too, and a repository whose configuration holds a key
  outside a list of data-only ones, a `credential.helper` or a `filter.*` for
  one, is left alone, the Git panel and the freshness card saying so and
  naming the key (0121). The timed fetch, and a checkout, which may fetch
  what a partial clone lacks, speak https, http and ssh only: a remote on a
  local path, a `git://` one or one behind a remote helper (a `codecommit::`
  URL) is no longer refreshed by the timer.
- Send the launch key in a header from the page instead of keeping it in a
  cookie, so no other server on the machine is ever sent it (0122). The
  terminal's socket takes it in its address.
- List only the `.env` files that are inside the project, so a link to a file
  elsewhere is no longer read.

## 1.54.1 - 2026-10-27

- Empty the clone's build folder once an update has installed, so updates no
  longer fill the disk (0120). The update to this release is typed by the
  version before it, which has no clean, so that first one leaves the folder
  for `cargo clean --release` by hand.

## 1.54.0 - 2026-10-27

- Show each job's last builds in a Builds by job section of Reports, judged
  against the job's own schedule in the time zone its Airflow source names
  (0119). A `.dbt-edith/jobs.yml` that sets a `timezone` is refused whole by
  1.53.0 and earlier, so everyone sharing it needs 1.54.0.

## 1.53.0 - 2026-10-26

- Draw a column's indirect edges, the ones a `WHERE`, a `JOIN` or a `QUALIFY`
  makes, as one dashed edge to a box for the whole model, named by its clause
  and in a hue of its own, where they were never asked of collin and so never
  seen. Regenerate once after updating, since the page now always asks collin
  for them (0118).
- Say on a column's card which models it keeps the rows of, beside which
  columns keep its own.
- Draw up to 1000 boxes for a column, up from 200, with a button to draw up to
  3000 for that column alone.
- Leave the indirect roles out of the stop at menu, whether the indirect
  button is on or not.
- Compare two kept collin runs without their indirect edges when only one of
  them holds any, and say so.

## 1.52.0 - 2026-10-26

- Show when each model was last built in each environment, and how that run
  ended, in the Catalog, as a dot on the lineage and in Reports, read on a
  click from the tables of the dbt_artifacts package (0117). An environment
  whose file sets DBT_TARGET keeps that target's runs and names the others the
  table holds.

## 1.51.0 - 2026-10-22

- List the tool's own checks in a Checks section of Reports, apart from dbt's
  warnings: sources without freshness, models with no test, models with no
  description and sources no node reads, for the project's own enabled nodes
  (0116).

## 1.50.0 - 2026-10-22

- Show how the last run ended on the dot of its section in Reports, red, amber
  or green, in the list and in its head, with the outcome in words on hover and
  for a screen reader, and the tool's colour for a run that ran nothing or
  only skipped.
- List the slowest nodes of the last run open, the first five with a button
  for the rest.
- Leave the groups of the job coverage with nothing in them undrawn.

## 1.49.2 - 2026-10-22

- Teach the shipped skill the limits of a big project and the jobs questions
  (0114).

## 1.49.1 - 2026-10-22

- List the accepted values in an MCP refusal of an invalid `--count-by`,
  `--output` or `--column-tool`, as the command line does.

## 1.49.0 - 2026-10-21

- Keep the job sources in the project's `.dbt-edith/jobs.yml`, which the Jobs
  settings write on request and the command line and MCP read, and ask which
  jobs run a node with `--describe`, a job's steps with `--job`, and the jobs
  of the project with `--overview` (0115).

## 1.48.0 - 2026-10-21

- Print an agent skill for dbt-edith with `--skill`, shipped as a file too, so
  a client loads it and asks edith before reading SQL (0114).

## 1.47.0 - 2026-10-21

- Show the location a node's config gives with each `.env` file on
  `--describe`'s card and JSON, and as the project writes it (0113).

## 1.46.0 - 2026-10-21

- Count a selection by kind, materialization, source, folder, tag or package
  with `--count-by`, on the command line and over MCP (0112).

## 1.45.0 - 2026-10-21

- Say it in a card in the middle of the page once the update is typed into the
  terminal, and again when dbt-edith is installed and waits for a restart, in
  place of the toast and the menu alone (0111).
- End the update command on a line saying to stop dbt-edith and start it
  again, printed only when the install worked (0111).
- Fail the update command in Git Bash on Windows when the build failed, the
  binary put back, where it used to end as if it had worked (0111).

## 1.44.0 - 2026-10-21

- Answer an agent over MCP with `dbt-edith --mcp`, through one tool that takes
  the command line's flags as a line, started only by a client configured for
  it (0110).
- Let that line carry `--run-collin`, stopped when the client cancels the call
  or closes its input (0110).

## 1.43.1 - 2026-10-20

- Offer "Update collin" to a collin older than 0.10.0, the first to read a
  seed's columns from the header of its CSV (0109).
- Give collin the project folder only when the manifest in use lies under it,
  so a manifest named from another project's folder gets none of that
  folder's seeds (0109).

## 1.43.0 - 2026-10-20

- Dress the app for Halloween behind a pumpkin in the top bar, orange and black,
  off until pressed and never in an export or a copied picture (0106).
- Pick the look, the random apparitions, the sound and the style of the
  lineage's boxes in a Theme section of the settings, kept on the machine
  (0106).
- Draw the lineage's boxes as tombstones, coffins or grimoires, with the full
  art up to 400 boxes on the canvas and a plainer version of the same style
  above it (0107).
- Draw the edges as silk with dewdrops, and grow a small web, with a spider
  that drops on its thread, on the box you select (0108).
- Add a ghost on Edith's E, a witch flying over the dock's tabs, zombies on the
  switches of the settings and a ghost and a pair of eyes that cross the page
  now and then at random (0106).
- Play the ghost's wail and the eyes' whisper, muted until you press the
  speaker, synthesised in the browser so no file is shipped (0106).

## 1.42.1 - 2026-10-20

- Zoom an exported lineage in as close as the canvas does on a tall graph: it
  stopped so far out that the boxes stayed too small to read.

## 1.42.0 - 2026-10-20

- Say on a macro's card and in `Cmd/Ctrl + K` how many nodes call it, through
  other macros too, open the list from the card, and list in Reports the
  project's macros no node reaches, with why each may still run (0105).

## 1.41.0 - 2026-10-20

- Name each version of a model apart, `orders.v2`, as `dbt ls` does: on the
  lineage, in the Catalog, which says beside it whether it is the latest, in
  search, `--select`, `--describe` and `--column`. `orders.v2` and
  `orders_v2` select that version and `orders` every one, `version:` selects
  the latest, the prereleases, the old ones or the models with none, and a
  `ref()` with no version links to the latest. The Jobs tab's head and the
  Compiled bar name a source by its source and table, `crm.customers`, and a
  generic test by its name without its hash (0104).

## 1.40.0 - 2026-10-20

- List the config paths of `dbt_project.yml` that apply to no resource, which
  dbt warns of as it parses, in a Warnings section of Reports with its chip on
  the button, and mark each key in the editor's gutter (0103).

## 1.39.0 - 2026-10-19

- Open `dbt_project.yml` at a var's line when its `var()` is clicked in
  the editor.

## 1.38.0 - 2026-10-19

- Open Custom selection from a button in the top bar as well, over the whole
  window with the sidebar, the card of a box only once it is clicked, closed
  by its × or Escape and resized by dragging its edge (0101).

## 1.37.2 - 2026-10-19

- Stamp a build with `git describe` only when its source is the top of its
  repository, and only from a release's tag, so a copy built inside another
  repository, or under a home folder kept in git, no longer shows that
  repository's tag in `--version` and the status bar, only its date, and a
  tag of your own in the clone no longer stands in for the release.

## 1.37.1 - 2026-10-19

- Stop calling a lineage `truncated` when its cap holds every box it
  reaches. A box met a second time once the cap was full set it, in model
  and column mode alike, and in column mode left a column the walk reaches
  around a stop under `stopped downstream at`.

## 1.37.0 - 2026-10-19

- Describe a node on the command line with `--describe`, several at once, as
  its Catalog card: relation, columns with types and tests, parents, children,
  tests and reach, the SQL with `--sql`, text or JSON (0102).
- Print the project's shape with `--overview`: counts by kind, materialization
  and folder, macros, selectors, tags and the caches beside the manifest,
  `--depth` levels down (0102).
- Print a selection or a column compact with `--output compact`,
  tab-separated under a header, without what the name and the path already say
  (0102).
- Refuse a flag of another mode beside `--select`, `--selector` or `--column`,
  `-s orders --up 1` say, with exit 2, where it was dropped without a word.
- Name a `--column` model as `name.vN` for a version of it or `package.name`,
  and say the nearest names when none matches.

## 1.36.0 - 2026-10-19

- Open every setting from one gear in the top bar, in a panel of its own, a
  section at a time: the project's profile, `.env` files and Python, the
  column lineage, Snowflake, the job sources, updates, and help with the
  tour, in place of the column lineage menu, the snowflake and the profile's
  button; the status bar's `env`, venv and update pills open their section,
  and the environments and the job sources are edited there rather than in a
  dialog (0100).
- Reload the manifest from circling arrows at the end of its freshness pill,
  in place of the Reload manifest button, turning until it is read.

## 1.35.0 - 2026-10-19

- Answer `--column` from a manifest alone with `--run-collin`, which runs
  collin over the manifest first and writes its cache beside it, and
  `--compiled DIR`, the folder of compiled SQL it reads, with collin 0.9.0;
  with `--indirect`, collin writes the indirect edges too (0099).

## 1.34.1 - 2026-10-19

- Offer collin 0.8.0, the first to list the roots the `generated` badge is
  read from, as "Update collin" to any older one, the old repository's 0.2.0
  included, and stop the Columns tab calling a version installed by its tag
  older, since no update reaches it (0096, 0098).

## 1.34.0 - 2026-10-18

- Draw collin's indirect edges, `filter`, `join_key` and `dedup_key`, thin
  and dashed under one `indirect` entry in the legend, read no badge from them,
  and say on a column's card what keeps its rows, `rows kept by:
  raw_orders.status (filter)`, apart from where its value comes from (0097).
- Hide collin's indirect edges from the column lineage by default, walk, draw
  and count them with an `indirect` button beside stop at, shown when the
  cache holds them, and say `indirect edges hidden` on the status line and in
  an export; the Catalog's Lineage counts leave them out too (0097).
- Leave the indirect edges out of `--column` unless `--indirect` asks for
  them (0097).

## 1.33.0 - 2026-10-18

- Name Datadorelix as the company, dbt-edith as the product and its version
  in the Windows `.exe`'s properties, which an antivirus reads on an unsigned
  file, and refuse a cross-built `.exe` without them (0093).

## 1.32.0 - 2026-10-18

- Badge a column its model's SQL sets from a literal `generated` on the
  column lineage, with its value on hover, in Catalog > Columns and in
  `--column`'s JSON, from the roots collin 0.8.0 writes. With them, `raw` is
  kept for the columns of sources, seeds and tables no node owns, and a
  model's column with nothing feeding it that is no root has no badge; a cache
  without roots badges as before (0096).
- List the `raw`, `generated` and `mixed` badges in the column lineage's
  legend even when no edge on the canvas carries a role.

## 1.31.0 - 2026-10-18

- Say under the box when what a selection names takes half or more of a cut
  canvas, as `source:*+` can, and that Draw all or a narrower selection draws
  the rest of their lineage (0087).

## 1.30.0 - 2026-10-18

- Open Custom selection in a panel over the editor and the dock, the canvas
  at the window's height: a click on a box shows its card beside the canvas,
  a double-click opens its file and rolls the panel up, and Escape gives the
  dock back its model's lineage as it was (0092).

## 1.29.2 - 2026-10-18

- Load a model thousands of columns wide at once, on the page and with
  `--column`: its column lineage, its catalog and its column tests were each
  matched to its columns one name at a time, over every column, which took
  under a second at 5 000 columns and over ten at 20 000.

## 1.29.1 - 2026-10-18

- Pass over a collin that git tracks, beside dbt-edith or on the `PATH`, whatever
  the case of its name, since it came with a repository, and say under Collin
  in the column lineage menu which one runs instead, or in the Columns tab
  that none does, and why; in a repository git will not read, pass it over
  too, with git's reason (0094).
- Start git, the browser's `cmd` and a Python found by name from the `PATH`
  alone, never from the folder dbt-edith sits in, where Windows looks first.
- Keep `profiles.yml` a link when saving it from the page: the file the link
  points at takes the change, its mode kept, rather than a plain file taking
  the link's place.
- Refuse to open a `profiles.yml` in the project that links to a file outside
  it, and say why on its button, rather than serve whatever file it names; the
  Snowflake script refuses it too, and never lists what a profile that is no
  mapping holds in its error.
- Refuse a link through a folder that is not there and back, a folder link on
  the way included, and on Windows a path segment naming a drive, such as
  `C:x`, as paths that leave the project.

## 1.29.0 - 2026-10-18

- Show a test in the Catalog by what it checks: the generic, its model and
  column, its severity, the config it changed such as `store_failures` or
  `where`, the relation its failures are stored in, and the models it reads,
  with no Columns tab, location or description to fill.

## 1.28.1 - 2026-10-18

- Draw the update in Edith's menu as a button, `Update to` the release, with
  what it types and the Enter and restart it needs right under it, and say
  `Installed: restart dbt-edith to run it` once the new binary is in place
  rather than offer the update again (0095).
- Fetch the release tags before the update's pull, so the build stamp names the
  release it built rather than the last one the clone had heard of.

## 1.28.0 - 2026-10-16

- Print one column's lineage from the command line with `--column model.column`,
  `--up` and `--down`, one JSON line per model reached with its file, its
  columns, the role of each edge and their tests, and say on stderr when the
  cache is older than the manifest and how to write one for a target you
  choose (0088).
- Stop offering the report `collin generate` writes beside its cache as a
  column lineage cache.
- Find the column lineage caches beside a manifest named by its file name
  alone with `--manifest`.

## 1.27.0 - 2026-10-16

- Draw a selection of up to 3000 nodes whole on the lineage, where it used to
  stop at 400; past that, draw what lies nearest what it names first, and say
  under the box that it is cut, with a button that draws the rest, this time
  only, slower to zoom and pan. The cut used to keep the first 400 names,
  which drew sources without the children they feed (0087).

## 1.26.1 - 2026-10-16

- Count in the column lineage's stop at menu the edges of each role on the
  canvas rather than in the whole project, whose count is on hover (0089).
- Count in the Catalog's Lineage cell every column upstream and downstream
  of a column, the whole chain rather than one level, which hovering it still
  says (0089).

## 1.26.0 - 2026-10-16

- Open Edith's menu by resting the pointer on the name at the top left, no
  click needed; a click there still opens it, and keeps it open (0086).

## 1.25.2 - 2026-10-16

- Tell the lineage's two copies apart: Fit, Export and Copy image carry a glyph
  before their name, a picture for Copy image, and the selection bar's Copy,
  which copies the names, is Copy names, under a list. When the row runs out of
  room even with the dock's tabs down to their glyphs, the three keep their
  glyph alone too, so the bar holds on a window 60px narrower than before.

## 1.25.1 - 2026-10-16

- Read a compiled SQL file only under the target directory, whatever path the
  manifest records for it: a `..`, an absolute path or a link out of it is
  refused, as a path the browser sends is.
- Hide a profile password or key written on the line after its key, past a
  comment, or inside a flow mapping on one line, and keep a hidden block under
  its key when a saved profile moved that key.
- Refuse to write through a link in the project that points at a file not
  there yet, which the write would have created outside the project.
- Hide `DB_PASS`, `DATABASE_URL`, `CONN_STR` and any value carrying a password,
  in a URL or as a `password=` parameter, in the hover card, whatever the
  variable is called, and say whether the name or the value gave it away.
- Look for a newer release from the config directory, not from the temporary
  one every account on the machine shares.
- Refuse to type a selection from a job file, or from the box, that holds a
  character the shell would act on between quotes, `$`, a backtick, `\`, `!`,
  a quote or a control character, and say why, rather than type it with the
  character dropped.

## 1.25.0 - 2026-10-14

- Raise an issue or a request, or give dbt-edith a star, from the GitHub mark
  in the top bar; the issue opens with the version, the build and the system
  already in it.

## 1.24.1 - 2026-10-14

- Say in `dbt-edith --help` what `--no-update-check` turns off: the hourly
  looks for a newer release from midnight to 09:00 Paris time, and the Check
  for updates button in Edith's menu, where it still said once a day (0079).

## 1.24.0 - 2026-10-14

- Find a macro by its name with `Cmd/Ctrl + K`, a package's as `package.name`,
  and open it on the line that defines it, in a file holding several (0084).
- Keep the search box on what is typed when an answer to fewer letters comes
  back last, and pick from what was typed on an Enter pressed right after it.

## 1.23.0 - 2026-10-14

- Keep what a legend entry names lit on the lineage, a materialization or what
  happened to a column, by clicking it, and fade the rest until it is clicked
  again.
- Stop the column lineage downstream at the roles ticked in **stop at**,
  beside the depth boxes, `window` say: each window edge is still drawn with
  the column it reaches, and nothing past that column (0083).

## 1.22.0 - 2026-10-14

- Find a node on the lineage with `Cmd/Ctrl + F`, by its name, and bring it to
  the middle of the canvas, with `Enter` stepping through the other matches and
  opening the chip or the group a test is folded under (0081).

## 1.21.0 - 2026-10-14

- Cut each folder band by its sub-folders, up to four levels down, from a
  levels box that opens under the by folder toggle when you pause on it: each
  sub-folder takes a run of columns inside its folder's, drawn as a rectangle
  within it in a hue per level, its name on a row of its own (0080).

## 1.20.0 - 2026-10-14

- Copy a node's name from the Catalog in one click, with the glyph beside it.

## 1.19.0 - 2026-10-14

- Count a dbt Cloud job as scheduled when its schedule trigger is on, even
  when the vars file writes its cron (0077).
- Count an Airflow DAG on an asset as scheduled when a scheduled DAG with no
  dbt task updates it, an ingestion DAG for instance, or a task with no command
  in a dbt DAG.
- Tell a job whose schedule is not read from one nothing schedules: an amber
  **Not read** pill for a schedule in Jinja or written as a mapping, an asset
  written as a mapping or one no file read updates, an empty list of assets,
  or a job no file defines, ranked after the scheduled jobs, and its models
  listed apart in the job coverage, out of its news.

## 1.18.4 - 2026-10-14

- Start the lineage at one level up and one down, and go back to that whenever
  the model changes other than from the canvas, from `Cmd/Ctrl + K`, a tab, a
  link or the Catalog, rather than keeping the depth set for the last one;
  going back and forward to another tab brings back the depth it was left with.

## 1.18.3 - 2026-10-14

- Offer the tour on the first page with a model to show, not the first page
  at all: a project opened before `dbt parse` spent the offer on a tour with
  half its steps left out, and the offer now comes once the parse lands (0078).
- Say in the tour's welcome whether there is no `manifest.json` yet or a
  manifest with no model, ringing the `?` button the tour is taken again from,
  and name the package of a model taken from one when the project has none of
  its own.
- Point the tour at the manifest's freshness badge right after its welcome:
  everything the page shows is read from `manifest.json`, so it is as true as
  the last `dbt parse`, and the step says what the badge's age and colours
  mean and that a click on it types `dbt parse`.

## 1.18.2 - 2026-10-14

- Show a toast above the dialog it answers, so copying the clone statements
  says it was done.

## 1.18.1 - 2026-10-14

- List every key of `dbt_project.yml` reaching a file in the breadcrumb's
  config menu when `dbt_project.yml` has Windows line endings and is not open
  in a tab: each folder key ending its line was dropped, and its configs shown
  on the level above.

## 1.18.0 - 2026-10-14

- Look for a newer dbt-edith every hour from midnight to 09:00 Paris time,
  then at the next midnight, rather than 24 hours after the last look, so the
  first start of a day sees what came out overnight, and try a look that got
  no answer again within the hour (0079).
- Open a menu from Edith at the top left, and from the update pill, with the
  version and build you run, when GitHub last answered or what git said when it
  could not, when it looks next, and a Check for updates button; a dot on the
  name says a newer release is out.
- List every release since yours in that menu, newest first, each by the title
  of the pull request that released it and linked to its notes.

## 1.17.0 - 2026-10-14

- Name an incremental model's strategy on its lineage box, in place of the
  word `incremental`, and the adapter's default when none is configured, read
  from the macros the manifest carries (0076).
- Show `append` with the strategy struck through when the macros dbt runs
  insert every row for want of a unique key, as Snowflake's merge does, and
  say why in the hover card, which names where the strategy comes from.
- Add Strategy and Unique key to the Catalog's context for an incremental
  model, and the key for a snapshot, one column per line, each opening its row
  in the Columns tab, where the key's columns carry a `KEY` tag and a Key sort.
- Bring a column asked for from a report, a column box or Snowflake's lineage
  into view in the Columns tab, clearing a filter that hid it, whatever the case
  of its name.
- Select a model keyed on a list with `config.unique_key:` and any of its
  columns, as `dbt ls` does.

## 1.16.1 - 2026-10-13

- Keep the Reports button in view before any report has something, dimmed,
  its tooltip saying what will fill it: it was hidden until the first dbt
  run, collin report or job source, so a first start never showed it (0075).
- Never ask to look at the job files during the tour: its Jobs step shows the
  jobs as last read, and a Reports panel it rolled up comes back as it was
  left rather than read again, which looked at every job file and dropped the
  Catalog's jobs.

## 1.16.0 - 2026-10-13

- Give a tour of the page, from Give me a tour at the top of the shortcut
  list: each step rings one part of it on a model of the project with parents
  and children, and says what it is for, from the editor and the
  `dbt_project.yml` button to the lineage, a Custom selection with Export and
  Copy image, the Catalog, column lineage, Jobs, Reports, the terminal and
  Search. A feature that is off says how to turn it on. It runs nothing,
  changes no setting, and puts the page back as it was when it ends (0074).
- Offer the tour once per machine, the first time a page opens, in a card
  under the `?` button, and keep in `tour.json` the version of dbt-edith that
  served the page, for a tour of what is new since.

## 1.15.1 - 2026-10-13

- Give each of the dock's tabs a glyph beside its name, and fold the tabs to
  their glyphs alone, each named by its tooltip, once the bar cannot hold
  every label beside the lineage's tools.

## 1.15.0 - 2026-10-13

- List a node's scheduled jobs first and its manual ones last in the Jobs tab,
  and say which is which on a pill: green **Scheduled** with the schedule, grey
  **Manual**. The jobs the coverage read are listed the same way, and its
  *Manual only* section is now *Only in unscheduled jobs*, since a job CI or a
  merge starts is not manual.
- List the steps that run a node in the Catalog's Preview, one chip each, `DAG
  › task` or `job › step`, coloured as their job's pill; a click opens the
  Jobs tab on that node, at that step. Steps not resolved and files not read
  that could run it are counted on a chip of their own.

## 1.14.1 - 2026-10-13

- Move by word with Ctrl+Left and Ctrl+Right and to the start and end of the
  command with Home and End in the terminal's Git Bash on Windows, and delete
  a word with Ctrl+Backspace and Ctrl+Delete, sending keys readline binds
  whatever its configuration, at bash's prompt only (0073).
- Move to the start and end of the line with Cmd+Left and Cmd+Right in the
  terminal on a Mac, delete the line with Cmd+Backspace, and the next word with
  Option+Delete, sending the shell what VS Code's terminal sends (0073).
- List the terminal's line keys of the platform in the shortcut list.

## 1.14.0 - 2026-10-13

- Colour the venv in the status bar: green with a full dot once one is
  active, amber with an empty one when the venv found is not activated, so a
  `deactivate` in the terminal shows at a glance (0069).
- Type the line that activates the venv in the terminal on a click on the
  status bar while it says inactive, and leave Enter to you; with several
  venvs in the project, the click lists them (0072).

## 1.13.2 - 2026-10-12

- Pan the lineage from a box without opening it: the click the browser fires
  at the end of the drag no longer selects the box and opens its file in place
  of the one being read, and a click that trembles a few pixels stays a click,
  in an exported file too.
- Stop a pan's release on the canvas from making a double click with the
  click before or after it, which re-centred the canvas on a box or fitted the
  view.
- Keep a wheel zoom, or a fit when the window is resized, in place when it
  comes in the middle of a pan.
- Show the grabbing cursor only once a press has become a pan.

## 1.13.1 - 2026-10-12

- Put Clone into my schema beside the Location heading, in Snowflake's colour,
  rather than at the far right under the Location table.
- Make the lineage's folder bands a by folder toggle beside the tests eye,
  lit in the accent colour while on, in place of the folders checkbox, with
  the dock's tabs a little tighter to make room for it.

## 1.13.0 - 2026-10-12

- Pick a job source from a list of folders rather than type its path: a
  Browse button beside each path, open already on a source being added, lists
  a folder's folders and YAML files, anywhere on the machine (0070).
- List the job coverage in Reports before any source is named, with Add a
  source in it.

## 1.12.1 - 2026-10-12

- Name the venv the terminal activated in the status bar, from bash's own
  `VIRTUAL_ENV` at each prompt, where the bar stayed on the one found at
  startup (0069).

## 1.12.0 - 2026-10-12

- Say in the status bar when a newer dbt-edith is out, asked of GitHub through
  git once a day, with a link to its notes and, for a binary built from a clone
  on the machine, the pull and the rebuild typed into the terminal; on Windows
  the running `.exe` is renamed out of the way first (0068).
- Add `--no-update-check`, which asks GitHub nothing.

## 1.11.3 - 2026-10-12

- Open the Columns tab from the column count in the Catalog's context.
- Reach the counts in the Catalog's context with Tab, and follow them with
  Enter or Space as with a click.

## 1.11.2 - 2026-10-12

- Close the Orchestration form once a save reads every source cleanly, saying
  what it read and showing Job coverage, or the Jobs tab when it shows a model.
  It stays open on a source that found no job, a file it refused, a walk stopped
  at a bound, sources it could not save, or edits made while it read; saving
  again used to repaint the same line, and looked like a save that did nothing.
  A form closed while it reads says what it read in a toast.

## 1.11.1 - 2026-10-12

- Show a file's new text in its tab when a `git pull`, a checkout or another
  program rewrites it, without reloading the page, and keep a tab's unsaved
  edits, asking before a save writes over the change on disk (0067).

## 1.11.0 - 2026-10-11

- Show the last dbt command's results in Reports, from
  `target/run_results.json`: the command and its selection, what ran of each
  kind against the manifest, the errors and failed tests with dbt's message,
  the warnings, the skipped nodes and the slowest, a model opening with the SQL
  dbt ran and a test at its line (0065).
- Count the last run's errors and failed tests on the Reports button until its
  section is shown.
- Say on the Reports button how many more models no scheduled job runs than
  when the job coverage was last looked at (0066).
- Keep the Reports button short: with several reports to look at, each chip
  gives its count alone, the tooltip saying each in full.

## 1.10.0 - 2026-10-11

- Gather what speaks of the whole project in a Reports panel that rolls down
  over the editor and the dock from the top bar or `Mod+Alt+R`, a section
  each, and rolls up when a file, a model or a dock tab is opened (0064). Its
  button is a glyph whose name unfolds on hover.
- Show collin's report as a section of Reports rather than a tab, its count of
  models to investigate riding on the Reports button where its badge was.
- Move the Jobs tab's Coverage view into Reports, as Job coverage; the tab
  lists the jobs of the node in focus, with a button to the coverage.
- Group the coverage's models under the deepest folder that holds two of them,
  each row saying its path below it, and list a model no folder gathers bare:
  with a folder per model, every model was a folder to unfold.
- Find a section of Reports as it was left, its folds and its scroll, when it
  is shown again after a model was opened from it.

## 1.9.0 - 2026-10-11

- Show which Airflow DAGs and dbt Cloud jobs run the node in focus, in a Jobs
  tab beside Catalog, read from their YAML in folders named in the settings,
  outside the project: each task or step with the terms of its selection that
  reached the node, its schedule, the jobs it waits for or starts, and a
  `dbt ls` button to check it against dbt. A step this tool cannot resolve is
  listed apart with the reason, never guessed (0063).
- List, in the Jobs tab's Coverage view, the project's own models, seeds and
  snapshots no scheduled job runs, those run only by jobs nothing schedules,
  and the steps that select nothing, each model opening on a click.
- Open an installed package's model from the lineage, the palette, the Models
  list or Coverage, in `dbt_packages`, where it used to fail with no such file.
- Read dbt Cloud jobs in dbt-jobs-as-code's schema, anchors and merges
  included, and Airflow DAGs in a proposed shape whose every key can be renamed
  to match a repository's own.
- Name the top bar's settings button for what it now holds, the project's
  settings, rather than Snowflake's features alone.

## 1.8.1 - 2026-10-11

- Ask git for the status in two calls rather than six, and the freshness badge
  in three rather than four, without the 30 ms wait each call used to pay: on a
  10 000 file project a status poll takes 250 ms rather than 405, the badge
  230 ms rather than 360.
- Pause the status and freshness poll while the tab is hidden, and catch up as
  soon as it is shown again.
- Search file contents on several threads: a pass over 10 000 files takes
  165 ms rather than 550.

## 1.8.0 - 2026-10-10

- Show collin's own findings on its report page when collin writes them, from
  0.5.0: each with what collin saw, what it usually means and what to do, in
  the same groups. A report from an older collin is filed here as before, and
  two versions are still compared on that filing (0062).

## 1.7.0 - 2026-10-10

- Compare what two collin versions read in the project, from the last run of
  each, kept under `target/collin/`: their figures and findings, edges by
  role and by origin, the edges only one has, and the models where they moved
  (0061).

## 1.6.0 - 2026-10-10

- Install other collin versions beside the one you have, each at its tag in a
  folder of its own, and pick the one that runs for the project under Collin
  in the menu. A cache another version wrote runs again under the one picked
  (0060).

## 1.5.0 - 2026-10-10

- Show collin's report whole, as a tab grouping every model and table it names
  by what would fix it: rerun dbt, fix in the project, check by hand, or
  collin's own. A run leaves a badge saying how many there are to investigate
  until the report is opened, and the Collin menu opens it at any time. A
  failed run no longer leaves a new report beside the old cache (0059).

## 1.4.0 - 2026-10-10

- Write the statements that clone a model and its direct parents from an
  environment into your own schema, with Clone into my schema under its
  Location table while Snowflake's features are on (0058). The source is each
  node's relation as the table resolves it with a `.env` file, the destination
  comes from your profile's active target and can be typed over, and you run
  the statements yourself: dbt-edith sends nothing to Snowflake.
- The settings menu names every Snowflake feature, the Snowsight menu included.

## 1.3.0 - 2026-10-10

- Resolve a dbt selector from the command line with `--select` or
  `--selector`, which prints the nodes and exits without serving (0057). Names
  go to stdout as `dbt ls --output name` prints them, or one JSON object per
  node with its parents, and stderr says which manifest answered and whether
  files changed since, so a script or an agent skill gets the answer in under
  a second where `dbt ls` parses the project again.

## 1.2.2 - 2026-10-10

- In the Selection box, `*`, a model's name and an fqn pattern such as
  `shop.staging.*` no longer select sources, which dbt reaches with `source:`
  alone (0056). `*` listed every source of the project, and a name also
  selected a source table of that name. A term only sources answer now says to
  use `source:`.

## 1.2.1 - 2026-10-10

- The same manifest gives the same answers on every start (0055). Opening a
  properties file shows its first node by name in the lineage, equal search
  hits read alphabetically, and a selection or a lineage capped for the canvas
  keeps the same nodes and edges every time. Each start used to pick its own,
  so one selection could draw a different number of edges after a restart.
  Past 5 000 matches, Copy holds the first 5 000 names by name.

- Under dbt-core, a test on a source's column that also reads a model, a
  `relationships` one for instance, shows on that column in the source's
  Catalog. It went to the model instead: onto its column of the same name, or
  nowhere.

## 1.2.0 - 2026-10-10

- The API and the terminal want the key each launch prints in its link, which
  the link turns into a cookie, so no other program on the machine reaches the
  project, its `.env` files, the profile or a shell, under any account and
  whatever headers it writes (0052). A tab opened without the link, or kept
  from an earlier launch, says so in a red bar at the top.
- The profile opens with its passwords, tokens and private keys replaced by
  `<hidden by dbt-edith>`, and saving puts them back unless one was typed over,
  so no credential from `profiles.yml` reaches the browser (0054).
- Opening a project runs nothing in it: the status bar reads a venv's Python
  and dbt versions from its files, and a venv git tracks, which came with the
  repository, is never run, for the status bar or the Snowflake script (0053).
- Replies are no longer kept in the browser's disk cache, other sites cannot
  load them or hold this window, the terminal's WebSocket is allowed on this
  port alone, and a request the browser marks as coming from another site is
  refused.
- Saving the profile goes through a temporary file only its owner can read, so
  its password is never readable by another account, even for an instant.
- `scripts/build_windows.sh` builds the `.exe` for the VM and writes its
  SHA-256 beside it, to check with `certutil` before running the copy.

## 1.1.0 - 2026-10-10

- A Snowsight menu beside each relation in Catalog > Location opens its table
  or view, its schema or its database in Snowsight, on the account the
  profile's target connects to (0051). It is there while Snowflake's features
  are on, and the first one of a page asks Snowflake for the account, which may
  bring a sign-in tab.

## 1.0.1 - 2026-10-10

- A new terminal activates the virtual environment dbt-edith was started in,
  typing its activate script once the shell has read its startup files, so
  `dbt` there is the venv's even when `~/.bashrc` puts pyenv's shims or conda
  first, and `deactivate` exists (0050). The venv's `PATH` reached the shell,
  but those startup files then pushed it behind another `dbt`.

## 1.0.0 - 2026-10-10

- dbt-edith is published under the Functional Source License 1.1 with an MIT
  future license (FSL-1.1-MIT), copyright Datadorelix, in place of MIT (0046).
  It stays free to use, in a company and for a client's work too; what it
  forbids is selling it, or a service built on it, as a substitute for
  dbt-edith. Each version also becomes MIT two years after it is published.

- Everything [the README](README.md) describes ships in this first release:
  the editor, the terminal, the model lineage read from `manifest.json`, the
  column lineage from Snowflake or collin, selectors resolved the way dbt
  resolves them, the Catalog, and the rest.

- Catalog > Columns marks with a ! each column collin's report says something
  about, its reason on hover: a column whose lineage collin lost, one the
  warehouse has and the compiled SQL does not produce or the other way round,
  one read from a CTE that lacks it, one a downstream model reads that this
  model's list lacks, and one matched by name to several parents. The note over
  the table counts them, and names the ones the table has no row for.

- dbt-edith accepts collin 0.1.0 and later, where it asked for 0.2.0: collin's
  repository started again, under the same license as this one, and its numbers
  with it (0047). A collin from it no longer gets "Update collin" in the
  column lineage menu, which would have reinstalled the same version.

- On Windows, `dbt` in the terminal is the one from the virtual environment
  dbt-edith was started in. The terminal had the machine's `PATH` instead, so
  `dbt` there could be another install, without the project's adapter, while
  the environment still looked active.

- A violet button in the top bar, a user and a plug, opens the dbt profile on
  every project, found where dbt looks for it, where `profiles.yml` showed only
  once the Snowflake script had run with Snowflake's features on (0049). With
  no file there, it turns grey, carries an amber dot and says where dbt would
  read one.

- The keys of `dbt_project.yml` reaching a file open from a round violet button
  at the end of the breadcrumb, a file with an arrow coming into it, in place of
  the file's name. The breadcrumb's segments now sit in the middle of their bar,
  where they sat a few pixels high with the top of a hover cut off.

- A Query history tab lists your own Snowflake queries of the last seven days,
  twenty at a time with Load 20 more, filtered by the role of the target dbt
  connects with or another target's, each linked to its page in Snowsight
  (0048). It is there while Snowflake's features are on, and reads Snowflake
  only when you open it or press Refresh.
