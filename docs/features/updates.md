# Updates

How dbt-edith learns a newer release is out, and what it offers to do about it.

It asks GitHub, through git, which releases there are, every hour from midnight
to 09:00 Paris time and then not until the next midnight, so the first start of
your day knows what came out overnight, wherever you are. When one is newer, a
pill after the version in the status bar says so and a dot marks Edith at the
top left. Clicking either opens the same menu: the version and build you run,
every release since yours by the title of the pull request that released it,
each linked to its notes, and, for a binary installed or built from a clone on
that machine, the pull of `main` and the install typed into the terminal for
you to run; restart dbt-edith once it finishes. Otherwise it says why it has no
command. The menu also says when GitHub last answered, or what git said when it
could not reach it, and when it asks next, and Check for updates asks again at
once. Nothing about your project is sent, a failed look is tried again within
the hour and stays out of the status bar, and `--no-update-check` turns it off
(0068, 0079).

## On Windows

To update, stop dbt-edith first, because Windows will not replace an `.exe` that
is running, then `git pull` and `cargo install --path .` again. The command the
status bar offers when a release is out needs no stop: it renames the running
`dbt-edith.exe` to `dbt-edith.exe.old` first, which Windows allows, and puts it
back if the build fails. Restart dbt-edith afterwards. A binary run from the
clone's `target\release` gets no such command: cargo builds it through a file
the rename does not free, so stop it before building.
