# 0086. Edith's menu opens under the pointer

Date: 2026-10-15 · Status: accepted · Amends 0079

**Trigger:** read before opening a menu under the pointer, or changing what
closes or holds one.

## Context

Edith's menu (0079) is read rather than worked: the version, the releases
since it, when GitHub last answered. Its user asked to see it without a click.
A click-opened menu takes the focus and the arrow keys, which a pointer passing
by must not take from the editor or the terminal.

## Decision

- **It opens once the pointer rests on the name for 150 ms**, so a pointer
  crossing the corner on its way to the browser's tabs opens nothing.
- **Opened that way, it takes no focus.** A key typed closes it and goes on,
  as past a hover card, Escape too, which vim in the terminal may be waiting
  for. A modifier alone leaves it open for a Cmd/Ctrl + click on a link.
- **It closes 300 ms after the pointer has left both the name and the menu.**
- **A click on the name or in it holds it**, as a click-opened menu, focus
  included: clicking the name out of habit must not close what was just
  opened, and Check for updates keeps its answer in view. A second click on
  the name closes it. A click alone still opens it, for the keyboard and a
  touch screen.
- **No menu opens while another is open**, the popup or one of those keeping
  their own (`anyMenuOpen`): two would take the same keys.
- **The name has no tooltip**, which would land on the menu; an
  `aria-description` says what the dot shows.
- It is `openPopup`'s mode, wired by `openOnHover`, for any other menu to use.

## Rejected

- **Opening at once, as the levels box does (0080):** that box sits where a
  pointer goes on purpose; the name sits where it passes.
- **Taking the focus on hover:** what is typed in the editor would land in it.
- **Every menu of the top bar under the pointer:** the others are worked, a
  switch or a row to pick, and only Edith's was asked for.
