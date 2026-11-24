# 0186. collin is downloaded on Windows, where cargo cannot build it

Date: 2026-11-24 · Status: accepted · Amends 0043 and 0060

**Trigger:** read before changing what the install and update entries type for
collin, what a collin release carries for Windows, or how dbt-edith's shell
commands check a download.

## Context

0042 and 0043 install and update collin with `cargo install`, typed into the
user's terminal. On the locked-down Windows machines this tool is built for the
Rust toolchain targets `x86_64-pc-windows-gnu` with no `gcc`, and one of
collin's dependencies, `psm`, compiles C: the command failed there for every
version, 0.11.0 included. 0060 rejected downloading a release binary because "a
release has no Windows binary". Since collin 0.12.2 every release carries
`collin-x86_64-pc-windows-msvc.exe`, named for its target and not its version,
and a `SHA256SUMS` with a line for it, so
`releases/latest/download/<name>` resolves. 0123 already types such a download
for dbt-edith's own `.exe`.

## Decision

- **On Windows the install and update entries type a download**, not cargo:
  the release's `.exe` and `SHA256SUMS` from `releases/latest/download`, the hash
  of the file compared with the line of `SHA256SUMS` ending in its name, and
  only then the file put in place. The page says so (`download` in `/api/meta`)
  and words the entries for it: "Download the latest collin" in place of "Check
  for a newer collin".
- **The same composer as 0123 writes it**, `update::compose_fetch`, for Git
  Bash and PowerShell 5.1, with what differs as parameters: where the hash is
  read from, the closing sentence, whether a file is there to step aside, and
  whether `<exe>.old` is kept. dbt-edith's own update is byte for byte what it
  was; collin's removes the `.old` once the new file is in, when it can, since collin is
  not running, and a failure anywhere leaves the file as it was.
- **The target is the collin found**, the default one after the refusals of
  0094 and 0128, else `collin.exe` beside dbt-edith when no file is there. A file
  there that was passed over is never written over: the cargo commands stay.
  Another shell than the two, or a path they cannot read in quotes, gets cargo too.
- **Command-line messages** that name the install or the update say, on
  Windows, to download that file: an install saves it as `collin.exe` beside
  `dbt-edith.exe` or on the `PATH`, an update over the collin the message names,
  since beside dbt-edith is searched first.
- Off Windows nothing changes but `"download":false`.

## Rejected

- **An HTTP client in dbt-edith.** 0003 stands, as in 0123: the user's own shell
  downloads, so what leaves the machine is theirs to run and read.
- **The `.zip` collin has published since 0.11.1.** Unpacking is a step more,
  and the two shells do it differently; the bare `.exe` is one file and one
  hash line.
- **Building with `windows-msvc` on the machine.** It needs Visual Studio's
  build tools, an install the machine does not allow.

## Consequences

The download follows `latest`, so it is only as good as the newest release's
assets: a release without the `.exe` fails at the first `curl` and prints no
closing line. A collin installed by tag (0060) is still typed as cargo on
Windows (docs/state.md). The command has run end to end only on Posix against a
folder, and by string for PowerShell: it has not yet run on Windows.
