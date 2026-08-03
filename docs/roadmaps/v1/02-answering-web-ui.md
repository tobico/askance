# 02. Answering web UI

## Goal

Open the server in a browser, see the pending Question Sets, open one, read
its Preface, review the attached Diff, answer the Questions, and submit —
unblocking the waiting CLI. Replaces the curl round trip from stage 01 as
the human's interface.

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
- **Submit warns on Unanswered, never blocks** — a confirmation lists the
  Unanswered questions before the Response goes out; submitting with zero
  Answers plus a set-level comment is a legitimate counter-question flow.
  Unanswered questions are sent as explicit `unanswered: true` markers,
  matching the stage 01 server contract.
- **Diff viewer in the set view** — renders the attached Diff (per-file,
  syntax-aware where cheap) so code-approval questions are reviewable without
  leaving the page. Sets without a Diff simply omit the section.
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
3. **Answer state + submit with unanswered warning** — form state; submit
   posts the Response, confirming first when questions are Unanswered.
   - Submitting with unanswered questions shows the warning naming them;
     confirming sends them as `unanswered: true`
   - A zero-Answer submit with only a set-level comment round-trips to the CLI
   - Successful submit unblocks a genuinely waiting CLI and navigates back to
     the list with the set gone
4. **Diff viewer** — render the attached Diff in the set view.
   - A set with a Diff spanning modified + untracked files renders per-file;
     a set without one shows no diff section

## Re-verify at start

- Stage 01's crate layout: does the server crate already anticipate Leptos
  SSR (feature flags, workspace member for shared types), or does mounting it
  need restructuring?
- Markdown rendering choice in a Leptos SSR context (server-side render of
  the Preface vs client-side) — pick whichever avoids shipping a JS markdown
  parser.
- Exact Response explicitness rules as implemented in stage 01 (what counts
  as an Answer vs `unanswered: true`) — the UI warning and payload must match
  the server exactly.
- Diff storage shape from stage 01 (raw unified diff vs structured) — pick
  the viewer approach accordingly.
