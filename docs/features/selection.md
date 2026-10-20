# Selection

Drawing what a dbt selector matches, in the Custom selection box or from the
command line.

## Custom selection

The **Custom selection** button in the top bar, or the one above the graph,
opens a panel over the whole window, the sidebar included, and draws there
whatever a dbt selector matches, at the window's width and height, so you can
draw the set you actually work on:

```
stg_customers dim_customers+ fct_orders
```

Until a box is clicked the canvas has the panel's whole width. A click on a
box opens its card from the Catalog beside the canvas, and the canvas moves
along only if the card would cover that box: that card comes half a second
later, so that a double-click still finds the box, and if you pan or zoom
meanwhile the canvas stays where you left it. Drag the card's left edge to make
it wider or narrower, for the rest of the session. The card's × or Escape
closes it, and so does a selection drawn without that box. A double-click opens its
file, and the panel rolls up, the dock's lineage moving onto that model.
Escape with no card open, the × at the top right, the top bar's button or a
file opened from the search rolls it up as well, and the dock is back as it
was: the model's lineage or the column's, and its card in the Catalog. With
the find bar open, Escape closes the bar first. The selection is kept: either
button draws it again
([0092](../decisions/0092-custom-selection-opens-in-a-panel.md),
[0101](../decisions/0101-custom-selection-opens-over-the-whole-window.md)).

A space is a union, a comma is an intersection, and `--exclude` may follow on
the same line. Whole commands paste in: `dbt ls -s "a b" --exclude tag:x` is
understood, and the box rewrites itself to the part that was resolved.

