# A static site

`dbt-edith PROJECT --site` writes a documentation site from what dbt wrote, as
`dbt docs generate` does, for a pipeline to publish. It is a folder: opened by
double-click from `file://`, or served from any static host, under a sub-path
too. It makes no request, not to a CDN, not to a server, not for telemetry, and
the page carries a policy that has the browser refuse one
([0177](../decisions/0177-a-static-site-is-written-from-dbts-artifacts.md)).

## What it is built from

`target/manifest.json`, and `target/catalog.json` when there is one, so a
column has the type the warehouse gave it; `--manifest` and `--catalog` name
others. From git, the commit `HEAD` names and `remote.origin.url`, through the
same gate as the Git tab ([0121](../decisions/0121-git-runs-nothing-a-repositorys-own-config-names.md)).
Nothing else:

- no `.env` is read and no `env_var()` resolved: a location is shown as the
  manifest wrote it, and the card has no environment to pick;
- no profile is opened and no warehouse reached;
- no collin run, no job file, no Snowflake: column lineage and the jobs are
  not in the site yet.

Every environment has its own version of the code, so the site describes the
one manifest it was given, and the header says which.

## Writing it

```
dbt-edith PROJECT --site [DIR] [--environment NAME] [--commit SHA] [--repo-url URL]
```

- **DIR** defaults to `PROJECT/target/edith-site/`. It must be strictly inside
  the project, never the project itself, and be empty or carry the marker
  `.dbt-edith-site`, which every run writes first. A marked folder is emptied
  and written again; any other is refused, exit 2, and left alone
  ([0178](../decisions/0178-the-site-is-written-inside-the-project-into-a-marked-folder.md)).
  A `..` in the path is refused, and so is a link that leads out of the
  project. A word after `--site` is the folder, so the project comes first.
- **`--environment NAME`** is a label, shown in the header and the page's title.
- **`--commit SHA`**, 7 to 64 hexadecimal characters; without it, the full
  `HEAD`. Outside a repository, or with git paused there, the site has no
  commit and says why.
- **`--repo-url URL`**, https, with no user, query or fragment. Without it the
  address comes from the origin, for exactly github.com, gitlab.com and
  bitbucket.org: the scp, `ssh://` and https forms are read, `.git`, a user, a
  token and an ssh port are dropped. An ssh alias such as `work.github.com` gives
  no link, and the run says to pass `--repo-url`. The origin itself is never
  printed or written.
- **Exit codes**: 0 written; 2 for no manifest, a refused folder or a bad value.
  stdout is the folder and the counts; stderr says what it could not do, no
  catalog, no commit, no link.

The same inputs give the same bytes: nothing written holds a path of the
machine, a time of the run or a duration. `--site` goes with `--manifest` and
`--catalog` alone, and over MCP it is refused.

## The folder

```
.dbt-edith-site          the marker
index.html               the page, with its policy and its data
app.css app.js lineage.js site.js favicon-32.png favicon.svg
vendor/                  exactly the files the page loads
LICENSE.txt THIRD_PARTY_NOTICES.md
data/index.js            window.EDITH_SITE = {...}
data/cards/NNNN.js       EDITH_SITE_CARDS(start, [card, ...])
```

`data/index.js` holds the header's facts, one entry per node as the canvas draws
it, who feeds whom, a rank by name, each node's search key and the folders of
the Project tab. A card, with its
columns, tests and parents, is in a bucket of about 200 nodes or 512 KiB, a
script appended to the page when a card is first opened: a project of
thousands of nodes loads its index once and a bucket per card. The browser
answers `/api/lineage`, `/api/node` and `/api/search` itself, with the walks the
server makes, written again in `web/site.js` and held to Rust's answers by a
golden file and by `scripts/walk_site.py`, which compares them for every node
of Jaffle Shop and Shopify.

## The page

It is the page of the server, with what a site has no use for left out: the
spaces, the search and git tabs, the editor, the terminal, Jobs, Compiled and
Run, the settings, the manifest's pill, the GitHub mark and the shortcut list,
and the status bar's venv, environment, branch, shell and update.

- **The Project tab** is the editor's file tree, in front, as an overview of the
  project's model folders. It lists only the files dbt builds a model, a seed or
  a snapshot from, and the folders above them: no test, no YAML, no macro, and a
  folder with none of those files never appears. It is made from the manifest's
  paths, not from the disk, in the order the server lists a folder (folders
  first, then by name without regard to case), and its icons take the
  materialization's colour as the editor's do. A package's files are where
  `dbt deps` put them, or, when the package was not found under the project
  when the site was written, where it would put them, `dbt_packages/NAME/...`.
  A file with several nodes, a legacy snapshot file, is one row, for the first
  by name. A click on a row focuses its node, lineage and Catalog, and opens no
  file; and whichever way a model is focused, its row is opened up to and lit.
  The Models tab beside it stays a flat list.

- **The header**, beside the project's name: the environment as a chip, the
  commit's first seven characters as a link to the repository at that commit
  (it opens in a new tab and tells the host nothing of where you came from),
  when dbt wrote the manifest, in your own clock, and dbt's version.
- **The Models list** and **`Cmd/Ctrl + K`** find a model, a source, a seed or a
  snapshot by name or by path; the palette says it searches no macro and no file.
- **The lineage and its depth**, the tests toggle, the tests under a model, by
  folder, Fit and Export work as on the server's page; a click on a box fills the
  **Catalog**, whose Preview and Columns are the manifest's and the catalog's,
  and which has no Open file.
- **The address** names the node on screen, `index.html#model.shop.orders`, so a
  link to it opens on it.
- **The theme** is kept in the browser, per browser.

## What is not in it

Each is a deferred entry in [state.md](../state.md): sources and the tree's other
files (macros, analyses), the SQL of a model, raw and
compiled, with links to its file in the repository; column lineage; the jobs;
dbt selector expressions; writing outside the project; and a mark for a dirty
working tree. Firefox is not verified: `script-src 'self'` is expected to allow a
page's own scripts from `file://` there, as it does in Chrome and Edge, and no
one has opened a site in it yet.
