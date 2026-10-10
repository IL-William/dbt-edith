# 0051. A relation opens in Snowsight by its name, on the session's account

Date: 2026-10-10 · Status: accepted · Amends 0048

**Trigger:** read before linking a dbt object to Snowsight, or changing how a
Snowsight address is spelled.

## Context

The Catalog's Location table gives a relation's full name with a Copy button.
Looking at the object itself, its data, its grants or its schema, meant pasting
that name into Snowsight's search. 0048 already builds a query's page from the
organization and account names, but only the history read them.

## Decision

- **A Snowsight menu beside Copy**, on both relations, resolved and built, with
  Snowflake's features on: the object, its schema, its database.
- **The account is the session's**, asked of the script by a new `session` op,
  which connects if nothing has yet. So the first menu may bring a sign-in tab,
  and waits for its click (0016). The page keeps the answer until the profile
  is saved or the features go off; a history page refreshes it.
- **Reading through the menu keeps the script running**, as the history does
  (0048): the flag that kept it for the history now keeps it for both.
- **The address is spelled from the name**:
  `#/data/databases/<DB>/schemas/<SCHEMA>/<kind>/<NAME>`, each part as
  Snowflake keeps it, an unquoted one in upper case, a quoted one as written.
  The kind comes from the materialization: `view` for a view or a materialized
  view, `dynamic-table`, and `table` for everything else.
- **Links, not calls.** The entries are anchors the browser opens in a new tab
  with no referrer; the menu waits for the account, the click opens the page,
  so no popup blocker stands between them.

## Rejected

- **The account from `profiles.yml`.** An `orgname-accountname` identifier
  would do, a locator would not, and 0048 lets only target names and roles
  leave the profile.
- **Asking Snowflake what the object is** (`SHOW OBJECTS`), for a source's kind
  and the name's case: a query per click, for what the name already says.
- **Opening the page after the account arrives**: a sign-in outlasts the click's
  permission to open a tab, and the page would be blocked.

## Consequences

Snowsight's addresses are not documented. Every part above was written from
how Snowsight spells them, not from a published scheme. A real account's
addresses for a database, a schema, a table and a view match it; a dynamic
table's and a materialized view's still wait on one (docs/state.md). A source
that is a view opens as a table.
