# 0042. collin runs on demand, as an executable found beside the binary

Date: 2026-10-06 · Status: accepted · Amends 0023 and 0031

**Trigger:** read before starting collin from the server, changing when it runs,
or bundling any column lineage producer with dbt-edith.

## Context

0023 put the parsing of SQL outside this binary, and 0031 offered Collin in the
column lineage menu. Nothing wrote collin's cache, though: the user ran
`collin generate` by hand and came back to pick it. collin is a Rust workspace
of its own, built on a SQL engine pinned to a git fork, and analyses a whole
project in about five seconds on 3341 models. It cannot answer one column.

## Decision

- **collin stays an executable of its own**, looked for beside dbt-edith, then
  on the PATH. It is not in `Cargo.toml` and not inside the binary, so starting
  dbt-edith costs what it did before, and nobody who ignores column lineage
  builds it.
- **It runs on request and never at startup**: when Collin is picked and its
  cache is missing or older than `manifest.json`, in the background; when a
  column is clicked while that is still so, before drawing it; and on the
  Columns tab's Regenerate button. One run at a time: a request arriving during
  a run waits for it.
- **It writes under a temporary name and is renamed into place**, so the watcher
  never loads half a cache and a failed run leaves the last one on screen. Its
  report goes to `target/collin.report.json`, a name discovery does not offer.
- **Installing is the user's command.** Collin's entry, without collin and
  without a cache, types `cargo install --git ...` into the terminal and stops
  at the prompt. collin's repository is public, so the clone needs no account.

## Rejected

- **collin-core as a crate.** It brings a SQL parser and a git dependency into a
  binary 0003 keeps small and 0023 keeps free of SQL parsing.
- **Embedding the collin executable** with `include_bytes!`. One file to copy,
  but every build of dbt-edith would build collin first, for every user.
- **dbt-edith downloading collin.** It has no HTTP client and calls nothing
  outbound (0003, 0008), and the VM it runs on may not reach GitHub.
- **Running collin whenever the manifest changes.** Five seconds of CPU after
  every `dbt parse`, for a tool that may not be picked.

## Consequences

A cache is judged stale by time alone: a `dbt parse` that changed no SQL still
makes the next click run collin. collin reads the compiled SQL, so a project
only parsed gives it little to read, and the Regenerate tooltip says to compile
first. A machine without cargo gets collin by copying `collin.exe` beside
`dbt-edith.exe`.
