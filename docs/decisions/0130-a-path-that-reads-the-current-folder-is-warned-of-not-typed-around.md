# 0130. A shell that reads the current folder is warned of, not typed around

Date: 2026-10-30 · Status: accepted · Builds on 0071, 0094

**Trigger:** read before changing what the page types into the terminal, which
button presses Enter there, or what the startup banner and Edith's menu say
about the terminal's shell and `PATH`.

## Context

Everything the page types runs in the project folder, in the terminal's shell.
Where that shell reads the current folder, a `git`, `dbt` or `cargo` committed
in the project runs in place of the one meant. The original item
(Deferred 16) said bash and zsh do so through an empty entry of `PATH`, checked
with a planted `git` and a leading `:`. That check used a shell that was not a
login one. dbt-edith starts `$SHELL -l`, and measured on a Mac:

- `path_helper -s` (run by /etc/zprofile and /etc/profile) drops empty entries
  and keeps relative ones: `:/usr/bin::/bin:.:rel` comes out ending `:.:rel`.
- With a planted `fakegit`, `env -i PATH=":/usr/bin:/bin" zsh -l -c fakegit`, and
  the same with `bash -l`, answer command not found, for a leading, a trailing
  and a doubled empty entry; with `.` in the PATH they run it. Without `-l`
  both run it for a leading empty entry.
- Git Bash's msys2 converts the Windows PATH skipping empty elements (read from
  its source, not run here: the Mac has no Git Bash), and a Windows PATH ending
  in `;` is common, so an empty entry there would warn falsely.

cmd.exe looks in the current folder first whatever the PATH holds (it gets no
update command, `update::shell_kind`, but a `--shell` may name it). PowerShell
drops empty entries but reads an entry starting with `.` against the current
location: in its source, command discovery splits the PATH removing empty
entries, and `ResolveCurrentDirectoryInLookupPaths` resolves those entries
against it (read from its source, not run here). It is the fallback shell where
Git Bash is missing. dash, sh and ksh keep an empty entry as bash does, which
`env -i PATH=":/usr/bin:/bin" /bin/dash -c fakegit` showed on a Mac.

## Decision

- **`program::reads_cwd` judges the PATH handed to the terminal, by the shell
  and platform, once at startup.** A relative entry warns for every shell that
  is not cmd.exe, PowerShell excepted unless the entry starts with a dot. An
  empty one warns for bash, zsh, sh, dash, ksh and mksh off Windows, except for
  a macOS login shell, and never for PowerShell. cmd.exe always warns. Another
  shell (fish, a custom `--shell`) is judged by relative entries alone, its
  handling of an empty one not being known.
- **It is told in the startup banner and as `path_reads_cwd` of `/api/meta`**,
  `{"why": "relative" | "empty" | "cmd", "entry": ...}` and left out when none
  holds. The entry is the first relative one, as the PATH writes it: the user's
  own PATH, sent to the page as text and to nothing else, printed escaped in the
  banner. Edith's menu shows one sentence naming the reason and the risk, and,
  for the two PATH reasons, a second saying the check reads the PATH dbt-edith
  hands the shell, which its startup files may still change.
- **The freshness badge types `dbt parse` and leaves Enter to the user while
  that holds**, its hover card saying so. It is the one button that pressed
  Enter (0071), because `dbt parse` reaches no warehouse; a program run in its
  place is another matter, and the person sees the line before it runs.
- **Nothing else changes.** The commands are typed as they were.

## Rejected

- **Typing each program by the absolute path `on_path` finds.** It skips the
  venv's `dbt` that the terminal activates (0050), which is the one meant. It
  lengthens every line the person reads before pressing Enter. And Git Bash
  wants its own form of a path, not Windows'.
- **Warning on any empty entry of the PATH.** It is what was first built, and
  it fired on the two default shells where the shell's own PATH is clean.

## Consequences

The warning is a statement about the PATH at launch and the shell's kind. A
startup file that adds an empty entry later, or removes one, is not seen, and a
shell that is not a login one on a Mac is taken to keep an empty entry.
