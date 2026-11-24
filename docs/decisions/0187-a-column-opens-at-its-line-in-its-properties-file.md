# 0187. A column opens at its line in its properties file

Date: 2026-11-24 · Status: accepted

**Trigger:** read before changing where a column's YAML glyph lands, or what
file it opens.

## Context

Nothing in the Catalog took you to where a column is declared. To edit its
description or its tests you opened the schema file and searched for the name,
in a file of a few hundred lines that holds dozens of columns. A test already
opens at its own line (0041), found over the editor's buffer. A column wants
the same, and the manifest gives it no line either.

## Decision

A YAML glyph after the column's name, and after its KEY tags, opens the node's
properties file: a model's `patch_path`, a source's own file, a snapshot's when
YAML declares it. The browser finds the line over the buffer, as 0041 does,
through the lookups the two share. Under the entry of the node (a source table
under its source) it takes, in this order:

1. the column's item in the `columns:` of the version, for a versioned model
   whose `v:` equals the node's version, as text or as a number, so `2` and
   `2.0` agree;
2. its item in the entry's `columns:`, where a version inheriting with
   `include: all` finds it;
3. the nearest thing found, for a column the YAML does not declare (one only
   catalog.json lists, or written another way): the version's `columns:`, the
   entry's, the version's item, the entry, so a column can be added there.

A name matches by its exact spelling first, then whatever its case. A column
found lands on its `name:` key and lights its whole item, description and tests
with it, less the blank and comment lines after it; the nearest thing lights
its own line. A click opens a preview tab and a double click keeps it; in the
Lineage space's card, where opening a file closes the panel and a second click
would fall through to what lies beneath, a click keeps the tab and no double
click is bound. The row's own click still opens the column lineage, which is
why the glyph sits in the name cell, outside that target, and why it stops the
click there.

A node of another package, a file under `dbt_packages/` and a static site have
no glyph.

## Rejected

- **A line sent by the server**: 0018 and 0003 refuse a YAML parser, and the
  buffer holds edits the file does not.
- **A click or double click on the row**: the row opens the column lineage,
  which can start collin, and the name stays selectable for copying.
- **A package's YAML**: `dbt deps` rewrites those files, so an edit there is
  lost, and the manifest does not place their properties file under
  `dbt_packages/`, so the node's package says it too.

## Consequences

A versioned model's test chips still look for `orders.v2` where the YAML says
`name: orders`, so they land at the top of the file. The base name taken out of
`versionChip` for this is what a fix for them reuses.
