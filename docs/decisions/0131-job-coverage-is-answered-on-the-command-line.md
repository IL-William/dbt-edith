# 0131. Job coverage is answered on the command line

Date: 2026-10-30 · Status: accepted · Amends 0115

**Trigger:** read before changing what `--job-coverage` prints or leaves out,
how its three outputs differ, or before making a mode other than the jobs' own
read the project's `.dbt-edith/jobs.yml`.

## Context

The Job coverage report (0063, 0077) says which of the project's own models,
seeds and snapshots no scheduled job runs, which only a job nothing starts or a
schedule not read runs, and which steps select nothing. An agent could reach it
only by calling `--job` once per job, names from `--overview --output json`,
and subtracting from the project by hand.

## Decision

- **A mode of its own, `--job-coverage`**, beside `--job` and `--job-order` and
  over the same read (`jobs::known`, once per call, 0115), with their errors
  and exit codes: 2 when the project has no `.dbt-edith/jobs.yml` or it does not
  read, 0 for any answer. It calls `jobs::coverage`, the function behind
  `GET /api/jobs/coverage`, so the page and the line cannot disagree. Over MCP
  it is the same line (0110), compared byte for byte by `walk_mcp.py`.
- **Text** opens with the page's sentence, worded as `jobsCoverageLine` words
  it, so a client that cuts the answer still says how much there was, then one
  section per list that has a row, in the page's order: run by no job, only in
  unscheduled jobs, only in jobs whose schedule is not read, ephemeral, steps
  that select nothing, steps not resolved. A node is `folder/name`; a step is
  `job › task`, its source, `file:line`, its command and why it is listed.
  **No list is cut**: the list is the answer, and an agent that met `+N more`
  would need JSON to get the rest. `--output name` prints this text, as the
  other job modes do.
- **`--output json`** is one object: `configured`, `total`, the six lists and,
  when a source was named and not read, `not_read`, which says the first list
  may be too long, and `files_refused`, the job files refused whole in the
  sources that were read (a tab in the indentation), left out when 0; the lines
  of text and the compact row say both. It is a view of its own, not `Coverage` serialised: the
  page's `jobs` and a step's `index` stay out of it, and the route the page
  reads sends what it sent.
- **`--output compact`** is a header and one tab-separated line per row, `list`,
  `name`, `where` (a node's folder, a step's source and `file:line`) and `why`
  (a step's alone), the same four columns for both. The header is printed even
  with no row: the answer ran and was empty.
- **A source named and not read** is said before the lists, where a client that
  cuts the answer still has it: a `not read:` line right under the sentence in
  text, a `not_read` row first under the header in compact, in stdout and not
  on stderr, since it changes what the rows below mean. When every source
  failed outright (the Airflow repository is not checked out, say) no job was
  seen and every node would read as run by no job, so the note is the whole
  answer, as for no source: JSON empties its lists too, keeping `total` and
  `ephemeral`. A source cut at its bound is a partial read: its
  walked files were read, so the lists stand and the note goes with them.
- **With no source in the file** it says so (`no sources in
  .dbt-edith/jobs.yml`) and lists nothing, for the same reason. Compact keeps
  its header and the sentence goes to stderr as a warning; JSON keeps `total`
  and `ephemeral`, which are the project's and true whatever the jobs.

## Rejected

- **`--count-by job` or `dag`.** `--count-by` counts a selection (0112), which
  the selection path resolves from the manifest alone. A job key would make
  every `-s` read the job files, and answer "how many nodes per job", which is
  not "which run in none".
- **Cutting the text at 15, as `--overview` does.** There the full list is a
  JSON call away and the overview is a map; here the rows are the answer.
- **Serialising `Coverage` as it is.** It would put the page's job list, and a
  step's position in it, in an agent's context for nothing, and tie the line
  to the route's shape.
- **A `--select`-style narrowing.** A folder of many models is a
  `--select` away from the nodes; subtracting the report from it is the
  agent's, as `--job-order`'s filter is (0125).

## Consequences

Amends 0115: the sources file feeds a fifth mode. The mode is in
`src/job_coverage.rs`; the agent skill points at it for the models no job runs
(0114).

0114 gave `help` at most 384 tokens; it has not held since `--job-order` (0125), and
the test's ceiling is now 1792 bytes, about 448 tokens, up from 1664, because
the new mode's line needed about 110 bytes. `help` enters the context only
when a client asks; the always-loaded tool entry and description did not grow
(the description lost ", with their flags" to take the new flag).
