# 0054. The profile is edited without its secrets

Date: 2026-10-10 · Status: accepted · Amends 0017

**Trigger:** read before changing what `/api/profiles` sends or accepts, or
which keys of a profile count as secret.

## Context

0017 made `profiles.yml` editable from the page, and accepted that a password
in it would reach the browser. Once there, it sits in the tab's memory, in the
devtools, on any shared screen, and within reach of every extension allowed on
the page. 0012 and 0019 already keep `.env` values on the server; the profile
was the one file with credentials that crossed.

## Decision

- **`GET` hides, `PUT` restores.** `src/redact.rs` puts `<hidden by dbt-edith>`
  in place of each secret value, a block scalar such as a private key included,
  and counts them. On save, every line still holding the placeholder gets the
  value on disk back, found by the keys that lead to it, `shop.outputs.dev.password`.
- **What is secret** is the vocabulary 0019 uses for environment variables,
  plus `pass`. A key naming where a secret is, `private_key_path`, is not one;
  neither is an `env_var()`, an empty value or `true`.
- **A placeholder with nowhere to go is refused**, a target renamed or a line
  pasted elsewhere, and nothing is written. Typing over the placeholder replaces
  the value, as any edit would.
- **A line scanner, not a YAML parser** (0003, 0018): a key by its indentation,
  a value running on over the lines indented further.

## Rejected

- **Masking in the browser.** The value would still be in the payload, which is
  what 0019 rejected for `.env` values.
- **Read only.** 0017's reason stands: an editor that shows the problem and
  cannot fix it sends the user elsewhere.

## Consequences

The tab and the status bar say how many values are kept on the server. A flow
mapping, `{ password: x }` on one line, is beyond the scanner and is sent as it
is. A `profiles.yml` inside the project is a project file too: the explorer
opens it through `/api/file`, unredacted, as it opens a `.env` (0015).
