# 0173. The linters are fetched by hash, and their rules aim at bugs

Date: 2026-11-11 · Status: accepted

**Trigger:** read before adding or changing a linter rule, before bumping a
linter, or before adding a file type or a folder the linters should cover.

## Context

Nothing checked the Python, the shell or the frontend beyond tests and a syntax
parse. With the rules below, `main` gave Ruff some seventy findings, ShellCheck
two and Biome about fifteen, none of them a failing build. The linters have to
run where the rest of `check.sh` does, a Mac with Rust and nothing else, and in
CI at the same versions: a Ruff newer on one side is a check that disagrees
with itself.

## Decision

- **`scripts/lint.py` fetches Ruff, ShellCheck and Biome** from their GitHub
  releases for macOS (arm64, x64) and Linux (x64), each asset pinned by the
  SHA-256 GitHub records for it, as 0085 pins a vendored file. The download is
  hashed before it is opened or made executable; a mismatch deletes it and
  exits 1. Only the expected member of an archive is read, and written under
  `target/lint-tools/<tool>-<version>-<hash>/`.
- **A version is at least seven days old and bumped by hand**, with its URLs,
  hashes and binary path. Dependabot cannot see these pins.
- **Exit 0 clean, 1 on a finding or a refused download, 2 when a linter cannot
  run** (offline with nothing cached, no pinned build for the platform).
  `check.sh` skips `lint` on 2 and names it at the end; CI fails on it, and
  skips the Monday run, which is for advisories.
- **The rules aim at bugs, not style.** Ruff: `E, F, W, B, UP, SIM, RUF, PLE,
  PLW`, minus `E501`, `RUF005`, `SIM105`, `PLW1510` and `E731`. Biome:
  `recommended` with `style` and `complexity` off, and `noUnusedFunctionParameters`,
  `useIterableCallbackReturn`, `noAssignInExpressions` and
  `noControlCharactersInRegex` (the terminal's escapes) off, on `web/app.js` and
  `web/lineage.js` only: `web/tests/` evals slices of `app.js`, so every name
  there is undeclared, and `web/vendor/` is not ours. Each switch, `noqa` and
  `biome-ignore` carries its reason beside it.
- **Nothing is formatted here.** Only Rust is (0172): a formatter would rewrite
  the hand-aligned Python, JavaScript and CSS, and linters never touch layout.

## Rejected

- **Whatever is installed.** Versions drift between machines and CI, most
  machines lack Biome and ShellCheck, and `check.sh` would skip them nearly
  everywhere.
- **npm or Node tooling**: a runtime and a `node_modules` in a repository with
  no build step (0004).
- **oxlint** beside or instead of Biome: a second tool for the same job, with
  nothing measured here to say it finds more.
