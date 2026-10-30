# 0133. A macro is described when no node has the name

Date: 2026-10-30 · Status: accepted · Amends 0102 · Builds on 0084 and 0105

**Trigger:** read before changing what `--describe` accepts as a name, the
order a name is looked for in, what a macro's card says, or what the command
line reads of a macro's file.

## Context

`--describe` took nodes only (0102), so an agent asked "what does this macro
take, and who calls it" read `macros/*.sql` and grepped for the calls the
manifest already counts (0105). The page answers that on a macro's hover card
and in Reports; the command line and the MCP tool, which run the same code
(0110), did not. Two things were open: a name a node and a macro share, and
which macro a bare name stands for, when dbt itself resolves a call from the
root project and a package may define the same name.

## Decision

- **`--describe NAME` is a node first, and a macro when no node has the name.**
  A node wins a clash, so an answer never changes because a package brought a
  macro of the same name. A name several nodes share is refused as before,
  never settled by a macro. Nodes and macros mix in one call, each printed
  once.
- **A macro is named three ways**: its unique_id (`macro.pkg.name`);
  `package.name`, which names it outright; and a bare name, resolved as dbt
  resolves a call from the root project: the root project's macro, then the
  adapter's package (`dbt_<adapter>`, a parent adapter's after it), then
  `dbt`. A package's macro is never reached by its bare name, since dbt does
  not reach it either (`Macros::resolve` already stops there for a call);
  the refusal offers `package.name` instead. Case counts, as in a call.
  `dbt.x` named outright is dbt's own `macro.dbt.x`, which the card shows; a
  call `{{ dbt.x() }}` may reach the adapter's macro of the same name, since
  dbt-core searches its merged internal namespace with the adapter's first.
- **A node wins `package.name` too**, since a package node's unique_id ends in
  it, so only the macro's unique_id always reaches the macro. When a node wins
  a name a macro also has, stderr says `name: a macro too, macro.pkg.name`,
  after the cards, on the command line and over MCP alike.
- **A name that is nothing says it looked among macros too**: `no node or
  macro named X`, with the nearest nodes first and the nearest macros after,
  a package's as `package.name`, five names in all.
- **The card** is the first line (`macro`, the name, `path:line`), then the
  package, the arguments, the description and `used by`, in the words of the page's card
  (0105): the summary the page's `macro_used` sends, so a macro no node
  reaches says `no reference found` and why, never `unused`, and the callers
  are cut where the page cuts them. JSON is one object with `kind: "macro"`,
  which no node has, the summary as `used`, and `line` and `end_line`.
  `--output compact` prints the card, as it does for a node. The arguments are
  the names the opening tag writes, read from the file the card reads for the
  line: the manifest lists only those the YAML documents, so most macros, and
  every one of dbt's, would read as taking none. The YAML's names are used when
  there is no tag to read, and `not documented` says so when there are neither.
  A materialization takes none, as dbt gives it no arguments. In JSON the key
  is `[]` for a macro known to take none and absent when nothing lists them.
- **The file is read for the line**, even without `--sql`, through the read
  `--sql` uses, confined to the project. That amends 0102, which let `--sql`
  alone read a file: the line is what makes the card a place to open, and the
  page finds it by reading the file too (0084). `--sql` adds the definition's
  own lines, from its opening tag to its closing one, a generic test and a
  materialization included, under the names dbt gives them; a macro written
  inside another ends at its own end, and a `{#` inside a tag's string or a raw
  block opens no comment, as the editor reads them. A macro of dbt's own,
  which the project holds no file of, has no line, and its first line marks the
  path `dbt's own:` so it is not opened in the project; an uninstalled
  package's says to run `dbt deps`, an installed one whose file is gone says
  so, and one linked from outside the project is not read and says so; all
  on stderr under `--sql`, with the exit code unchanged.
- **`--compiled-sql` (0132) has nothing to print for a macro**, which dbt
  renders into the SQL of the nodes calling it: the card is printed and stderr
  says `name: a macro has no compiled SQL of its own`, as it does for a seed,
  with the exit code unchanged.

## Rejected

- **A `--macro` flag**: one more mode for an agent to learn, and to choose
  between when the name is all it has. The refusal and the sample card tell
  which was found.
- **Macros first**: every model named like a macro would change its answer
  with a package update, and the name an agent copies from a lineage is a
  node's.
- **The arguments from the manifest's `macro_sql`**: it is kept for the few
  macros that bear on an incremental strategy, since keeping every body is the
  manifest's bulk again (0102).
- **Searching installed packages for a bare name**: dbt does not, and an
  answer from a package the project's own call would never reach is wrong
  where it looks right. Offering `package.name` in the refusal costs one more
  call.
- **Listing the callers' names on the card**: the page's list is a Reports
  panel because it has hundreds; the card keeps the page's counts.

## Consequences

The line of a macro is one file read per card, whatever the output. A macro
the manifest lists under a path a later commit moved has no line and, under
`--sql`, a note. The agent skill names the macro in its row for `--describe`
within its 4096 bytes.
