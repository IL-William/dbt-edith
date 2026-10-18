# What dbt-edith does

The [README](../../README.md) says what dbt-edith is and how to install and run
it. These pages say what each part of it does, in full: what to know before
changing a feature, and where someone using it looks something up. Why it is
built that way is in [docs/decisions/](../decisions/), and what is not built
yet in [docs/state.md](../state.md).

When a change alters what a feature does, its page here changes in the same
pull request.

| Page | What it covers |
| --- | --- |
| [editor.md](editor.md) | links in the editor, Jinja colours, hover cards, find, tabs, breadcrumbs |
| [explorer-and-search.md](explorer-and-search.md) | the file tree, the dbt profile, search by name and inside files |
| [lineage.md](lineage.md) | the graph, how it follows the editor, whether it is still true, reading it, sharing it |
| [selection.md](selection.md) | Custom selection, named selectors, `--select` on the command line |
| [catalog.md](catalog.md) | the Catalog tab, where a model lives, environments, Compiled and Run |
| [column-lineage.md](column-lineage.md) | column lineage from Fusion, Collin or Snowflake, `--column` on the command line |
| [jobs-and-reports.md](jobs-and-reports.md) | the Jobs tab, job coverage, Reports, Last run |
| [snowflake.md](snowflake.md) | the Snowflake switch, Snowsight links, Clone into my schema, Query history |
| [terminal-and-git.md](terminal-and-git.md) | the terminal, the Python environment, the Git tab |
| [updates.md](updates.md) | how it learns a newer release is out, and what it offers |

## Every action

| Action | |
| --- | --- |
| click a `ref()` / `source()` / `source_model` name | jump to that model's file |
| click a macro call, `{{ hub() }}` or `{{ dbt_utils.star() }}` | open the file that defines it, at its `{% macro %}` line |
| click a table, model or seed name in a properties `.yml` | move the lineage onto it, staying in the file |
| click a folder key under `models:` in `dbt_project.yml` | open that folder in the explorer, expanded |
| the violet file at the end of the breadcrumb, in a model | the keys of `dbt_project.yml` that reach it, and what each sets; pick one to open it there |
| `Cmd/Ctrl + K` | search models, sources, macros and every file in the project |
| `Cmd/Ctrl + F` | find in the file you are in, in the Compiled or Run SQL, a node on the lineage, or filter the Catalog's columns |
| the copy glyph beside the name in the Catalog | that name on the clipboard |
| `?` outside a text box, `F1`, or the `?` in the top bar | every shortcut, in one list |
| Give me a tour, at the top of that list | a walk through each part of the page, on a model of your project |
| the GitHub mark in the top bar | raise an issue or a request on GitHub, your version already in it, or give dbt-edith a star |
| `Cmd/Ctrl + S` | save the current file |
| `Cmd/Ctrl + Alt + S` | save every modified file |
| `Cmd/Ctrl + Alt + X`, `Alt + W`, or middle-click a tab | close a tab |
| `Cmd/Ctrl + Alt + Shift + X` | close every tab |
| right-click a tab | close it, the others, those to its right, or those with nothing unsaved |
| `Cmd/Ctrl + Alt + P` / `Cmd/Ctrl + Alt + N` | back to the link you followed, and forward again |
| `Cmd/Ctrl + \`` | jump to the terminal |
| click a column in Catalog > Columns | draw its lineage, fetched from Snowflake when Snowflake is the picked tool |
| the column lineage menu in the top bar | pick Fusion, Collin or Snowflake; a greyed tool says what it lacks |
| the snowflake in the top bar | Snowflake's features on or off for the project, and where its jobs are read from |
| the user and plug in the top bar | the dbt profile, opened in the editor; violet when there is one, grey with an amber dot when dbt would find none |
| the Query history tab | your own Snowflake queries of the last 7 days, twenty at a time, each linked to Snowsight |
| Snowsight beside a relation in Catalog > Location | its object, schema or database, opened in Snowsight |
| Clone into my schema, under Catalog > Location | the statements that copy a model and its direct parents from an environment into your schema, for you to run |
| hover a lineage node, a `ref()`, a macro or a `var()` | a card with what it is |
| hover a folder key in `dbt_project.yml` | the folder it configures, or why there is none |
| the dot beside Reload manifest | how stale the lineage is, and one click to re-parse |
| the Search tab in the sidebar | find a word inside every file, not just in their names |
| click a segment of the breadcrumb bar | a menu of that folder's contents, or of the neighbouring keys |
| click a lineage node, a model's or one of its columns' | select it, fill the Node panel, and open its model's file as a preview tab, the canvas staying put |
| double-click a lineage node | re-centre the lineage on it and open its file, pinned |
| `+N` badge on a node | pull in one more level of parents or children |
| the Custom selection button above the graph | draw your own set of models, in dbt selector syntax, or a named selector, in a panel over the editor and the dock |
| click a box in the Custom selection's panel | show its card beside the canvas; a double-click opens its file and rolls the panel up |
| wheel / drag | zoom and pan the lineage; a drag that starts on a box pans too, and opens nothing |
| Export above the graph | save what the canvas shows as one HTML file anyone can open |
| Copy image above the graph | the same picture as a PNG on the clipboard, for a ticket |

On a French keyboard `Alt + W` cannot be typed, so a tab closes with
`Cmd + Alt + X` there; every shortcut this tool binds carries `Cmd` for that
reason (0037).

## The tour

Give me a tour, at the top of the shortcut list, walks through each part of the
page on a model of your project with parents and children, and one a job runs
once the Jobs tab has a source, picked the same way every time: the
`manifest.json` everything is read from and how fresh it is, the editor and the
`dbt_project.yml` button, the lineage, a Custom selection and how to share it,
the Catalog, column lineage, Jobs, Reports, the terminal and Search. Each step rings what it is about and says what it is for,
and a feature that is off says how to turn it on. It runs nothing and changes
no setting, and when it ends the page is as you left it. The arrow keys walk it
and Escape ends it.

It is offered once, the first time a page opens on the machine with a model
to show, by a card under the `?` button: on a project with no `manifest.json`
yet, it comes once `dbt parse` writes one. The machine remembers it in
`tour.json`, in dbt-edith's config directory, with the version of dbt-edith
that served the page.
