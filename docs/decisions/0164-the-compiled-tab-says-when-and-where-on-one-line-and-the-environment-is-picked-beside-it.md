# 0164. The Compiled tab says when and where on one line and the environment is picked beside it

Date: 2026-11-09 · Status: accepted · Amends 0147 and 0139

**Trigger:** read before changing the line at the top of the Compiled or Run
tab, its environment picker, the note under it, how a swapped relation is marked
in the SQL, or the description the dock's row shows while either tab is open.

## Context

The head of both tabs was a green or amber bar with a left rule: the file's date
in bold mono, the age beside it, the path on a second line, then the button, and
under it a row of environment buttons and a sentence (`Shown for PROD
(.env.prod): 3 relations swapped`) with a fold of what may differ, titled in the
environment's colour. The buttons were filled pills; the swapped relations were underlined in a blue no
environment owns. The design draws one line for it all.

This amends 0147 and 0139 in three things only: the buttons become a radio group
in the line, the `Shown for` line becomes a note led by the environment's tag,
and the marks in the SQL take the environment's colour.

## Decision

- **One line for when and where.** A pictogram (a check, or a triangle when
  something the file was built from moved: the state shapes of 0148, as the
  manifest pill draws them), `Compiled 2 min ago` (`Last run 2 min ago` on the
  Run tab), then the path, then the picker and the button. The age is said as
  the manifest's card says one (`cardAge`). When the file is stale,
  `changed since: ...` follows in the warning text colour. The line wraps on a
  narrow dock, and the path, as before, is cut at its left so that the
  identifying segment stays. A stale file keeps the amber bar the head had: the
  line takes a wash in the warning colour and a 2px rule at its left, drawn
  inside it so the line does not shift; a fresh one stays plain.
- **The date is a hover away,** on the age (`Compiled 08/11/2026 13:56:31`). The bar
  led with it because it is the number you can compare with something; the age
  leads now because it is what a glance asks, and the date stays one hover from
  it.
- **The picker is a radio group.** `Relations for`, then the files in the order
  the menu lists them and the manifest's own last, as `envChoices` gives them
  to the menu (0147). It is drawn only when there is a
  `.env` file to pick, as before. Each name is in its environment's colour
  (`--env`, from the tone the page already gives it), the picked one tinted and
  ringed in it, the manifest's in the page's greys. It is a lettered name, never
  a dot (0148). The picked button is the one tab stop; the arrows, Home and End
  move and pick, wrapping as a radio group does, and the focus is given back
  after the repaint. A pick is `selectEnv` as before, so the status bar, the
  Catalog and the Builds marks follow (0147).
- **One note, led by the tag.** The environment's tag, the count in bold, then
  one sentence of the rest: `PROD 3 relations swapped, read from .env.prod; 1
  kept as compiled; 2 not found in this SQL.` With nothing swapped the bold part
  is today's words. With no file picked the note says `As dbt compiled it.`, or
  on a sandbox manifest `As your own compile wrote it, into <schema>, nothing
  swapped.` (0143). It does not say which database the relations moved to: a
  swap can reach several, and the counts and the file are what the page knows
  for sure.
- **Where the SQL may differ stays a fold,** inside the note under its
  sentence, open when drawn: `May differ beyond its relations (2)`, one line per
  finding naming the macro and the line; the summary leads with the warning
  triangle. A finding in a macro still opens the macro at its line.
- **A swapped relation is marked in its environment's colour** (a wash and a
  dotted underline in that colour), and its hover is the call it stands for,
  as the board draws it, and the relation it replaced: `ref('stg_customers'),
  for PROD`, then `was "jaffle_shop"."main"."stg_customers" as compiled`. A
  source reads `source('jaffle_shop', 'raw_orders')` and a versioned model
  `ref('orders', v=2)`, built from the node's id and name. A relation kept as compiled
  stays amber. `--env-swap`, the blue, is gone from every theme.
  *Amended in 2.20.7: a source is split by its own name, which the server now
  sends, so a dot in either part reads right; the model's own relation reads
  `{{ this }}`, and its suffixed forms in a Run statement `{{ this }} with the
  __dbt_tmp suffix`, as the model's SQL writes it.*
- **The dock's row describes the open tab,** at the right, in one dim line cut
  with an ellipsis: the tab's own tooltip, read from `data-about` where
  `wireTabs` stores it, so the two cannot drift. It is hidden while the fold of
  the tabs measures, as the Jobs and terminal tools are.
- **The old head is gone:** `.fresh-bar`, the environment chips and the bar
  under them and the `Shown for` line. The fold of findings stays, in the note.

## Rejected

- **Keeping the date in the line.** The design leads with the age, and the line
  is shared with the picker; the date is a hover away.
- **A dot per environment.** An environment is its name in its colour (0148); a
  dot had also been a table's and `up to date`.
- **The manifest's own first, as the design draws it.** 0147 lists it last in
  the menu, and the picker keeps that order.
- **Naming the database in the note.** A swap can reach several.
- **One warning line for the findings, as the design draws it.** The fold says
  how many and lists each macro and line, which one line cannot.
- **A row drawn when there is no `.env` to pick, as the design draws it always.**
  0147: a project with no `.env` file sees what it saw before.

## Consequences

- The picture `catalog-compiled` and any other that shows the tab is
  retaken with `scripts/take_shots.py`, which now waits for the
  note's tag and presses `.artifact-redo`.
- A project with very many `.env` files makes the group wider than a narrow dock
  (docs/state.md).
  *Amended in 2.20.7: the group wraps inside its ring on a narrow dock,
  `Relations for` staying beside it.*
- A theme added later gives every `--env-*` colour a mark of 3:1 on its sidebar
  and, since a name sits on its own wash, on the picked radio's 16% and the
  note tag's 8% plus 14%, and the dotted underline of a swapped relation, in
  the full colour, on the editor's page and its 10% wash, in every theme;
  `web/tests/themes.js` checks them.
- Light's CI, STG, UAT and other colours are darker than before (`#2979ce`,
  `#06848b`, `#b46330`, `#89782b`) to pass that, and the change shows wherever
  Light draws them: the status bar's pill, the Catalog's tags, the Builds marks.
