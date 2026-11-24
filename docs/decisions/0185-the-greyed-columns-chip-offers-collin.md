# 0185. The greyed Columns chip offers Collin

Date: 2026-11-24 · Status: accepted · Amends 0184

**Trigger:** read before changing what the Columns chip of `Cmd/Ctrl + K` does
while Collin is not the column lineage tool, or what the box offers beside its
answer.

## Context

0184 greys the chip when another tool is picked, with a tooltip, so the feature
can be found. On the first real try (11-24) the tooltip was a dead end: a click
or Tab did nothing, and the person who had just typed a column's name had to
leave the box, open the settings, pick Collin and come back to type it again.
The chip had said what was missing and then offered no way to supply it.

## Decision

- **A press on the greyed chip offers Collin.** A click or Tab toggles an offer
  and asks the box again, the focus staying in the input; the chip stays greyed
  and is pressed while the offer shows. The foot's `Tab columns` stays shown
  while the chip is greyed, hidden only on a static site, which has neither the
  chip nor a server to offer anything. The offer is off at every opening, as the
  chip is (0184), and the chip's tooltip is now `Columns need Collin: click or
  Tab to use it`. A chip that is available is as it was.
- **One row on top, selected first, above the usual answer.** Typing keeps it,
  so `amount` then Enter picks Collin and searches `amount` as columns. It is
  not a hit: the foot's count leaves it out, and the arrows and Enter treat it
  as any row. What it says follows collin as the page knows it:
  - collin found: "Use Collin for column lineage". Its pick switches the tool
    as the settings do and the chip wakes pressed, with the box asked once more.
    Switching to Collin runs it in the background when its cache is missing or
    older than the manifest, as the settings' pick does (0042), and the tooltip
    says so. The columns it brings are not there yet when the box is asked, so
    the box is asked again when the run ends, while it shows columns. A second
    pick while the switch goes on is ignored.
  - collin not found: "Install collin". Its pick closes the box and types the
    install command into the terminal, for the person to run, as the settings'
    row does (0042), the chip there saying it came from the palette (0161).
  - collin found and set aside (git tracks it, 0094, or another account can
    replace it, 0128): the row carries the note the settings carry, and its
    pick opens the settings on the Column lineage section, where the way out is.
    A collin found beside a refused one is the first row, as in the menu.
- **The wake is the switch's own.** The tool changing while the box is open
  already greys or wakes the chip (0184); with the offer showing it wakes
  pressed, so the answer is asked once whoever switched the tool. A pick whose
  switch failed (the page toasts) leaves the offer as it was, and a box closed
  or opened again meanwhile is not touched.

## Rejected

- **Picking Collin as soon as the chip is pressed.** A press would change the
  tool of the whole project, which every other view follows, on the strength of
  a chip that said it was off. The row asks first, and the pick is Enter.
- **Showing the offer in the answer's footnote.** The note under a columns
  answer says why a column may be missing; an action there is a line nobody
  would see while typing, and Enter could not reach it.
- **The offer without the chip.** The chip is where the feature is found
  (0184); a row appearing unasked above every search would be noise.
