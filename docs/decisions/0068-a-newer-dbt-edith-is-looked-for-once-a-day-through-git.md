# 0068. A newer dbt-edith is looked for once a day, through git

Date: 2026-10-12 · Status: accepted, amended by 0079 and 0095

**Trigger:** read before making dbt-edith reach the network on its own, or
changing when it looks for a release or what the update offer types.

## Context

Updating is `git pull` then `cargo install --path .`, and nothing said when
there was something to pull: a release was found by looking at GitHub. 0042
rejected downloading collin, and 0043 and 0060 asking GitHub about it, since
dbt-edith "calls nothing outbound" (0008). That was already not quite so: the
freshness badge fetches the project's own remote every ten minutes (0025),
through git.

## Decision

- **git lists dbt-edith's tags**, `git ls-remote --tags --refs` on its public
  repository, run from the temporary directory with no repository looked for
  above it. No HTTP client (0003), no account, and nothing about the project
  goes with it: the request is the one a pull of dbt-edith would make, and the
  machine that updates it already pulls. The user's git configuration applies,
  a proxy above all. The newest `vX.Y.Z` is the release; any other tag is not.
- **Once a day for the whole machine.** The answer is kept with its time in
  the config directory (0011), and a start within the day takes it from there.
  Starts that overlap before an answer is kept each ask once: a lock would cost
  more than the request. A failed look waits the day out in its process and is
  kept nowhere, so the next start tries again. It runs in the background and says nothing when it
  fails. `--no-update-check` asks nothing and shows nothing.
- **A newer one is a pill after the version in the status bar**, whose menu
  links the release's notes (0045) and, for a binary built from a clone still on
  this machine, types the update into the terminal for the user to run, as
  collin's install is (0042). The pull names `origin main`, where every release
  is tagged, whatever branch the clone has out; one that cannot fast-forward to
  it stops the command there. One `cargo install` put in a `bin/` beside
  cargo's records is pulled and installed again; one left in the clone's
  `target/release/` is pulled and built again. Anything else, a copy carried
  from another machine above all, gets the notes, no command, and the reason.
  Telling these apart looks at which files exist there, never inside them.
- **On Windows the running `.exe` is renamed to `.exe.old` first**, which
  Windows allows where replacing it is refused, and renamed back when the build
  fails. The page then says to restart dbt-edith; the next update replaces the
  `.old`. This frees `cargo install`'s copy, not a build in the clone: cargo
  links `target/release/` from the file it writes in `deps/`, the same file
  under another name, which the rename leaves running. That one gets no command
  on Windows, and the page says to stop dbt-edith before building.
- **The command is written for sh, bash and zsh (Git Bash included) or for
  PowerShell**, by the terminal's shell. Any other gets none: `cmd` has neither
  language, fish and nushell read the command otherwise, and `wsl` would look
  for a Windows path inside Linux. A path holding a character either shell
  would read inside double quotes gets none either, `!` and, on Windows, a
  bracket among them.

## Rejected

- **The page asking GitHub's API.** Its CSP allows `'self'` alone (0015);
  opening it to a host for one request weakens the rule every route relies on.
- **Asking at every start.** Several starts a day would each reach GitHub to
  learn the same thing.
- **Downloading and replacing the binary.** No release carries one (state.md),
  and running an update unasked is what 0042 refuses for collin.
- **`cargo install --git` at the tag.** It rebuilds from GitHub rather than the
  clone the README installs from, and leaves that clone behind its binary.
- **Pulling the branch the clone has out.** A clone left on a feature branch
  would build that branch, not the release the pill names.

## Consequences

A machine where git cannot reach GitHub never shows the pill, and loses
nothing. The Windows rename is written from Windows' rules and tested on a
Mac; the VM is where it is proven.
