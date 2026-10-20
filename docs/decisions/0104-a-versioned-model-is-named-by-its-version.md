# 0104. A versioned model is named by its version

Date: 2026-10-19 · Status: accepted · Amends 0024

**Trigger:** read before changing what a node is named, what a model's bare
name reaches, or how the implicit method matches a name.

## Context

A versioned model is one node per version in the manifest, each with the
model's `name`, its own `version` and the model's `latest_version`, which dbt
fills with the highest when the YAML names none. The graph read `name` alone:
v1 and v2 drew two boxes of one name, the Catalog copied a name `--select`
reads as every version, and the implicit method matched `v2`, the leaf dbt
never matches on a versioned model.

## Decision

- **A version is named `name.v<version>`**, dbt's `search_name`, as
  `dbt ls --output name` prints it, everywhere a name shows or is copied,
  `--output json` included, whose `name` is already a source's
  `source.table` where dbt's JSON writes both bare. The Catalog says beside
  it whether it is the latest, an old one or a prerelease.
- **The version is kept as the manifest wrote it**, a number or a string, and
  compared as dbt's `UnparsedVersion` compares: as numbers when both read as
  one, as text otherwise. Python's `==` and truth decide the rest: `1` equals
  `1.0` and not `"1"`, and a version 0 is never `old`.
- **The bare name reaches the latest**, as a `ref()` naming no version does,
  in the editor, the resolve route, `--describe` and `--column`: the version
  `latest_version` names, else the highest. `ref('orders', v=2)` and
  `version=2` reach that version; an unquoted value is a version only as a
  number, since `v=my_var` is not known before dbt renders it.
- **On the command line the bare name is pooled, not looked up last.**
  `--describe` and `--column` take each package's latest version and the
  nodes of that very name together, in the exact pass and in the one that
  ignores case, then narrow to the enabled ones and refuse several. A
  versioned model beside a model, a seed or the same model of another
  package is refused for the unique_id, where a single lookup by name picked
  one by the order of the manifest, and a disabled unversioned twin gives
  way to the versions.
- **The implicit method matches as dbt 1.12's `is_selected_node` does**: the
  model's name, or the fqn's two last parts joined by a `_` against the
  selector's two last, so `orders`, `orders.v2` and `orders_v2` reach v2 and
  `v2` alone reaches nothing. `version:` selects models only, and a value
  other than `latest`, `prerelease`, `old` or `none` is refused, as dbt
  refuses it.

## Rejected

- **The bare name kept, the version beside it**: what is copied would still
  select every version, and the page's names would not be `dbt ls`'s.
- **`--column` refusing a bare name for naming several nodes**: a `ref()`
  reads it as the latest, and the answer prints the unique_id it read.

## Consequences

A manifest with no versioned model answers byte for byte as before, but
that `version:` now answers and the unknown-method error lists it. dbt
Fusion 2.0.8 lists a prerelease under `version:none` too; dbt-core, which CI
compares against, does not, and neither does this.
