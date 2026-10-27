# 0120. The update empties the clone's target once installed

Date: 2026-10-27 · Status: accepted · Amends 0095 and 0111

**Trigger:** read before changing what the update types after the install, or
where a cargo-installed binary's build leaves its files.

## Context

The update 0095 types for a binary cargo installed is `git pull`, then `cargo
install --locked --path "<clone>"`. With `--path`, cargo builds in the clone's
own `target/` (`cargo help install`: by default it uses the target directory in
the workspace of the local crate) and never removes what an earlier build left
there. Every dependency bump and every new rustc leaves a full copy of the
dependencies beside the old ones, so the folder only grows, until it fills a
small disk.

Measured on a 10-core Mac: a release build from an empty `target/` takes 79 s
and leaves 257 MB; a rebuild reusing it, recompiling only dbt-edith (one
codegen unit, thin LTO), takes 69 s. Each copy of dbt-edith in
`target/release/deps` is only 5 to 6 MB: the growth is the stale copies of the
dependencies.

## Decision

- **Once the install worked, the command empties the clone's
  `target/release`**: `cargo clean --release --manifest-path
  "<clone>/Cargo.toml" --target-dir "<clone>/target"`, built with the same
  quoting as the other paths, so one that cannot be quoted still gives no
  command at all. `--release` because an install builds nothing else, and the
  clone's `target/debug` and test binaries belong to whoever develops in it
  (checked on cargo 1.98.1: exit 0, `debug/` left alone, the same
  `CACHEDIR.TAG` guard).
- **Only for an install.** A binary built in the clone runs from
  `target/release`, which the clean would delete, so its command is unchanged.
- **The folder is named, not looked up.** A `CARGO_TARGET_DIR` or
  `build.target-dir` the user set may be shared with other projects, so only
  the clone's own folder is emptied. When cargo built elsewhere, cleaning a
  folder that is not there prints that it removed nothing and exits 0 (checked
  on cargo 1.98.1), and recent cargo also refuses to clean a folder without its
  `CACHEDIR.TAG`.
- **It runs after the install and outside the group that puts the binary
  back.** A failed clean changes nothing: the install worked, the binary stays,
  and the closing line (0111) is still printed, since the menu learns the
  install from the binary's time anyway. In the Posix shells the clean and the
  line sit in one brace group, `{ clean; printf ...; }`, whose `;` lets the line
  follow whatever the clean did; in PowerShell they sit in the success branch of
  `$?`, never the `else` that restores, and a failed native command does not
  stop the line there.

## Rejected

- **Emptying the whole `target/`.** The install only builds into
  `target/release`; the rest would wipe the debug build and test binaries of a
  clone someone also develops in, and on Windows an editor's rust-analyzer can
  hold files under `target/debug`, so the clean would fail partway.
- **Keeping `target/` and only documenting `cargo clean`.** It relies on being
  remembered, and usually once the disk is already full.
- **`cargo clean -p dbt-edith`.** Its own copies are the small part.
- **Removing only the stale files, by age.** cargo-sweep does it, but it is a
  third-party tool, and the mtime of an artifact cargo reuses does not move, so
  age does not tell a stale file from a used one.
- **A temporary `--target-dir` for the install.** cargo does not delete a
  folder it is given, so the pile only moves to the temporary folder.
- **Cleaning whatever target directory cargo actually uses.** It may be shared
  with other projects.
- **Letting a failed clean stop the closing line.** The binary is installed;
  saying nothing would leave the person waiting on a build that ended.

## Consequences

Every update compiles all the dependencies again: about ten seconds more on the
Mac, and more on a machine with fewer cores. The command is typed by the running
binary, so the first update to this release is typed without the clean, and the
folder already there is emptied by the update after it, or by hand with `cargo
clean --release` in the clone. cargo writes `CACHEDIR.TAG` only when it creates
the folder, so a `target/` made some other way is refused by every clean (exit
101) and keeps growing; that is harmless to the update, since the closing line
still prints.

A `build.target` or `CARGO_BUILD_TARGET` the user set makes the install build
into `target/<triple>/release`, which `cargo clean --release` leaves even with
the same setting in force (checked on cargo 1.98.1): for them the folder keeps
growing as before, and "an install builds nothing else" holds only for the
host's own folder.

The menu, the pill and the installed card learn the install from the binary's
time, which changes before the clean runs, so they can say installed while it
is still running. Stopping dbt-edith then ends the clean midway with the
terminal it runs in, which is harmless: what it left is emptied by the next
update.
