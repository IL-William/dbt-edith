# 0043. collin is updated by the command that installs it, and its version decides how

Date: 2026-10-06 · Status: accepted, amended by 0060 and 0098 · Amends 0042

**Trigger:** read before raising the collin version dbt-edith accepts, or
changing what the menu offers once collin is installed.

## Context

0042 installs collin by typing `cargo install --git` into the terminal, and
said nothing of updating it. Rerunning that command already updates: cargo
checks the remote, answers "already installed" in half a second when the
commit is the same, and rebuilds when GitHub has a newer one. Nothing offered
to, and dbt-edith could not tell an old collin from a new one: collin had no
`--version` before its 0.2.0.

## Decision

- **dbt-edith holds the oldest collin it accepts**, `WANTS` in
  `src/collin_run.rs`, raised when dbt-edith relies on something a later
  collin does. It starts at 0.2.0, the first to answer `--version`.
- **It asks `collin --version` locally**, once per binary, bounded at five
  seconds. Older than `WANTS`, or no answer: the menu offers "Update collin",
  the install command with `--force`, so a collin installed from a path or
  copied beside the binary is replaced, and the Columns tab says so first.
  *Amended by [0098](0098-collin-is-wanted-at-0-8-0-and-a-version-installed-by-tag-is-never-called-older.md):
  never while a version installed by its tag runs, whatever its number.*
- **Otherwise the menu offers "Check for a newer collin"**, the plain command,
  and cargo decides.
- **Either is a line indented under Collin, shown while Collin is the tool
  picked**, with no check and no dot: an action on that tool, never a tool of
  its own, which a separate row with Collin's colour read as.

## Rejected

- **dbt-edith asking GitHub for the latest collin.** It calls nothing outbound
  (0008), as 0042 already rejects downloading; cargo makes the check when the
  user runs the command.
- **`--force` on the check too.** It rebuilds for a minute even when nothing
  changed.
- **Pinning the install to a release tag.** The check could then never find
  anything newer than the tag dbt-edith was built with.
  *Amended by [0060](0060-collin-versions-sit-side-by-side-and-one-is-picked.md):
  other versions install at their tag beside the default, which this check
  still updates.*

## Consequences

Until collin's 0.2.0 is on its `main`, every collin reads as older, and the
update installs the same one. Raising `WANTS` is how a dbt-edith release says
which collin it needs, and it belongs in the same change as what needs it.
