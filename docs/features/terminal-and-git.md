# Terminal and Git

The shell in the dock, the Python environment it runs in, and the Git tab.

## Terminal

The terminal runs `$SHELL -l` on macOS
and Linux, and Git Bash on Windows, falling back to PowerShell when Git Bash is
not installed. It starts with the environment dbt-edith was started in: activate
the project's virtual environment first, and `dbt` in the terminal is that
environment's. Each new terminal then types that environment's own activate
script, `source …/activate` for bash and zsh, so it comes first again after
your `~/.bashrc` or `~/.zshrc`, which may have put pyenv or conda before it.

## Python environment

The status bar shows the virtualenv, labelled `venv` when it was active as
dbt-edith started and `venv (inactive)` when it was merely found in the project,
with the Python and dbt versions in the tooltip. When several are present, the
one that actually contains dbt wins.

With bash in the terminal, Git Bash included, the bar then follows the
terminal: `source .../activate` or a project's own activation script, and the
bar names that venv at the next prompt; `deactivate`, and it says `venv
(inactive)` again. A venv made in the project since dbt-edith started is read
when the terminal names it. Any other venv, outside the project or tracked by
git, is named without its versions, unless dbt-edith was started in it. zsh,
fish and PowerShell do not say which venv they hold, and there the bar stays as
dbt-edith started
([0069](../decisions/0069-the-status-bar-names-the-venv-the-terminal-reports.md)).

The bar is green when the venv is active and amber when it is not. Click it
while it is amber and the line that activates that venv is typed in the
terminal, `source .../activate` for bash, waiting for Enter; with several venvs
in the project, the click lists them to pick one
([0072](../decisions/0072-a-click-on-an-inactive-venv-types-its-activate-line.md)).

Those versions are read from the venv's files, `pyvenv.cfg` and the names of
its installed packages: opening a project runs nothing it holds. A venv that git
tracks came with the repository, and its `python` was written by whoever wrote
the repository, so dbt-edith never runs it, neither for the status bar nor for
the Snowflake script; the tooltip names it
([0053](../decisions/0053-a-project-is-read-before-anything-in-it-runs.md)).

## Git

A third sidebar tab stages, commits, pulls and pushes, and the branch name in
its header (and in the status bar) opens a filtered branch switcher rather than a
list, since a long-lived repository easily carries hundreds of branches.

Two rules hold across the whole git surface:

- **Nothing destroys work.** There is no `-f`, no `--hard`, no `clean`, no
  `push --force` anywhere in `src/git.rs`. The worst any button can do is create
  a stash. When a branch switch is refused because local changes are in the way,
  the blocking paths are listed and the only offer is to stash them, tagged with
  the branch being left, recoverable with `git stash pop`.
- **Nothing can hang the server.** Network commands run with
  `GIT_TERMINAL_PROMPT=0` and ssh in batch mode, so a missing credential fails
  in seconds instead of waiting forever on a prompt nobody can answer. Every
  command has a deadline, and both pipes are drained by their own threads so a
  chatty hook cannot deadlock.

Commit fetches first, so the ahead/behind counts next to it are current; a
failing fetch never blocks the commit. Hooks always run: pre-commit hooks can take
a while and can fail, and their output is shown in full rather than bypassed
with `--no-verify`. Pull is `--ff-only`, so it can
never start a merge on its own; if the branches have diverged it says so and
leaves the choice of merge or rebase to you.

Clicking a changed file opens a side-by-side diff of HEAD against the working
tree, in its own tab, read-only: a `+` or `-` sits next to every changed line,
regions with no counterpart on the other side are hatched, long identical
stretches collapse, and a ruler down the right edge shows where the changes are
and jumps to them. New files show entirely as added, deleted ones as removed.
Editing stays in the file itself, one button away.

Conflicted files get their own section. Opening one highlights the
`<<<<<<<` / `=======` / `>>>>>>>` blocks and puts *keep ours / keep theirs /
keep both* above each one; a block only counts once all three markers are
present, so a half-edited file is left alone. When a file is done, one button
marks it resolved (`git add`), and the merge can be aborted at any point.
