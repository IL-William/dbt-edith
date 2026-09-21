# 0025. Manifest freshness is measured in file times, not in commits

Date: 2026-09-21 · Status: accepted

**Trigger:** read before changing what the freshness badge claims, or before
comparing the manifest against a commit.

## Context

The lineage is drawn from `manifest.json` (0002), which dbt wrote at some point
in the past. Nothing on screen says how far in the past, so a graph parsed
before a `git pull` looks exactly like one parsed a minute ago. The question
the user actually has is narrow: can I trust what I am looking at?

The manifest records no commit, and `target/` is not committed, so there is no
sha to compare. The obvious substitute, "was the last commit made after the
manifest was written", is wrong in the most common direction: committing
changes no file, so parsing and then committing would report a manifest that is
still perfectly true as stale. Pulling a month-old commit fails it the other
way.

## Decision

Compare modification times. A manifest is out of date exactly when a file dbt
parses is newer than it, and `src/freshness.rs` walks the resource directories
to find those files. git is asked one question, not two: for each file that is
newer, is it dirty? That splits the answer into the two cases whose advice
differs, and gives the badge its three states.

- **fresh.** Nothing dbt parses is newer. The lineage matches the files.
- **edited.** What is newer is uncommitted, so it is the user's own work in
  progress and they know what changed.
- **stale.** Something newer is clean in git, so it arrived with a checkout, a
  pull or a merge, which is the case nobody has in mind.

Falling behind the default branch is reported beside that answer, in its own
segment, never folded into the colour. A feature branch ten commits behind
`main` still has a manifest that is true for the code in front of you, and
colouring it amber would spend the signal on something that is not wrong.

Deletions move no mtime, so the same walk records which files it saw and the
nodes the manifest names are checked against that set. Only under a directory
the walk covered: a node configured in `dbt_project.yml` names that file as its
own, and it sits at the project root.

## Rejected

- **Comparing against the last commit.** Wrong in both directions, above.
- **Age alone.** A manifest untouched for three weeks on a branch nobody has
  touched for three weeks is correct. Ageing it to amber trains the user to
  ignore the badge, which costs exactly the trust it is there to give.
- **Reading `dbt_project.yml` for the resource paths.** It would need the
  scanner 0018 describes, to answer something the manifest already answers:
  the directories its own nodes live in. `macro-paths` set somewhere
  unconventional is the gap that leaves.
- **Hashing the files.** Exact, and it reads every byte of a few thousand files
  on a timer to improve an answer mtimes already get right.

## Consequences

The badge polls, so both halves are cached for a moment and the walk shares the
`git status` the git panel already runs. A walk restricted to the resource
directories is what makes that affordable: measured on a 3 341 model project,
tens of milliseconds beside the 400 ms `git status` that was already being paid.

Counting how far `main` is ahead needs remote refs, and reading them is free
while fetching them is not. `api::watch_remote` fetches every ten minutes, read
only and deadlined like everything else in 0007, and the badge says when the
refs were last refreshed rather than implying the count is current.

Clicking the badge runs `dbt parse`, in the user's terminal, under their own
environment. It is the one command in the UI that is run rather than typed and
left at the prompt, because it is the whole point of the button; the server
still never runs dbt (0002).
