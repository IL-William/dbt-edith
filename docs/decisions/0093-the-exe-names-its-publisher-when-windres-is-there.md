# 0093. The .exe names its publisher when windres is there

Date: 2026-10-16 · Status: accepted, amended by 0123

**Trigger:** read before changing what the `.exe`'s properties say, how
`build.rs` finds `windres`, or what a Windows build does without one.

## Context

The `.exe` named no company, product or version in its properties, which
antivirus heuristics hold against a fresh unsigned file, on a managed VM whose
antivirus nobody here picks. Signing stays deferred in `docs/state.md`.

## Decision

- **`build.rs` writes a VERSIONINFO script and runs `windres` on it**, for a
  Windows target, read from `CARGO_CFG_TARGET_OS` since it runs on the host:
  Datadorelix, dbt-edith, the version `Cargo.toml` names, the copyright line of
  `LICENSE` with its licence (0046), `dbt-edith.exe`.
- **`windres` is `<arch>-w64-mingw32-windres`, then `windres`**, as Homebrew,
  Linux packages and WinLibs name it: the binutils that bring the `as` and
  `dlltool` the GNU toolchain already needs bring it too.
- **Without it, or when it fails, the build warns and goes on**, and watches
  `PATH` for the next one. The resource changes what Windows says about the
  file, not what it does, and the update the status bar types (0068) must not
  fail on a machine that built yesterday. `scripts/build_windows.sh`, which
  makes the `.exe` taken to the VM, refuses one without it.
- **MSVC gets none, with the warning.** Its `rc.exe` is off `PATH` outside a
  developer prompt, found only through the registry.
  *Amended by [0123](0123-the-windows-exe-is-built-by-ci-and-the-update-downloads-it.md):
  the `.exe` a release carries is MSVC, built by CI, and has none; it scanned
  cleaner than the GNU one with the resource.*

## Rejected

- **embed-resource**: fourteen crates new to the lockfile, `cc`, the `toml`
  stack, `vswhom` and `winreg` among them, to find an `rc.exe` nobody here uses.
- **winresource**: six, the `toml` stack, to read fields cargo already hands a
  build script. Either is built on every Mac and Linux too, a build
  dependency's `cfg` being the host's (0003).
- **The object written by `build.rs`**, needing no tool: a resource tree and a
  COFF file by hand, to spare a tool the route already brings.
- **Failing without `windres`**: a broken build for a property.

## Consequences

The year in the copyright is written in `build.rs`. An antivirus may still flag
an unsigned binary: this removes one reason, not all of them.
