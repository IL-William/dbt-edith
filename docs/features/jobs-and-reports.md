# Jobs and Reports

Which of the orchestrator's jobs run a node, and the panel of what speaks of
the whole project.

## Jobs

The **Jobs** tab, beside Catalog, says which of your orchestrator's jobs run
the node in focus: an Airflow DAG and the task in it, or a dbt Cloud job and the
step of it. The jobs are read from their YAML, wherever it lives, which is often
another repository, and each step's dbt command is resolved by the same engine
as Custom selection, never by running dbt. Only a step that runs, builds, tests
or checks the node is listed: a `dbt compile` selects every model and runs
none, so its job would sit under each of them.

```
AIRFLOW  DAG shop_daily   (SCHEDULED 0 5 * * *)          orchestration · sales/shop_daily.yml:2
  task marts    dbt run -s +tag:daily --exclude tag:wip    [dbt ls]
                via +tag:daily · after stage · awaited by shop_reports › wait_daily
DBT CLOUD  Job Daily build [[daily]]   (SCHEDULED 0 */2 * * *)   cloud · jobs.yml:14
  step 3/3      dbt build --select +orders                 [dbt ls]
                via +orders
```

The jobs that run on their own come first, with a green **Scheduled** pill
that gives the schedule, and the ones nothing starts come last, with a grey
**Manual** one. Next after the scheduled ones come the jobs that may run on
their own, on something no file read says, with an amber **Not read** pill
that says what: a schedule in `{{ }}` or `{% %}` or written as a mapping, an
asset written as a mapping or one no file read updates, an empty list of
assets, a dbt Cloud job started by one no file defines. A CI or merge job,
which something starts but no clock, sits before the manual ones, its pill
saying what starts it.

**via** names the terms of the selection that reached the node, so a staging
model listed under a marts task says why. **dbt ls** types into the terminal the
`dbt ls` that should list what the step acts on, without running it, which is
how you check this tool against dbt. A step that cannot be read or resolved is
never guessed at: it is listed apart, under *Not resolved*, with the reason. A
`state:modified+` compares with another run's artifacts, a `$MODELS` or a
`{{ var }}` is only known when the job runs, and a flag this tool does not know
might take a value or not.

