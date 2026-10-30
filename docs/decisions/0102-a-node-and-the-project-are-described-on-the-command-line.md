# 0102. A node and the project are described on the command line

Date: 2026-10-19 · Status: accepted, amended by 0110, 0113, 0114 and 0132 · Follows 0057 and 0088

**Trigger:** read before adding to what the command line prints of a node or
the project, or before letting it read a file of the project.

## Context

0057 and 0088 gave an agent a selection and a column in one command each, and
nothing on what a node is or on the project's shape: its description, columns
with types and tests, where it is built, how an incremental model merges, its
SQL. So it read files. And `--select`'s JSON spent half its bytes on what the
name and the path already say: the unique_id is the kind and the name.

## Decision

- **`--describe NAMES` and `--overview` answer from the graph the Catalog is
  painted from**, the catalog merged in, so the page and the command line cannot
  disagree. Text by default, since a card is read, JSON on demand. Several names
  in a call, each resolved as `--column` resolves a model's, a test or an
  exposure now a node that can be named too.
- **`--output compact` drops what can be derived**: a tab-separated line per
  node or per column under a header, which `cut -f` reads.
- **`--sql` is the one file read**: the node's own, through the read the page
  uses, confined to the project root. Still nothing served, stored or run.
  *Amended by [0113](0113-the-card-resolves-a-location-per-env-file.md): the
  card reads the project's `.env` files too, for the locations they resolve.*
  *Amended by [0132](0132-the-compiled-sql-of-a-node-is-read-from-its-file-under-the-target.md):
  `--compiled-sql` reads a second file, the node's compiled one under the
  target folder.*

## Rejected

- **An MCP server**: a protocol and a long-lived process, where one command
  answers. It may wrap these later.
  *Amended by [0110](0110-an-agent-may-ask-over-mcp-through-one-tool.md):
  it wraps them, in one tool, when the client is configured for it.*
- **Full cards from `--select`**: one card per node is the JSON's size again.
- **`raw_code` from the manifest for `--sql`**: it is deserialized for every
  node, the bulk of the manifest, for one answer.
- **A skill here**: the caller's work (0057); the recipes page is its text.
  *Amended by [0114](0114-a-skill-is-shipped-and-printed-never-installed.md):
  a skill now ships as a file and `--skill` prints it.*

## Consequences

A stale manifest answers stale, and only stderr says so. A card is the
manifest's view: a column only the warehouse has shows when a catalog is there.
