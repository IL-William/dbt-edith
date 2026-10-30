# dbt-edith on Windows

Three routes lead to an `.exe`: download the one each release carries, build it
elsewhere and copy it over, or build it on the Windows machine. Whichever, running
it needs nothing installed.

## Download it

Each release on the [releases page](https://github.com/IL-William/dbt-edith/releases)
carries two files, built by CI on Windows, each with its SHA-256 beside it:

- `dbt-edith.exe`, for x64: nearly every Windows PC. It also runs on Windows on
  ARM, under emulation.
- `dbt-edith-arm64.exe`, native to Windows on ARM.

To tell which machine this is, open Settings, System, About, and read System
type: "x64-based processor" or "ARM-based processor". Or, in PowerShell:

```
Get-ComputerInfo | Select-Object OsArchitecture, CsSystemType
```

`CsSystemType` reads `x64-based PC` or `ARM64-based PC`; it takes a few seconds.
From Git Bash, the same is
`powershell -c "Get-ComputerInfo | Select-Object OsArchitecture, CsSystemType"`.

Download the file and its `.sha256`, and before running it check that the hash
is the published one:

```
certutil -hashfile dbt-edith.exe SHA256
```

It must match the `.sha256` file. That file is published beside the `.exe`, so
with [`gh`](https://cli.github.com) there are two checks that do not rely on it:

```
gh release verify-asset vX.Y.Z dbt-edith.exe -R IL-William/dbt-edith
gh attestation verify dbt-edith.exe -R IL-William/dbt-edith --signer-workflow IL-William/dbt-edith/.github/workflows/release.yml --source-ref refs/heads/main
```

The first says the release published this very file and that it cannot have
changed since; it works for releases from 1.57.0 on, the first immutable ones.
The second says this repository's release workflow built it, run from `main`;
it works for releases after 1.61.0.

The file is built with Microsoft's toolchain and carries no version resource.
Keep it in a folder of your own on your `PATH`, as the next section shows, and
run it. Update, in the menu under Edith, then
downloads the next release, checks its SHA-256 and replaces the file, with the
running one renamed to `dbt-edith.exe.old` or `dbt-edith-arm64.exe.old` until you
restart; it keeps fetching the file of the architecture it was built for.
Nothing is built, so Rust is not needed for dbt-edith.

To move from a build made on the machine, press Update in Edith's menu and pick
**Download the release's .exe**: it replaces that file where it is, so nothing
changes on your `PATH`. A build older than 1.56.0 offers cargo alone, so the
first move is either one more update with cargo, which brings 1.56.0, or the
manual download above, which is only for a first install: in that case run
`cargo uninstall dbt-edith` too, or `~/.cargo/bin` may come first on your `PATH`
and keep starting the old one. collin is still built with cargo when asked for,
so Rust stays useful there.

## Build it on a Mac or Linux, then copy it

Cross-compile (needs `mingw-w64`, available through Homebrew or your package
manager):

```
rustup target add x86_64-pc-windows-gnu
./scripts/build_windows.sh
```

The script builds `target/x86_64-pc-windows-gnu/release/dbt-edith.exe` and
prints its SHA-256, which it also writes beside it in `dbt-edith.exe.sha256`.
Copy the `.exe` to the machine and, before running it, check that it is the file
that was built:

```
certutil -hashfile dbt-edith.exe SHA256
```

The two hashes must match. A file that came through a share or a remote desktop
can be a different one, and nothing else would say so. Then run it: no
installer, no admin rights. It imports nothing but Windows system
libraries, so there is no runtime to place beside it. The terminal uses ConPTY,
which ships with Windows 10 and 11. Windows may warn about an unsigned
executable that arrived by copy, which is what an in-house build looks like
to it. Its properties, under Details, name dbt-edith as the product, the
version it was built at and Datadorelix in the copyright, and
`(Get-Item dbt-edith.exe).VersionInfo` in PowerShell lists every field, the
company among them. The mingw-w64 `windres` compiles them in, and the script
refuses an `.exe` built without them (0093).

To type `dbt-edith` from any project there, as you would elsewhere, keep the
`.exe` in a folder of your own and put that folder on your `PATH`. Neither step
needs admin rights. In Git Bash:

```
mkdir -p ~/bin && mv /c/Users/you/Downloads/dbt-edith.exe ~/bin/
echo 'export PATH="$HOME/bin:$PATH"' >> ~/.bashrc
```

In PowerShell, touching your own `PATH` and not the machine's:

```
[Environment]::SetEnvironmentVariable('Path',
  [Environment]::GetEnvironmentVariable('Path', 'User') + ';C:\Users\you\bin', 'User')
```

Updating is replacing that one file, and nothing else.

## Build it on the Windows machine

Rust alone is not enough there: linking needs a C toolchain, and Windows does not
ship one. There are two ways to get it.

**With Visual Studio Build Tools** and their C++ workload, which is what rustup's
default toolchain expects. `cargo install --path . --locked` then works as on
any other machine. Installing Build Tools takes administrator rights.

**Without administrator rights**, use Rust's GNU toolchain and a mingw-w64 of
your own. The GNU toolchain is not enough by itself: `windows-sys`, which tokio
and clap depend on, links Windows functions as `raw-dylib`, so rustc calls
`dlltool`, which calls the assembler `as`, and Rust does not ship `as`
([rust-lang/rust#140704](https://github.com/rust-lang/rust/issues/140704)).
WinLibs, from winget, installs for your user only and brings both. In Git Bash:

```
rustup toolchain install stable-x86_64-pc-windows-gnu
rustup default stable-x86_64-pc-windows-gnu
winget install --id BrechtSanders.WinLibs.POSIX.UCRT -e
```

winget unpacks it under `%LOCALAPPDATA%\Microsoft\WinGet\Packages`, in a folder
named after the package. Put that folder's `mingw64\bin` on your `PATH`, open a
new terminal, and check that `as --version` answers:

```
echo 'export PATH="/c/Users/you/AppData/Local/Microsoft/WinGet/Packages/<WinLibs folder>/mingw64/bin:$PATH"' >> ~/.bashrc
```

Then `cargo install --path . --locked` from the clone builds it and drops
`dbt-edith.exe` in `%USERPROFILE%\.cargo\bin`, which rustup already put on your
`PATH`. WinLibs also brings the `windres` that names the publisher and the
version in the `.exe`'s properties. Where none is on `PATH`, and always with
Visual Studio Build Tools, the build goes on without them and cargo says so
(0093):

```
warning: dbt-edith@<version>: dbt-edith.exe is built without its version resource: ...
```

A clone made before `.gitattributes` kept `web/vendor/` out of line-ending
conversion still holds those files with CRLF when `core.autocrlf` is on, which
is Git for Windows's default: the vendored audit then fails on every file, and
the binary embeds other bytes than the ones audited. Once, in that clone:

```
rm -r web/vendor && git checkout -- web/vendor
```

The two errors this avoids, word for word, so that searching for them lands here:

- `error calling dlltool 'dlltool.exe': program not found`: `dlltool` itself
  was not found.
- `dlltool.exe: CreateProcess`: `dlltool` ran but could not start `as`.
