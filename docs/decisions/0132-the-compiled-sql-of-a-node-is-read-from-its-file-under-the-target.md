# 0132. The compiled SQL of a node is read from its file under the target

Date: 2026-10-30 · Status: accepted · Amends 0102 · Builds on 0057 and 0110

**Trigger:** read before changing what `--compiled-sql` prints or reads, before
letting the command line or MCP read another file of the target folder, or
before reading `compiled_code` from the manifest.

## Context

`--sql` (0102) prints the node's own file: the Jinja as written. An agent that
asks what a model really selects, which table a `ref()` became, or what a macro
expanded to, had only the page's compiled tab, or `target/compiled` found by
hand, where a generic test's file name is a truncated name plus a hash that
nothing derives (the manifest records it). The page already answers this: its
compiled SQL route looks the file up, confined to the target folder, and says
how fresh it is.

## Decision

- **`--compiled-sql`, with `--describe`, as `--sql` is.** After the card, and
  after the node's own file when both are asked for, it prints the compiled SQL
  under a `compiled sql` heading, indented, for every node in turn. In JSON it
  is the key `compiled_sql`, only when the file was read. It goes with
  `--describe` and is refused beside any other mode, like `--sql`.
- **The file dbt wrote, found by the page's lookup.** `compiled::look_up`, the
  one the compiled tab uses, over the folder that holds the manifest (`target/`
  by default, wherever `--manifest` puts it). A path the manifest records is
  held to that folder, as a path the browser sends is to the project: a `..`, an
  absolute path or a link out of it is not read. The two files this mode reads
  are therefore the node's own, in the project, and its compiled one, in the
  target folder, both confined.
- **Said on stderr, never in the answer.** A node with no compiled form (a seed,
  a source, an exposure) says `no compiled SQL of its own`. A node dbt has not
  compiled, or whose file is gone, says where it looked and the command that
  makes it, `dbt compile -s NAME`. A file older than the model file, its properties
  file, `dbt_project.yml` or the newest macro under `macros/` is printed and
  said to be older, the page's own rule. The card is still printed
  and the exit code is `--sql`'s: unchanged by a file.
- **A changed macro is named by its path.** The note and the page's compiled
  and run tabs say `the macro macros/dates/fiscal_year.sql`, not the file name,
  which two folders can share. When two macros changed at the same second the
  first by path is named, so the note is the same on every run (0055).
- **Over MCP, allowed.** No word of the line becomes a path (0110): the target
  folder is the one beside the server's manifest.

## Rejected

- **`compiled_code` from the manifest.** It is only there when dbt compiled the
  node while writing that manifest (`dbt parse` leaves none), it makes
  `--describe` deserialize the bulk of the manifest for one answer (the reason
  0102 gave against `raw_code`), and it would answer from a file that is not the
  one the page shows, so the two could disagree.
- **A value on `--sql`** (`--sql compiled`, or `both`). It would make one flag
  two modes, with a value to check beside the name: `stray` and the MCP
  allow-list (0110) admit and refuse flags by name. Two switches ride the lists
  as they are, and `--sql` keeps meaning what it meant.
- **Reusing `--compiled DIR`.** It names the folder collin reads SQL from and
  belongs to `--run-collin` (0099); MCP refuses it because it is a path.
  `--compiled-sql` takes none.
- **Running `dbt compile` for the caller.** The binary never runs dbt (0002).

## Consequences

The answer can be older than the model: only stderr says so, as for the
manifest. The check compares four things, so it misses a change to a source's
YAML or to a parent's alias or schema, which also change the compiled SQL. A project compiled with `--target-path` elsewhere is found only when
the manifest sits beside it, as the page finds it. The compiled file is read up
to what the page reads of one, 2 MiB, and stderr says when the end is left out.
