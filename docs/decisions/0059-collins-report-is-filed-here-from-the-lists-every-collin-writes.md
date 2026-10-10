# 0059. collin's report is filed here, from the lists every collin writes

Date: 2026-10-10 · Status: accepted · Amends 0042

**Trigger:** read before changing what the report page lists, what the badge
counts, or how a collin run leaves its report.

## Context

dbt-edith read two things out of `target/collin.report.json`: a summary for
the Columns tab, and the notes on the model on screen. Nothing showed the
report whole, so finding what a run turned up meant opening the JSON. The ask
was one page after a run, grouped by what to do, behind a badge rather than
in the way. Several collin versions side by side are asked for next, so the
page has to read a report from any collin someone still has.

## Decision

- **The server files the report by kind** (`src/collin_report.rs`), from the
  lists collin has written since 0.1.0: no compiled SQL, a set aside file,
  missing and unexpected columns, a stale YAML, an undeclared relation and the
  rest. The page words each kind and groups them by who acts: rerun dbt, fix
  in the project, check by hand, collin's to fix.
- **Two scopes of one name are a finding only where they set the compile
  aside.** From 0.4.0 collin reads them apart and names them on a model it
  calls parsed, where there is nothing to fix.
- **The badge counts models and tables, each once**, and shows only for a run
  finished since the page last opened the report. A run counter on collin's
  status tells two runs that both read done apart.
- **A run writes its report under a temporary name and renames it with its
  cache**, or removes the old one when it wrote none. Written in place, a
  failed run left a report describing edges nobody could see.

## Rejected

- **A findings section written by collin.** Only a report from the next collin
  would have it, and comparing versions needs the old ones too.
- **Wording the kinds on the server.** The page already words a column's notes,
  and one place for the words keeps the two from disagreeing.
- **Opening the page when a run ends.** A run starts in the background on a
  column click (0042), and a page opening then takes the screen from what was
  being read.
- **Counting findings.** A model in three kinds would count three times.

## Consequences

A list a later collin adds is ignored until it gets a kind here. On Fivetran's
Shopify package, collin at 0.4.0's code: 107 to investigate, 20 models whose
YAML is behind the warehouse and 87 tables two nodes claim; 13 models with two
CTEs of one name are read apart and not listed.
