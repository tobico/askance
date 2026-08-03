# Askance — plan

A single-user Leptos web service that receives Question Sets from coding
agents and lets the human answer them from any device on the tailnet; the
agent blocks until the Response is submitted. See `CONTEXT.md` for the
glossary and `docs/adr/` for recorded decisions.

## Agent integration (CLI)

- Companion CLI in this repo: `askance ask` reads a YAML Question Set from
  stdin or a file argument.
- The CLI derives `project` and `branch` from the working directory,
  worktree-smart: in a linked worktree it reports the root repo's name (via
  `git rev-parse --git-common-dir`). Agents never supply these.
- The CLI captures the repo's Diff at send time — all uncommitted changes
  including untracked files, binary contents omitted — and attaches it to the
  Set. Absent when the tree is clean or the CWD isn't a git repo. This powers
  code-approval review in the web UI.
- It POSTs the Set, then long-polls with reconnection — no expiry — until the
  Response arrives, and prints it as YAML on stdout. Exit only on delivery or
  being killed.
- Agents run it via a background shell command (e.g. Claude Code Bash
  `run_in_background`).
- Default server URL is localhost; overridable by env var so agents on other
  tailnet machines can point at this box.
- The CLI validates the Set against the schema before sending and fails with
  a clear error.

## Wire format (YAML both directions)

YAML chosen over JSON for fewer tokens and because the markdown Preface goes
in as a `|` block scalar rather than a JSON-escaped string.

Question Set:

- `title` — short, for the pending list (agent-supplied)
- `preface` — markdown block scalar
- `questions[]`:
  - `label` — agent-supplied, opaque to the server (e.g. `Q7`; numbering is
    monotonic across the agent's session, so only the agent can assign it)
  - `text`
  - `options[]` (optional): `n`, `text`, `recommended` (at most one per
    question)
  - `subquestions[]` (optional): `letter`, `text`, `options[]` — leaves only;
    two levels maximum, enforced by schema
- No multi-select. Server stamps `id` and `created_at`; CLI adds `project`,
  `branch`, and `diff`.

Response (mirrors the Set):

- per Question / Sub-question: `selected` (option number) and/or `free_text`,
  or an explicit `unanswered: true` marker — every question appears one way
  or the other, so the agent can't silently miss an open question
- unanswered questions are legal (the grammar's "still open" state): the
  human may submit with any number unanswered — down to zero Answers plus a
  comment — e.g. to ask a counter-question that redirects the discussion
- set-level `comment` (optional)

## Server

- Axum + Leptos SSR, SQLite persistence.
- No app-level auth: binds for tailnet/localhost use only; Tailscale ACLs are
  the perimeter.
- Sets are never auto-withdrawn. The UI shows Liveness ("agent waiting" /
  "agent disconnected") derived from whether a long-poll connection is
  currently held. Orphaned Sets are archived manually.
- Answered Sets are kept forever in the Archive.

## Frontend (responsive PWA)

- Pending list: title, project, branch, age, Liveness badge. Archive view.
- Set view: rendered Preface, then Questions with radio Options and per-
  question free-text fields; Recommendations visually highlighted.
- Diff viewer in the set view: renders the attached Diff for code approval.
- Explicit "accept all recommendations" button — fills every unanswered
  Question with its Recommendation; nothing is pre-selected on load;
  individual Answers can still be overridden before submit.
- Submit warns — but does not block — when Questions are Unanswered: a
  confirmation lists them before the Response goes out.
- Draft Answers autosaved to localStorage per device.
- Web Push: one notification per new Set, deep-linking to it. VAPID keys
  auto-generated on first run, stored in SQLite.
- Served over HTTPS via `tailscale serve` (secure context for the service
  worker and push).

## Deployment

- Nix flake: package (server + CLI) and a NixOS module (systemd service with
  port and db-path options).
- Runs on the same box the agents run on.

## Adoption (final stage)

Amend `tobico-skills` (question grammar + question-asking skills) so agents
route question sets through `askance` when the CLI is present, falling back
to asking in chat.
