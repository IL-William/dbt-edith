# 0128. A collin others can replace is not run

Date: 2026-10-30 · Status: accepted · Amends 0060 · Builds on 0094

**Trigger:** read before running a collin from a folder under the config
directory, changing what makes an install refused, or reading who owns a file.

## Context

0060 installs each collin version into `<config>/collin/<X.Y.Z>/` and runs the
one the project picks. 0094 refuses a collin a repository brought, which is the
other side of the same question: whose binary is it. The folder under the config
directory is the user's own by default, but on a host where it is shared with
another account (a group-writable home, a mounted share, a `sudo cargo install`
that left root's files there) that account can replace the binary, and the next
column click or Regenerate runs it as the user. Nothing looked at who could
write there.

## Decision

- **On Unix, an install is run only if the binary and every folder from it up to
  `<config>/collin/` inclusive belong to the account running dbt-edith and have
  no write bit for group or others** (owner equals the effective uid, mode
  `& 0o022` is zero). The check is `collin_run::installs_checked`, which splits
  the installs into those safe and those refused, each with a sentence.
- **The account is a parameter.** `effective_uid()` calls `geteuid`, declared
  in an `extern "C"` block since std links libc already, so no crate (0003); the
  checks take the uid as an argument, as `program::on_path_in` takes its PATH, so
  a test can be another account.
- **A refused install is listed apart and never picked.** It is not among the
  versions the menu offers, picking it is a 409 with the sentence, and the project's
  saved pick of it falls back to the default collin, the menu saying why and not
  that it was removed. The sentence names the path, who can write it or who owns
  it, and one command that mends every part: `chmod -R go-w "<root>/"`, or
  `sudo chown -R "$(id -un)" "<root>/" && chmod -R go-w "<root>/"` for files
  another account owns (the trailing slash follows a linked root). It travels as `Refusal::Shared`,
  in the menu's `refused` field with `refused_shared` beside it, so the menu and
  the Columns tab say it where they say a tracked collin is passed over. The
  command line, `--column --run-collin` and MCP never run a version installed
  side by side (0099), so they have nothing to refuse; their passed-over line
  would print the sentence all the same.
- **A link below `<config>/collin/` is refused**, each part read with
  `symlink_metadata`: a link's own mode says nothing of the file it leads to,
  and whoever owns the target can replace it. `<config>/collin/` itself may be a
  link, as dotfiles leave it; what it leads to is checked.
- **What lies above `<config>/collin/` is not looked at.** The config directory
  is the user's by 0011; a shared one is the host's to fix, and checking up to
  `/` would refuse every home a group may enter.
- **The install command mends its own modes.** On Unix the command the menu
  types ends with `; chmod -R go-w "<config>/collin/"` (the slash follows a
  linked folder; `;` and not `&&`, which nushell rejects, so it also runs after
  a failed install, harmlessly): a desktop umask of 002 (Debian, Ubuntu, Fedora) leaves what cargo writes group-writable, and every
  install typed from the menu would otherwise be refused. The PowerShell form
  is unchanged. The plain `INSTALL` and `UPDATE` write to cargo's own bin
  folder, not to this root.
- **Windows is unchanged.** It reports ownership by SID and ACL, which `std` does
  not read (state.md, Deferred 31).
- **The default collin, beside dbt-edith or on the PATH, is not checked**: it is
  where the user put it (0042), and 0094 already holds it to git. Except that
  one which is a refused install's binary, found because the user put its `bin/`
  on the PATH as cargo's warning says, is dropped (`without_refused`, by
  canonical path), or the refusal would be undone by the same file found
  another way.

## Rejected

- **Hashing the binary.** Nothing is known to compare it with: each build has its
  own hash, as 0094 says, and the check would trust whatever was there the first
  time.
- **Refusing only the binary, not its folders.** Whoever can write the folder
  renames another file onto the binary, so the file's own mode proves nothing.
- **Following a link and checking its target's chain.** It would reach outside
  the folder this decision bounds, to owners nobody listed, for an install cargo
  never makes as a link.
- **Accepting a group-writable part whose group is the owner's private group**,
  as OpenSSH does: the umask-002 case would then pass untouched. It needs the
  group database (`getgrgid`, `getgroups`) through more FFI; it may come later.
- **Accepting root's files.** `sudo cargo install` is the usual way an install
  comes to be root's, and root's folder inside the user's home is exactly
  what the user did not mean; the sentence says `chown`.

## Consequences

"Writable by nobody else" is the mode bits' claim. On macOS an extended ACL
(`chmod +a`) grants write without changing them, and is not seen; Linux ACLs
show in the group bits through the mask, so are. When `<config>/collin/` is a
link, the folders above its target are outside the config directory, so 0011's
"the config directory is the user's own" does not cover them, and they are not
looked at either.

The check reads the modes when the menu is opened and when a run starts, then the
process is started a moment later: in that window only the account itself, root,
or whoever can write `<config>` (not checked) could swap the file in. A group that works together on one config directory
sets `chmod -R go-w` and loses nothing, since only the owner runs collin. An install
made by hand under an umask that leaves the group write bit, or by an earlier
dbt-edith's command, is refused until `chmod`.
