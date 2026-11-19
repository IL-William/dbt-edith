# 0182. The site carries collin's column lineage

Date: 2026-11-19 · Status: accepted · Amends 0177 and 0099

**Trigger:** read before changing what `--site` reads of `target/`, whether it
starts collin, how the browser answers `/api/collineage` and `/api/colreach`, or
what the page says of a cache older than the manifest.

## Context

The static site (0177) left column lineage out, with the trap written down: the
cache is large, `/api/collineage` is a walk the page would have to write again,
and a lineage drawn from a cache older than the manifest is a wrong picture
published with a commit's name on it. A reader of the documentation asks the
same question of a column as of a model: where does it come from, and what reads
it. 0177 also says no collin is reached; 0099 says collin runs on the command
line only when `--column` asks.

## Decision

- **The cache is collin's, beside the manifest, and nothing else of `target/`.**
  `column_lineage.collin.json`, taken only if the listing the page's menu is made
  from lists it (the report, a file of a newer version and a half written one are
  not caches), and `collin.report.json` for the notes on the cards, when the
  cache names collin as its tool. A Fusion or Snowflake cache is not read: the
  site has no menu to pick one and the file it carries is named by the site, not
  chosen by the page. A cache that cannot be read is left out with a warning,
  exit 0.
- **`--site` alone never starts collin.** `--site --run-collin` runs it first, over
  the manifest as it stands, as `--column --run-collin` does (0099), writing the
  same two files beside the manifest; a run that writes no cache, or one that
  cannot be read, ends with exit 2 and writes nothing, and a folder refused is
  refused before collin runs. `--compiled` and `--catalog` go to collin as they do
  there. Over MCP `--site` stays refused, whatever else is beside it.
- **A cache older than the manifest is carried, with a warning.** By the files'
  modification times, which a checkout resets: the guess errs towards saying so.
  stderr says it, the index says `cll_stale`, and the page says it in three
  places: a short word after a column canvas's status line, the sentence as its
  title, a hint in the Columns tab in a tone of its own, and the line of the export
  that names the cache. Not in the legend or the header. The warning names the
  command that writes it again.
- **The cache is one script, loaded whole when a column is first asked about.**
  `data/cll.js`, `EDITH_SITE_CLL({...})`: a column is a number (where its node
  starts plus its slot, so the order of a column is the number's), and the file
  holds the edges in the order the walks read them, `from` as the step from the
  one before, and, by column, what the cards of columns say (types, test counts,
  roots, expressions, the names collin lost, the columns read from a table no
  node owns, the predicates each model applies with the columns each reads). A
  name and a type are interned. Nothing in it is an absolute path, a mtime or the
  producer: the same files, with the same times, give the same bytes (the
  index's `cll_stale` comes from the times), and the file is named bare in
  the index, never by the path it was read from. The index's meta says whether
  there is any, the tool, the source, the target, when the cache says it was made,
  the edges, the roles and how many edges each has, and whether it is stale.
- **The browser answers `/api/collineage` and `/api/colreach` with the server's
  walks, written again** in `web/site.js`: the column walk with its cap, its cut,
  its stops and its boxes for the indirect edges; the answer built from it; the
  names a card says; the levels the depth menu lists; and the whole chain of every
  column of a node, in one pass with the strongly connected parts and a bit per
  start, as `reach_all` does it, which was measured fast enough that no table of
  counts is written beside the file. The golden file holds each to Rust's answers
  for every column of three caches (an invented one that holds each shape a
  cache can have, one from a seed that is a tangle, and one of edges alone), for
  the deep, capped and cut walks of the first, and `scripts/walk_site.py` holds
  them to the server's for every column of Jaffle Shop and Shopify.
- **The page keeps what a site can do and leaves the rest.** A column goes
  straight to its canvas: no menu to pick another cache, no Regenerate, no
  Snowflake, no collin to ask. The indirect button, the stop at menu, the
  depth menus, the Columns tab's chain and the hover cards work as on the server.
  A card's link to a model's file and its compiled text, for a column collin read
  in part, opens the model's Code tab.
- **A note on a card is cut of this machine.** collin's note on a model may say
  where it read the SQL: the project's root is cut from every text in it, a text
  that still holds an absolute path (`/x`, `C:\x`, `C:/x`, `\\?\C:\x`, a share)
  is dropped whole, a name that does is removed from its list, and a `sql_file`
  that is not below the project is dropped. A text is judged whole and never
  rewritten word by word, which would keep the tail of a path with a space in it.

## Rejected

- **A table of the chains, written beside the file.** A count per column of every
  node: far smaller than the file, and the walk is not what costs, so it was only
  worth writing if the port were too slow, which it is not.
- **The cache as buckets per node.** A walk crosses nodes at random; a bucket per
  node is a request per step, and the page asks for nothing but its own files, one
  at a time.
- **Refusing a cache older than the manifest.** A site written in CI from a checkout
  has every file at the checkout's time; it would never carry a cache, and the
  warning says what is known.
- **Reading the other caches, or letting the page pick.** The site is a picture of
  one cache, as of one manifest, and a published page has no menu to pick another
  from; whoever writes the site chooses by what is in `target/`.
