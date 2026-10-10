# 0048. The query history is read live, twenty at a time, and never kept

Date: 2026-10-08 · Status: accepted · Amends 0016 and 0017

**Trigger:** read before showing anything read from Snowflake other than
column lineage, linking to Snowsight, or changing when the script runs.

## Context

Seeing what one's own dbt runs and worksheets sent to Snowflake meant finding
the query again in Snowsight. The script answered lineage alone, and ran only
while Snowflake was the column lineage tool.

## Decision

- **`INFORMATION_SCHEMA.QUERY_HISTORY_BY_USER`**: the connected user's queries,
  live, for seven days, with no privilege to ask for. Its limit applies before
  any `WHERE`, so the script scans 10 000 and cuts the page after the filter.
- **Twenty a page, then "Load 20 more"**, each page starting after the oldest
  row shown, by start in epoch nanoseconds and then query id.
- **The connected target's role by default**, or another target's that signs
  in as it does, or every role. A target signing in as someone else is greyed:
  its queries are another user's history.
- **The script tags its sessions `dbt-edith`** and leaves those queries out.
- **Read when the tab opens or is refreshed; never polled, stored or logged.**
- **The script runs while any Snowflake feature wants it** (amends 0016), and
  stops when Snowflake's features go off.
- **Statements, errors and timings now leave the server** (amends 0017), the
  user's own, to their page, behind the same guard (0015). Of the targets,
  only names and roles leave.
- **Snowsight is a link, not a call**: built from `current_organization_name()`
  and `current_account_name()`, opened by the browser in a new tab with no
  referrer. Without an organization there is no link; the id is copied.

## Rejected

- **`ACCOUNT_USAGE.QUERY_HISTORY`**: imported privileges, and 45 minutes late.
- **A connection per target**: a sign-in each, for another user's queries.
- **Polling, a cache on disk, a large first page.**
- **A cursor on `END_TIME_RANGE_END`**: it drops a query that started before
  the cursor and ended after it.
- **Renaming `sf_lineage.py`**: it would move 0016, 0017 and the tests for a name.

## Consequences

The Snowsight URL, what one role sees of its user's other roles, and whether
reading the history wakes a warehouse wait on a real account (docs/state.md).
