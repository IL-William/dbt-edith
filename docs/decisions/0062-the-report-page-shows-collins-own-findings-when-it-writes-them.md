# 0062. The report page shows collin's own findings when it writes them

Date: 2026-10-10 · Status: accepted · Amends 0059

**Trigger:** read before changing what the report page lists for a collin that
writes `findings`, what the badge counts for it, or what the comparison of two
versions files.

## Context

0059 filed collin's report here, from the lists every collin writes, and
rejected a findings section written by collin: only a report from the next
collin would have it, and comparing versions needs the old ones too.

collin 0.5.0 writes one anyway (its 0040), for its own CLI and any reader that
is not dbt-edith: the models grouped by cause, each with who can act, a
severity, what collin saw and what it usually means, and what to do. Its
words are written beside the SQL that produces each finding, and say things
this page cannot, such as that many compiles failing at once usually mean the
target was compiled against another environment's databases.

## Decision

- **The report page shows collin's findings when the report has them**, in
  the groups the page already uses: `inputs`, the manifest and catalog dbt
  wrote, under "Rerun dbt"; `project` under "Fix in the project"; `collin`
  under collin's own. An owner this page does not know is checked by hand.
  collin's order and words are kept.
- **Without them, nothing changes:** a report from a collin before 0.5.0 is
  filed and worded here, as 0059 has it.
- **The badge counts what the page lists:** the models collin's findings name,
  each once, when there are some.
- **The comparison of two versions still files both sides here** (0061), from
  the lists every collin writes, so two versions are compared on one filing
  whether or not either wrote findings.

## Rejected

- **Keeping 0059 as it was.** The page would word a cause collin words better,
  next to the code that finds it, and miss what only collin can say.
- **collin's findings in the comparison too.** A 0.4.0 report has none, and the
  comparison would set two filings against each other.
- **Merging the two lists.** Each kind would be said twice, in two wordings.

## Consequences

On a 3507 model Snowflake project, collin 0.5.0: three findings, two views
older than their code, three models reading only their own table, and YAML
behind 1282 tables. The kinds filed here stay computed for the comparison,
and for any collin before 0.5.0.
