# 0188. A schema YAML is brought in line with the warehouse by a patch you review

Date: 2026-11-24 · Status: accepted

**Trigger:** read before changing what Update YAML writes, where a type is
decided, how a properties file is edited, or letting any button write a file
without the user seeing the change first.

## Context

Edith already says a YAML is behind its table: the columns only the warehouse
has are in italics in Catalog > Columns, and collin lists them in Reports.
Nothing fixed it. Fixing it means editing a properties file, which dbt-edith had
never done, and deciding what to write for a type, which no file says: the
manifest holds the types a YAML declared, `catalog.json` those of the table, and
`merge_catalog` fills the first with the second in place, so after it a declared
type can no longer be told from a read one.

## Decision

- **A patch on the file's text, shown in the editor.** The button makes the
  change in the buffer of the YAML's tab, marks the lines, says what it did, and
  stops: nothing is written until the user saves. Removals are in the same
  patch for that reason, and not behind a switch: a column dropped from the
  table is a column to drop from the YAML, and the person who reviews the change
  is the one who sees what it cost, a description or tests, which the card says.
- **The text, not the manifest.** The patch is applied to the file as it is now,
  open or not, with unsaved edits too, never to what the manifest says of it. A
  YAML edited since the last parse is patched right, every change is checked
  against the text before it is made, and one the text no longer matches is
  skipped and said. Comments, quoting and every byte not touched stay: lines are
  inserted, replaced or removed, the file is never written back from a model of
  it. (The patcher also keeps CRLF and a BOM; the editor does not, it holds and
  saves plain line breaks, which is its own debt, in `docs/state.md`.) A shape a patch placed by lines cannot follow
  (flow style, an anchor, an alias or a merge key, several documents, a tab in the
  indentation, CR-only line breaks, a column with no name, `versions:`) is
  refused for that model with its reason, and a column declared twice is listed
  and never merged. No YAML crate (0003): `yamlOutline` already gives each key and
  item its line, depth and end.
- **The types stay on the server.** What two spellings fold to, the spelling
  written, the contract trap and the case of a name are decided in
  `src/yaml_sync.rs`, once, and sent as what to write; the page compares and
  decides nothing about a type. A second table in JavaScript would drift from
  the first, and the command line and MCP can answer from the first alone later.
  `RawCatalog` keeps what it threw away for this, on the structures the
  graph already has and not in a copy of every column: on each `Column` its
  `index`, so the table's own order, whether the YAML wrote its `data_type`, and
  the catalog's type where it is not the one held; on a `Node` the relation the
  catalog describes, so a catalog written for another schema is `not_checked`
  and never read as drift.
- **Base types, as a contract compares them.** `TEXT` is `varchar(100)`,
  `NUMBER` is `number(38,2)`, aliases folded on Snowflake and not elsewhere. A
  bare `timestamp` is never a mismatch against a `timestamp_*`, which one it means
  depends on the account's `TIMESTAMP_TYPE_MAPPING`.
- **The contract trap.** Snowflake's catalog says `NUMBER` with no precision, and
  a bare `number` in an enforced contract is `number(38,0)` in the table's DDL: it
  rounds decimals. On a model whose contract is enforced a number without
  precision is not written; it is listed as held, "read it from Snowflake". A
  missing column in that case is still added, with no `data_type`, so the tool
  never leaves a column out of the contract; dbt then asks for the type instead of
  building a table that rounds.
- **A catalog's age is told by file times** (0025), for one model at a time: the
  model's `run/` file newer than `catalog.json` says the table was rebuilt after
  the catalog read it, and older than the model's file, `dbt_project.yml` or a
  macro, or than its schema file on a model whose contract is enforced, says the
  table is not what the code now builds. A schema file changes a table's columns
  only through a contract, so without one it is not an input. Either asks first,
  naming the catalog's date, with Update anyway. No run file, no flag. The
  whole-project answer carries no flag, which cost a stat or more per model. On a
  contract model the schema file the update itself saves moves the second flag,
  until the model is run.
- **A catalog of a deployed environment is compared, and says whose.** A catalog
  is often written against a reference environment (a `.env` file) and not the
  manifest's target, and refusing it would refuse the most useful case. A model
  whose catalog relation is not the manifest's is compared with the environment
  whose `.env` file resolves the model's location, in full, to that relation (the
  first by file name when several do), and `not_checked` when none does. Only the
  files the environment menu shows are read, by its own rule (`envs::hidden`), and
  a file counts only if the place depends on it: a schema, a database and an alias
  that fall back to the manifest's, and one part read from the file, since a location written nowhere or literally is the same under every
  file and would be credited to the first. The answer carries only the file's
  name, never a value of any other (0012). The
  run-file flags say nothing of a deployed table, since a local run builds the
  manifest's target: it is `deployed`, with `code_after_catalog` when the model's
  file is newer than the catalog. The button then says the catalog describes that
  environment, built from code that may be behind the branch, so that a column the
  branch added shows as a removal and one it dropped as an addition: each removal
  is to be checked against the SQL.
