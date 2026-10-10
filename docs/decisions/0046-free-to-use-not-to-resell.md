# 0046. Free to use, not to resell

Date: 2026-10-07 · Status: accepted

**Trigger:** read before changing the license, the copyright line, or the
`license` field of `Cargo.toml`, and before bundling code whose license would
have to be weighed against this one.

## Context

dbt-edith was published under MIT, which lets anyone sell it, or a service
built on it, as long as the notice stays. The intent was never that: it is
meant to be free for whoever uses it, a company and its consultants included,
but not something a third party can sell as its own product.

## Decision

- **The license is FSL-1.1-MIT**, the Functional Source License 1.1 with an MIT
  future license, as published at fsl.software. Any purpose is allowed except a
  competing use: making the software available to others in a commercial
  product or service that substitutes for it, or offers substantially the same
  features. Internal use, and professional services for a client, are allowed
  by name.
- **Each version becomes MIT two years after it is published.** That is part of
  the license, not a promise kept by hand, and it is what keeps the restriction
  to the newest work.
- **The licensor is Datadorelix**, the company, not a person: `Copyright 2026
  Datadorelix` in `LICENSE`.
- **The repository starts again at 1.0.0**, one commit, under this license. The
  0.x versions stay MIT for whoever has them, since a license already granted
  is not withdrawn; their history is kept outside this repository.

## Rejected

- **MIT, kept.** It allows exactly the resale this is meant to stop.
- **PolyForm Noncommercial.** It forbids any commercial use, so a company could
  not use the tool internally, nor a consultant on a client's project, which are
  the users it is for.
- **PolyForm Shield.** It also forbids competing with any product the licensor
  provides using the software, which reaches other consultants working with
  dbt-edith, and it never ends.
- **Business Source License and Elastic License 2.0.** BUSL needs an
  additional use grant written for the product and a change date picked by
  hand; Elastic is written for hosted services and keeps its restriction
  forever. FSL does what is wanted with nothing to fill in.

## Consequences

dbt-edith is source-available, not open source in the OSI sense, and is not
described as open source. `Cargo.toml` carries the SPDX identifier
`FSL-1.1-MIT`. The libraries in `web/vendor/` and the crates the binary links
are MIT, Apache-2.0 or similarly permissive, which allow being shipped under
this license; `THIRD_PARTY_NOTICES.md` is unchanged. Bundling anything
copyleft would need weighing against this decision first.
