# 0091. What speaks of the whole project opens in a panel

Date: 2026-10-16 · Status: accepted · Amends 0064

**Trigger:** read before adding a panel over the editor and the dock, moving
a view between the dock and a panel, or changing what opens, closes or covers
one.

## Context

0064 rolled the reports down over the editor and the dock, the sidebar beside
them, because a list of the whole project wants the window's height and the
dock is short. More views are asked to open the same way: a Custom selection
drawn at full height, the settings, which crowd the top bar until it breaks
onto two lines under about 1000px, the git changes, and a model's lineage
seen whole. Written one by one, each would decide again what goes inert,
where the focus goes back, what Escape closes and what rolls it up, and each
would decide it a little differently.

## Decision

- **One mechanism for every panel.** `PANELS` lists each one with its
  element, what paints its button, what it does once rolled up, and how the
  guided tour puts it back. `openPanel` and `closePanel` do the rest, and
  `S.panel` names the panel that is down. Every rule 0064 set for the reports
  holds for each: it covers `#main`, the editor and the dock, and never
  replaces them; what is under it is `inert`; anything that shows a file or a
  dock tab rolls it up, and so do Escape and the close keys, the focus going
  back where it was.
- **One panel at a time.** Opening one rolls up the one that was down. The
  focus a close from the keyboard gives back is the one from before the first
  of them.
- **What goes where.** What follows the file in front stays in the dock: a
  model's lineage, the Catalog, Compiled, Run, Jobs, and the terminal. What
  speaks of the whole project, or wants the window's height, opens in a
  panel: Reports, and after it Custom selection, the settings, git, and a
  model's lineage seen whole.
- **The tour notes the panel that was down**, rolls it up while it runs, and
  puts it back as it was left.

## Rejected

- **A mechanism per panel**: the rules above would be written several times
  and drift apart, and a panel opened over another would leave the first one
  down underneath, still holding the focus it should give back.
- **Several panels down at once**: each covers the whole of `#main`, so the
  one underneath could be neither seen nor reached.
- **One panel with a rail for everything**, the reports' rail holding the
  selection and the settings too: a selection is drawn, not worked through
  as a list, and a rail mixing reports with settings is two things to look
  for in one place.
- **A browser window of its own**: a second page with its own launch key,
  terminal and state to keep in step with the first, which a locked-down
  machine may also refuse to open.

## Consequences

Each later panel is an element with the `panel` class, an entry in
`PANELS`, and its way in. Moving a view out of the dock leaves the dock with
what follows the file, and the top bar with fewer buttons once the settings
move.
