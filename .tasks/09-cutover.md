# 09. Cutover

## What to build

The SPA becomes the served viewer. The flake builds the production assets
and embeds them in the server binary (rust-embed); axum serves them with the
SPA fallback for every non-API path, keeping today's caching policy — hashed
assets immutable for a year, HTML always revalidated. Then the demolition:
`crates/app` and `crates/frontend` deleted, cargo-leptos and the pinned wasm
toolchain dropped from the flake and workspace, the vendored mermaid bundle
and its update script removed. README and PLAN.md are updated to describe
the new shape and stop stating the no-JS principle, pointing at ADR-0003.

## Acceptance criteria

- [ ] `nix build` yields a single binary serving the SPA; the phone loads
      it over the tailnet with push still working
- [ ] No Leptos crates, cargo-leptos, wasm-bindgen pin, or vendored mermaid
      remain anywhere in the repo or the flake
- [ ] Hashed assets are cached immutable and HTML revalidated, matching
      today's policy
- [ ] README and PLAN.md describe the SPA architecture and reference
      ADR-0003; `cargo test`, vitest and `nix flake check` are all green
