# 04. `askance ask` CLI

## What to build

The agent-facing surface, per ADR-0001. `askance ask [file.yaml]` reads a
Question Set as YAML from the file argument or stdin, validates it
client-side with the same `schema` validation as the server (failing before
sending, with a pointed error naming the offending Question), enriches it,
POSTs it, and blocks until the Response arrives, printing it as YAML on
stdout and exiting 0. Exit only on delivery or being killed.

Enrichment, derived — never agent-supplied:

- `project` and `branch` from the CWD, worktree-smart: in a linked worktree
  the root repo's name, via `git rev-parse --git-common-dir`
- the Diff: all uncommitted changes including untracked files' contents,
  binary contents omitted, captured once at send time; absent when the tree
  is clean or the CWD isn't a git repo

The wait is a reconnecting long-poll against task 03's wait endpoint:
"nothing yet" responses, connection drops, refused connections, and server
restarts are all absorbed silently by retrying (with a short backoff) — no
expiry, per ADR-0001. Server URL defaults to localhost and is overridable
by an environment variable.

## Acceptance criteria

- [ ] A schema-violating Set fails validation locally with an error naming
      the offending Question; nothing is sent
- [ ] Run from a linked worktree, the stored Set's `project` is the root
      repo's name
- [ ] The Diff includes an untracked file's contents; on a clean tree the
      Set carries no Diff
- [ ] Killing and restarting the server mid-wait: the CLI reconnects
      silently and still delivers the Response, exiting 0
- [ ] The Response arrives on stdout as YAML with the markdown/multi-line
      fields as `|` block scalars; retry noise goes to stderr, not stdout
