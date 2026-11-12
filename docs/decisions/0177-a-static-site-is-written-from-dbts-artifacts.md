# 0177. A static site is written from dbt's artifacts

Date: 2026-11-12 · Status: accepted · Amends 0026

**Trigger:** read before changing what `--site` reads or writes, what the page
it writes may do, or how it answers a question the server answers.

## Context

`dbt docs generate` writes a site for a project. dbt Docs v2 shows a blank page
from `file://`, needs a CDN, sends telemetry and shows no column lineage without
a login. A team that deploys from a pipeline wants the documentation as a
folder: opened by double-click, served from any static host, built from what
dbt wrote and nothing else. Every environment has its own version of the code,
so a site describes the one manifest it was given.

## Decision

- **`dbt-edith PROJECT --site [DIR]` writes a folder and exits.** It reads
  `manifest.json` and `catalog.json` when there is one, and from git the commit
  `HEAD` names and `remote.origin.url`, through the gate (0121). No `.env` is
  read, no `env_var()` resolved, no profile opened, no warehouse and no collin
  reached, no job file read. A location is shown as the manifest wrote it.
- **The page opens from `file://` and asks for nothing.** Classic scripts only:
  no ES module, which `file://` refuses, and no `fetch`. The data is a script,
  `data/index.js`, and each card is in a bucket of `data/cards/NNNN.js`,
  appended to the page when the card is first opened. The page carries a policy,
  `default-src 'none'; script-src 'self'; style-src 'self' 'unsafe-inline';
  img-src 'self' data:; connect-src 'none'; base-uri 'none'; form-action
  'none'`, so "no request" is a promise the browser keeps, and no inline script.
- **The browser answers with the server's own walks.** `web/site.js` ports
  `Graph::lineage`, `levels` and `search` and the routes' clamps, line by line,
  and serves `/api/lineage`, `/api/node` and `/api/search` to the page through
  the `api` object it already uses. A golden file written by a Rust test from an
  invented graph holds the port to Rust's answers (`tests/fixtures/site_case.js`),
  and `scripts/walk_site.py` compares the two for every node of Jaffle Shop and
  Shopify. What the data holds is the numbers the walks read (adjacency, a rank
  by name so no string is compared in JavaScript, the search key, what each node
  is attached to) and one entry per node as the canvas draws it, minus what a
  walk decides.
- **The header names what the site is.** The environment, from `--environment`;
  the commit, 7 characters shown and linked to the repository at that commit;
  when the manifest was written; dbt's version. A commit is `--commit` or
  `HEAD`; the repository is `--repo-url`, https, no user, query or fragment, or
  `remote.origin.url` for exactly github.com, gitlab.com and bitbucket.org, a
  user, a token and an ssh port dropped. A host that only looks like one, an
  alias of `~/.ssh/config` or `github.com.evil.io`, gives no link, and the run
  says to pass `--repo-url`. The origin is never printed and never written.
- **The file tree is built from the manifest's files, not the disk.** The Project
  tab lists the files a model, a seed or a snapshot is built from and the folders
  above them, in the server's order, which Rust writes into the data so that no
  name is sorted or lower-cased in the page; a site holds no file, and a click on
  a row focuses the node.
- **The same inputs give the same bytes.** Nothing written holds a path of the
  machine, the time of the run or a duration.
- **The order of the work:** the manifest, then the folder (0178), then the
  data, then the files; a run that fails before the last step has emptied
  nothing.
- **`web/site.js` is linted as `web/app.js` and `web/lineage.js` are**, by
  Biome with 0173's rules, and `EdithSite` is a global beside `Lineage`.

## Rejected

- **One file**, as an export (0026). The index of a project with thousands
  of models runs to megabytes and its cards to tens of them; one file loads
  all of it to read a card.
- **JSON fetched from the folder.** `file://` refuses `fetch` and blocks module
  imports, which is the page dbt Docs v2 shows blank.
- **Shipping each lineage precomputed.** Every node, up to twenty levels each
  way, with and without tests, under each cap: the size grows with the product,
  and the walk is a few milliseconds in the browser. This amends 0026's
  rejection of shipping `lineage.js` with a payload, which held for a file that
  must show with no script; a site is a page, and `lineage.js` is what draws it.
- **A CDN, a font or an analytics call**, for the promise that the page asks
  for nothing.
- **A link guessed for any host.** A commit page has a pattern per host, and a
  wrong guess is a link to nothing, or to somebody else's page.
- **Resolving `env_var()` against a `.env`.** It would put one environment's
  values, and a secret's neighbour, into a file that is published.
- **A dirty-tree marker.** Under Fusion the pipeline's own `deps` rewrites
  `package-lock.yml`, so a tree is always dirty and the mark says nothing.
