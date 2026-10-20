# Explorer and search

The file tree, the dbt profile it does not show, and the two searches: by name
with `Cmd/Ctrl + K`, and inside files with the Search tab.

## Explorer

Folders carry the state of what is inside them:

| | |
| --- | --- |
| green | holds a file with unsaved changes in the editor |
| amber | holds a file that is saved but not committed, or untracked |

Files are coloured the same way, so a green folder can be followed down to the
buffer that is still unwritten. Status comes from `git status --porcelain`, run
server-side, cached for 1.5 s and polled every 5 s, so committing from the
built-in terminal clears the colours on its own. A hidden tab stops asking,
and catches up the moment it is shown again.

File types get their own icon and colour; `.sql` files use a database glyph.

The header above the tree names the project and carries two buttons. The first
collapses every folder, which is what following three `ref()` calls into a deep
project usually calls for; folders already read stay in the page, so reopening
one asks the server nothing. The second reopens the tree down to the file being
edited, and is greyed when the open tab is the dbt profile, which lives outside
the project.

That profile is not in the tree, unless the project holds one: the Project
section of the settings, behind the gear in the top bar, opens it instead.
dbt-edith looks for it where dbt does, in `DBT_PROFILES_DIR` as dbt-edith was
started with it, then in the project, then in `~/.dbt`, and the section names
the file it found. When there is none, it names where dbt would read one, and
the gear and the section's row carry an amber dot. It is the one file outside the project
dbt-edith opens, and saving it writes that file in place: dbt-edith never
creates one (0017, 0049). A profile that is a link, `~/.dbt/profiles.yml`
pointing into a dotfiles repository say, stays a link: the save writes the
file it points at.

A `profiles.yml` in the project is one of the project's files, and is held to
it as every path the editor opens is. When it links to a file outside the
project, dbt still follows the link, but dbt-edith does not open it: the
button turns grey with its dot, its tooltip says `profiles.yml: not opened`
and why, and a click says the same. To edit that file here, start dbt-edith
with `DBT_PROFILES_DIR` set to the folder it links to. A link that stays in the
project opens as any file does
([0094](../decisions/0094-what-a-repository-brings-is-held-to-it-what-you-bring-is-followed.md)).

Its passwords, tokens and private keys stay on the server. The editor shows
`<hidden by dbt-edith>` in their place, and the status bar counts them. Saving
with the placeholder left as it is keeps the value on disk; typing over it
replaces it. An `env_var()` is shown as it is, since it names where a secret is
rather than holding one
([0054](../decisions/0054-the-profile-is-edited-without-its-secrets.md)).

## Search

`Cmd/Ctrl + K` searches three things at once: dbt nodes from the manifest, the
macros it lists, and a flat index of every file in the project. That last half
matters more than it sounds: a script or a dotfile is not a dbt node, and a
manifest-only search can never find one.

A macro is found by its name, a package's as `package.name`, and opens on the
line that defines it, so one of twenty in a `utils.sql` is one search away. Only
a macro whose file is in the project is listed, so dbt's own are not. A file
named after a macro gives way to the macro, as a model's `.sql` does to the
model. A generic test or a materialization is a macro too, under the name dbt
gives it: `test_<name>`, `materialization_<name>_<adapter>`. Each row
says how many nodes reach the macro, `used by 12`, or `no reference found`,
which its card and the Macros section of Reports explain (0105).

The index is a directory walk, redone every 20 seconds, so a new file or a
branch switch shows up without any invalidation logic. `target`, `logs`, `.git`
and virtualenvs are skipped, since `target` alone holds more files than the rest
of the project put together. `dbt_packages` is indexed but ranked below your own
files, so reading an automate_dv macro is one search away without ever
outranking your own code.

## Search in file contents

The **Search** tab in the sidebar looks inside every indexed file, which
`Cmd/Ctrl + K` cannot: that one matches names, so a column used in forty models
is invisible to it. Type three letters or more and the matching lines appear
grouped by file, with the match highlighted; clicking one opens the file in the
preview tab with the cursor on that line.

It is case-insensitive and plain text, not a pattern. Several files are read at
once, so results follow typing rather than waiting for Enter. Files it passed
over, binaries and anything over 2 MB, are counted in the status line rather
than quietly dropped.

`.env` files are never opened by it (0020). Searching for a variable's name
finds where it is used, never where it is set; the environments in the
settings' Project section answer that other question, by name.
