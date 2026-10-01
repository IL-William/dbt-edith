# 0036. A config key in `dbt_project.yml` links to its folder on disk

Date: 2026-10-01 · Status: accepted

**Trigger:** read before linking anything in `dbt_project.yml`, or before
reading `model-paths` and its siblings.

## Context

The editor links `ref()`, `source()`, a macro call and the names a properties
file declares. The one file where nothing was linked is `dbt_project.yml`, and
`yamlDeclared` says why in a comment: "the `models:` of `dbt_project.yml`, a
mapping of configs, declares nothing". It declares no node, but its keys are
folders, so reading a per-folder config and then going to see that folder meant
finding it by hand in the tree.

Two things were missing. Which key is a path: a key under `models:` can be the
project name, a package name, a folder, a resource name, or a config written
without its `+`. And where a path starts: `model-paths`, `seed-paths`,
`snapshot-paths`, `analysis-paths`, `macro-paths` and `test-paths` were read
nowhere in the binary, which `src/freshness.rs` and 0025 already called a known
gap.

## Decision

The shape is 0028's, one resource kind over. The browser scans the open buffer
for the chains of keys under the blocks whose keys are paths, and the server
places each chain, answering only for the ones it finds inside the project,
through `files::resolve`. A folder first, then the per-block extensions on the
leaf, since the last key of a chain may name one model rather than a directory.

`src/project.rs` reads the six path keys with the scanner it already has for
`vars:`, in all three shapes a project writes them (`["models"]`, `models`, and
a block sequence), with the pre-1.0 aliases `source-paths` and `data-paths` and
dbt's defaults when a key is absent. A value it cannot read gives that block no
roots at all, and so no links: refusing is 0018's rule, and a guessed root
would point a config at the wrong folder.

**What the first segment means is the server's decision**, not the scanner's:
equal to the project `name:` it is the level dbt requires and is dropped,
otherwise a folder under `dbt_packages/` or `dbt_modules/` of that name makes it
a package, as 0028 looks for one, and anything else is tried as written. A
package is known by that folder rather than by a node carrying its name,
because a package of macros alone owns no node and would otherwise read as a
missing folder. Inside one the conventional roots are used, since its own
project file is not read, and a key that is nothing but the package name opens
the package folder itself. Several roots resolve to the first one that exists,
and the hover card names the others.

**A key that resolves to nothing is marked, dashed, and its card says why.**
dbt only warns at parse time that such a config configures nothing, so this is
worth saying where the key is. What keeps the mark honest is that a config may
be written without its `+` in this file: `+materialized: table` and
`materialized: table` are both legal, and the second looks exactly like a folder
name.

So a key is kept out the way dbt keeps one out, by name and then by shape. dbt
reads a key as a config when it starts with `+` or is one of its config names,
and as a path segment otherwise, so `DBT_CONFIGS` in `web/app.js` carries those
names: every key of `config` on a model, a seed and a test node of an 18 825
node manifest, plus the snapshot and seed configs the documentation lists,
which that project has none of. The shape rule then catches anything the list
has not heard of: a key carrying a value on its line opens no mapping, so it can
name no folder.

The manifest keeps one bounded job, and never the link: the count of nodes
under the resolved folder, so a folder that exists and holds nothing says so in
the card rather than looking the same as one full of models. The card only
calls that config dead when the freshness badge says the manifest matches the
project (0025); otherwise it names the stale manifest first, because a folder
added on a branch since the last parse is in no manifest either.

A key under `data_tests:` is looked for in the test tree **and** in the model
tree. That is not a hedge between two guesses: a singular test's fqn mirrors its
file under `test-paths`, while a generic test's mirrors the path of the model it
hangs on, so one key names a folder in one tree and the next names one in the
other. Both shapes sit in the same block of the project this was built for, and
the manifest shows dbt applying the config through each: 329 tests carry a tag
set through a key that only exists in the model tree.

## Rejected

- **Deriving the folder from the manifest**, through each node's `fqn`, which is
  exactly what dbt matches these keys against and is already in memory. Better
  semantics, and it still loses: the tree reveals what is on disk, a folder can
  exist with no model in it and is still worth opening, and the server runs with
  no manifest at all, which is the likeliest state right after a folder rename.
- **A YAML parser**, for the third time (0003, 0018, 0022).
- **Reading each package's own `dbt_project.yml`** to learn its roots. One more
  file read for a setting rarely moved; the conventional roots cover the rest,
  and a package that moved its own gets no link rather than a wrong one.
- **The shape rule alone**, with no list of config names. It reads
  `materialized: table` correctly and `docs:` with a mapping under it wrongly,
  and dbt's own answer is the list, so leaving it out would mean being less
  right than dbt on purpose.
- **Linking the values of `model-paths` itself.** The scanner walks keys; a
  flow sequence of values needs its own ranges. An obvious follow-up.

## Consequences

The gaps, so the next reader does not find them as bugs. A key holding a slash
(`staging/crm:`) is refused: to dbt that is one name and matches nothing, so
linking it would point at the one folder the config does not reach. A config dbt
adds after this list was written, used bare and with a mapping under it, reads
as a missing folder until the name lands in `DBT_CONFIGS`; and a folder
genuinely named after a config stays plain text, which is the safe way round,
since dbt would not configure it either. A package with a custom `model-paths`
links no further than its own folder.
A package's own `dbt_project.yml`, opened from `dbt_packages/`, is not linked at
all, since its roots and its name are its own. The chains come from the buffer
and the roots from disk, so an unsaved `model-paths` edit is not honoured until
it is saved, the same staleness the ref links carry against the manifest.
