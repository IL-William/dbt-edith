# 0123. The Windows .exe is built by CI and the update downloads it

Date: 2026-10-27 · Status: accepted · Amends 0068, 0093, 0095 and 0111

**Trigger:** read before changing what a release carries, how the Windows
`.exe` is built, or what the update types for a binary that came from a release.

## Context

Endpoint protection on Windows flagged an `.exe` that cargo had built on the
machine itself, at low confidence, by an on-device engine. dbt-edith was built
there with the GNU toolchain, and the update typed the same build again (0068),
so every machine held a one-off binary no scanner had seen.

The 1.51.0 source was built three ways and sent to VirusTotal:

| Build | Engines that flagged it |
| --- | --- |
| windows-gnu, with VERSIONINFO (0093) | 1 of 72 (Microsoft, `Wacatac.C!ml`) |
| windows-gnu, without | 1 of 73 (McAfee, `ti!`, a rare-file reputation flag) |
| MSVC | 0 of 73 |

The flagging vendor's own engine there flagged none of the three, so VirusTotal
cannot reproduce what an on-device sensor does: only a machine running one can
tell.

## Decision

- **Each release carries two `.exe` files built by CI** on a native Windows
  runner with the MSVC toolchain: `dbt-edith.exe` for x64, which runs on
  Windows on ARM too, under emulation, and `dbt-edith-arm64.exe` for Windows on
  ARM. Each has a `.sha256` in the format `sha256sum -c` reads, with the bare
  file name. A `windows` job builds them, `release` needs it and stays the only
  job that writes, and `gh release create` with files makes a draft, uploads and
  publishes, so the tag and the files appear together. Each is built and run on
  a native runner, x64 on `windows-latest` and ARM64 on `windows-11-arm`, where
  the real Git Bash and PowerShell run the download command against a local
  server before the build. A pull request that touches the workflow,
  `Cargo.toml`, `Cargo.lock`, `build.rs` or `src/` builds both without
  publishing. Once the release is out, the job fetches each `.sha256` from the
  URL the update downloads from and compares it with the published file, so a
  wrong name or a redirect off HTTPS shows then and not at the next release; the
  release is immutable by then, so a failure there is mended by the next one.
- **Those builds are told they are the release's** (`DBT_EDITH_RELEASE_ASSET`,
  read with `option_env!`), and their update downloads: `{REPO}/releases/latest/download/<asset>`
  and its `.sha256`, the asset named by the architecture the binary was built
  for, so an ARM64 build never swaps itself for the emulated one. `latest` names
  no version, so the command is composed once at start. A build without the
  variable is unchanged.
- **On Windows the menu offers a choice, the download first.** The server sends
  a list of commands, the recommended first: a release's own `.exe` has the
  download alone; one `cargo install` put in `bin/` has the download and then
  the install, as today; one anywhere else, a GNU build copied over by hand, has
  the download; one run from anywhere under the clone's `target/` is a
  developer's build and has none (0068). Off Windows it is the one command as
  before. A refused command (a shell it is not written for, a path it cannot
  quote) is left out, and a list left empty says the first refusal. The download
  comes first because a machine that built its own is the one the measurement
  was taken for, and the button is how it moves to the file CI built; cargo
  stays beside it for whoever wants to compile.
- **The command downloads beside the running file, checks the hash, and only
  then swaps**: `<exe>.new` and `<exe>.sha256`, the running file renamed to
  `<exe>.old`, the new one renamed into its name, the old one put back if that
  fails. Any failure replaces nothing, removes what came down and prints no
  `DONE` (0111). Written in both languages like `compose`; `curl` and
  `sha256sum` come with Git Bash, and PowerShell 5.1 uses `Invoke-WebRequest`
  and `Get-FileHash`, in a script block so a profile's `Stop` cannot throw past
  the put-back and no preference outlives the command. A release published
  between the two downloads fails the check.
- **HTTPS guards the file, the hash guards the download.** In Git Bash `curl`
  is held to HTTPS on every redirect (`--proto '=https'` and `--proto-redir
  '=https'`); PowerShell's `Invoke-WebRequest` starts on HTTPS, but the command
  does not restrict where it is redirected. The `.sha256` comes from the
  same release and server as the file, so it catches a truncated or corrupted
  download, not a tampered release.
- **Only Windows gets it**: an asset is built there alone.

## Rejected

- **cargo-xwin on Linux.** A third-party tool and a Windows SDK downloaded for
  every build.
- **The GNU toolchain in CI.** It is the build the measurement flagged.
- **A Rust HTTP client** (0003). The typed command keeps one mechanism and
  shows what runs.
- **Signing now.** An endpoint's certificate exclusion pins one certificate,
  Azure Artifact Signing certificates last three days, and an OV certificate
  costs money. Left in `docs/state.md`.
- **Always downloading on Windows.** It would take a branch build from a
  developer who installed it with cargo, and leave the machine that can
  compile only the file CI made.
- **Cargo only.** It is the update that left every Windows machine holding a
  one-off binary.
- **A versioned URL.** The command would change with each release and need the
  version at start.

## Consequences

*Amends 0068* (no release carries a binary), *0093* (the asset is MSVC and has
no version resource), *0095 and 0111* (the update has a third kind).

A machine that built its own moves to the release by pressing Update and
picking the download, which replaces that file where it is; the manual download
is for a first install. `collin.exe` is not carried. A proxy that blocks `.exe`
downloads from GitHub stops the update at the first download, with curl's or
PowerShell's error.

A limit: a build outside `<clone>/target/` is offered the download, which
overwrites it. A custom `CARGO_TARGET_DIR` is one, and so is a clone moved or
renamed after the build, since the clone's path is baked in at build time and
no longer matches.
