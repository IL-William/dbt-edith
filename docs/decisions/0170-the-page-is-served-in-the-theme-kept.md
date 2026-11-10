# 0170. The page is served in the theme kept

Date: 2026-11-10 · Status: accepted · Amends 0150 and 0052

**Trigger:** read before changing what the server writes into `index.html`, what
of `look.json` leaves the server without the key, or when a page reads the look
again.

## Context

The page started in Night and changed once `/api/look` answered, so a person who
kept Light, Graphite or Halloween saw a flash of Night on every load, and another
open page kept its theme until it was reloaded. 0150 left the flash for a later
step, with the config read before the first byte. The page cannot do it for
itself: the content security policy allows no inline script, so a script in the
head that reads the theme is refused, and `/` is fetched with no key, since the
page's files stay open (0052).

## Decision

- **The server writes the theme's name on the `<html>` tag of `index.html`**, from
  the look it reads for that request. `web/index.html` keeps `data-theme="night"`
  as its own value, and the swap stays inside the `<html ...>` tag, so a
  `data-theme` further down is never touched.
- **What leaves without the key is that one value**: the theme's name, from the
  closed list of `Theme`. Never `follow`, the apparitions or the boxes, and never a
  string the list does not hold: a `look.json` naming another theme reads as Night
  (0150).
- **With Follow on the page is left as it is.** The server cannot know the
  system's scheme, so the page keeps Night until `/api/look` answers.
- **Halloween's page is served as Halloween but its night art and box classes wait
  for the answer**: the box styles are set by the first look applied, and the night
  art needs the browser's clock. The witch is placed whenever the theme is on and
  she has no place, since a page served as Halloween is already on at its first
  look.
- **A page reads the look again when it is shown or focused once more**, after its
  own saves, and drops an answer that is not news or that follows a choice made
  here. A theme picked in another window shows in this one when it is next seen.

## Rejected

- **An inline script in the head**: the policy refuses it (0015's `script-src
  'self'`), and loosening it for a theme is a price no flash is
  worth.
- **A copy of the theme in `localStorage`**: 0011 keeps what is kept outside the
  page, and 0150 rejected it for the same reason; a cleared profile or another
  port would lose it.
- **A script file in the head**: one more request on every load, for a flash that
  one attribute ends.
- **A client hint for the scheme (`Sec-CH-Prefers-Color-Scheme`)**: Chromium only,
  and it needs a `Critical-CH` retry.
- **Light's block under a media query**: 126 names, and it breaks the test that
  holds the four blocks to one set.
- **A poll every few seconds, or the `storage` event**: the poll repaints the
  settings under the hand, and the event needs `localStorage`.

## Consequences

- A reload opens in Light, Graphite or Halloween with no flash of Night.
- With Follow on the page is Night until `/api/look` answers, and Halloween's night
  art waits for that answer too.
- A window beside the one where the theme was picked keeps its theme until it is
  focused.
- The theme's name goes out without the key; nothing else of the look does.
  This origin's pages and any local process that reaches the port can read it;
  CORP `same-origin`, `frame-ancestors 'none'` and no CORS header keep other
  sites' pages out.
