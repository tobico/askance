# API core + CLI

The agent-facing contract of Askance, working end-to-end before any UI exists:
an agent runs `askance ask questions.yaml` (or pipes YAML on stdin), the
Question Set lands in the server's SQLite store, a Response submitted via
`curl` is printed by the still-waiting CLI as YAML, and the CLI exits 0.

Decisions re-verified at planning (2026-08-03): axum 0.8.9 with Leptos 0.8.x
to come in stage 02 — `leptos_axum` is additive, so plain REST routes are
unconstrained; YAML via **serde-saphyr 1.0** (serde_yaml is archived and
serde_yml is RUSTSEC-flagged; serde-saphyr is pure Rust, actively maintained,
and emits multi-line strings as `|` block scalars); REST routes live under
`/api/v1/...` because Leptos server functions claim `/api/{fn_name}` by
default in stage 02; long-poll with client-side timeout+reconnect confirmed
over SSE (`tailscale serve` imposes no idle timeouts or buffering). Cargo
workspace with `schema` / `server` / `cli` crates, keeping `schema` free of
server-only deps so it stays WASM-safe for stage 02's hydrate build.

Roadmap stage: [01: API core + CLI](docs/roadmaps/v1/01-api-core-and-cli.md)

## Tasks

- [x] 01: Workspace, dev shell, and skeleton server — [details](01-workspace-and-skeleton-server.md)
- [x] 02: Submit a Question Set — [details](02-submit-question-set.md)
- [ ] 03: Answer and deliver — [details](03-answer-and-deliver.md)
- [ ] 04: `askance ask` CLI — [details](04-askance-ask-cli.md)
- [ ] 05: End-to-end example + quickstart — [details](05-example-and-quickstart.md)
