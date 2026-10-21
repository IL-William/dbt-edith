# 0114. A skill is shipped and printed, never installed

Date: 2026-10-21 · Status: accepted · Amends 0057, 0102 and 0110

**Trigger:** read before changing the skill file, `--skill`, or what is written for a client.

## Context

0057, 0102 and 0110 left the skill to the caller. Tried with agents, a skill
ran or not by its description: a one-line one was not loaded, one naming the
questions it answers nearly always was. Written per caller, it goes stale.

## Decision

- **The skill is a file**, `docs/agent-skill/SKILL.md`: the page's recipes,
  habits and freshness rule, under a description listing the questions it answers.
- **`dbt-edith --skill` prints it** for the installed version, compiled in, and
  exits 0. It reads, writes and starts nothing; every other flag, `--mcp`
  included, is refused beside it.
- **The caller installs it**, by redirecting the output where its client looks;
  0110's rule that the binary never writes a client's settings stands.
- **It says what 0113 decided**: the per-`.env` lines are the config before
  `generate_*_name`, not where dbt builds; a source's line is exact.
- **The tests hold it to the binary**: every long flag in it exists, every
  selection it cites resolves and every command parses, MCP line included; not
  what a recipe means. A new description is measured with agents.
- **It costs context**: about 190 tokens always, 800 once loaded, against the
  tool's 153 and at most 384 for `help`; it needs no server, and a client that
  defers MCP tools finds it where it may not find the tool.
- **Amends 0057 and 0102 (the skill is the caller's) and 0110 (nothing shipped
  for a client).** MCP needs no skill: `--skill` is refused there.

## Rejected

- **The binary writing `.claude/skills` or a client's settings**, or an
  `--install-skill` flag: a write nobody asked for, a path per client to keep
  right; a redirect says it in one line.
- **`--skill` over MCP**: a client with a tool list has no skill folder.
- **A one-line description only** (not loaded), or **generating it from the
  clap help** (flags, not recipes).
