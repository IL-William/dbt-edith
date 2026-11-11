# 0172. Only the Rust code is formatted, by rustfmt with a packed config

Date: 2026-11-11 · Status: accepted

**Trigger:** read before changing `rustfmt.toml`, before formatting another
language (JavaScript, CSS or Python), or before resolving by hand a conflict in
formatted code: never do that, take one side and run `cargo fmt`.

## Context

The Rust code was written by hand, wide and packed: long lines, short bodies on
one line, arrays of short items on one. rustfmt with no configuration would
have rewritten that style, so nothing enforced a layout and every author
laid it out their own way.

Measured on the 51 `.rs` files of this repository (64,148 lines, 3.03 MB):

| | lines | bytes |
| --- | --- | --- |
| rustfmt, no `rustfmt.toml` | about +35% | about +10% |
| rustfmt, the config below | about +1% | about +0.4% |

The other languages were measured the same way, and none survives a
formatter:

- **JavaScript and CSS** (Biome and Prettier-style formatters): they collapse
  the aligned trailing comments of `web/app.js`, explode compact objects, and
  break every one-line CSS rule. `web/app.js` grows by about 12% in lines even
  at width 160, the CSS by about a factor of 2.8, and no option keeps either
  layout.
- **Python** (`ruff format`): about +12% in lines at width 140, with every list of
  strings put one per line.

## Decision

- **`cargo fmt --check`** runs in `check.sh`, right after Clippy, and in the
  `rust` job of CI before the tests, not on the Monday run (a new stable
  rustfmt must not turn it red when nothing was pushed). `check.sh` prints the
  names of the files that differ, at most thirty, then a count and
  `run cargo fmt`, and skips itself without the `rustfmt` component.
- **`rustfmt.toml`** holds four stable options, each there to keep the layout the
  code already had: `max_width = 140`, `use_small_heuristics = "Max"` (a short
  struct, call or `if` stays on one line), `fn_params_layout = "Compressed"` and
  `short_array_element_width_threshold = 40`.
- **Run `cargo fmt` before a Rust change is done**, as `check.sh` is.
- **A conflict in formatted code is never resolved by hand.** Take one side of
  the file whole and run `cargo fmt` again, so the result is what the tool
  writes and not a layout that only one person's editor agrees with.
- **Only Rust is formatted.** JavaScript, CSS and Python are linted instead
  (0173), which never touches their layout.
- **A block whose alignment is the point** may carry `#[rustfmt::skip]` with a
  one-line reason.

## Rejected

- **rustfmt's defaults**: a third of the lines move and the code grows by a third,
  for a style nobody here wrote.
- **Formatting every language**: the measurements above. A diff that rewrites
  most of `web/app.js` and the style sheet buries every later change in blame,
  and the aligned comments are how that file is read.
- **Formatting Rust only on the lines a change touches** (`--file-lines`): it
  is unstable, and a file would hold two layouts until every line had been
  touched.
- **Nightly-only options** (`imports_granularity`, `wrap_comments`): `cargo
  install` on the locked-down machine and CI both use stable (0171), and a
  nightly would be the only place the check ran.
