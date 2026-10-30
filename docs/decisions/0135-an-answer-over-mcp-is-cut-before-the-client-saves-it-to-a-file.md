# 0135. An answer over MCP is cut before the client saves it to a file

Date: 2026-10-30 · Status: accepted · Amends 0110 · 0057 holds for the command line

**Trigger:** read before changing how long an answer over `--mcp` may be, the
line that closes a cut one, or before cutting an answer on the command line.

## Context

0110 rejected "a cap on long answers": the command line's answer is the answer,
a client cuts a long one, and a selection that long is asked again narrower.
That left out what the cut costs. Claude Code's documentation, in its section
on MCP output limits (https://code.claude.com/docs/en/mcp.md), checked on
2026-10-30, says:

- A warning appears past 10 000 tokens.
- Past 25 000 tokens (`MAX_MCP_OUTPUT_TOKENS`) or past 50 000 characters of
  text, the result is not shown. It is saved to a file under
  `~/.claude/projects/`, which the agent then pages through with Read.
- Under 50 000 characters (and 25 000 tokens) the result is shown whole.
- An error result over about 11 000 characters keeps only its first and last
  5 000.

The MCP specification has no field for a cut answer: `CallToolResult` has
`content`, `structuredContent` and `isError`.

So an agent that asks for a big folder does not get "ask
narrower"; it gets a file it pages through, or gives up. Observed, not in the
documentation: that folder is outside the agent's working directory, so reading
it asks for a permission.

## Decision

- **`MAX_ANSWER` is 48 000 UTF-16 units** in `src/mcp.rs`, under the 50 000 with
  room for the closing line. Units, as the client counts: Claude Code 2.1.314
  sums the JavaScript `length` of the text blocks, which counts a character
  outside the Basic Multilingual Plane twice, as Rust's `chars()` does not. It
  counts both text blocks (the answer and the freshness block), and the cut is
  made in the first, so the second stays whole.
- **A longer answer is cut after its last whole line that leaves room for one
  closing line.** The line says how many lines of how many were left out, and
  what makes the answer shorter in the mode asked: `--count-by folder` (or
  kind, tag, package) and a narrower `--select` for a selection; for a
  selection or a column, a shorter form of the output only where there is one:
  `--output compact` when it was json, `--output name` (names only) when it was
  compact, and nothing when it was names, which are the shortest (compact adds
  the kind, path, tags and parents, and a column's depth, source and tests);
  fewer names, or no `--sql` or `--compiled-sql` (whichever was given),
  for `--describe`; a narrower `--select` or
  `--describe NAME --columns` for `--columns` with a selection, and fewer names
  for `--columns` with `--describe` (json points at compact; its names output
  is the cards' column blocks, which compact does not shorten); a
  smaller `--depth` for `--overview`; a smaller `--up` or `--down` and
  `--stop-at` for `--column`; for `--job` and `--job-order`, which have no flag
  that shortens them, `--describe NAME` or a counted selection; and for
  `--job-coverage`, whose output forms are not shorter ones, `--job NAME` or
  `--describe NAME`. The line
  is one line and begins `answer cut at`.
- **`--output json` stays JSON line by line.** Every JSON answer is one object
  per line, so cutting at a line boundary keeps each kept line valid, and the
  closing line is itself an object, `{"cut": "..."}`, so a reader that parses
  every line still can. An answer that is one object on one line
  (`--overview`, `--job`, `--job-order`, `--job-coverage` and a counted
  selection) has no whole
  line to keep: it is refused, with the closing line alone and `isError` set,
  rather than cut mid-object into text that does not parse; its line adds
  "Or leave out --output json", since the text form is cut and not refused. The
  same happens to a text answer whose first line is over the cap.
- **An error with no answer in it is not cut**: a refusal is a sentence or two.
  An error that carries an answer is: `--describe a,b,typo` refuses with the
  cards found still in the first block. Claude Code keeps only the first and
  last 5 000 characters of an error past about 11 000, so such a one is cut at
  `MAX_ERROR`, 10 000, by the same rule, and stays an error.
- **The command line is never cut** (0057: `--output name` prints the whole
  answer). So under the cap the two answers are still byte for byte the same,
  which `scripts/walk_mcp.py` compares; it also checks that every answer it
  compares is under the cap, and the cut itself is tested in `src/mcp.rs`.

## Rejected

- **Setting `_meta["anthropic/maxResultSizeChars"]` on the tool.** For text,
  Claude Code uses that value, up to 500 000, instead of its own limits. It is
  that client's key and other clients ignore it, and it would hand an agent up
  to 500 000 characters of context in one answer.
- **Cutting on the command line too.** A person or a script piping the answer
  wants all of it, and nothing there saves it to a file.
- **A paging flag (`--offset`).** One more flag in every mode, and a second call
  by an agent that would do better with a count or a narrower selection, which
  the closing line names.
- **Counting tokens.** There is no tokenizer here. The assumption, as read in
  Claude Code 2.1.314 on 2026-10-30: it estimates tokens as length over 4 and
  counts for real only above 12 500, so 48 000 units (12 000 estimated) stay
  under its 25 000 tokens whatever the text. It bites only if a user sets
  `MAX_MCP_OUTPUT_TOKENS` under about 24 000, and then the client saves the
  answer to a file as before.

## Consequences

An agent gets the first lines and the way to ask for the rest in the answer
itself, without a permission prompt. An answer under the cap is
unchanged. The 48 000 and the client's figures
are dated here, and a client that changes its limit changes the case for the
number, not the rule.
