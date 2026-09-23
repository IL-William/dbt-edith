# 0028. A macro call resolves by name, the way dbt resolves it

Date: 2026-09-23 · Status: accepted

**Trigger:** read before linking anything to a macro, or changing which macro a
call in the editor reaches.

## Context

The editor links `ref()` and `source()`; macros are the other names a model's
Jinja uses. The manifest lists every macro dbt could call: some 1 700 on the
18 825 node project in September 2026, under 200 of them the project's own. Two
facts make "which file" harder than a name lookup. Names repeat: about fifty
are defined in more than one package, some because the project wraps a package
macro under the same name. And a package macro's path is relative to the
package, not to the project.

## Decision

The browser scans the buffer for `name(` and `pkg.name(` inside Jinja blocks
and posts the names with the file's path. `src/macros.rs` resolves each the way
dbt's `MacroNamespace` does: `pkg.name` is that package's, a bare name is the
file's package's, then the root project's, named by `dbt_project.yml` when the
manifest predates dbt-core 1.6. Installed packages are never searched for a
bare name, since dbt does not search them either. A link comes
back only when the file exists in the project: a root path as written, a
package path under `dbt_packages/<package>/`, then `dbt_modules/`, through
`files::resolve`. The browser finds the `{% macro %}` line in the file it opens.

## Rejected

- **Scanning `macros/` for `{% macro %}` blocks.** It finds a file per name, not
  which of two a call reaches, and it misses every package.
- **`depends_on.macros`.** It says what a node or a macro reached: the 300
  models calling one project macro all list it. It is one list per node, not
  one answer per word, so it cannot say which call is which when the project
  and a package share a name, and an unsaved edit is in no manifest yet.
- **Every macro sent to the browser.** The order would be rewritten in
  JavaScript, away from `cargo test`, over paths the disk may no longer have.
- **Linking dbt's own macros.** They live in site-packages, outside the project
  (0017).
- **Reading `packages-install-path`.** One more key for the 0018 scanner, for a
  setting rarely moved; the default and the pre-1.0 name cover the rest.

## Consequences

A call is linked only when dbt would reach it from this file and its file is on
disk. Anything else stays text, with no missing mark as a `ref()` gets, because
to a scanner `log()`, a Jinja builtin and a typo look alike. Inside a package
macro a bare call resolves as if that package were the caller, which dbt only
decides at run time. Not followed: `adapter.dispatch` targets, generic tests
named in YAML (`- dbt_utils.accepted_range:`), and materializations.
