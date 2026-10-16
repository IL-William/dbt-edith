# Updates

How dbt-edith learns a newer release is out, and what it offers to do about it.

It asks GitHub, through git, which releases there are, every hour from midnight
to 09:00 Paris time and then not until the next midnight, so the first start of
your day knows what came out overnight, wherever you are. When one is newer, a
pill after the version in the status bar says so and a dot marks Edith at the
top left. Resting the pointer on Edith opens a menu, as a click on either does:
the version and build you run, every release since yours by the title of the
pull request that released it, each linked to its notes, and, for a binary
installed or built from a clone on that machine, the pull of `main` and the
install typed into the terminal for you to run; restart dbt-edith once it
finishes. Otherwise it says why it has no command. The menu also says when
GitHub last answered, or what git said when it could not reach it, and when it
asks next, and Check for updates asks again at once. Nothing about your project
is sent, a failed look is tried again within the hour and stays out of the
status bar, and `--no-update-check` turns it off (0068, 0079).

The update is the menu's button, `Update to` the newest release, and right
under it is how it goes: it types the command into the terminal below, Enter
there runs it, a minute or two, and dbt-edith then has to be stopped where it
was started and started again, since the version that started serves the page
until then. The pull brings the release tags as well, so the build stamp names
the release it built. Once the new binary is in place, which dbt-edith tells
by the time its own file was written, the menu says `Installed: restart
dbt-edith to run it` in place of the button and the pill reads `restart for`
the release, within a few seconds of the update ending, and of an update run
by hand once the menu is opened (0095).

Opened by the pointer, the menu closes once the pointer has left Edith and the
menu, or at the next key, which goes on to the editor or the terminal you were
typing in: it takes no focus. A click on Edith or in the menu keeps it open
until a click elsewhere, a second click on Edith or Escape. While another menu
is open, the GitHub one or a breadcrumb's say, resting on Edith opens nothing
(0086).

## On Windows

To update by hand, stop dbt-edith first, because Windows will not replace an
`.exe` that is running, then `git pull` and `cargo install --path . --locked`
again. The command the menu types when a release is out needs no stop: it
renames the running `dbt-edith.exe` to `dbt-edith.exe.old` first, which Windows
allows, and the old one goes on serving the page from there; it puts it back if
the build fails. Restart dbt-edith afterwards. A binary run
from the clone's `target\release` gets no such command: cargo builds it through
a file the rename does not free, so stop it before building.
