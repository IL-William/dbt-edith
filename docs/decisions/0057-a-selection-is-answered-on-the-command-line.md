# 0057. A selection is answered on the command line, without a server

Date: 2026-10-10 · Status: accepted · Follows 0024 and 0052

**Trigger:** read before adding a flag that answers and exits instead of
serving, or before letting anything but the page read the selection engine.

## Context

An agent skill that answers lineage questions, "what is downstream of this
model", "which sources never reach that layer", runs a dbt selector each time.
The one written for the project this tool is measured on runs `dbt ls`, two
to four times a question, and each call parses the project again: 14 to 20
seconds with dbt Fusion 2.0.6 on its 19 233 nodes, measured on 2026-10-10. It
also needs the project's Python environment and a profile, and half of that
skill works around differences between dbt-core and Fusion on the same flags.

The engine here answers the same selector from the manifest in milliseconds
(0024), and is compared with `dbt ls` on two projects (0033, 0056). But the
only way in was the API, which wants a port, the key the launch printed, and
a process to stop afterwards, and 0052 keeps other programs off it on purpose.

## Decision

- **`--select LINE` and `--selector NAME` resolve, print and exit.** They are
  decided before anything a server needs: no port bound, no key drawn, no
  browser, no settings read or written, no catalog, no column lineage cache,
  no watcher. `--manifest` still says which manifest.
- **The page's entry points, in the route's order.** A `--selector` inside the
  line wins, then `select::select` with tests counted in. A pasted
  `dbt ls -s ...` is stripped as the box strips it. So the command line and the
  box cannot disagree, and the comparison with dbt asks both.
- **dbt's defaults, not the box's.** Tests are in, as in `dbt ls`, each joining
  when one of its parents is selected; the box leaves them out until the eye is
  opened. `--output name` prints what `dbt ls --output name` prints, the whole
  answer with no cap. `--output json` prints one object per line with dbt's key
  names and `depends_on.nodes`, the parents, sorted, since the edges are what
  `dbt ls --output name` cannot give.
- **stdout is the answer, stderr what it is worth.** The manifest's path, its
  date and the dbt that wrote it, then the freshness badge's verdict (0025)
  with the files newer than the manifest. The answer is the manifest's, so a
  caller that edited models has to be told. It is computed after the answer is
  written, since the walk and git are most of the time on a large project.
- **Three exit codes**: 0 answered, empty included; 1 refused, with the
  sentence the page shows and a caret under the term; 2 nothing could be asked,
  which is also what clap returns for a wrong flag.

## Rejected

- **A server started by the caller and asked over HTTP**, as
  `scripts/compare_with_dbt.py` does: a port, the key scraped from stderr, and
  a process to clean up, for every question. 0052 made that awkward for a
  reason.
- **A library crate, or a Python binding**: a second public surface to keep
  stable, for callers that only want an answer.
- **An MCP server**: a protocol and a long-lived process, where one command
  already answers.
- **A `dbt-edith ls` subcommand**: the project is a positional argument with a
  default, so `ls` would be read as a project folder of that name, and one
  shape of command line is easier to explain than two.
- **The box's own default for tests**: an answer that differs from `dbt ls`
  by every test would be compared with dbt by subtracting them, which is the
  workaround this is meant to remove.

## Consequences

The fidelity risk 0024 accepted now reaches whatever a skill decides on the
answer, a deletion included. The comparison with dbt asks the command line
every selection it asks the box, so the two cannot drift, and a skill should
still confirm with `dbt ls` before an action it cannot undo. A stale manifest
answers stale, and only stderr says so. No skill ships here: writing one is
the caller's work, and this is the command it would call. On that project the
same answer took 0.9 s, 0.16 s of it loading the manifest and the rest the
freshness walk and git.
