# 0094. What a repository brings is held to it, what you bring is followed

Date: 2026-10-16 · Status: accepted · Amends 0017, 0042 and 0049

**Trigger:** read before running or opening a file found beside dbt-edith or in
the project, or writing a file that may be a link.

## Context

The security review of 2026-10-15 found a repository's file taken for the
user's, and the reverse. A `dbt-edith.exe` copied into a project ran the
`collin.exe` committed beside it (0042), its `--version` at every page load. A
project's `profiles.yml` linking to a key was served, redaction finding no key
in it, and the Snowflake script's error listed its characters. A
`~/.dbt/profiles.yml` linked from dotfiles became a plain file at its first
save. And `files::confined` judged a link through a missing folder,
`nodir/../..`, by its spelling.

## Decision

- **A collin git tracks never runs**, beside dbt-edith or on the PATH, as a
  venv git tracks never does (0053): `git ls-files` is asked in its folder.
  The search goes on to the next one, and the column lineage menu and the
  Columns tab name the one passed over. The user's own, untracked, runs.
- **A `profiles.yml` in a folder of the project is held to it** by
  `files::resolve`, however dbt found it there. One linking out is refused by
  the server and the script alike, the route with a 403, the button saying
  why. `DBT_PROFILES_DIR` naming the folder it links to opens it.
- **A profile outside the project is followed through its links**, as dbt
  follows them, and any profile is saved through them: the file at the end,
  held as above, takes the text and keeps its mode, and the link stays.
- **A `..` after a missing folder is refused**, and on Windows a colon in any
  segment: `C:x` is relative to drive C's current folder.

## Rejected

- **Falling back to `~/.dbt`.** dbt reads the linked file, not that one.
- **Following links in `write_atomic` for every caller.** A cache under
  `target/` would be written through a link the repository commits.
- **Trusting a collin by its hash or its folder.** Each build has its own hash,
  and the VM's copy goes wherever the binary goes.

## Consequences

A collin copied over a tracked one is refused, git tracking the path; where git
cannot answer, nothing is passed over. A `profiles.yml` linking to a monorepo's
shared one needs `DBT_PROFILES_DIR`. A save still breaks a hard link.
