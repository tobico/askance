# Agent integration via a blocking CLI, not MCP or skill-driven polling

Agents need to submit a Question Set and then wait — possibly hours — for the
human's Response. We ship a CLI (`askance ask`) that POSTs the Set and holds a
reconnecting long-poll until the Response arrives, printing it to stdout.
Agents run it as a background shell command, so the wait survives harness tool
timeouts and works in any agent that can run a shell command, with no
per-harness configuration.

## Considered Options

- **MCP server** — tightest integration, but hours-long tool calls are at the
  mercy of each harness's MCP timeout, and every project/agent needs MCP
  config. Can still be added later as a facade over the same HTTP API.
- **Skill-driven polling (curl)** — zero client software, but burns agent
  turns polling and trusts the model to keep polling correctly for hours.

## Consequences

- The CLI is the compatibility surface: it also derives `project` and
  `branch` deterministically from the working directory (worktree-smart), so
  agents never supply them.
- No server-side wait expiry: a wait ends only when the Response is delivered
  or the CLI process is killed.
