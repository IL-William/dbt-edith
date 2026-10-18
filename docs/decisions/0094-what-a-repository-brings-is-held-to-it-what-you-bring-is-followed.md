# 0094. What a repository brings is held to it, what you bring is followed

Date: 2026-10-16 · Status: accepted · Amends 0017, 0042 and 0049

**Trigger:** read before running or opening a file found beside dbt-edith or in
the project, starting a program by its name, or writing a file that may be a link.

## Context

The security review of 2026-10-15 found a repository's file taken for the
user's, and the reverse. A `dbt-edith.exe` copied into a project ran the
`collin.exe` committed beside it (0042), its `--version` at every page load. A
project's `profiles.yml` linking to a key was served, redaction finding no key
in it, and the Snowflake script's error listed its characters. A
`~/.dbt/profiles.yml` linked from dotfiles became a plain file at its first
save. `files::confined` judged a link through a missing folder by its spelling,
and on Windows `git`, `cmd` and `python` were looked for beside the binary first.

## Decision

- **A collin git tracks never runs**, beside dbt-edith or on the PATH, as a
  venv git tracks never does (0053): `git::tracks` asks in its folder, case
  aside and as spelt, kept against the file and the index. In a repository git
  gives no answer for, one is passed over with git's reason, the failure kept
  five seconds, a timeout a minute. The menu names the one passed over and the
  one in use.
- **A program started by its name is found on the PATH alone**, an absolute
  entry of it, never in the binary's own folder.
- **A `profiles.yml` in a folder of the project is held to it** by
  `files::resolve`, however dbt found it there. One linking out is refused by
  the server and the script alike, which walk it by the same rule, the route
  with a 403. `DBT_PROFILES_DIR` naming the folder it links to opens it.
- **A profile outside the project is followed through its links**, as dbt
  follows them, and any profile is saved through them, the link kept.
- **A path not there yet is walked name by name**: every link on the way
  followed, eight at most, each `..` taken against the folder reached and
  refused after a missing one. On Windows a colon in any segment is refused.

## Rejected

- **Falling back to `~/.dbt`.** dbt reads the linked file, not that one.
- **Following links in `write_atomic` for every caller.** A cache under
  `target/` would be written through a link the repository commits.
- **Trusting a collin by its hash or its folder.** Each build has its own hash,
  and the VM's copy goes wherever the binary goes.

## Consequences

A collin copied over a tracked one is refused, and so is the user's own in a
repository git will not read until `safe.directory` lets it; a venv there still
runs (0053). A `profiles.yml` linking to a monorepo's shared one needs
`DBT_PROFILES_DIR`. A save still breaks a hard link.
