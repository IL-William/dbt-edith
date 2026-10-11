# 0063. A slow git status is answered with a setting you run

Date: 2026-10-11 · Status: accepted

**Trigger:** read before passing a setting to git, changing what the git panel
suggests, or starting anything of git's that outlives a command.

## Context

The status the page polls every 5 s spends most of its time looking through
every folder for new files: on a project of about 10 000 files, measured on
2026-10-11, some 190 ms of which 170 went to that search. git can remember
which folders did not change, `core.untrackedCache`, which brought it to tens
of milliseconds there with the same answers. The cache lives in the index, so
once built, every git command in the clone uses it, not only dbt-edith's.

## Decision

dbt-edith never turns it on. Once git's status has been slow for two polls
running (100 ms or more, and until one comes under 50), and nobody set
`core.untrackedCache` or `feature.manyFiles` for the clone, the git panel
shows `git config core.untrackedCache true` with a Copy button, for the user
to run. Any value set, `false` included, is a choice, and the note goes. The
config is read only once the status is slow, so a quick repository pays
nothing for it.

## Rejected

- **Passing `-c core.untrackedCache=true` to dbt-edith's own status.** No
  config is written, but the first call builds the cache into the index, which
  the user's own git then keeps using: a change to their repository nobody
  asked for. On a file system whose folder dates cannot be trusted, a network
  share for one, new files would then go unseen, in dbt-edith and in git.
- **`core.fsmonitor`.** It speeds the comparison of tracked files, a small part
  of the time here, and only with the cache does it help the search. It starts
  a daemon per repository that outlives dbt-edith.
- **Listing no untracked files (`-uno`).** A new model would lose its colour in
  the explorer, and the freshness badge would call it committed.
- **A threshold on the number of files.** How slow a status is depends on the
  machine and its disk as much as on the repository.

## Consequences

The first status after the command builds the cache and is the slowest; the
note is already gone, since the setting is then chosen. On a slow disk the note
can show where the cache helps less than it did here. Running it costs about a
quarter of a megabyte of index on a project this size, and `false` takes it
back.
