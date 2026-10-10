# 0050. The terminal activates the venv dbt-edith was started in

Date: 2026-10-10 · Status: accepted

**Trigger:** read before changing what the terminal types on its own, or which
Python environment its shell starts with.

## Context

The terminal gets dbt-edith's own environment, `PATH` included, and on the
VM `dbt` there was still a dbt-core without the project's adapter. The
venv's `Scripts` folder did reach the shell, but Git Bash runs `~/.bashrc`
again, and that one put pyenv's shims in front of it. An editor's terminal
has the same problem and solves it by activating the venv again once the
startup files have run; in VS Code's, on the same machine, `dbt` was the
venv's. pyenv's or conda's setup in `~/.zshrc` or `~/.bashrc` does the same
on macOS.

## Decision

- **When dbt-edith was started with `VIRTUAL_ENV` set, each new terminal gets
  that venv's activate script typed into it**: `source` for bash and zsh, the
  `.fish` script for fish, `Activate.ps1` for PowerShell. Another shell, or a
  venv with no script for it, gets nothing typed. `pty::activation` holds it.
- **Only that venv.** One merely found in the project is never activated, so
  dbt-edith still reports the environment it was given rather than choosing
  one. That was why activating at startup was rejected when the terminal
  was given that environment; activating the venv already handed over does
  not choose.
- **It is typed once the line editor reads.** On Unix the terminal echoes on
  its own until then, and the line would show twice, so the writer waits for
  echo to go off, up to ten seconds, and types nothing if it never does. A
  Windows console echoes on read, so there the line is queued at once.
  Anything the page sends meanwhile, keys or a `dbt ls`, queues behind it.

## Rejected

- **Fixing `~/.bashrc` on the machine.** It fixes one machine, and the next
  user with pyenv or conda hits it again.
- **A generated `--rcfile` or `ZDOTDIR`.** That is invisible, but it writes
  files outside the project, and each shell needs its own trick.
- **Restoring `PATH` without the script.** `deactivate` and the prompt's
  `(venv)` stay missing, and a venv's own activate script stays the only one
  that knows what activating it means.

## Consequences

Each new terminal starts with one visible `source …/activate` line, and the
shell's history keeps it.
