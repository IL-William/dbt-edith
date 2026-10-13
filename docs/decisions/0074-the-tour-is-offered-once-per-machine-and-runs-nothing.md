# 0074. The tour is offered once per machine, and runs nothing

Date: 2026-10-13 · Status: accepted

**Trigger:** read before adding a step to the guided tour, making it open,
click or type anything, or changing when it is offered or what `tour.json`
holds.

## Context

dbt-edith does a lot that nobody finds by looking: the Custom selection box,
Export, the `dbt_project.yml` button at the end of a model's breadcrumb, the
Jobs tab before it has a source. Someone handed the binary on a locked-down
machine has no one beside them to show it. A tour has to point at each part
where it sits, on a project the tour knows nothing about, without doing
anything the user did not ask for.

## Decision

- **Each step rings one part of the page and says what it is for**, on a model
  the server picks: one of the project's own, with parents and children,
  then one a job runs once the Jobs tab has a source, then the most on its
  thinner side up to three, then the neighbourhood closest to eight boxes,
  then the first by name. So every step about a model has something to show,
  the Jobs tab included, no hub with hundreds of children fills the first
  canvas, and one manifest and one set of job files give the same model on
  every start (0055).
- **It runs nothing and writes no setting.** It opens the model's file as a
  preview, moves the lineage and the dock, draws one selection and opens the
  `dbt_project.yml` menu, all of which a click does too, and puts them back
  when it ends. It points at the Terminal tab and never opens it, since opening
  it starts a shell. It points at Export and Copy image and never presses them.
- **A step about a feature that is off says how to turn it on**: Add a source
  in the Jobs tab, the snowflake button for Snowflake's column lineage, what
  makes the Reports button appear. A step that needs a model is left out when
  the manifest has none.
- **It is offered once per machine, not per project**, by the first page to
  ask, and the asking records it: the tool is the same in every project, and an
  offer per project would come back with each. Not now leaves it in the
  shortcut list, where it always is.
- **`tour.json`, in the config directory beside `update.json`, holds when the
  offer was made and the newest version that served a page here**, nothing
  else. The version is for a tour of what is new since the last one seen,
  planned in `docs/state.md`: it has to be kept from the first release that
  has a tour, or the first update after it has nothing to start from.
- **The page under the tour takes no click and no key while it is up**, and
  the app's own shortcuts wait for it to end. The arrows, Enter and Escape
  walk it.

## Rejected

- **Once per project, in the project's settings.** It would come back for
  every project, about the same tool.
- **`localStorage`.** Lost per browser, as 0011 found for every other setting,
  and a fresh browser profile would offer it again.
- **Offering it with no config directory, every start.** An offer nothing can
  remember is nagging. There the tour stays in the shortcut list only.
- **Clicking through the features for the user**, pressing dbt ls, Export or
  Copy image to show what they do. Each writes something, a terminal line, a
  file or the clipboard, and the tour promises to change nothing.
- **Opening the terminal for its step.** It starts a shell, and with a venv
  types its activate line (0050).

## Consequences

A feature that changes the page may need its step changed: `web/tests/tour.js`
fails when a step points at an id the page no longer has, and
`scripts/walk_tour.py` walks every step in headless Chrome on Jaffle Shop,
checking each ring, what the page shows under it, that no setting is written
and that ending it puts the page back. A fresh config directory shows the offer
once, screenshots taken in one included.
