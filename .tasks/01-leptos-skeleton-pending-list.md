# 01. Leptos SSR skeleton + pending list

## What to build

Mount Leptos — SSR with full cargo-leptos hydration — alongside the existing
agent API routes, so one binary serves both, and deliver the first page: the
pending list. Pending means Sets without a Response, newest first, each row
showing title, project, branch, and age. (Liveness badge is stage 03.)

This is the restructuring slice. Stage 01 left the server as plain axum with
no Leptos anticipation beyond keeping REST under `/api/v1/` (clear of
`/api/{fn_name}`, which Leptos server functions claim). Expect: the canonical
Leptos app layout (a shared UI crate compiled natively for SSR and to wasm
for hydration), cargo-leptos driving the build, and the wasm32 target plus
tooling added to the flake dev shell. The store also gains its first
list-shaped query — the pending list can be drawn from the lifted columns
(title, project, branch, created_at) without deserializing Set bodies.

Age is displayed relative (from the Set's `created_at`, which is RFC 3339).
Layout is responsive from the first page — phone-first, per the stage brief.

## Acceptance criteria

- [ ] Submitting a Set via the CLI makes it appear in the pending list with
      title, project, branch, and age
- [ ] Pending Sets are ordered newest-first
- [ ] Answered Sets do not appear in the list
- [ ] The agent API routes still pass stage 01's tests unchanged
- [ ] `cargo leptos` (or the equivalent wired build) works inside the nix
      dev shell
