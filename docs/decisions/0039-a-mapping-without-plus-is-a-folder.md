# 0039. A key holding a mapping is a folder, whatever its name

Date: 2026-10-05 · Status: accepted · Amends 0036

**Trigger:** read before deciding whether a key of `dbt_project.yml` is a
config or a path, or before touching `DBT_CONFIGS`.

## Context

0036 tells a config from a folder "by name and then by shape": a key is a
config when it starts with `+` or is one of dbt's config names, which
`DBT_CONFIGS` lists. It says that is dbt's own rule. It is not.

0038 found it by comparing its levels with an independent walk of the same
file: 31 models of the 18 825 node project disagreed, every one under a folder
named `contract`. Its project file has ten `contract:` keys holding `+schema`,
`+tags` or `+materialized`, and the manifest shows dbt applying them: one of
those tags is written nowhere else in the project. The name list read each of
them as the `contract` config, so none of the ten, nor the fourteen folder keys
below them, was linked, and 0038 left those 31 models a level short.

dbt-core states the rule in two places, `_get_config_paths` in
`config/runtime.py` and `_project_configs` in `context/context_config.py`: a
key holding a mapping is a path segment unless it starts with `+`; anything
else is a config. No list of names is consulted. Run on an invented project
with both engines:

| written | dbt-core 1.11.11 | dbt Fusion 2.0.6 |
| --- | --- | --- |
| `contract:` holding `+tags`, folder exists | folder, tags applied | folder, tags applied |
| `schema:` holding `+tags`, folder exists | folder, tags applied | folder, tags applied |
| `meta:` holding `owner:`, no such folder | unused config path, not applied | unused config path, not applied |
| `docs:` holding `node_color:`, folder exists | folder, `node_color` a custom key on its models | folder, `node_color` an error |
| `+meta:` holding `team:` | applied | applied |

Fusion is stricter than the rule, not different from it: it refuses a config
written bare at all, `materialized: view` included.

## Decision

The shape alone decides, as in both engines: a key holding a mapping is a path
segment unless it starts with `+`, whatever its name. `projectPathKeys` drops
the name test, and a folder named `contract` or `schema` links like any other.

`DBT_CONFIGS` stays, for a smaller job: naming the mistake the rule punishes.
A key placed nowhere whose name is a config, a `docs:` or `meta:` written
without its `+`, gets a card saying dbt reads it as a folder, finds none, and
applies nothing under it, and that `+docs:` is the config. dbt says as much
only once, at parse time, as an unused path; Fusion as an error on the key
below it.

Measured on that project: 571 keys where there were 547, the 24 new ones being
the ten `contract:` keys and the fourteen folders below them, every one placed
on disk in 150 ms for the whole round trip. 0038's levels now agree with the
independent walk for all 3 825 models.

## Rejected

- **Keeping the list, with an exception for a config name holding `+` keys.**
  It would have fixed `contract:` and nothing else, with a guess standing in
  front of a rule dbt states in one line, and still read a bare `meta:` as a
  config dbt never applies.
- **Dropping `DBT_CONFIGS`.** A dashed `docs:` would then say "no such folder",
  which is true and explains nothing: the reader meant a config, and the card
  is where to say what dbt made of it.

## Consequences

A project writing `docs:` or `meta:` without its `+` sees that key dashed
where it used to be plain text. That is the point: dbt-core ignores it with a
warning and Fusion refuses it, so the old plain text was claiming a config
dbt never applied. Where a folder of that name exists, the key links to it,
which is also what dbt does: it applies what is under the key to that folder,
as custom keys. A config name missing from the list costs only the sentence in
the card; the link itself no longer depends on it.
