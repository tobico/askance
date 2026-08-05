# 02. The viewer's JSON API

## What to build

A private `/api/ui/` namespace on the axum server: eight endpoints
mirroring the Leptos server functions — pending list, archive list, load
Set, submit Response, archive Set, push public key, subscribe, unsubscribe —
reusing the existing serde view types verbatim, so the JSON carries the same
server-rendered HTML fragments (Preface, Diff) the Leptos UI ships today.
The agent contract under `/api/v1/` is untouched.

Test-led: port the *content* assertions from the SSR page tests (rendered
markdown, Diff highlighting, Diagram detection, standing) into Rust API
tests first, red, then make the endpoints green. The API tests write golden
fixture JSON to committed files — the vitest suite consumes those same files
later. ts-rs generates TypeScript types from every payload type so the wire
shape has one source of truth.

## Acceptance criteria

- [ ] All eight endpoints respond under `/api/ui/` with the view-type JSON;
      `/api/v1/` is byte-for-byte untouched
- [ ] The content assertions from the SSR page tests are ported to API tests
      and green
- [ ] Golden fixture JSON, written by the API tests, is committed
- [ ] ts-rs emits TypeScript types for every payload type as part of the build
- [ ] Submitting a Response through the new endpoint settles a waiting agent,
      same as the server-function path it replaces
