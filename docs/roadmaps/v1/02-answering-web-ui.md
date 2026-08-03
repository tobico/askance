# 02. Answering web UI

## Goal

Open the server in a browser, see the pending Question Sets, open one, read
its Preface, answer every Question, and submit — unblocking the waiting CLI.
Replaces the curl round trip from stage 01 as the human's interface.

## Decisions in force

- **Leptos SSR on the existing axum server** (PLAN.md “Server”) — one binary
  serves both the agent API routes and the UI. Responsive layout from the
  start: this UI is the phone experience in stage 04, not a desktop-first
  page to retrofit.
- **Set view shape** — rendered markdown Preface first, then Questions in
  order: radio Options (no multi-select), a free-text field on every Question
  and Sub-question, and one set-level comment box.
- **Recommendations are highlighted but never pre-selected** — the ★ Option
  is visually marked; accidental submission of unread recommendations must be
  impossible (the accept-all affordance arrives in stage 03).
- **Submit is gated on completeness** — disabled until every Question and
  Sub-question has an Answer (Option and/or free text), mirroring the
  server-side rejection from stage 01. The grammar's “still open” state must
  be unrepresentable.
- **Pending list shows** title, project, branch, age. (Liveness badge is
  stage 03.)
- No auth, no user accounts — single-user tool on the tailnet.

## Proposed tasks (provisional)

1. **Leptos SSR skeleton + pending list** — mount Leptos alongside the API
   routes; list pending Sets newest-first.
   - Submitting a set via CLI makes it appear in the list with
     title/project/branch/age
   - Answered sets do not appear
2. **Set view: render the ask** — Preface as markdown, Questions with
   labelled Options, ★ highlighted, free-text fields present.
   - A set using every grammar feature (options, sub-questions, mixed nodes)
     renders correctly
3. **Answer state + gated submit** — form state, completeness gate, submit
   posts the Response.
   - Submit stays disabled until every Question/Sub-question is addressed
   - Successful submit unblocks a genuinely waiting CLI and navigates back to
     the list with the set gone

## Re-verify at start

- Stage 01's crate layout: does the server crate already anticipate Leptos
  SSR (feature flags, workspace member for shared types), or does mounting it
  need restructuring?
- Markdown rendering choice in a Leptos SSR context (server-side render of
  the Preface vs client-side) — pick whichever avoids shipping a JS markdown
  parser.
- Exact Response completeness rules as implemented in stage 01 (free text
  only vs option required when options exist) — UI gate must match the server
  exactly.
