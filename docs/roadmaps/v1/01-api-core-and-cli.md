# 01. API core + CLI

## Goal

An agent runs `askance ask questions.yaml` (or pipes YAML on stdin), the Set
appears in the server's SQLite store, a Response submitted via `curl` is
printed by the still-waiting CLI as YAML, and the CLI exits 0. The whole
agent-facing contract works before any UI exists.

## Decisions in force

- **Blocking CLI, no expiry** — [ADR-0001](../../adr/0001-blocking-cli-for-agent-integration.md).
  The CLI holds a reconnecting long-poll until the Response arrives or it is
  killed; there is no server-side timeout. Transient drops (laptop sleep,
  tailscale blip) must reconnect silently.
- **YAML both directions** (PLAN.md “Wire format”) — chosen over JSON for
  token economy and because the markdown Preface rides in a `|` block scalar.
- **Direct encoding of the question grammar** — two levels max, sub-questions
  are leaves, at most one `recommended` Option per question, no multi-select.
  The schema enforces these invariants; the CLI validates before sending and
  fails with a pointed error.
- **Labels are agent-owned and opaque** — only the agent knows its session
  counter (`Q7`), so the server never assigns or interprets labels.
- **CLI derives `project` and `branch`**, worktree-smart: in a linked
  worktree, report the root repo's name (`git rev-parse --git-common-dir`).
  Agents never supply these — determinism over trust.
- **Every question appears in the Response, answered or explicitly
  unanswered** — per Question/Sub-question: `selected` and/or `free_text`, or
  `unanswered: true`. Unanswered is legal (the grammar's "still open" state);
  a Response with zero Answers plus a set-level comment is a valid
  counter-question. The server rejects only Responses that omit a question
  entirely — explicitness is the invariant, not completeness.
- **CLI attaches the Diff** — all uncommitted changes including untracked
  files, binary contents omitted, captured once at send time; absent when
  the tree is clean or the CWD isn't a repo. Powers code approval in the UI
  (stage 02).
- **No app-level auth** — tailnet is the perimeter; server binds for
  localhost/tailnet use. CLI defaults to localhost, env var overrides.
- **SQLite** persistence; server stamps `id` and `created_at`.
- Terms per [CONTEXT.md](../../../CONTEXT.md): Question Set, Answer vs
  Response distinction matters in API naming.

## Proposed tasks (provisional)

1. **Submit a Question Set** — cargo workspace (server + cli crates); axum
   server with `POST /api/sets` validating the YAML schema and persisting to
   SQLite.
   - Valid example set is stored and returns its `id`
   - Schema violations (three levels deep, two `recommended`, missing title)
     are rejected with errors naming the offending question
2. **Answer and deliver** — Response submission endpoint + long-poll endpoint.
   - A Response omitting a question is rejected; one marking it
     `unanswered: true` is accepted, including the zero-Answers case
   - A long-poll waiting before submission receives the Response on submit;
     one arriving after gets it immediately
3. **`askance ask` CLI** — stdin/file input, client-side validation,
   `project`/`branch` detection, Diff capture, reconnecting long-poll, YAML
   Response on stdout.
   - From a linked worktree, `project` is the root repo's name
   - Diff includes an untracked file's contents; omitted on a clean tree
   - Killing and restarting the server mid-wait: CLI reconnects and still
     delivers
4. **End-to-end example + quickstart** — `examples/` sample set, README walk
   through the curl round trip.
   - A fresh checkout can reproduce the ask→answer→deliver loop from the README

## Re-verify at start

- Leptos/axum current versions and whether Leptos's axum integration
  constrains how plain REST routes are mounted (stage 02 will add SSR to this
  server — pick crate layout accordingly).
- Rust YAML crate choice (serde_yaml is archived/deprecated — check the
  current successor, e.g. `serde_yml` or alternatives).
- Long-poll vs SSE for the wait endpoint — brief assumes long-poll with
  timeout+retry; confirm nothing about tailscale serve buffering changes the
  calculus (relevant by stage 04).