- **Types alone are a status.** A model whose only differences are columns with no
  `data_type` is `untyped`, apart from `disagrees` (columns to add or remove, a
  type that is another type), so a project that declares no types does not list
  every model as disagreeing. The button is offered for both, and says `types only`
  for the first.
- **A model has no YAML:** the page proposes `<model>.yml` beside its SQL,
  creates it with `version: 2` only (an empty stamp creates a file that is still
  absent), and patches it in the editor like any other. The file stays on disk if
  its tab is closed unsaved. One that exists already, or is open, is opened and
  patched, never overwritten.
- **Only models.** Seeds, snapshots, sources, ephemeral and versioned models and
  an installed package's are not compared. `GET /api/yaml-sync` answers for the
  whole project or one model, so a bulk update can reuse it.

## Bulk: a folder from Reports

The Catalog updates one model, in the editor. Reports updates a layer, a folder
of models at any depth, and the editor cannot be the review there: it holds one
file at a time, and a folder is dozens.

- **A dialog is the review.** Update opens a wide `modal()` that reads each file
  from the disk with its stamp, runs the Catalog's patch on it for each of its
  models in turn, and shows file by file what changes and what a removal loses.
  Nothing is written before its own button, and a file changed on the disk since
  it was read is refused by the stamp and skipped, so the review is of what is
  written. The same patch, so the same refusals and skips.
- **What is left out, and why.** Models behind the catalog or the code (the
  freshness flags, asked for the models of the folder alone through the POST form
  of `/api/yaml-sync`, the whole-project answer carrying none), models a patch
  refuses, and files open with unsaved edits, which a write would race. **Also the
  ones behind** includes the first kind, never the others: a refused model cannot
  be patched, and an unsaved file is the user's.
- **A deployed catalog alone leaves no model out.** It flags every model, since a
  local run builds the manifest's target and none of its run files can be older
  or newer than a deployed table in a way that means anything, so leaving them
  out would leave out the whole project in the most useful case. It is said once
  at the top of the dialog, with the removals as the risk: a column the branch
  added reads as one to remove.
- **Types only is off by default.** A project that declares no types has every
  model with columns to type, and an update of that would rewrite every file in
  the project at one click. The models are listed, as `types`, and a switch brings
  them into the counts and into Update.
- **A new file has its full content.** A model with no YAML gets `<model>.yml`
  beside its SQL written whole, as a new file in the preview, created with the
  empty stamp. The file is looked for when the dialog reads, and one that is
  there then is patched like any other; one that appears between that read and
  the write is refused by the empty stamp (a 409) and skipped, said on the card.
  No stub, since no editor shows the change afterwards.
- **A patch is made once per file and kept.** Patching reads the whole file, and
  a file holds many models, so the plan is made file by file between frames, with
  its progress said, and a file's result is kept until the models it was made for
  change: asking for the models behind patches the files they are in and no
  other.
- **Reads a few at a time, writes one at a time.** Four reads in flight, writes in
  order: a folder of hundreds of files does not open hundreds of requests, and a
  skipped file is the one that changed.
- **The section is dbt's, quiet.** It reads `catalog.json`, which dbt writes, and
  never lights the tab: a project with no types would always light it.

## Rejected

- **Writing the file from the server.** The patch is the user's to review, and
  the editor's tab is where unsaved edits live; a server write would race them.
- **Rewriting the YAML from the manifest.** It loses comments and order, and
  patches what the file held at the last parse.
- **The type logic in the page too.** One table is one place to be wrong.
- **A bare `number` with a note.** The contract it would sit in would round, and
  the note would be read after the build.
- **Removals behind a switch.** Half an update leaves a YAML that still disagrees,
  and the review is what makes them safe.
- **Updating a folder in the editor.** One tab per file, dozens of dirty tabs to
  review and save one at a time; the dialog is the one place that shows the whole
  change.
- **Leaving a deployed catalog's models out as behind.** It would leave every one
  of them out.
- **Types only on by default.** One click rewriting a file per model of a project
  that declares no types.
