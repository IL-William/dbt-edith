# 0158. The depth is written as dbt writes a selector

Date: 2026-11-08 · Status: accepted

**Trigger:** read before changing the depth control above the lineage, what its
menus list, or what its copy button writes.

## Context

The lineage's depth was two number boxes, `up` and `down`, that said nothing of
what a level adds or whether one exists: a model with no children took a 5 in
its down box and drew the same canvas. The design writes the depth the way dbt
does, `2+customers+1`, around the focus's name, each number opening a menu of
the levels that exist.

## Decision

- **Two buttons around the name, over the boxes.** `#up` and `#down` stay as the
  inputs holding the depth, hidden, `type=number`, 0 to 20: every reader and
  writer (the canvas's `+N` badges, the Catalog's counts, Back and Forward, the
  tour, the export) keeps working untouched. The control paints from them and a
  pick writes one and fires **one** `change`; two would race two requests. In
  selection mode the buttons are disabled, as the boxes were: the expression's
  `+` carries the depth there.
- **The shown number is held to the levels that exist, the box keeps its value.**
  A model with no children shows `+0` while `#down` still holds 1, so the depth
  still carries to the next model (the rule of 1.18.4, back to 1 unless the
  move came from the canvas, is unchanged). The side
  with no level is `aria-disabled`, not `disabled`, so its title can say why
  (`customers has no children`), and the other side's menu ends on a footer
  saying so.
- **The levels are counted on the server, for the page's two routes only.**
  `/api/lineage` and `/api/collineage` carry `levels`, which `--select`,
  `--column`, `--describe` and MCP never do, so their answers stay byte for byte
  as they were. Model mode: a plain walk per side over the graph nodes, without
  the canvas's cap, which is the walk `lineage()` makes and dbt's `N+model`
  selects (a level is what lies at that shortest distance; tests are leaves and
  are left out). Column mode: the draw's own walk, extracted from
  `column_lineage` into one function that both share, run one level deeper than
  the menu lists, with the request's cap, cut roles and indirect switch. Upstream
  is walked alone; the downstream levels follow the request's own upstream walk,
  because the canvas shares one map and one cap between the two sides (a column
  met upstream is not counted again, and an upstream side that fills the cap
  leaves the downstream one cut at its first step, which the menu says as the cap's
  cut and never as a childless model), so a
  row says what the canvas would draw. A column counts at its distance, a stop
  included, a model's box at the step the walk first boxed it. Twenty levels are
  listed, a twenty-first only says the graph goes on; a level the cap cut short
  is the last one listed, reads `or more` and is never `all`.
- **The copy writes dbt's syntax with a zero side dropped** (`Copied 1+customers`),
  from the numbers shown, and `source:` before a source's name, because dbt's
  name method searches no source. It is not offered in column mode: no dbt
  selector says a column's depth. It differs from the Export menu's `dbt ls`
  line, which writes nothing for a source; that is left as it was.
- **A long name is cut in the middle** (12 characters, an ellipsis, 11) past 24,
  in the control, the menu's head and its rows, with the whole name in the title
  and the accessible name. In a narrow dock the name is the first thing
  foldDockTabs folds, measured, before the tabs' labels and Export's: it gives
  way to the 3 by 12 bar of its kind (0148), the box's own `.dot`, since it
  repeats the editor's tab and the focus box.
- **The menus are the app's own** (`openPopup`): arrows, Enter or Space, Escape
  back to the button; a menu opened before the answer says `Counting the
  levels…` and fills in place, the focus kept. The row already drawn only closes
  the menu. A new ask closes a depth menu and never another popup.
  *Amended in 2.20.2: a menu whose request failed says `The levels could not
  be counted.` until the next ask.*

## Rejected

- **Typed numbers (board B1).** Nothing says what is there to type, and 20 is
  always accepted.
- **One text field holding `2+customers+1` (board B3).** A free-form selector
  invites what the canvas cannot draw and needs a parser beside the engine's.
- **Counting the levels in the page from a deeper answer.** The cap hides what
  lies past it, and a second request would follow every draw.
- **A route of its own, asked when a menu opens.** A wait in the menu, and the
  disabled side needs the answer before any open.

## Consequences

- Each page lineage request does a second walk on the server (two more in
  column mode); on a project's graph that is the cost of one more breadth-first
  search, with no payload.
- The levels follow what the canvas would draw, so they change with the roles
  stopped at, the indirect button and a cap a click lifted.
- Metrics, semantic models, saved queries and functions are not loaded, though
  dbt's graph holds them as parents and children, so `dbt ls` can count more
  than a menu row says.
- dbt's name method also matches a bare name against a package or a model's
  first folder (`is_selected_node`), so the copied `1+customers` selects the
  models under `models/customers/` too, as the Export menu's `dbt ls` line does.
