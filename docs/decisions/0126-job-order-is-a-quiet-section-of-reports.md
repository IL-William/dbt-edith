# 0126. Job order is a quiet section of Reports

Date: 2026-10-28 · Status: accepted

**Trigger:** read before changing what the Job order section of Reports lists or
when it is listed, whether it lights the Reports button, or which sources it
reads.

## Context

`--job-order` (0125) answers which steps of the jobs should wait for which, and
an agent reads it in a terminal. Someone without one, or reading along with an
agent, had no way to see the answer, nor to open a model it names. Reports is
where the job coverage already lives (0064), with the same files read and the
same sources (0063).

## Decision

- **A section of Reports, `Job order`, beside Job coverage**, drawn from
  `GET /api/jobs/order?look=true`. The body is the command line's JSON object
  as it is, flattened, with `sources_refused` beside it as the other jobs routes
  carry it: one shape for the two, so a field the command line gains is a field
  the page can read. The route sits behind the same guard and key as the others
  (0015, 0052), and the job files are looked at as the section opens and never
  on a click in it (0063).
- **Quiet**, as Checks is (0116): listed in the rail, never lighting the button,
  never the section the panel opens on, no chip. It is a question asked, not
  news: a project with jobs would nearly always have some answer, and a pair
  that only the clock orders is often intended, so a count on the button would
  be a light that stays on. A chip for the pairs the files order the other way
  alone, which are rarely meant, is a later step.
- **No index entry of its own.** The index is polled, and an entry for the
  order would work it out on every poll. The section is listed when the `jobs`
  entry is, and is kept fresh against it: that entry is the manifest's time, the
  count of sources and the coverage's counts, worked out without looking at the
  job files, so the head turns into "Newer results: read again" when the manifest
  moved or what the jobs read elsewhere changed, as the coverage's does. An edit
  of a job file that leaves the counts alone, a new sensor or `depends_on`, moves
  nothing there: the files are looked at again as the section opens or on Read
  again.
- **Drawn as the answer is**: the counts, then a fold for each group that has a
  pair, in the order the command line lists them, a pair naming the step, the
  steps it should wait for and both schedules, and its edges. A model of an edge
  opens with the Jobs tab, which lists the steps that build it. The page draws
  ten edges a pair and a button for the rest, the command line five, and JSON
  all of them: the cap is a matter of what fits, so it sits in the page.
- **The sources in force**, as every route of the jobs reads them: the project's
  `.dbt-edith/jobs.yml`, else the settings' (0115). The command line reads the
  file alone (0115), so a source kept for you only is in the section and not in
  the command's answer. That is said in the page's feature description rather
  than hidden: the page is yours, the command line is shared.
- **No button to name a source** on the section. A save from the sources form
  lands on Job coverage (0100), and a form opened from here would not come back.
  When no source is named, or the file does not read, the section says what the
  Jobs tab says.

## Rejected

- **Lighting the button on pairs.** It is the first version of Checks again
  (0116): lit on nearly every project, saying nothing.
- **An index entry with the pair counts**, for a chip and for the freshness. It
  makes every poll of the index read the jobs and work the order out; the `jobs`
  entry already moves with the manifest and the coverage's counts, which is as
  much freshness as the coverage has, and a job-file edit that keeps its counts
  is caught by the look as the section opens.
- **Drawing the text of `--job-order`.** The JSON carries the notes as data
  (`idle_sensors`, `group_sensors`, `later`), so the page words them itself
  and the models are links.
- **Re-showing the section when the jobs may have changed**, as Builds by job
  is. The section calls the same `forgetCatalogJobs` after its own read, as the
  coverage does, so it would draw itself again for ever.

## Consequences

The route is `GET /api/jobs/order` in `src/api.rs`, the section is drawn by
`renderJobOrder` in `web/app.js` and tested by `web/tests/joborder.js`. Amends
nothing: 0125 keeps its meaning, and 0116's reasoning for a quiet section is
reused.
