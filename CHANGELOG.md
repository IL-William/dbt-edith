# Changelog

What shipped, and when. This file is appended to, never rewritten, which is
what makes it the only place that keeps the order:
[docs/state.md](docs/state.md) says where the work stands today and is rewritten
as it moves, the [README](README.md) says what the tool does now, and
[docs/decisions/](docs/decisions/) says why it is built this way. A feature
belongs in the README too; only here does it carry a version.

One entry per change someone using dbt-edith would notice, written as a
sentence saying what it does, the way a commit subject is. A number in brackets
points at the decision behind it. A version's date is the day its bump merged,
which tags and releases it with its section here as the notes (0045): the
newest dated heading has to be the version `Cargo.toml` names.

## Unreleased

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
