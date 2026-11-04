# 0143. A sandbox manifest's relations are swapped where the file places them

Date: 2026-11-04 · Status: accepted · Amends 0139 · Builds on 0010

**Trigger:** read before changing when the Compiled or Run tab keeps a
relation as compiled, what `S.meta.sandbox` means to the swap, or how a
redirected location is judged.

## Context

0139 keeps a relation as compiled when dbt built it elsewhere than its config
says, since a `generate_*_name` macro moved it and that is not evaluated.
On a manifest built with a developer target whose macros put every model into
one `database.schema`, that is every model: every relation was kept for every
environment and the SQL never changed. That is the manifest a developer usually
has. 0010 already treats it: `S.meta.sandbox`, set when nearly every model is
built in one place, makes a move into it expected in the Catalog, and the
card's landing, the lineage's relation and collin's matching already give the
config's resolution per environment there. Only the swap refused.

## Decision

- **A relation built into the sandbox is not kept for having moved its
  database or schema.** `builtInSandbox(location, sandbox)` says whether the
  built `database.schema`, lowercased, is the sandbox, and the Catalog's
  location table and `envArtifactRelation` both ask it. The redirect refusal
  applies only to a relation built elsewhere than the sandbox, and to every
  redirect when the manifest has no sandbox. Its reason is unchanged.
- **The alias is not exempt.** The sandbox is a `database.schema`, so it
  explains where a model was put and says nothing of its name: a model whose
  alias moved went through a `generate_alias_name` or a configured alias the
  file would not reproduce, and stays kept even when built into the sandbox.
- **What the file resolves is where the config places it.** In a sandbox the
  redirect says only that the developer target was used. The `placing` finding
  under the bar names an override that reads the target, when there is one, and
  says it is not evaluated; an override that does not read it is not a finding.
- **The manifest's choice is named `my compile` on a sandbox manifest**, with
  the sandbox beside it (`dev_db.dbt_someone, as dbt wrote it`), in the
  environment menu, the settings' pick, the pill with no file picked and the
  Builds labels. There the manifest choice is the developer's own compile, and
  "manifest" did not say where that is. Its tooltip is exact about what it
  shows: the Catalog's locations stay as dbt parsed them, the SQL and the Built
  column are the compile's. The `.env` files keep their meaning: `.env.dev` stays the
  shared DEV environment. `manifestChoice(sandbox)` holds the words, so the
  four places cannot drift.

## Accepted risk

A project whose macro also moves the model under the chosen target gets the
config's name. This is what the Catalog's resolved column, the card's landing
and the Snowflake relation already show, and the finding names the override.

`S.meta.sandbox` means 95% of at least 20 models built in one
`database.schema`, not a developer target. A project with no custom schemas,
compiled for prod or CI, also reads `my compile`: the same weakness the
Catalog's "Sandbox manifest" note has.

## Rejected

- **Evaluating the override's condition on `target.name`** for each `.env`
  file's `DBT_TARGET`: 0139 rejected evaluating the Jinja, and it would be a
  second dbt.
- **Dropping the redirect refusal everywhere**: a move outside a sandbox, such
  as dbt's default `<target schema>_<custom schema>`, would be written wrong
  with no finding to say so.
- **A "my .env" setting** marking one file as the developer's: it takes that
  file's own meaning away, `.env.dev` being the shared DEV, and repeats what
  the manifest choice already shows.

## Consequences

On a sandbox manifest the Compiled and Run tabs now change with the
environment. A model built outside the sandbox in such a manifest is still
kept as compiled.
