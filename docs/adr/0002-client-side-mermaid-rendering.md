# Client-side mermaid.js as a narrow carve-out from the no-JS principle

The viewer renders all agent markdown on the server and ships no JS and no
markdown parser to the browser — a principle stated in the renderer, the
highlighter, and the README. Mermaid diagrams in Prefaces are worth having
(a Question about structure is grasped faster as a picture), but mermaid has
no pure-Rust implementation: it renders only in a browser. We vendor
mermaid's single-file build plus a small init script into `assets/` and load
them **only on pages whose rendered markdown contains a Diagram**, keeping
the principle intact for everything the server *can* render.

## Considered Options

- **Server-side rendering via headless browser** (mermaid-cli/puppeteer) —
  keeps the browser JS-free but drags Chromium into the Nix closure and the
  server runtime for a single-user tailnet service.
- **A pure-Rust-renderable diagram DSL** (svgbob, DOT) — keeps the principle
  absolute, but agents write mermaid natively and would produce it anyway.
- **Unconditional script load** — simpler, but every diagram-free page pays
  the ~2–3 MB bundle for nothing.

## Consequences

- The principle's honest statement becomes "no JS unless the server cannot
  do the work"; the carve-out is documented at the points it touches.
- Diagram source is agent-authored and untrusted, so rendering runs at
  mermaid's strict security level, and the sanitized `pre.mermaid` source
  block is the permanent fallback whenever the script is absent or the
  diagram invalid.
- A committed minified bundle lives in `assets/` (updated via
  `tools/update-mermaid.sh` at a pinned version), so a build still needs
  nothing but cargo.
