# 0151. A file's icon is coloured only by what the manifest builds it as

Date: 2026-11-06 · Status: accepted · Amends 0148 and 0150

**Trigger:** read before adding a file type to the tree, colouring or reshaping
a file's icon, or showing git's status on a row.

## Context

The tree gave every file type a colour of its own: orange for SQL, amber for
YAML, blue for Markdown, green for JSON and CSV, grey for the rest. None of
them meant anything: an orange cylinder looked like a model, and the amber of a
YAML file was the amber of an incremental model. 0148 gave each meaning one
shape and kept colour for meaning; the design for the themes (0150) draws the
tree the same way: the shape says what the file is, the colour says only what
the manifest builds it as, and git's status is a letter at the end of the row.

## Decision

- **The shape follows the extension.** A cylinder for `.sql`, a dotted list for
  `.yml` and `.yaml`, a rounded M for `.md`, Py for `.py`, a terminal for `.sh`,
  braces for `.json`, a table for `.csv`, a page for anything else. They are
  drawn at 14 px, stroked 1.4, as the design's other icons are.
- **Colour means a materialization, or is neutral.** The file of a model takes
  `Lineage.nodeColor` of its node, the colour its box has on the canvas; the
  `.csv` of a seed and the `.sql` of a snapshot take the seed's and the
  snapshot's. A `.sql` the manifest does not define (an analysis, a macro, a
  test) keeps the cylinder in the neutral grey. The design names three more
  colours: dbt's violet (`--dbt-yaml`) for YAML, Python's yellow (`--py`) for
  `.py`, and the dimmer text for Markdown and shell. Both variables were
  declared in all four themes and read by nothing; Light's twins hold 3:1 on its
  sidebar (`web/tests/themes.js`).
- **The manifest's answer comes with the listing.** `/api/dir` gives a file
  `kind` and, for a model, `materialized`, when `Graph::node_built_in` finds a
  model, a seed or a snapshot whose own file it is: not a properties file, which
  the index lists for the nodes it declares, not a disabled node (the canvas
  dims it, so its file is grey), and not an installed package's node that was
  never found under `dbt_packages`, which would colour the project's own file of
  the same name. A Python model is a model: its Py glyph takes its box's colour,
  so a table does not read as an incremental model next to Py's yellow. The page
  keeps the marks by path (`S.fileMarks`), from every listing, the tree's, the
  breadcrumb menu's and a diff title's folder; a new manifest clears them, lists
  the opened folders again and redraws the icons that moved. With no manifest
  every `.sql` is grey, and a new model or a changed materialization shows once
  dbt writes the manifest, as on the canvas.
- **Git's status is a letter at the end of the row**, a `.gs` span the tree's
  paint adds and removes: M in the warning colour, U and A (a file staged as new)
  in the OK colour (`--git-m` and `--git-u`, Light's twins at 4.5:1 on its
  sidebar and a selected row, since the letter is 10.5px), dimmed on a folder
  that holds changes by mixing it into the sidebar rather than by opacity; an
  unsaved buffer has the pencil it has in its tab. Each has a tooltip and an
  `aria-label` (Modified, Added, Untracked, Holds changed files, Holds new
  files, Unsaved changes, Holds unsaved changes), and the row's own title still
  says the state in words. The name keeps its own colour.
- **Three pictograms take the design's drawing** (`--m-clock`, `--m-snow`,
  `--m-newer`): a clock of radius 6 with hands from `M8 4.5`, a flake of three
  plain lines, and an update arrow that points up. The other masks already were.
- **The `--f-sql`, `--f-yml`, `--f-json`, `--f-csv` and `--f-other` colours are
  gone.** `--f-md` is still the outline's heading mark and `--f-macro` a
  macro's.

## Rejected

- **Keeping the orange for any `.sql`**: it would say "model" about files that
  are not.
- **A request for the nodes' paths from the page**: the tree already lists the
  folder, and the server holds the index there.
- **Colouring a properties file by the first node it declares**: it declares
  many, of different kinds.

## Consequences

- A new file type needs a glyph and a class, never a colour of its own.
- The icons in the tabs and in Open editors are not drawn yet (the tabs keep
  their bar, 0148).
