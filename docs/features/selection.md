# Selection

Drawing what a dbt selector matches, in the Custom selection box or from the
command line.

## Custom selection

The **Custom selection** button above the graph swaps the canvas from one model's
neighbourhood to whatever a dbt selector matches, so you can draw the set you
actually work on:

```
stg_customers dim_customers+ fct_orders
```

A space is a union, a comma is an intersection, and `--exclude` may follow on
the same line. Whole commands paste in: `dbt ls -s "a b" --exclude tag:x` is
understood, and the box rewrites itself to the part that was resolved.

| | |
| --- | --- |
| `my_model` | by name, or by a dotted path into the fqn (`shop.staging.*`, `staging.my_model`) |
| `+my_model`, `my_model+` | everything upstream, everything downstream |
| `2+my_model`, `my_model+3` | as far as that many levels |
| `@my_model` | it, its children, and every parent of those children |
| `tag:`, `path:`, `file:`, `package:` | the usual dbt methods, with `*` and `?` wildcards |
| `resource_type:`, `source:`, `exposure:` | by kind, by source, by exposure |
| `config.materialized:` | and `config.schema`, `config.database`, `config.alias`, `config.incremental_strategy`, `config.unique_key` |
| `--selector name` | a named selector from `selectors.yml`, [below](#named-selectors) |

Disabled nodes are never returned, as in dbt. Tests take part only when the
tests **eye** is open, and then each term brings the tests of what it
selected, a test joining whenever a parent of it did, which is dbt's own
default. So a test excluded by name stays out, and `+` and `@` reach tests the
way dbt's graph does. A term that matches nothing is called out under the
box rather than dropped, and it says whether the term matched only disabled
nodes or only tests. The count beside the box is the whole selection; the line
in the corner is what fits on the canvas, which says `400 of 2422 drawn` when a
selection is larger than the canvas will take.

**Nothing runs dbt for this.** The expression is resolved against
`manifest.json`, which is the same data dbt reads, so the answer arrives in
milliseconds and needs no profile ([0024](../decisions/0024-selectors-resolved-from-the-manifest.md)).
The price is that the answer is this tool's, not dbt's. Two buttons exist for
that: **Copy** puts the matching names on the clipboard one per line, the way
`dbt ls --output name` prints them, and **dbt ls** types the equivalent command
into the Terminal tab without running it, so you can press Enter and compare.
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
only tests shows the models they check. The count, **Copy** and **dbt ls** are
the selector's own answer either way.

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
two sets.

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
