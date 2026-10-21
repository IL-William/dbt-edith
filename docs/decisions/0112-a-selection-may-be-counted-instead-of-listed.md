# 0112. A selection may be counted instead of listed

Date: 2026-10-21 · Status: accepted · Follows 0057 and 0110

**Trigger:** read before adding a key to `--count-by`, before letting another
mode count, or before answering a question over MCP that a recipe would pipe.

## Context

Two questions an agent asks of a project are counts: how many models sit in
each layer upstream of one, and which sources feed it. The command line
answered them with a list piped to `cut | sort | uniq -c`, and the MCP line
cannot (0110): it takes no shell, so a pipe is refused. An agent over MCP could
only read the whole list and count it itself, which spends the tokens a count
exists to save.

## Decision

- **`--count-by KEY` on `--select` or `--selector`**, with `kind`,
  `materialized`, `source`, `folder`, `tag` or `package`. It resolves the
  selection as a list does, with the same exit codes and stderr, and prints
  counts in its place. An empty selection is exit 0 and no row.
- **One shape for every key**: `count` and the key as a header, a row per value,
  the nodes selected as `(all)`. `name` and `compact` print the same text,
  `json` one object with `by`, `total` and `counts`. Rows go by count then
  value; `folder` goes in path order, so numbered layers read in order.
- **`--depth` is reused for the folders**, as `--overview` has it, default 3,
  and is refused with any other key.
- **A source is counted by the name the manifest gives it**, kept on the node,
  and never by cutting the node's name at a dot, which breaks for a package's
  source and for a name that holds one.
- **The MCP line allows it**, so the pipe is not needed there either. 0110 stands
  as written: the line still takes no shell.

## Rejected

- **A tool per count**: the cost 0110 turned down, three tools' worth of
  description in every context, for one question each.
- **A pipe in the MCP line**: a shell by another name, and the splitter exists
  to have none.
- **The depth inside the key, `folder:3`**: a second grammar to parse and to
  document beside `--depth`, which `--overview` already has.
