# 0099. A column may run collin first, when the command line asks

Date: 2026-10-19 · Status: accepted · Amends 0088 and 0042

**Trigger:** read before letting the command line start a program, before
changing what `--run-collin` hands collin or where it writes, or before
making `--column` produce lineage without being asked.

## Context

0057 and 0088 gave a script or an agent a model's lineage and a column's in
one command each. The column's still needed a cache written beforehand, which
on a project nobody opened in the page meant three steps by hand: a dbt
command, then `collin generate` with five paths, then `--column`. Asked for the
lineage of a project from its artifacts alone, a manifest and the SQL dbt
compiled, an agent had none of it: no cache, a stderr asking which target to
compile for, and collin writing its cache, until 0.9.0, where the command ran
rather than beside the manifest `--column` reads.

0088 rejected running collin when the cache is stale: seconds of CPU and a
write into `target/` behind a question, against a target nobody named. Both
halves hold for a question that does not ask for it. They do not for one that
does: the write is the point, and the target is the one the manifest was
compiled for, whichever it is, which is what a question about these artifacts
wants answered.

## Decision

- **`--run-collin` runs collin once, before the answer**, then answers from
  the cache it wrote. It goes with `--column` only, and never with
  `--column-lineage` or `--column-tool`, which name another cache.
- **It runs once the model is known.** The model is the manifest's to name,
  so a name it does not hold is refused, exit 1, before collin is started:
  run first, a typo would rewrite the page's cache for nothing.
- **It runs every time it is asked.** Freshness is judged by time alone (0042),
  and a cache newer than the manifest may come from another catalog or another
  folder of SQL: a caller asking twice in a row pays collin twice, in seconds
  on the largest project measured, and asks the second time without the flag.
- **It writes what the page's Regenerate writes**: `column_lineage.collin.json`
  and `collin.report.json` beside the manifest, under a temporary name and
  renamed, so the page then shows this run and a failed run leaves the old
  cache standing. The temporary names carry the process's id: the page's
  runner keeps its own runs to one at a time, but cannot wait for another
  process, and each run starts by removing what its temporary name holds.
  collin is the one found beside the binary or on the PATH, never one git
  tracks (0094); the version the page picked for the project (0060) is in the
  settings, which this mode does not read.
- **It hands collin what the command line names**: the manifest, `--catalog`
  else `catalog.json` beside the manifest, and `--compiled DIR`, the folder dbt
  compiled into, else nothing, so collin reads `compiled/` beside the manifest
  (dbt-collin's 0046), and its own `--indirect` when the answer is asked to
  cross indirect edges (0097), which collin writes only when told. Every path
  is made whole first: collin runs in the project, and the paths are relative
  to where the command was typed.
- **`--compiled` needs collin 0.9.0**, the first to read it. A collin older
  than that is never started with the folder, and stderr gives the update
  command. `WANTS` is not raised for it: someone who never names a folder is
  never sent to 0.9.0. A folder that is not there is refused before collin
  runs, as collin 0.9.0 refuses it: it would read no file, and every model it
  was named for would come out matched by name over the page's cache. So is
  a `--catalog` named and not there, which would leave every column
  unconfirmed. A collin older than `WANTS` still runs, as the page runs one it
  offers to update, and stderr says so with the update command.
- **stdout stays the answer's.** One line on stderr says which collin wrote
  which file, in how long, and how many models it read the SQL of; collin's
  own lines come only when it fails, with exit code 2. When collin found no
  compiled file for some models the manifest has no SQL for, stderr says how
  many and where it looked, since their edges are name matches the answer
  does not show. The steps printed when there is no cache, or a stale one,
  now end with this flag, unless `--column-tool` or `--column-lineage` named
  the cache, which the flag cannot go with.

On collin's Jaffle Shop manifest with its `compiled_code` taken out, as a
`parse` leaves it, copied alone into a folder, its compiled SQL in another,
asked from a third with no project in sight: `--column stg_payments.amount
--down all --run-collin --compiled ../sql/compiled` ran collin 0.9.0 in 50 ms,
read the SQL of 5 of 5 models, wrote 31 edges, all parsed, and answered with
three models and seven columns: `amount` in `stg_payments`, its aggregates in
five columns of `orders` and in `customer_lifetime_value`. The walk is the
canvas's, so the page draws the same, which was not driven here.

## Rejected

- **Running collin only when the cache is missing or older than the
  manifest**, as picking Collin does: a first run without `--compiled` leaves a
  cache of name matches newer than the manifest, and the second, with it,
  would answer from that one without a word.
- **A separate `dbt-edith generate`**: the project is a positional argument
  with a default, the reason 0057 gave against `ls`, and it is
  `collin generate` again.
- **Passing the version of collin the page picked (0060)**: the settings are
  the page's, and the command line reads none, so that an answer does not
  change with what a browser tab last clicked.
- **Recording a target**: the manifest does not say which one it was compiled
  for, the deferred "A target for Regenerate".
- **An MCP server**, asked again for an agent: 0057's reasons hold, a
  protocol and a long-lived process where one command answers.

## Consequences

The command line can now start a program, when asked by name, and the rule of
0042 reads "never at startup, and from the command line only when
`--run-collin` asks". 0088's "nothing is produced here" holds for every
question without the flag. A cache this writes describes the manifest's own
target and says "target not recorded", as the page's do.
