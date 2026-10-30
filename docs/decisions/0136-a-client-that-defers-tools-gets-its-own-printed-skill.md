# 0136. A client that defers tools gets its own printed skill

Date: 2026-10-30 · Status: accepted · Amends 0114

**Trigger:** read before changing `docs/agent-skill/mcp/SKILL.md`, what
`--skill` accepts, or the name of either skill.

## Context

0114 ended on "MCP needs no skill". A client that defers MCP tools, Claude Code
among them, shows the agent only the tool's name until it searches for it, and
an agent that never searches does not find `edith`. A variant telling the agent
to load the tool and call it was tried earlier, on Jaffle Shop with Claude
Code only: an agent used 1 to 3 calls where, without a skill, it used 9, and it
did not fall back to `dbt ls`. That was an earlier text, never shipped; the one
shipped here was not measured. Other clients (Copilot among them) are untried.

## Decision

- **`--skill mcp` prints a second skill**, `docs/agent-skill/mcp/SKILL.md`,
  compiled in as the first is. It tells the agent to load the deferred tool
  named `edith` and gives the recipes as lines for the tool, without the program
  and the project (0110). Its description says the tool is deferred, being all
  the agent has before the skill is loaded.
- **`--skill` alone prints the same skill**, led by `--depth 2` and one sentence
  shorter; `--skill command` is the same. Any other word is refused with the accepted ones, as `--output` does.
- **Its name is `dbt-edith-mcp`**, so a client holding both, in
  `.claude/skills/dbt-edith` and `.claude/skills/dbt-edith-mcp`, sees two
  skills and not one name twice.
- **Printed, never installed**, as 0114 says; `--skill` stays refused over MCP.
- **Both skills and `help` lead with `--overview --depth 2`**: at 1 the folders
  only repeat the nodes line, at 2 they are the layers, and below that the list
  grows with the project. The job names do not depend on depth.
- **Held to the binary as the first is**: the same 4096 bytes, and every line it
  cites goes through the tool's own reader, so a flag the tool refuses fails
  the build.

## Rejected

- **Installing it**, or a flag that writes it where a client looks: 0114's reason.
- **A separate flag** (`--skill-mcp`): one flag with a value says the same.
- **One skill naming both routes**: a client with only one pays for both, in
  a file already at its cap.
- **Raising the cap** for the new lead line: the first skill was shortened.
