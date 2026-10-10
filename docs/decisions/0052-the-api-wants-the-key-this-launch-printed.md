# 0052. The API wants the key this launch printed

Date: 2026-10-10 · Status: accepted · Amends 0015

**Trigger:** read before adding a route, changing what the printed link holds,
or letting anything reach the API without the key.

## Context

0015 checks that a request says it comes from this server's own page. A
browser cannot lie about `Host` and `Origin`, but every other program can, under
any account on the machine: one `curl` with the right two headers got a shell,
read the project, `.env` included, and the profile. On a laptop that program
already runs as the user. On a shared Windows host, a VM with several sessions
or a terminal server, it is someone else. 0015 rejected a token for breaking
the printed link and every bookmark, not for being unneeded.

## Decision

- **Each launch draws a 256-bit key** and prints it in the link it opens:
  `http://127.0.0.1:4321/?key=…`. No crate: `RandomState`, seeded from the
  operating system's random source, keys SipHash over a counter, in a thread of
  its own so that seed serves nothing else (0003).
- **Opening the link swaps the key for a cookie**, `HttpOnly`, `SameSite=Strict`,
  named after the port since cookies ignore ports, and redirects to `/`, so the
  key leaves the address bar. Bookmarks and new tabs then work until the
  browser closes. A wrong or stale key sets nothing.
- **Every `/api/` and `/ws/` request needs it**, reads included: a read is how
  the `.env` and the profile leave. The cookie, or `Authorization: Bearer` for a
  tool. Without it, `401` and a sentence the page shows in a bar that stays.
  The page's own files stay open, so a tab without the key can say why.
- **It adds to 0015, it replaces nothing.** Host and Origin are still checked
  first, and `Sec-Fetch-Site` now backs `Origin` on anything that is not a read.

## Rejected

- **Keys in a file**, as Jupyter writes one. A stable key would keep a tab
  alive across restarts, at the price of a secret on disk outside the project.
- **A switch to turn it off.** A setting that disables a guard gets turned off.
- **A Unix socket checked by peer.** The VM is Windows.

## Consequences

After a restart, an open tab shows the bar until the new link is opened. `curl`
needs the bearer header from the printed line. Where accounts share a Unix
host, the key passes through the browser's command line when dbt-edith opens
it; `--no-open` and pasting the link avoids that. Any local server on another
port of 127.0.0.1 receives the cookie if the user browses to it, since cookies
are per host.
