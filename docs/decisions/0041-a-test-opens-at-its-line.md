# 0041. A test opens at the line that declares it

Date: 2026-10-05 · Status: accepted

**Trigger:** read before changing where opening a data test lands, or what
`host` in a payload means.

## Context

Opening a generic test opened its properties file at the top, and a file of
a few hundred lines holds dozens of tests. The manifest gives a test no line,
under dbt-core or Fusion: only its file, the node it is attached to, its
column and its generic. On the project measured, 13 000 generic tests sit in
3 000 files, under `tests:` four times as often as `data_tests:`, and one in
seventy shares its node, column and generic with another, differing only by
its arguments.

## Decision

The server sends a test's `host`, the name of the node whose YAML declares
it: `attached_node`, else the parent declared in the test's own file, a
source test under dbt-core. The browser finds the line over the buffer
(0022): the host's entry, its column's tests or its own, then an item naming
the generic, bare or through its package. Of several, the one whose `name:`
is dbt's name for the test wins, else the first. Short of the item it lands
on the nearest thing found, and short of the entry at the top of the file.
A click on a test row, a Catalog chip, the Preview and Open file all land
there.

## Rejected

- **Reading the YAML on the server**: 0018 and 0003 refuse a YAML parser, and
  the buffer holds edits the file does not.
- **Sending `file_key_name`**: a string per test, kept for thousands of them,
  and no table for a source.
- **Telling two of one generic apart by their arguments**: no payload holds
  them, and dbt hashes them into a long name.

## Consequences

Measured over that project's tests, all but nine land on their own item, and
those nine on their model's tests, their column written in the YAML otherwise
than dbt recorded it. A test declared in a file other than the one shown
replaces the preview tab, so Back has only the pinned tabs to return to.
