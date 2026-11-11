# 0171. Clippy runs on stable with every lint an error

Date: 2026-11-11 · Status: accepted

**Trigger:** read before silencing a Clippy lint or adding an `#[allow]`, before
changing the Clippy step of `scripts/check.sh` or of the `rust` job in CI, or
before pinning a Rust toolchain.

## Context

Nothing read the Rust code for mistakes beyond what `rustc` warns of, so lints
accumulated unseen: about fifty findings of some twenty lints, none a
correctness bug, most mechanical (`unnecessary_sort_by`, `needless_lifetimes`, `unnecessary_map_or`).
A lint nobody has to fix is a lint nobody reads.

## Decision

- **`cargo clippy --all-targets --locked -- -D warnings`** runs in `check.sh`,
  right after `cargo test`, and in the `rust` job of CI, on the stable
  toolchain. Any lint fails the run. `check.sh` prints one line per finding, at
  most thirty, then a count, and skips itself without `clippy`.
- **Three lints are allowed for the whole crate**, in `[lints.clippy]` of
  `Cargo.toml`, each a design choice:
  - `too_many_arguments`: the printers of the command line take their inputs as
    arguments, not through a context struct built to be read once;
  - `type_complexity`: a few payload types nest tuples, and an alias would only
    rename them;
  - `result_large_err`: a handler's `Err` is an axum `Response`, which is what it
    returns on the way out.
- **Any other lint is fixed, or allowed where it fires**, with a one-line
  comment saying why that place is not the lint's case. No other crate-wide allow.
- **Not on the Monday run.** That run is for advisories, which appear with
  nothing pushed; a new stable Clippy brings new lints the same way, and must not
  turn it red. The next push shows them.

## Rejected

- **Pinning a toolchain** (`rust-toolchain.toml`): `cargo install` from the
  clone would make every user fetch that toolchain, and the locked-down Windows
  machine can install nothing (0001). The cost of stable is that a new Clippy can fail
  a push, fixed in the push it fails.
- **Allowing whole groups** (`style`, `complexity`, `perf`): the groups hold the
  lints worth having along with the ones that are not, and an allow on a group
  hides the next lint added to it.
- **Warnings only, never failing**: nobody reads a warning list that stops
  nothing.
- **A context struct for the arguments, aliases for the nested types**: churn
  across the printers for no reader gain.
