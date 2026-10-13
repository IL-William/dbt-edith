# 0069. The status bar names the venv the terminal reports

Date: 2026-10-12 · Status: accepted · Amends 0050

**Trigger:** read before changing what the status bar says about Python, or
handing the terminal's shell anything beyond dbt-edith's own environment.

## Context

The status bar named the venv as dbt-edith started. Activating another in the
terminal, which is how a project switches between dbt-core and Fusion, left the
bar on the old one. A venv is activated by a script sourced into the shell, so
only the shell knows which one it has.

## Decision

- **bash prints its `VIRTUAL_ENV` before each prompt**, as OSC 6973 ended by
  ST, from a `PROMPT_COMMAND` in its environment, after any it was handed. The
  hook unexports itself, so a bash opened from it elsewhere does not print it.
  xterm.js draws nothing and hands it to the page.
- **The page names that venv** with its versions when the server read it: the
  started one, or one found in the project, matched by its folder or by the
  folder its activate script sets. That script is read, never run, because the
  canonical root is spelled otherwise behind a mapped drive, a `subst` or a
  symlink. A folder matching nothing makes the page ask for the project's venvs
  again, once, for one made since startup. The path never goes to the server.
- **Before the first prompt and after the shell exits**, the bar says what it
  said at startup. **zsh, fish and PowerShell say nothing**, and it stays so.

*Amended by [0072](0072-a-click-on-an-inactive-venv-types-its-activate-line.md):
while the bar says inactive, a click on it types the line that activates the
venv it names, or lists the venvs found to pick one.*

## Rejected

- **Reading the shell's environment from outside.** macOS shows the one a
  process started with, and Git Bash keeps its variables apart from Windows'.
- **Reading `(venv)` off the prompt.** A theme or `VIRTUAL_ENV_DISABLE_PROMPT`
  drops it, so its absence would not mean the venv is gone.
- **A hook for zsh, through a generated `ZDOTDIR` or a typed line.** 0050
  rejected both for the activation.
- **Sending the reported path to the server.** Any output can forge it, and it
  can point outside the project (0017).

## Consequences

A `~/.bashrc` assigning `PROMPT_COMMAND` instead of adding to it drops the hook.
bash 3.2, the one macOS ships, loses `PIPESTATUS` across the hook; 4.3 and later
keep it. Any program can print the sequence, which only changes words in the
bar. ConPTY passes an unknown OSC through, perhaps out of order with the text.
