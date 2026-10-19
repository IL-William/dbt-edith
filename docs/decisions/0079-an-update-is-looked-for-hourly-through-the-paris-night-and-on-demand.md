# 0079. An update is looked for hourly through the Paris night, and on demand

Date: 2026-10-14 · Status: accepted, amended by 0086, 0095 and 0100 · Amends 0068

**Trigger:** read before changing when dbt-edith looks for a newer release,
what Edith's menu shows, or what a look that got no answer says.

## Context

0068 kept an answer for 24 hours from the moment it came, so a start made
earlier in the day than the look before it asked nothing, and showed what was
out the day before: a release published overnight stayed unseen until the
hourly loop crossed the 24 hours, up to a day later. That is how a start on
the VM showed no update with several out.

A look that got no answer waited out the same 24 hours in its process, so a
start made before the network was up showed nothing until the next day. And
nothing said when the last look was made or how it went: no pill read the same
as up to date, as never asked, and as GitHub out of reach.

## Decision

- **An answer stands until the next turn: every hour on the hour from midnight
  to 09:00 in Paris, then midnight again.** It is kept in the config directory
  for every dbt-edith on the machine, as before. dbt-edith is released from
  Paris, overnight there, so a dbt-edith left running through those hours sees
  a release within the hour, and from 09:00 everything the night brought is
  out. A dbt-edith left running wakes at each turn and asks, up to two
  minutes after it, by a delay each process draws once: every dbt-edith on
  the machine wakes at the same turn, and the first to ask keeps its answer in
  the file, which the others read rather than ask again.
- **The first start of anyone's day asks**, wherever the machine is: no two
  turns are more than fifteen hours apart, so every night crosses one.
- **Paris time is worked out here**: the EU's rule, summer time from the last
  Sunday of March to the last Sunday of October at 01:00 UTC, in a few lines
  rather than a time-zone crate (0003). The machine's own zone is never read
  by the server, so the turns are the same instants on every machine. The
  night the clocks go back, the first 02:00 is the turn; the night they go
  forward, the skipped 02:00 is the moment summer time starts.
- **The button's look runs on a task of its own**, so a page closed or
  reloaded while git waits leaves it to end and keep its answer, and a look
  dropped on its way, whoever asked, says it is no longer on its way: a flag
  left set would refuse every later look and show `Looking…` for good.
- **A look that got no answer is tried again an hour later**, in its process,
  and is kept nowhere, as before.
- **Edith, at the top left, opens a menu**: the version and build stamp of the
  binary serving the page, a newer release with its notes and its command as
  the pill's menu had them (the pill now opens this same menu; *amended by
  [0100](0100-the-settings-open-in-a-panel-of-their-own.md): the pill opens
  the settings' Updates section, which draws it on the page*), when the last
  answer came and what it said, what git said when a later look got none, when
  the next look is due, in the machine's own time as the page reads it, and a
  button that asks GitHub at once, whatever is kept. Its tooltip names the
  version, and a dot on the name says a newer release is out.
- **The menu lists every release since the running one**, newest first, one
  row each: its version, and the title of the pull request that released it,
  on one line, linking to that release's notes. Twenty at most; the older
  ones are counted, and a last row links every release. On a window shorter
  than the menu, it scrolls, its button within reach. A pull request is a
  release (0045): it is squashed into one commit whose subject is its title,
  and the release workflow tags that commit, while one that changes nothing
  anyone would notice bumps nothing and carries no tag. So the tagged commits
  hold the list, and only them is fetched, through git as the look is: from a
  bare repository made for it in the temporary directory and removed after,
  the commits alone, no tree and no file, a few kilobytes. The titles are
  kept in the same file, by version, so no dbt-edith on the machine fetches
  one twice. Without all of them, the menu links the newest release's notes
  alone, as before, and says nothing else of it.
- **So a pull request's title is written for that line**: fifty characters at
  most, one idea, the one someone using it would notice, in their words, a
  verb first. AGENTS.md says it under Conventions, and CI's naming check
  holds titles to fifty characters, down from 72. A longer title merged before
  is cut with an ellipsis, whole in the row's tooltip.
- **git's reason is shown in that menu only**, its first line, with any user
  and password taken out of a URL in it: a `url.<base>.insteadOf` in the
  user's configuration can put them in the address git names. The status bar
  still says nothing of a failed look.
- **The page asks the server every ten minutes**, and every two seconds while
  a look is on its way, so an answer made at a turn shows soon after.

## Rejected

- **The CHANGELOG's sections in the menu.** A file to fetch, and every bullet
  of every release: one release takes a screen, past what a menu holds.
- **A link to GitHub's comparison of two tags.** It leaves the app for a list
  of every commit, the ones that changed nothing anyone would notice among
  them.
- **Midnight and 09:00 alone.** A dbt-edith left running through the night
  saw a release published after midnight only at 09:00.
- **Every hour of the day.** Fourteen more looks a day for a dbt-edith left
  running. What those hours bring is seen at the next midnight in Paris, which
  is 18:00 in New York, by the first start of the next day, or at once from
  the button.
- **The machine's own midnight.** The standard library knows no time zone, and
  reading the machine's takes a crate or code for each platform, for a rule
  the Paris turns already keep.
- **Asking at every start**, rejected by 0068 for the same reason: several
  starts a day would each reach GitHub for one answer. The button is there
  for whoever wants to know now.
- **A menu that opens on hover.** It closes when the pointer leaves the name
  for the button below it. The tooltip names the version on hover, and the
  click opens the menu, as Collin's does.

## Consequences

Ten looks a day for a machine, besides retries after a failure and the
button's; two dbt-edith that draw nearly the same delay can both ask at a
turn. A release published during the Paris day is seen at the next
midnight, or at once from the button. A machine whose clock is wrong turns at
the wrong moment, as anything it dates does.

On a proxy that takes the connection and never answers, a look lasts about
five minutes, `Looking…` all along: git is stopped at thirty seconds, but
git-remote-https, which it started, holds its output until curl gives up on
connecting, and is waited for. Not waiting for it would leave one behind per
look, fetch and push for as long as the proxy stalls, and git has no setting
for that wait: `http.lowSpeedTime` was tried, and does not cover it.
