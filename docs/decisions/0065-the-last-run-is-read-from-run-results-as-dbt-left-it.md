# 0065. The last run is read from run_results.json as dbt left it

Date: 2026-10-11 · Status: accepted · Amends 0064

**Trigger:** read before changing what the Last run section says, what leaves
`target/run_results.json`, or when it is read.

## Context

Whether the project's last build went through was a question for the
terminal that ran it, gone once its window was. dbt keeps the answer in
`target/run_results.json`: each node it ran, how that ended, how long it took,
dbt's message. The Reports panel (0064) has room for it, and its button is
where a failure should be said.

## Decision

- **The file is read as dbt left it, and never kept.** dbt writes it again on
  every command that runs or compiles nodes, so it describes one invocation.
  The section names the command and its selection, says when the file was
  written, and says so when `compile`, `show`, `docs generate` or
  `run-operation` wrote it over a run.
- **Only the selection leaves.** `--select`, `--exclude` and `--selector`, and
  never `invocation_command`, `vars` or `env`, where a secret would sit, nor a
  result's `adapter_response` or compiled code (0012). A message is cut to
  600 characters: the first lines, not the log. dbt's message can quote a line
  of the compiled SQL, which the Run tab already shows whole.
- **A node absent from the file was not run**, which says nothing about it.
  Each kind is counted as how many ran of how many the manifest holds, an
  ephemeral model aside since it never runs on its own; what did not run is
  never listed. A result is gone from the manifest only for a kind the manifest
  holds: unit tests are read from their own section of it, and a macro run by
  `run-operation` or `show --inline`'s query never was a node.
- **Whose run it is is said, not guessed.** The run and the manifest are the
  same invocation when their ids agree; a manifest written since says the
  project may have moved on; a table named otherwise than in the manifest
  says the run was likely for another target.
- **Every status is read in a few words**, success and pass alike as ok, and
  one this tool does not know is shown as dbt wrote it, Fusion's spelling
  included.
- **Errors and failed tests are news on the Reports button**, under 0059's
  rule: only for a file written since the section was last shown. To count
  them, the index parses the file, once per write, through the cache the
  section reads too: 0064's index otherwise reads file times alone.

## Rejected

- **A history of runs.** It would be kept by this tool, which keeps nothing of
  the project's, and `dbt_artifacts` and dbt Cloud already keep one.
- **Statuses on the canvas.** Worth doing once the section has been used; a
  stroke for a run that may be for another target says more than it knows.
- **Reading `logs/dbt.log` in this section.** The warnings dbt logs are a
  section of their own, planned as deferred item 7: this one reads the
  results file alone, which says how each node ended and nothing more.
