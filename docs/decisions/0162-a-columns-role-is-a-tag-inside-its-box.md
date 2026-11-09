# 0162. A column's role is a tag inside its box

Date: 2026-11-09 · Status: accepted · Amends 0096, 0097, 0118 and 0160

**Trigger:** read before changing where a column box says its role, how a role's
colour is used as a word, the size of a column's box, how a lit edge is coloured,
or the shape a role has in the legend, the stop at menu and the column card.

Amends 0096 (where a column's badge sits), 0097 and 0118 (what a lit edge
shows), and 0160 (a column's box keeping its old look until its own step).

## Context

A column's role, `AGGREGATE`, `TRANSFORM`, `RAW`, was a tag drawn in the 14px
above the box's right end. On a canvas of columns stacked 14px apart it sat
against the bottom of the box above it and read as that box's. A column's box
was 180 by 40, in the UI font at 11.5px, with its own 6px bar flush to the left,
beside a model's 220 by 48 (0160). A lit edge turned the accent blue whatever
its role, so the colour the legend explains went out exactly when an edge was
being looked at. A role was an 8px square in the legend, the stop at menu and the
column card, while its edges are lines.

## Decision

- **The role is a tag in the box**, on the second line's baseline at the right,
  10px in: capitals in mono 9.5px at weight 600, 14px tall, radius 3, an outline
  of the role's colour at 55% and no fill of its own (the box shows through).
  Nothing sits above a box's right end any more. The steps and PARTIAL pills stay
  above its left, under their own class (`rolebadge`); the tag has `roletag`.
  Which role a box wears is unchanged (`nodeRoles`), and so are the aria-label,
  the hover card and the export, which draws the tag with inline colours.
- **A column's box is 250 by 48**, a model's but 30px wider, with the same bar,
  radius, ring and text start. Its name is mono at 12.5px and its second line
  11px mono, the server's `model  ·  type`. The name keeps the box's width; the
  second line stops 8px short of the tag, a clip counted from the tag's width
  (`boxRoom`), in the drawing, in the Halloween art's own budget and in the
  export's spelling. The class that marked a column box (`.col`) is gone: the rules
  that differ in column mode hang on a class of the canvas (`colcanvas`), so a
  model's box on a column canvas, drawn at the column's size, is drawn like the
  others there.
- **The tag's word reads on the box.** The role colours are marks, 3:1 on a card
  and 3:1 on Light's page, and several are under 4.5:1 as text. The outline and
  the edges keep the role's colour; the word takes a twin, `--r-<role>-text`,
  the colour taken toward the text colour until it holds 4.5:1 on the box, on its
  focus fill, on find's current match and on the dark fill Halloween puts behind
  a tag, in each theme.
  `transform` is a step lighter in Night, Graphite and Halloween (`#e05a5a` to
  `#e86b6b`); Light keeps its `#c03a3a`. `mixed` and a role the palette does not
  know are neutral all through, outline and word in the dimmed text colour, and
  the legend lists them in it too.
- **A column's edge wears its role's colour lit or not**, and a lit one is only
  thicker: `.edge.hi` is `stroke: var(--edge-col, var(--edge-hot))`, so a model
  canvas, which sets no colour, keeps its accent and its 2px. Edges are 1.5px at
  full opacity, and a lit one on a column canvas 3px: with the colour kept, the
  width is all that tells it, and in a fan of six out of one box 2px would barely
  show.
- **A role is a line, 14 by 2**, solid in the legend, the stop at menu and the
  column card; the roles that keep rows (`indirect` and its clauses) are the same
  line dashed, as their edges are (0097, 0148). The kind's bar in the model
  legend is unchanged.
- **Halloween**: the tag has a dark fill (`--bg-2`) and a whole outline over
  stone, wood and leather. The coffin moves it in and up, inside the end the
  clip cuts below 70% of the height (6px in and 7px up under the art, whose
  names it lifts 3px; 12px in and 2 up in the plain look past 400 boxes); the
  grimoire moves
  it 8px in, inside its gold frame. The tombstone's moss is as wide as the box,
  240px and 222 when ringed. No rule tells a column box by its id any more.

## Rejected

- **The tag on the edge**: a fan of six edges out of one column stacks six tags.
- **The role as the box's fill**: it hides the materialization the bar carries.
- **A coloured fill behind the tag**: the role ramp runs from a light slate to a
  dark grey, so no one word colour reads on all of it; the box's own fill does.
- **The board's 85% opacity on a column's edges**: on the page it takes `window`
  and `inferred` under 3:1 in Night, Graphite and Halloween (2.75 to 2.89), and
  in Light `passthrough`, `rename`, `cast`, `generated`, `filter`, `join_key`,
  `dedup_key` and `indirect` (2.63 to 2.90).
- **The role's own colour as the tag's word**: `window` and `inferred` are under
  3:1 on Night's box, and Light's `passthrough`, `rename`, `cast` and `generated`
  about 3.4:1.

## Consequences

- A column canvas is 30px wider per column of boxes, so a fit zooms a little less
  and a long column name is cut at 29 characters (a model's at 25).
- A box whose second line carries the longest role word, `PASSTHROUGH`, keeps
  about 20 characters of `model  ·  type`; the card and the label say it whole.
- The coffin's names are cut under the art to the room the tag leaves them, 17
  characters at most and about 12 to 14 beside a long role word, so that they
  clear the tag lifted into their rows; the second line of every skin stops 8px
  short of its tag, at 5px a character.
