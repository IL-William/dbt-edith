# 0149. The terminal loads libraries from System32, and the Snowflake script wants a fixed connector

Date: 2026-11-05 · Status: accepted · Builds on 0053 and 0094

**Trigger:** read before loading a library by name on Windows, adding a
dependency that does, or changing which `snowflake-connector-python` the
Snowflake script accepts.

## Context

0053 and 0094 hold what a project brings. Two gaps were left. On Windows the
terminal's `portable-pty` loads `conpty.dll` by bare name, to prefer a sideloaded
copy over kernel32's (`win/psuedocon.rs` lines 55 to 59 in 0.9.0). The file is
not in System32, so the loader goes on to the `.exe`'s folder, the current folder
and the PATH: a `conpty.dll` committed in a project or left beside the download
would run when the terminal opens. And `tools/sf_lineage.py` used whatever
connector the venv had, though it hands that library the user's credentials, and
the GitHub advisory database lists two for 2026: GHSA-5cc2-282f-jjq2 (TLS host
names not verified, fixed in 3.18.1 and 4.7.1) and GHSA-qqj6-54q6-cxv6 (secrets
written to logs, fixed in 4.7.3, every 3.x release being in its range).

## Decision

- **`main` first calls `SetDefaultDllDirectories(LOAD_LIBRARY_SEARCH_SYSTEM32)`**
  (`src/dll_search.rs`), declared `extern "system"` from kernel32, so no crate
  (0003). By-name loads then search System32 only: the sideload fails and
  `portable-pty` takes kernel32's `CreatePseudoConsole`, as when the file is
  absent. A failed call is written once to stderr and the run goes on.
- **The script refuses a connector older than 4.7.3**, the first release that
  fixes both, or one whose version it cannot read, with a sentence naming the
  version found, the minimum and the upgrade command. The check sits in
  `connector()`, so `serve` stops before it says `ready`.

## Rejected

- **`LOAD_LIBRARY_SEARCH_DEFAULT_DIRS`**: it keeps the `.exe`'s folder, where a
  file left beside the download would sit.
- **Patching `portable-pty`** to drop the sideload, for a file nothing ships.
- **Accepting 3.18.1 and later on 3.x**: it fixes only the TLS advisory.
- **A warning instead of a refusal**: a line nobody reads while the credentials
  go through the old library.

## Consequences

- The search path cannot be widened again. It covers loads by name made once
  `main` has started; the C runtime the msvc `.exe` imports is resolved by the
  loader before that, so linking it statically is a Deferred item (state.md 39).
- 4.7.3 wants Python 3.10 or later, as the script does. A new advisory is a new
  constant in the script and a line here.
- The refusal's command names the interpreter that ran the script. dbt-snowflake
  before 1.10.6 pins the connector below 4.0.0, the pin being its own, so where
  it is installed at such a version the command upgrades both together, and says
  that this moves dbt-core to 1.10 or later. A newer one is left alone.
