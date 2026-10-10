# 0040. A model's tests hang grouped by their generic

Date: 2026-10-05 · Status: accepted · Amends 0034 and 0035

**Trigger:** read before changing how the tests under a model are grouped,
labelled or coloured, or what a payload says about a test.

## Context

Hanging under its model (0034), every test read the same: dbt's generated
name, about 50 characters at the median on the project measured, clipped at
25, in one grey. Its 15 000 tests were three quarters generics on a column,
a fifth generics on the whole model, a few hundred singular, a few hundred at
`severity: warn`. Columns carry one or two tests each, so a row per column
would have changed almost nothing; a row per generic shrinks the most tested
model from about 120 rows to under 20, and the models past five rows from
about 700 to under 200.

## Decision

The lineage payload sends a test's `test_name`, `column`, `namespace` and
`warn`. Under a host, the tests of one generic at one level, through one
package, share a row once there are two, `not_null  12 cols`, which opens into
a row per test, labelled by its column; a test alone reads `unique · id`; a
singular test keeps its name. Generics on the whole model go first, then
singular tests, then column generics: the model as a whole before its columns.
A host whose rows, opened, fit in five starts open; past five rows the chip
folds the rest. Opening is per group until reload. The focus is never folded
away. Each kind has a grey, lighter in that order, and a shape; a warn test
has an amber triangle. A closed group's row takes one edge from each other
parent of its tests.

## Rejected

- **A row per column**: as many rows as today, and nothing to hold a test on
  the model or a singular test.
- **New labels without grouping**: readable, but the same piles.
- **Grouping on the server**: as 0035, the payload would stop being the answer.
- **Colour alone**: the column grey sits close to an ephemeral model's.
- **A hue per kind**, the first version's indigo and lavender: they read as
  kinds of model. A grey keeps a row a test.
- **Column generics first**, as that version had: the most numerous, they
  pushed the checks on the whole model past the fold.
- **The package from `depends_on.macros`**: a list per node, read by nothing else.

## Consequences

A closed group draws none of its tests, and the status line counts only what
a chip hides; a model with five checks on itself shows its column tests behind
the chip. An export keeps the rows as drawn, its text the labels rather than
dbt's names, which move to the tooltips. A payload carrying a test must carry
`test_name` and `column`, or the test is painted as singular.