**Job coverage**, a section of [Reports](#reports) that **Coverage in Reports**
in the tab opens, turns the question round: which of the
project's own models, seeds and snapshots no scheduled job runs, which run only
in jobs nothing schedules (*Only in unscheduled jobs*: manual, CI, a merge),
which run only in jobs whose schedule is not read and in no scheduled one
(*Only in jobs whose schedule is not read*), which steps select nothing, and
which jobs were read.
A model sits under the deepest folder it shares with another listed model,
with the rest of its path beside its name, and one no folder gathers is listed
on its own. A group with nothing in it is not drawn. An installed package's
models are left out, so the report is about the project's own. A job counts as
scheduled when it is on a clock, a cron or a preset such as `@daily`, or when it
starts after one that is: an Airflow DAG on an asset a scheduled DAG updates,
whether that DAG runs dbt or not, a dbt Cloud job started by a scheduled job's
end. A dbt Cloud job whose schedule trigger is on is scheduled even when the
vars file writes its cron. A DAG with `schedule: null`, a CI job or a merge job
is not. A job that hangs on something no file read says is neither, and the
news on the Reports button counts only the models surely not scheduled
([0077](../decisions/0077-a-start-no-file-says-is-neither-scheduled-nor-unscheduled.md)). Click a model to open it: the reports roll up,
and the Jobs tab lists its jobs. Coming back finds the list as you left it,
folds and scroll.

Point dbt-edith at the jobs in the Orchestration section of the settings,
behind the gear in the top bar, which **Sources…** and **Add a source**, in
the tab and in the Job coverage section of Reports, open too: one or more
folders or files, absolute or relative to the project, each an Airflow or a
dbt Cloud source. **Browse…**
beside a path, open already on a source being added, lists one folder at a
time, its folders and YAML files only: a click opens a folder or picks a file,
and **Use** writes it from the project when it sits within two folders above
it, in full otherwise
([0070](../decisions/0070-a-source-is-picked-from-folders-the-server-lists.md)).
Where the sources are kept is chosen the first time they are saved: **In the
project, `.dbt-edith/jobs.yml`** (shared, committed, the default), or **For me
only**, in the settings outside the project. The first writes that file, a
`jobs:` list of sources, and only on a save of the form, never on opening it;
the second writes no file in the project. Once the file exists the form edits
it and the settings are ignored, with the file's say on top: a file that cannot
be read gives no source and shows why, at its line, and the settings do not
stand in. A save of the form never writes over a version of the file it was not
shown, as when a `git pull` changed it meanwhile; open the section again to read
it. Nothing deletes the file, and the command line and MCP read it too, for
[the jobs](command-line.md#a-job)
([0115](../decisions/0115-job-sources-may-live-in-the-project.md)). It may be
written by hand:

```yaml
jobs:
  - kind: airflow
    path: ../orchestration/dags     # from the project
    label: orchestration
    keys:
      dag_id: pipeline.name         # only the keys that differ from the shape below
  - kind: dbt_cloud
    path: ../orchestration/dbt_cloud
```

A repository supplies that file, so it takes less than the settings do: a path
from the project, at most two folders above it (`../..`), with no hidden folder
or file in it (`.ssh`, `.dbt`, a `.env`, or one the system hides: macOS'
`Library`, Windows' `AppData`) and never a `profiles.yml`, which the folder walk
of such a source skips too, with every hidden entry. A link along the path is
followed and held to the same where it leads, and the path may not lead to your
home folder or any folder above it. A source
that is not so refuses the whole file, at its line, and the form's save refuses
it too, saying why under the choice; **For me only** takes any path, an
absolute one included, as it always did. Nesting past 64 levels is refused as
well, in this file and in the job files. The Jobs tab shows why a file that
does not read gives no job.

Saving reads them at once. When every source found jobs, no file was refused
and the sources were saved, the form closes on what it found: the Jobs tab when
it is showing a model, Job coverage otherwise. Anything else keeps it open on
what went wrong, file by file, with the line of anything it could not read, and
so do edits made while it read. Opened from Job coverage, Escape or the × goes
back to it. They are read again only when something moved:
when the manifest is reloaded, or when the Jobs tab or the sources form opens
and a job file was added, removed or changed since. A click inside the tab reads
nothing, so a `git pull` in the jobs repository shows the next time the tab is
opened. Only `.yml` and `.yaml` files are read,
never a hidden file or one reached through a symlink, and what reaches the page
is what was parsed: names, schedules and the selection in each command. The
value of `--vars` or `--args` is shown as `…`, since that is where a secret
would sit
([0063](../decisions/0063-jobs-are-read-from-folders-you-name.md)).

dbt Cloud jobs are read in [dbt-jobs-as-code](https://github.com/dbt-labs/dbt-jobs-as-code)'s
schema, anchors and `<<` merges included: `name`, `execute_steps`,
`schedule.cron`, `triggers`, `job_type`, `job_completion_trigger_condition`,
`environment_id` and `settings.target_name`. With `run_generate_sources: true`,
the `dbt source freshness` dbt Cloud runs first is listed as a step of its own.

Airflow has no standard for this, so dbt-edith proposes a shape, one file per
DAG, and lets every key be renamed:

```yaml
dag_id: sales_daily
schedule: "0 6 * * *"        # a cron, a preset, null for manual, or a list of assets
tasks:
  - task_id: build_sales
    command: dbt run --select tag:sales   # `dbt` in front is optional
    depends_on: [stage_sales]             # optional, as is everything below
    outlets: [sales_ready]
sensors:
  - task_id: wait_crm
    external_dag_id: crm_daily
    external_task_id: crm_load
```

A key is a dotted path, so a factory that writes `pipeline: {name: ...}` and
calls its tasks `steps` and their command `dbt` is read with `pipeline.name`,
`pipeline.cron`, `steps` and `dbt`. The tasks may also be a mapping keyed by
task id, the way dag-factory writes them. A task with no command is not a
dbt task and is left out, but the assets it updates still start the DAGs
scheduled on them; so do those of a DAG file with no dbt task at all, an
ingestion DAG for instance, which is read for its id, its schedule and its
outlets alone and is listed nowhere. The form draws the example in your own keys as you
type them.

What the YAML cannot say, this tab cannot either: tasks a factory builds in
Python, a paused DAG, a schedule on one task that only the factory interprets,
`--vars` or `--target` changing which models are enabled, and sources with no
freshness configured. A `--project-dir` is taken to name this project.

## Reports

**Reports**, a sheet with a bar chart in the top bar whose name unfolds under
the pointer, rolls a panel down over the editor and the dock
with what speaks of the whole project, a section each, listed on its left:
the last dbt command's results once dbt has written them, the warnings dbt
would give as it parses once there is one, collin's report once collin has
written one, the coverage of your jobs once the Jobs tab has a source, and the
project's macros while one is reached by no node and nothing explains it.
The sidebar stays, and anything that shows a file or a dock tab rolls the
panel up again: a name in a report, a file in the
tree, the palette, a dock tab, Escape. `Ctrl+Alt+R` (`⌥⌘R` on a Mac) opens and
closes it. While no section has anything to show, the button is dimmed and its
tooltip says what will fill it; it opens on the job coverage, where a source is
added (0075).

What is new since you last looked rides on the button, a chip per report in
its colour: the last run's errors and failed tests in red, the config paths
that apply to nothing in dbt's orange, collin's models to investigate in pink,
and in amber how many more models no scheduled job runs than when you last
looked at the coverage. Alone, a chip says its words, "12 to
investigate"; with others, its count alone, so the button stays short, and the
tooltip says each in full. The panel opens on the first of them. A section is read again
each time it is shown, and never redrawn while you read it: when its file
changes underneath, its **Read again** button says so (0064).

**Last run** reads `target/run_results.json`, which dbt writes again after
every `build`, `run`, `test`, `seed` or `snapshot`, and after `compile` too. It
names the command and its selection, then counts what ran of each kind against
what the manifest holds, and lists the errors and failed tests with dbt's
message, the warnings, the skipped nodes and the slowest, five of which show
at first, **Show all** giving the rest. The status dot of the section,
in the list and in its head, says how the command ended: red on an error or a
failed test, amber on a warning or a partial success, green otherwise; the
other sections keep their tool's colour. A model opens with its Run tab, the
SQL dbt executed; a test opens at its line in its YAML, and a unit
test at its YAML. A node the
command did not reach is counted, never listed: not run says nothing about it.
The section says when the manifest was written after the run, or when the run
names tables the manifest names otherwise, likely another target. Errors and
failed tests ride on the Reports button until you look (0065).

**Warnings** lists what dbt would warn of as it parses, worked out from the
manifest without running dbt: for now, the config paths of `dbt_project.yml`
that apply to no resource, dbt's `UnusedResourceConfigPath`. A misspelt
folder, a folder renamed or emptied, a `meta:` written without its `+`: no
node's fqn begins with the key's path, so what it sets reaches nothing. Each
path is written as dbt writes it, `models.jaffle_shop.stagign`, beside its
line, which opens `dbt_project.yml` at the key, and says why: no node of the
block's kind and no disabled node is under it. The list is dbt's, checked
against dbt-core and Fusion, a disabled node counting as dbt counts it. When
`dbt_project.yml` was saved after the manifest, or other files changed since,
the section says so first: a folder added since the last parse is in no
manifest. The count rides on the button from the first look of each page,
then only when it rises, and only while the manifest matches the project. In
the editor, each such key carries an amber mark in the gutter, grey while the
manifest is behind: its hover card says why, and a click opens the section. A
key reached through a YAML alias is marked at the alias, and keys sharing a
line share one mark, whose card lists each. A `dbt_project.yml`
this cannot read, a key written twice for one, lists the section to say at
which line (0103).

**Macros** lists the project's own macros that no node reaches, from what the
manifest records each node and each macro calling, and from each node's SQL
and hooks, for the calls dbt did not record: a branch on `execute`,
`is_incremental()` or `target.name`, a `{% call %}` body, and, under dbt
Fusion, every hook. None is ever called unused: the manifest keeps no trace
of `dbt run-operation`, of another package's call, nor of what dbt runs
itself. So each is grouped under the likeliest reason it may still run,
what is left unexplained first: nothing in the manifest calls it, called
only by macros no node reaches either, a generic test no enabled test uses;
then a variant `adapter.dispatch` did not pick for the manifest's adapter;
run by dbt for the nodes that name it, a materialization or a strategy macro,
with how many nodes do; or called by dbt itself, by its name, as
`generate_schema_name` or a macro of dbt's own the project overrides, or from
what dbt runs, its own macros included, which is how a dispatch variant dbt
picked reads; not dbt's cross-database helpers nor its generic tests, which
run only when something calls them, nor a project's `default` materialization
where the adapter has its own. Each name opens the macro at its line. The
click on a macro's card, `used by 9 models`, opens this section on that
macro, at its top, listed then without lighting the button:
every node reaching it, by kind, with the macro it comes through when it does
not call it itself, or a note when dbt recorded no call, then the macros
calling it. Under dbt Fusion, which lists a macro reached through another as
a direct call, fewer nodes read as reached through other macros. The section
is listed, and lights the button, only while one of the first three reasons
holds, or a materialization or strategy macro no node names, and the panel
then opens on it rather than on an empty coverage; it carries no chip, since
the count would never go (0105).
