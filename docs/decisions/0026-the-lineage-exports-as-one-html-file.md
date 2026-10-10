# 0026. The lineage exports as one HTML file, built in the browser

Date: 2026-09-23 · Status: accepted

**Trigger:** read before changing what an exported graph carries, or adding a
way to export one.

## Context

A lineage is often needed where dbt-edith is not: in a ticket, saying which
models a release rebuilds. Its readers have nothing installed. A tracker shows
an image or a PDF in place, and only downloads an .html file to be opened.

## Decision

Export serialises the SVG already on the canvas (`Lineage.snapshot`) and wraps
it in one self-contained page (`exportDocument`) that the browser saves like
any download. The server is not involved. The page pans and zooms by rewriting
its viewBox, so it is complete without scripts and prints to one vector page;
that print is the PDF. Copy image rasterises the same SVG. A policy in the file
forbids every fetch. Its header carries only fields passed by name: the `dbt
ls` command that lists the same nodes, and when and from what it was drawn,
never an absolute path or a value from a `.env` file.

## Rejected

- **Generating a PDF**, by hand or with jsPDF and svg2pdf vendored, for what
  printing the page already gives: vectors and searchable text.
- **A PNG alone.** It blurs when zoomed, which is what the file is for.
- **A server route.** A write path (0011) for a layout that exists only in the
  browser (0006).
- **A bare .svg.** No command, no legend, no pan, and Windows often opens one
  in a viewer that ignores its CSS.
- **A .txt beside it for the command.** Two downloads per click trip the
  browser's multiple-download prompt, and the files part ways in a ticket.
- **Copying the canvas CSS through the CSSOM.** It hands over what the
  exporting browser made of the rules, dropped ones included, and no harness
  can test it.
- **Shipping lineage.js and the payload to redraw.** Nothing shows without
  scripts, and JSON inside a script needs escaping of its own.

## Consequences

The canvas rules exist twice, in app.css and in `exportCanvasCss`, and
`web/tests/export.js` holds every copied line to app.css. `exportViewer` is
inlined with its own source text, so it may name nothing from app.js, which the
same harness checks. An export is a snapshot: it states when it was made.
