# 0180. The site's SQL is the manifest's, and each file is linked at the commit

Date: 2026-11-19 · Status: accepted · Builds on 0177

**Trigger:** read before changing where the site's SQL comes from, what the
Code tab says of a node dbt did not compile, or how a link to a file in the
repository is made.

## Context

The static site (0177) showed a model's columns and its place in the graph but
not its SQL, and linked the commit but not the files. A reader who finds a
model in the documentation wants what it selects, and the file it is in. Decision
0132 kept `compiled_code` out of the command line, on three grounds: a manifest
written by `dbt parse` has none, reading it costs deserializing the bulk of the
manifest, and it could disagree with the file the page shows. None of the three
holds for the site.

## Decision

- **The SQL is the manifest's `raw_code` and `compiled_code`, and nothing
  else.** A second, targeted pass over the same file reads those fields and
  `language` of each node, mapped onto the graph's numbering (the nodes, then the
  first entry of each disabled one, as `Graph::build` takes them). The server's
  `RawNode` is unchanged and keeps no code in memory. No file of `target/` is
  read, so the site stays the picture of one manifest: the compiled text and the
  source cannot come from two moments.
- **A node dbt did not compile says so.** A manifest written by `dbt parse` or
  the language server has no `compiled_code`. The Code tab's Compiled view then
  says that dbt did not compile the node when it wrote the manifest, and that
  `dbt-edith --site` is to run after `dbt build`, `dbt compile` or `dbt docs
  generate`; the run says it once on stderr when no node has any.
- **The text is in buckets of its own.** `data/sql/NNNN.js`, each `EDITH_SITE_SQL(start,
  [entry | 0, ...])`, cut at 200 nodes or 512 KiB as the cards are but on its
  own cuts, since a node's SQL is a hundred times a card. An entry is `{raw,
  compiled?, lang?}`, `lang` only for a Python model, which the page shows as
  plain text: no mode for it is loaded. A line ending is one, CRLF becoming LF, and
  the text goes through the same escaping as the cards, which keeps `</script>`
  and U+2028 in SQL from ending the script.
- **The tab is the manifest's, whatever the node.** Models, snapshots, analyses,
  tests and operations have one; a seed, a source and an exposure have no code of
  their own. A generic test's `raw_code` is the macro call, shown as it is: the
  SQL it compiles to is the useful part, and the other view.
- **A file is linked where the host has it at the commit.** The header's commit
  is the one, `--commit` or `HEAD`. The project's place in its repository is
  `git rev-parse --show-prefix`, through the gate (0121): a project below the root
  would otherwise link to a file that does not exist. The pattern is the host's,
  as the commit's is: `/blob/<sha>/` for GitHub and any host named by
  `--repo-url`, `/-/blob/<sha>/` for GitLab, `/src/<sha>/` for Bitbucket. The page
  is given the base, `file_base`, and adds the node's path with each segment
  escaped, so a space or a `#` in a folder's name stays in its segment.
- **A package's file has no link.** Only a node of the root project is in the
  repository; the page tells it by the node's package and `meta.root`, the
  root package as the tree reads it (the manifest's name, or `dbt_project.yml`'s
  when the manifest has none), so no list is shipped for it.
- **No git, no file links.** Outside a repository, with git paused there or with
  no answer, the commit stays linked if `--commit` and `--repo-url` were given,
  and stderr says no file is linked and why. A host whose file address takes a
  query, Azure DevOps (`dev.azure.com`, `visualstudio.com`), has its commit
  linked and no file.

## Rejected

- **`target/compiled/` as a fallback.** A file of another moment than the
  manifest, in a folder a pipeline may have cleaned: the page would show two
  pictures at once.
- **Reading the SQL with the manifest, in `RawNode`.** The server would hold the
  code of every node in memory for a feature it does not have.
- **A link guessed without the prefix.** Right for a project at the root of its
  repository and a link to nothing for one below it.
- **A `--repo-path`.** A site written from a checkout git cannot place (an
  exported archive) would need it; it is a flag to add when one asks.
- **Azure's file address.** `?path=` and `&version=GC<sha>` make a pattern the
  host's other links do not share, and none of them has been tried.
- **A Python highlighter.** The bundle has no Python mode, and a mode is a
  vendored file to audit for a language a site shows rarely.
