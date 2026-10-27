# 0121. git runs nothing a repository's own configuration names

Date: 2026-10-27 · Status: accepted · Amends 0053

**Trigger:** read before adding a git command to `src/git.rs`, before running
git on a timer or in a folder the user did not pick, and before extending the
list of keys git may find in a repository's configuration.

## Context

0053 rejected guarding git hooks: a clone brings none, so a hook was installed
by the user. A project copied with its `.git`, as an archive or a shared
folder, brings `.git/config` and `.git/hooks` as well. The status poll runs
`git status` every five seconds, and git then runs what the repository's
configuration names: `core.fsmonitor` to ask which files changed,
`post-index-change` when the index is rewritten, a clean filter for a file it
hashes, `credential.helper`, `remote.*.uploadpack` or `core.sshCommand` on the
timed fetch, a `protocol.ext.allow`. Opening the folder was enough.

## Decision

- **Every call** passes `-c core.fsmonitor=false` and `-c
  submodule.recurse=false`, sets `GIT_SSH_COMMAND` to the batch-mode ssh the
  network calls already used, which wins over the repository's
  `core.sshCommand`, and sets `GIT_NO_LAZY_FETCH=1` unless it is a fetch, a
  pull, a push or a checkout (which, like the timed fetch, is held to the
  https, http and ssh transports): a partial clone fetches a missing object from its
  promisor remote whenever a command wants one, the poll's `status` included.
  Pull and the fetch buttons pass `--no-recurse-submodules`, a submodule
  having a configuration the gate never reads.
- **A call nobody clicked** (the poll, the timed fetch, the questions asked of
  a folder before something is run) also passes `-c core.hooksPath=/dev/null`.
  The timed fetch adds `maintenance.auto=false`, `gc.auto=0` and a transport
  list of https, http and ssh, which the repository's own `protocol.*` keys
  cannot widen. The poll's `status` adds `--ignore-submodules=dirty`. A click
  keeps its hooks: they are the user's.
- **A gate before any call.** `git config --list --show-scope --name-only`
  reads, and runs nothing; the keys of scope `local` and `worktree` are held to
  a list of data-only keys (`allowed` in `src/git.rs`). One outside it pauses
  git for that repository: no call is made, the Git panel and the freshness
  card say so and name the key, and `tracks` answers an error, so a collin is
  refused as for any git that gives no answer. A venv is not: `venv::committed`
  reads that error as not tracked, as it does for an archive with no `.git`
  (0053). A repository whose configuration cannot be listed at all is paused
  with git's reason and the pause is not remembered. git in the terminal is
  unaffected. The verdict is kept per root against the mtime and size of
  `config` and `config.worktree`, so the poll adds no process; a file written
  in the last two seconds is read again.
- **Left off the list on purpose.** `include.path` and `includeIf`, since a
  file that appears later or a condition that turns true would change a verdict
  kept against the config alone; the proxies, `http.proxy` and
  `remote.*.proxy`, which with `http.sslVerify=false` route the timed fetch,
  and the credentials it finds, through a host the repository names; and the
  parts of `lfs.*` that run a program, `lfs.extension.*` and
  `lfs.customtransfer.*`.
- **The list assumes edith's fixed commands**, all builtins: no alias, merge,
  diff driver, mergetool or signature check of the repository's choosing. A
  command that reaches one of those shrinks the list first.

## Rejected

- **A trust prompt per repository.** The same click everyone learns to make.
- **Neutralising only the keys known today.** A list of the program-naming keys
  is one git release behind; a list of the data-only keys fails closed.
- **Judging the global and system configuration too.** They are the user's.
- **Following includes.** It needs a verdict kept against every file reached,
  and a condition such as `onbranch` can change without any file changing.

## Consequences

A repository that keeps `filter.lfs.*`, a `credential.helper`, an `include` or
a proxy in its own configuration is paused until the key moves to the user's
global one. A remote on a local path is not refreshed by the timer. Where the
user's own `core.sshCommand` set a key or an option, git run from here no
longer uses it; the network calls already did not. The bare `Not a git
repository` is replaced by the sentence naming the key.
