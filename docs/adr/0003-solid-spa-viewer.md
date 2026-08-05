# Solid SPA viewer over a JSON API, replacing the Leptos SSR/hydration UI

The Leptos viewer made the phone pay for the architecture: a multi-megabyte
wasm bundle (6.5 MB dev, ~1 MB release) downloaded and hydrated before the
page was interactive, on the device the whole tool exists to serve. We
rewrite the viewer as a SolidJS SPA — TypeScript, vite, pnpm, TanStack Query
as the data layer — built to static assets, embedded in the same single axum
binary (rust-embed), and served at the same URLs. The eight Leptos server
functions become a private `/api/ui/` JSON namespace, kept apart from the
versioned agent contract under `/api/v1/`, which does not change at all.

Rendering of agent-supplied content stays server-side in Rust: the API ships
sanitized HTML fragments (pulldown-cmark + ammonia for markdown, syntect +
two-face for the Diff), so the browser never parses untrusted markdown and
the sanitization story lives in one place. ts-rs generates the TypeScript
types from the Rust view types, and golden fixtures written by the Rust API
tests feed the vitest component suite, so the wire shape cannot drift
silently between the two languages.

Supersedes [ADR-0002](0002-client-side-mermaid-rendering.md): the viewer now
requires JavaScript, so the no-JS principle that made client-side mermaid a
carve-out is retired. Mermaid becomes an ordinary pnpm dependency,
dynamically imported only on pages whose Set carries a Diagram; the vendored
bundle and its update script go away. A Diagram still degrades to its
readable source when rendering fails.

## Considered Options

- **Stay on Leptos and optimize** (islands, smaller wasm) — keeps one
  language, but hydration cost is structural to the wasm-SSR pairing, and
  the dev experience was not the pain (the phone was).
- **SolidStart SSR** — richer framework, but drags a Node runtime into a
  deploy that is deliberately one Rust binary on a tailnet.
- **Axum-rendered HTML + Solid islands** — preserves a no-JS reading path,
  but splits every page across two rendering systems for a viewer whose one
  user always has JS.
- **Client-side markdown/diff rendering** (marked + DOMPurify + shiki) —
  the conventional SPA shape, but re-solves sanitization of untrusted agent
  content in a second language for no benefit.

## Consequences

- The no-JS degradation story is gone: the SPA is the only viewer. The
  server-rendered fragments keep first-paint cheap, but nothing renders
  without JS.
- Two toolchains: the Nix flake builds the pnpm/vite frontend and embeds it;
  cargo-leptos and the wasm toolchain leave the build at cutover, along with
  `crates/app` and `crates/frontend`. The server-side renderers move to a
  new `crates/render`.
- The SSR page tests' assertions split by responsibility: content assertions
  (markdown, Diff highlighting, Diagram detection) become Rust tests against
  the JSON API; page behaviour becomes vitest component tests.
