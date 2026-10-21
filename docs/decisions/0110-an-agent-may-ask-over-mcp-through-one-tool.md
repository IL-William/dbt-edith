# 0110. An agent may ask over MCP, through one tool

Date: 2026-10-21 · Status: accepted, amended by 0113 · Amends 0057, 0099 and 0102

**Trigger:** read before changing `--mcp`, the `edith` tool, what its line
accepts, what the server reads or writes, or before adding a second tool.

## Context

0057, 0099 and 0102 each rejected an MCP server: a protocol and a long-lived
process where one command answers, and a tool described in every context
against a skill line. It was asked for again on 2026-10-20, for Copilot's agent
mode and Claude Code: some clients have no terminal to run the command in, and
a long-lived process can keep the graph the command reloads on every call.

The reasons against did not vanish; two of them were measured on 2026-10-21
(`scripts/walk_mcp.py --measure`, one machine, 3 passes, medians) on Jaffle
Shop and on Fivetran's Shopify v1.10.0 with the manifest, catalog and cache of
dbt-collin's fixture. Orders of magnitude, not current facts.

## Decision

- **Opt-in.** `dbt-edith --mcp [PROJECT] [--manifest P] [--catalog P]` answers
  over stdin and stdout and exits when stdin closes. Nothing configures it: no
  `.mcp.json`, no `.vscode/mcp.json`, and the binary never writes a client's
  settings. The command line and the skill keep working; the two coexist.
  Nothing is read at start, so a client can be configured before `dbt parse`
  and a probe process costs about 3 ms.
- **One tool, `edith`, taking a command line.** The line is split by a
  splitter of our own (quotes, no shell), parsed by the command line's own clap
  definition and accepted only by listing: the flags of the four questions and
  `--output`, by clap argument id. A flag added later is refused until someone
  decides it is a question. The project, the manifest and the catalog are the
  server's, fixed at start; no word of the line becomes a path. `help` or
  `--help` answers a reference that enters the context only when asked.
- **The text is the command line's.** Two blocks: stdout, then stderr (the
  manifest, its date, the freshness line), an empty one left out; `isError` is
  the exit code not being 0. No `structuredContent`.
- **`--run-collin` is allowed per call**, under the guard of 0099 and 0109:
  the collin beside the binary or on the PATH, never one git tracks (0094),
  a 600 s deadline, the cache replaced only by a finished run. A cancellation
  or the end of input kills it. That is why the tool is not marked read-only.
- **Only `openWorldHint` is written, as false.** The spec's default is true,
  and the tool reaches nothing outside the project and the programs on this
  machine: no network, no warehouse. VS Code asks to confirm a result only when
  the hint is truthy, so false is both the truth and the quiet choice.
  *Amended by [0113](0113-the-card-resolves-a-location-per-env-file.md): a
  `.env` symlink may lead the card's read outside the project.*
- **The legacy handshake only.** `initialize` (2025-11-25, answering the
  client's version when it is 2025-06-18 or 2025-03-26), `ping`, `tools/list`,
  `tools/call` and `notifications/cancelled`. Claude Code probes with
  `server/discover` and falls back to `initialize` on the -32601 it gets; VS
  Code sends `initialize` 2025-11-25. The modern 2026-07-28 handshake is left
  for later.
- **Two threads.** One reads stdin and answers `initialize`, `ping` and
  `tools/list` itself, so none waits behind a call; a worker answers the calls
  in order. A cancelled call writes no response.

## What it adds, measured

- **Faster per call, 1.5x to 2x**: a warm call took 4.9 ms against 9.8 ms for
  the command line on Jaffle Shop and 34.9 against 53.0 ms on Shopify. Checking
  that the kept graph is still good costs microseconds; what it saves is the
  process start and the load (9 ms for the manifest and 39 ms for the graph
  `--describe` and `--overview` use on Shopify). What is left is the freshness
  line, whose git processes run on every call.
- **Memory is kept**: 3.5 MiB after the handshake, 8.5 MiB (Jaffle Shop) and 30
  MiB (Shopify) after 20 lines, against 7 and 18 to 22 MiB for one command.
- **Context**: the tool and the instructions are about 153 tokens, against 48
  for a one-line skill description and 447 for a tool per mode (bytes over
  four, not Claude's tokenizer).
- **Clients without a terminal** can ask at all. Approving the one tool
  approves collin runs too, since the tool cannot be split by flag.

## Rejected

- **A tool per mode**: three times the bytes in every context, for the same
  answers.
- **A separate binary** (0001), or the `rmcp` crate (0003): an async stack and
  schema macros for six methods.
- **`structuredContent`**: VS Code shows it to the model instead of the text,
  and the JSON is already a flag.
- **A cap on long answers**: the command line's answer is the answer. Clients
  cut them (Claude Code stops at 25 000 tokens); a selection that long is asked
  again narrower.
- **Reading `CLAUDE_PROJECT_DIR`**: one client's variable deciding the project.
- **Keeping one git answer for a second or two**, which would take the warm
  call on Shopify from about 35 ms to well under 1: not asked for, and it would
  make the freshness line late.
- **Forbidding `--run-collin` at start**: the flag is already per call.

## Consequences

- **A stale manifest answers stale**, as on the command line; the second block
  says so.
- **0057, 0099 and 0102 are not edited beyond a note**: their Rejected entries
  are what was weighed then, and this is what followed.
- `graph.rs` prints "column lineage capped" to the server's own stderr, not to
  the answer.
- On Windows a client that ends the server with `TerminateProcess` can leave a
  running collin to its end. No CI job covers Windows, and the stand-in collin
  of the check is a shell script.
- A line is at most 16 KiB and a message 1 MiB, and a longer one is refused.
  The modern handshake and the other limits are in `docs/state.md`.
