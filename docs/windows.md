# dbt-edith on Windows

Two routes lead to the same `.exe`: build it elsewhere and copy it over, or build
it on the Windows machine. Either way, running it needs nothing installed.

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
to it.

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
`PATH`.

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
