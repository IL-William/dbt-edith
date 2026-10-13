# 0072. A click on an inactive venv types its activate line

Date: 2026-10-13 · Status: accepted · Amends 0050, 0069

**Trigger:** read before making the status bar type anything in the terminal,
or changing where a venv's activate line is made.

## Context

The status bar says `venv (inactive)` when the venv it names was found in the
project but is not the terminal's, and its tooltip says to source the activate
script. That script sits under the project's full path, in `Scripts` on
Windows, and was typed by hand. A project with a dbt-core venv and a Fusion one
switches between them this way, and the bar names only one of the two.

## Decision

- **While the bar says inactive, a click types the line that activates the
  venv it names**, and leaves Enter to the user, as every button does (0071).
  With several found, the click opens a menu of them instead, the named one
  first. An active venv offers nothing.
- **The server makes the line**, with `pty::activation`, the function behind
  the line each terminal gets at startup (0050): the script each shell
  sources, its quoting, and the `C:/` spelling Git Bash takes. Each venv in
  `/api/meta` and `/api/venv` carries it. A venv with no script for the
  terminal's shell carries none, and is not offered.
- **Nothing is activated without the user.** 0050 still holds: dbt-edith
  activates on its own only the venv it was started in. The line names a path
  the page already had, and a venv git tracks is never found, so it is never
  offered (0053).

## Rejected

- **Making the line in the page.** It would repeat the per-shell scripts and
  the quoting of `pty::activation`, and the page knows the shell only as the
  text the status bar prints.
- **A relative `source env/bin/activate`.** The terminal may have left the
  project's root by the time of the click.
- **Pressing Enter.** Which venv is active decides which dbt runs next, and
  0071 leaves that to the user.

## Consequences

zsh, fish and PowerShell do not report their venv (0069), so there the bar
stays inactive after the line has run, and clicking it again types the line
again.
