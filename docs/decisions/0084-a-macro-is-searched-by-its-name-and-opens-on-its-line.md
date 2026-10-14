# 0084. A macro is searched by its name, and opens on its line

Date: 2026-10-14 · Status: accepted · Amends 0028

**Trigger:** read before changing what `Cmd/Ctrl + K` lists, how it ranks a
macro, or where opening one lands.

## Context

The palette listed dbt nodes and every file. A macro was findable only through
its file, so one of several in `utils.sql` was not findable by its name at all,
and a file named after a macro opened at its top. The manifest already lists
every macro with its package and its path (0028).

## Decision

- **A macro is matched by its name alone**, or by `package.name` once the query
  has a dot. Its path is left to the file half of the box.
- **Ranked by how the name matches first**: the name itself, then a name that
  starts with the query, then one that holds it. Within each, the project's
  before a package's, then the shorter name. So `star` finds `dbt_utils.star`
  above a project macro that merely contains the word.
- **Only a macro whose file is in the project is listed**, placed as a link is
  (0028), against the project's real path worked out once per query. A package
  with no folder under `dbt_packages/` or `dbt_modules/` is passed over whole
  after one look for that folder, so dbt's own few hundred macros, and an
  adapter's, cost that look and not one per macro. The others are looked for
  until four times as many as the box can list were not found: a manifest
  older than a move of `macros/` names files that are gone, and one letter
  would look for them all. The list it cuts says nothing of the cut.
- **Answers are shown in the order their queries were asked**, and Enter waits
  for the last one, since three requests a query make a late answer likelier,
  unless the arrows picked a row of the list shown. An Enter still waiting
  does nothing once the box has been closed or opened again.
- **A row reads as the project calls it**: bare for the project's,
  `package.name` for a package's. Macros come after the nodes and before the
  files, and only once something is typed.
- **A file named after a macro listed is dropped**, as a model's `.sql` is,
  `is_positive.sql` for `test_is_positive` too. A file named otherwise stays,
  since it holds more than the macro that matched.
- **Opening one lands on its definition**, found in the buffer by
  `macroDefLine`, which now also reads `{% test x(` as `test_x` and
  `{% materialization x, adapter='y' %}` as `materialization_x_y`, the names
  dbt gives them. A link to one of those lands there too.

## Rejected

- **Matching the path too**, as nodes are: typing a file's name would list
  every macro in it and push the file itself out.
- **Every package's macros below the project's**, as the file search ranks
  `dbt_packages`: an exact package name would sit under any project name that
  holds the query.
- **Placing every macro once, when the manifest loads**: a file moved since
  would keep its stale place until the next reload, where a stat per query is
  cheap.
