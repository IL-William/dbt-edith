# 0088. A column is answered on the command line, from the cache already written

Date: 2026-10-15 · Status: accepted · Follows 0057 and 0083

**Trigger:** read before letting anything but the page walk the column lineage,
or before letting a command line produce one.

## Context

0057 gave an agent a model's lineage in one command. The question that follows
is a column's: what does changing it reach. collin can answer none of it alone:
it parses a whole project in seconds and writes a cache, and it knows its own
edges only, not Fusion's or Snowflake's. The walk over a cache, with its depth
and its stops, is this binary's (0083), and so is the manifest a name like
`orders.amount` has to be found in.

## Decision

- **`--column MODEL.COLUMN` walks, prints and exits**, as `--select` does: no
  port, key, browser, settings or catalog. `--up` and `--down` take a number
  of levels or `all`, 0 by default, with no cap on what is reached;
  `--stop-at` takes the roles of the stop at menu. The walk is
  `Graph::column_lineage`, so the canvas and the command line cannot disagree.
- **One JSON line per model reached**, with dbt's key names, its
  `original_file_path`, its `materialized`, and only the columns reached, each
  with its depth, the columns it is read from with the role of each edge, its
  tests, and how many edges the answer leaves out where the depth or a stop
  ran out. A model touched in thirty columns costs one line, and an edge is
  written once, at the column it reaches, so a rename is followed.
- **Nothing is produced here.** collin is never started and Snowflake never
  asked. The cache is the one `--column-lineage` names, the newest of
  `--column-tool`'s, else collin's or Fusion's newest, since Snowflake's
  holds only the columns fetched from the page, else the one the page loads
  with nothing picked.
- **A cache older than the manifest still answers**, and stderr says so with
  the steps that would write one, the target written `<TARGET>`: which target
  it must describe is the user's to say, since the manifest, the catalog and
  collin's run must all come from it, under its own `.env`. stderr lists the
  profile's target names and the `.env` files to choose from, and the command
  of the dbt that wrote the manifest: Fusion's `docs generate` writes no
  catalog unless told. collin's report from the same run says whether it set
  files aside as compiled for another target.
- **The exit codes of 0057.** No cache at all is 2, with the same steps.

## Rejected

- **In collin**, as `collin lineage`: its edges only, the walk written twice,
  and no manifest to find a model's file or tests in.
- **Running collin when the cache is stale**: seconds of CPU and a write into
  `target/` behind a question, against a target nobody named.
- **One line per edge**: the model's path and type repeated on every column.

## Consequences

An answer is the cache's, so a stale one answers stale and only stderr says
so. A cache written from the page records no target until Regenerate offers
one, which is deferred. Discovery now asks a cache for `edges`, so the report
`collin generate` writes beside its cache is no longer offered as one.
