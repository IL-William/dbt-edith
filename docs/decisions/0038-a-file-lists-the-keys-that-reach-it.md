# 0038. A file lists the keys of `dbt_project.yml` that reach it, by its fqn

Date: 2026-10-05 · Status: accepted, amended by 0039

**Trigger:** read before showing anything about where a node's config comes
from, or before deriving an fqn from a path.

## Context

0036 goes from a key of `dbt_project.yml` to its folder. The other way, from a
model to the keys configuring it, still meant reading the project file and
matching folders by eye. The obvious version of that, "open the key for this
model's folder", does not survive the project this was built for: of its 3 825
models, 362 have a key on their own folder, 3 185 are configured from one
folder up, 276 from two to four folders up, and 2 by their own name. Every
model sits under three to eleven nested keys below `models:`, each of which
can set something.
So the answer is a list of levels, not one place.

## Decision

dbt's own rule decides which keys reach a file: a key applies to every node
whose fqn its chain begins (`fqn_search` in dbt-core). The fqn is built from the
path, the way dbt builds it: the project's `name:`, the folders under the
resource path the file sits in, and the file's name without its extension. That
matched the manifest's `fqn` for all 3 341 models, 8 seeds and 609 singular
tests of that project, and it needs no manifest, which 0036 chose for the same
reason.

The browser does it, as it scans the keys for 0036: the project file comes from
the open buffer when there is one, so an unsaved key counts, and the server is
asked only for the roots, by `/api/project/resolve` with no keys, which stats
nothing. The longest root holding the file gives its kind, and with it the
blocks whose keys can reach it. A model is never reached through `data_tests:`:
those keys configure its tests, not the model, and the test block's roots
include the model roots only for that reason (0036). A singular test under
`test-paths` is reached through both test blocks. A file under `dbt_packages/`
or `dbt_modules/` takes its package's name and the conventional roots, as 0036
places a package. The key of the block itself reaches the whole kind, packages
included, and a first key that is neither the project nor the package reaches
nothing, as in dbt.

A button at the end of the breadcrumb opens the list, broadest first, indented
as the project file nests it, each level with what it sets on its own line of
YAML; picking one opens `dbt_project.yml` at that key. **It shows where, never
what wins.** The menu says so in its first line: `dbt_project.yml` is the lowest
of dbt's three places for a config, under the file's `config()` and its
properties YAML, and `tags`, `meta` and the hooks add up across levels rather
than replace each other. What a level sets is read by dbt-core's test, a `+` or
a value that is no mapping, rather than by `DBT_CONFIGS`, so a config dbt added
after that list still shows.

## Rejected

- **Placing every key with `place()` and keeping those whose folder holds the
  file.** A stat per key on every click, and wrong for a key without the
  project level, which `place()` tries as written and dbt reads as a package.
- **The manifest's `fqn`.** Exactly the right value, and absent in the state
  where a file was just moved, as 0036 found for the forward direction.
- **The config the file ends up with**, values struck through when a deeper
  level replaces them. Right only for the configs that replace, and only once
  the file's `config()` and its YAML are read too; the manifest's resolved
  `config` holds the answer but not where each value came from.
- **`data_tests:` keys on a model.** They are about its tests.

## Consequences

A versioned model's fqn ends with its version and its file name carries it
too, so a key naming the model itself reaches no version of it here. A legacy
snapshot is named by its `{% snapshot %}` block, not its file, so a key naming
one snapshot is missed the same way; folder keys are unaffected in both. Files
under `tests/generic/` are macros, and still get the levels of the test blocks.
The roots come from disk, so an unsaved `model-paths` edit is not followed, as
in 0036. Which keys are paths is 0036's reading, as corrected by 0039: before
it, 31 models of that project were missing the level of a `contract:` folder.
