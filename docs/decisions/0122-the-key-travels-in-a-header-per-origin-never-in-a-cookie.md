# 0122. The key travels in a header, per origin, never in a cookie

Date: 2026-10-27 · Status: accepted · Amends 0052

**Trigger:** read before changing how the page or a tool presents the launch
key, before adding a way in that is not a plain request, or before setting any
cookie.

## Context

0052 swapped the printed link's key for a cookie. Cookies are kept per host and
not per port, so the cookie, `HttpOnly` and `SameSite=Strict` notwithstanding,
went to every server on 127.0.0.1 the browser visited, and 0052 said so in its
consequences. A page served from another port of the same machine, a
development server or a notebook, received a key that opens a shell.

## Decision

- **The server sets no cookie and reads none.** `/?key=` serves the page like
  `/`, with no redirect.
- **The page keeps the key itself.** At start-up it reads `key` from the
  address and removes it from the address bar with `history.replaceState`.
  Once the server has accepted it, the page stores it in `localStorage`, which
  a browser keeps per origin and so per port: a stale link, a bookmark from an
  earlier launch, cannot overwrite the key a newer tab stored. On a refusal the
  page reads storage again and retries once if another key is there. Where
  storage is refused the key lives in the page's memory. A reload or a new tab
  finds it in storage; a restart draws a new key and the old one gets the bar
  0052 describes.
- **Every request sends `Authorization: Bearer <key>`.** That is what `curl`
  already sent. A WebSocket handshake cannot carry a header from a browser, so
  `/ws/` also accepts a `key` parameter in its address; the API does not, as an
  address ends up in histories and logs.
- Host, Origin and `Sec-Fetch-Site` are checked as before.

## Rejected

- **`sessionStorage`.** A new tab would have no key, which the cookie allowed.
- **Keeping the cookie, scoped by path.** Cookies ignore ports, not just paths.
- **The key in every address.** It would stay in the history and a bookmark.

## Consequences

The key stays in the browser's storage for this port after the server stops;
it opens nothing then, a new launch drawing a new one. Script access to the
page's storage is script access to the key, as it was to the page.