| | |
| --- | --- |
| `my_model` | by name, or by a dotted path into the fqn (`shop.staging.*`, `staging.my_model`) |
| `my_model.v2`, `my_model_v2` | one version of a versioned model, whose name alone takes every version |
| `+my_model`, `my_model+` | everything upstream, everything downstream |
| `2+my_model`, `my_model+3` | as far as that many levels |
| `@my_model` | it, its children, and every parent of those children |
| `tag:`, `path:`, `file:`, `package:` | the usual dbt methods, with `*` and `?` wildcards |
| `resource_type:`, `source:`, `exposure:` | by kind, by source, by exposure |
| `version:` | `latest`, `prerelease`, `old` or `none`, against a model's versions |
| `config.materialized:` | and `config.schema`, `config.database`, `config.alias`, `config.incremental_strategy`, `config.unique_key` |
| `--selector name` | a named selector from `selectors.yml`, [below](#named-selectors) |

Disabled nodes are never returned, as in dbt. Tests take part only when the
tests **eye** is open, and then each term brings the tests of what it
selected, a test joining whenever a parent of it did, which is dbt's own
default. So a test excluded by name stays out, and `+` and `@` reach tests the
way dbt's graph does. A term that matches nothing is called out under the
box rather than dropped, and it says whether the term matched only disabled
nodes or only tests. The count beside the box is the whole selection; the line
in the corner is what the canvas drew.

The canvas draws a selection of up to 3 000 nodes. A larger one is drawn in
part. A warning under the box says so, and the corner reads, say, `3000 of 4210
drawn`. The part drawn is the one nearest the start of the selection: first what
its terms matched themselves (the sources of `source:stripe+`, the model of
`+orders`, but not every model of `+orders,resource_type:model`, where the
second term only filters), then the nodes one edge away from those, then two,
and so on, with tests after the models and sources. A selection without `+` or
`@`, `tag:nightly` say, begins at its roots instead. Every box drawn past the
start sits next to one drawn before it, and a `+N` badge counts the neighbours
left out. **Copy names** still lists every match, up to 5 000 names, and a
narrower selection draws the rest. So does **Draw all**, beside the warning, up
to 20 000 nodes: for that selection only, since past the cap zooming and panning
slow down, a lot on a slow machine. Another selection, an emptied box, or the
tests eye, is cut at 3 000 again
([0087](../decisions/0087-a-capped-selection-draws-what-is-nearest-its-start.md)).

The start can take most of the canvas by itself: `source:*+` on a project
with thousands of sources draws the sources and few of the models they feed.
Those few hang off the first sources by name, and the later ones stand alone
with their `+N` badge. When the start takes half the canvas or more, the warning
counts it, and says what draws the rest: "Too large to draw whole: 3000 of
4210 on the canvas, 2650 of them what the selection names. Draw all or a
narrower selection draws the rest of their lineage; Copy names lists every
one." It says lineage rather than children because a start can grow either
way: `+tag:finance` starts at the tagged models and draws their parents. A
selection of more than 20 000 nodes offers `Draw 20000`, which the warning
says draws more of their lineage. The count is of what the selection named
itself: a model drawn in place of a test it named is not in it. A cut that
reaches the tests has drawn
every model and source first, and a selection that is all start is drawn
from its roots, so neither says it.

**Nothing runs dbt for this.** The expression is resolved against
`manifest.json`, which is the same data dbt reads, so the answer arrives in
milliseconds and needs no profile ([0024](../decisions/0024-selectors-resolved-from-the-manifest.md)).
The price is that the answer is this tool's, not dbt's. Two buttons exist for
that: **Copy names** puts the matching names on the clipboard one per line, the
way `dbt ls --output name` prints them, and **dbt ls** types the equivalent
command into the Terminal tab without running it, so you can press Enter and
compare.
With the tests eye closed, add `--exclude "resource_type:test"` to dbt's side,
which is the only routine reason the two counts differ.

A method this build does not know, `state:` for instance, is refused by name
rather than ignored, because a silently dropped term would draw far too much and
look right doing it.

### Named selectors

The **Selectors** button beside the box lists the selectors the project defines
in `selectors.yml`, with their descriptions, filtered as you type. Picking one
writes `--selector name` into the box, which you can also type, or paste as
`dbt ls --selector name`. A named selector stands alone, as it does in dbt,
which ignores `--select` and `--exclude` beside it: the box refuses them
instead.

It is resolved from the manifest too, where dbt stores every selector already
parsed, and resolved whole. Each criterion keeps its own `indirect_selection`,
so a `buildable` one keeps only the tests whose inputs all sit upstream of the
selection, and an `empty` one keeps none, as `dbt ls --selector` would
([0032](../decisions/0032-named-selectors-from-the-manifest.md)).

The tests **eye** means something else here. The selector's definition
already decided which tests belong, so the box only decides what is drawn.
Either way the models the selected tests belong to are drawn, dimmed when the
selector did not select them itself, so you see what a test reads and how the
tests chain. On, the tests are drawn too, each hanging off its models; off,
only the test boxes go and the corner counts them, so a selector that keeps
only tests shows the models they check. The count, **Copy names** and **dbt ls**
are the selector's own answer either way.

Two things are said rather than guessed. A selector using `state:`, `result:`
or `source_status:` is listed but refused, with the reason, since those compare
with another run's artifacts, which this tool does not read; **dbt ls** still
types its command. And some dbt Fusion releases write the manifest without
`indirect_selection`. When `selectors.yml` sets it
and the manifest lost it, every criterion answers with dbt's default until the
project is re-parsed with a dbt that keeps it, and an amber note says so on
each selector whose answer that could change. A selector of tests alone never
shows it: it brings no tests of its own for a mode to let through.

### From the command line

The same engine answers without the browser. `--select` resolves one line,
prints what it matches and exits, so a script or an agent skill can ask for a
lineage with no server, no profile and no dbt:

```
dbt-edith path/to/project -s '+stg_orders+'          # one name per line, as dbt ls --output name prints them
dbt-edith path/to/project -s 'stg_orders+' --exclude 'resource_type:test' --output json
dbt-edith path/to/project --selector recon_tests
```

It takes what the box takes: the same methods and operators, `--exclude`
inside the line or as a flag of its own, a pasted `dbt ls -s ...`, and a named
selector as `--selector name`. Tests count as they do in `dbt ls`, a test
joining whenever one of its parents is selected, and
`--exclude 'resource_type:test'` leaves them out. `--output json` prints one
object per node, with dbt's key names (`unique_id`, `resource_type`,
`original_file_path`, `fqn`, `materialized`, `tags` and the rest) and
`depends_on.nodes`, the node's parents, which is what tells the edges between
two sets. Its `name` is the one `--output name` prints, `crm.customers` for a
source and `orders.v2` for a version, where dbt's own JSON writes the name
bare, and a version carries the manifest's `version` and `latest_version`.
`--output compact` prints a tab-separated line per node under a header
instead, a fraction of the JSON's size: see
[command-line.md](command-line.md#a-selection).

stdout carries the answer and nothing else. stderr says what it is worth: the
manifest that answered, when it was written and by which dbt, then the verdict
the [freshness badge](lineage.md#is-the-lineage-still-true) would give, naming the files
newer than the manifest. The answer is the manifest's, not the working tree's,
so after editing models, run `dbt parse` first or read that line.

| Exit | Meaning |
| --- | --- |
| 0 | answered, an empty answer included, which stderr says |
| 1 | refused: a method this build does not know such as `state:`, a malformed term, or a selector it cannot resolve, with the reason and a caret under the term |
| 2 | nothing was asked: no project, no manifest, an unreadable one, or a wrong flag |

With `--select` or `--selector` nothing is served: no port, no key, no browser,
no settings read or written, no catalog
([0057](../decisions/0057-a-selection-is-answered-on-the-command-line.md)).
It reads the manifest dbt already wrote, where `dbt ls` parses the whole
project again first.
