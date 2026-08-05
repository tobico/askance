# Solid viewer

Rewrite the viewer as a SolidJS SPA (TypeScript, vite, pnpm, TanStack Query)
served from the same single axum binary, replacing the Leptos SSR/hydration
UI. The driver is runtime cost on the phone — the wasm bundle plus hydration
— and the rewrite retires the no-JS principle that made client-side mermaid
a carve-out. Rendering of agent-supplied content stays server-side in Rust;
the viewer talks to a private `/api/ui/` JSON namespace; the agent contract
under `/api/v1/` and the CLI do not change. Strict feature parity: nothing
added, dropped, or redesigned. Decisions recorded in
[ADR-0003](../docs/adr/0003-solid-spa-viewer.md).

Test-led throughout: each task ports or writes its tests red before making
them green.

## Tasks

- [x] 01: Extract the render crate — [details](01-render-crate.md)
- [x] 02: The viewer's JSON API — [details](02-ui-api.md)
- [x] 03: Web scaffold and walking skeleton — [details](03-web-scaffold.md)
- [x] 04: Set view as a record — [details](04-set-view-reading.md)
- [ ] 05: Diff viewer — [details](05-diff-viewer.md)
- [ ] 06: Set view answering — [details](06-set-view-answering.md)
- [ ] 07: Archive and Liveness — [details](07-archive-and-liveness.md)
- [ ] 08: PWA and push — [details](08-pwa-and-push.md)
- [ ] 09: Cutover — [details](09-cutover.md)
