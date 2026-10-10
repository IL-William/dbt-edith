# 0060. collin versions sit side by side, and the project picks the one that runs

Date: 2026-10-10 · Status: accepted · Amends 0042 and 0043

**Trigger:** read before changing which collin runs, where versions are
installed, or what makes collin's cache stale.

## Context

0042 runs the collin beside dbt-edith, else the first on the PATH, and 0043
updates it with the command that installs it, rejecting a tag pin so the check
could always find something newer. One collin at a time then: going back to an
older one to see whether a gap is new, or comparing what two versions read,
meant reinstalling over the only one there was.

## Decision

- **A version installs into its own folder**, `<config>/collin/<X.Y.Z>/`, under
  dbt-edith's config directory (0011), by `cargo install --tag vX.Y.Z --root`
  typed into the terminal for the user to run, as the plain install is. The
  folder's `bin/` holds that collin; one without it yet is not listed.
- **The default stays the one beside dbt-edith or on the PATH**, and "Update
  collin" or "Check for a newer collin" still reach it. A version installed by
  tag is not offered either: its tag has nothing newer.
- **The project picks the one that runs**, among those found, saved with its
  settings. A version from the browser is matched against the folders found,
  never joined onto a path (0015). One picked and since removed leaves the
  default running, and the menu says so.
- **A cache written by another collin is stale**, as one older than the
  manifest is. Its `producer` names the version, so switching versions runs the
  new one once, and a cache from a collin that names none is judged by its
  time alone.
- **A run waiting out another takes its outcome only from the same collin.**
  Asking for a version while another runs runs that version next.

## Rejected

- **dbt-edith listing collin's tags.** It calls nothing outbound (0008); the
  version is typed, and cargo says when a tag does not exist.
- **Downloading release binaries.** No HTTP client (0003), and a release has no
  Windows binary.
- **Several collins on the PATH under other names.** The PATH is the user's,
  and dbt-edith never writes outside its project and its own config directory.
- **One choice for every project.** Settings are per project (0011), and two
  projects can want two versions.

## Consequences

A version is a minute or two of cargo build the first time. On Fivetran's
Shopify package, the release v0.4.0 as default and v0.1.0 installed beside it:
picking 0.1.0 ran it, 4875 edges and 117 to investigate, 23 of them compiles
set aside for two CTEs of one name; picking the default back ran 0.4.0, 5548
edges and 107.
