# 0085. What is installed is what was audited

Date: 2026-10-15 · Status: accepted

**Trigger:** read before writing an install or build command into the docs or
the status bar, before adding or replacing a file under `web/vendor/`, and
before changing how Dependabot bumps the lockfile.

## Context

CI checks `Cargo.lock` against the RustSec advisories and `web/vendor/`
against OSV (0003). Both answer for one set of versions, and three things
built another. `cargo install --path .` ignores the lockfile unless `--locked`
is passed, so the first install on a new machine built the newest versions the
ranges allow, some twenty crates away from the lockfile that day, while the
status bar's update command had carried `--locked` since 0068. The vendored
audit compared version strings, so a file under `web/vendor/` edited by hand
passed as long as it claimed the right version. And Dependabot proposed a crate
the week it was published, which is the week a compromised release is still up.

## Decision

- **Every install command carries `--locked`**, and so do the `.exe` build and
  the `cargo test` of CI and `check.sh`: the README, `docs/windows.md`, the
  updates page and `scripts/build_windows.sh`. A lockfile a test would have
  rewritten fails there instead of at the next install, and what a user builds
  is what CI checked, crate for crate, or cargo refuses to build.
- **Each file under `web/vendor/` is pinned by its SHA-256** in
  `scripts/audit_vendored.py`, beside the path it has upstream; a file the
  script does not list fails the audit, the version it names has to be the one
  the notices and `codemirror.js` carry, and online the pins are compared with
  the npm tarball of that version, the only place a hash is taken from. The
  pins need no network, and `.gitattributes` keeps the bytes as they are on a
  new Windows checkout; a clone made before it checks `web/vendor/` out once
  more (`docs/windows.md`).
- **Dependabot waits seven days** before proposing a version, for crates and
  for actions alike. A security update is not held. A version yanked once it
  is in the lockfile is for `cargo audit --deny yanked` to refuse, since
  `--locked` builds it where an unlocked resolve would have moved on.

## Rejected

- **`cargo vet` now.** Measured on this lockfile with the public registries
  imported, about a fifth of the crates are covered at these versions; the
  rest would be exempted on day one, and every bump would fail until someone
  read the diff. The figures and the trap are in `docs/state.md`, where
  adopting it, with trusted publishers for the crates one author maintains,
  is deferred.
- **Attaching a signed binary to releases**, which would spare users a build
  and their antivirus a fresh unsigned executable: the signing certificate is
  a decision of its own, deferred in `docs/state.md`.
