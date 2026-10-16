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

## Unreleased

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
