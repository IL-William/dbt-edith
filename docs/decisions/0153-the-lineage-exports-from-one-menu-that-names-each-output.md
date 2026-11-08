# 0153. The lineage exports from one menu that names each output

Date: 2026-11-08 · Status: accepted · Amends 0026, 0035, 0097 and 0140

**Trigger:** read before changing an output, a word, the head or the file name
of the Export menu, or the look of the toggles above the lineage.

## Context

The canvas left by two buttons, Export (the HTML page) and Copy image (the PNG
on the clipboard); the PNG as a file had none. The design draws one Export
button whose menu names each output, with borderless buttons around it.

## Decision

- **One Export button, one menu**, opened by `openPopup`. Its head says what the
  canvas shows and `exportCount` how much (`4 nodes · 3 edges`, `8 columns`,
  tests counted as nodes); each row says its use and format (`EXPORT_ROWS`);
  its foot is `exportName` without its extension, on the exporter's local day.
  With nothing drawn the head says so, the rows cannot be picked and there is
  no foot.
- **The three outputs keep their contents** (0026, 0106, 0150). Save as image
  is new: the same PNG, saved once it is drawn, after the click, as the
  clipboard's fallback always was; the HTML page is still saved inside the click.
- **A pressed toggle is a state, not a warning.** Tests, By folder and Indirect
  take the `--select` wash and the text colour; the Tests eye is always open,
  and the number of tests drawn moves to its tooltip. Indirect takes the
  design's dashed arrow, where 0097 gave it an eye. Stop at keeps its accent,
  being a state of the walk.
- **Fit is its glyph alone**, a thin line separates the groups of tools, and
  the dock's modes are one quiet group with the mode in front raised: Settings'
  controls keep theirs. The third mode reads Selection.

## Rejected

- **Two buttons in a full bar** (Export-Options, option A): the PNG file would
  have made a third.
- **A preview panel with "Light, for print"** (option C): 0150 settles a file's
  theme.
- **Amber for "on"**: it spends a colour that means something on a mere state.
