# Updates

How dbt-edith learns a newer release is out, and what it offers to do about it.

It asks GitHub, through git, which releases there are, every hour from midnight
to 09:00 Paris time and then not until the next midnight, so the first start of
your day knows what came out overnight, wherever you are. When one is newer, a
pill after the version in the status bar says so and a dot marks Edith at the
top left. Resting the pointer on Edith opens a menu, as a click does, and the
pill opens the same in the Updates section of the settings:
the version and build you run, every release since yours by the title of the
pull request that released it, each linked to its notes, and, for a binary
installed or built from a clone on that machine, the pull of `main` and the
install typed into the terminal for you to run, or for a release's own Windows
`.exe` the download of the next one (below); restart dbt-edith once it
finishes. Otherwise it says why it has no command. The menu also says when
GitHub last answered, or what git said when it could not reach it, and when it
asks next, and Check for updates asks again at once. Nothing about your project
is sent, a failed look is tried again within the hour and stays out of the
status bar, and `--no-update-check` turns it off (0068, 0079).

The update is the menu's button, `Update to` the newest release, and right
under it is how it goes: it types the command into the terminal below, Enter
there runs it, a minute or two, and dbt-edith then has to be stopped where it
was started and started again, since the version that started serves the page
until then. A card in the middle of the page says so once the command is typed
(Press Enter in the terminal to run the update), and goes by itself after
fifteen seconds, or at its button or at Escape pressed in the terminal or on the
page. The command ends, when the install worked and not otherwise, on a line of
its own saying the same restart in the terminal. The release tags are fetched
before the pull, so the build stamp names the release it built. For a binary
cargo installed, the command also empties the clone's `target/release` once the
install worked (`cargo clean --release`), which cargo otherwise keeps filling
with the leftovers of every build, a full copy of the dependencies for each one
that changed; so each update builds from scratch, a little longer, and a failed
clean changes neither the install nor the closing line (0120). The clone's
`target/debug` is left alone, and a binary built in the clone keeps its folder,
since it runs from there. Once the new binary is in place, which dbt-edith tells
by the time its own file was written, the menu says `Installed: restart
dbt-edith to run it` in place of the button and the pill reads `restart to
update`, within a few seconds of the update ending, and of an update run by hand
once the menu is opened (0095). A second card, `dbt-edith is installed`,
appears the moment the page learns it, whether or not the button was pressed
here, and stays until closed with its button or Escape in the terminal or on the
page; it is not shown for an install the page found already done when it loaded,
which the pill covers (0111). Neither, nor the menu's heading, names the new
version, since a release out after the install may not be the one installed: a
restart onto one still behind is offered the update again.

Opened by the pointer, the menu closes once the pointer has left Edith and the
menu, or at the next key, which goes on to the editor or the terminal you were
typing in: it takes no focus. A click on Edith or in the menu keeps it open
until a click elsewhere, a second click on Edith or Escape. While another menu
is open, the GitHub one or a breadcrumb's say, resting on Edith opens nothing
(0086).

## On Windows, a download or a build

Each release carries a Windows `dbt-edith.exe`, and `dbt-edith-arm64.exe` for
Windows on ARM, built by CI with the SHA-256 of each beside it
([windows.md](../windows.md)). On Windows the menu offers a choice of buttons,
the first the recommended one:

- **Download the release's .exe**, offered to every binary there but one run
  from anywhere under its clone's `target`, a developer's build, which it would
  replace. A build outside that folder is offered it too, and it overwrites it:
  one made with a custom `CARGO_TARGET_DIR`, or in a clone moved or renamed
  since, whose path the binary no longer finds. It types a download of the next
  release's file, of the architecture this one was built for, taken from the
  latest release, checked against its published SHA-256 and swapped in. In Git
  Bash `curl` is held to HTTPS on every redirect; PowerShell starts on HTTPS,
  but the command does not restrict where it is redirected. The `.sha256` comes
  from the same release and server, so it catches a truncated or corrupted
  download, not a tampered release. Nothing is replaced when the download fails
  or the hash differs; the terminal says so for a hash, and the command then
  ends with no closing line. A release published between the two downloads fails
  the check, and pressing the button again fetches the new one. It needs `curl`
  and `sha256sum` in Git Bash, which Git for Windows brings, or PowerShell 5.1.
  Git Bash's `curl` reads the proxy environment variables (`HTTPS_PROXY`), not
  Windows' proxy settings, so behind a proxy the PowerShell terminal or the
  manual download may work where Git Bash does not.
- **Update to 1.2.3 with cargo**, as below, for a binary cargo installed: it
  compiles on this machine.

A release's own `.exe` has the download alone. Both put the running `.exe` aside
as `<name>.old` and back if the swap fails (0123).

## On Windows, built here

To update by hand, stop dbt-edith first, because Windows will not replace an
`.exe` that is running, then `git pull`, `cargo install --path . --locked` and
`cargo clean --release` (0120). The command the menu types when a release is out
needs no stop: it renames the running `dbt-edith.exe` to `dbt-edith.exe.old`
first, which Windows allows, and the old one goes on serving the page from
there; it puts it back if the build fails. In Git Bash that command then ends as
a failure, with no closing line; PowerShell prints no closing line either, but
its status stays a success. Restart dbt-edith afterwards. A binary run from the
clone's `target\release` gets no such command: cargo builds it through a file
the rename does not free, so stop it before building.
