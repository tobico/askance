# Mermaid diagrams

Prefaces should be graspable at a glance. Agents get a way to say the
structural part of a Preface as a picture: a ```` ```mermaid ```` fence
renders as a Diagram in the viewer, degrading to its readable source wherever
it cannot render. The rendering is client-side mermaid.js — a deliberate,
narrow carve-out from the no-JS-in-the-browser principle (ADR 0002) — vendored
into `assets/` and loaded only on pages that actually contain a Diagram. The
agent-side half amends the question grammar (canonical in tobico-skills) so
agents reach for diagrams, and glance-able structure generally, when authoring
Sets — with a stronger push at code approval gates, where an architectural
picture of the delta pays off most.

## Tasks

- [ ] 01: Mermaid fence fallback — [details](01-mermaid-fence-fallback.md)
- [ ] 02: Diagram rendering — [details](02-diagram-rendering.md)
- [ ] 03: Theme and fit — [details](03-theme-and-fit.md)
- [ ] 04: Semantic palette — [details](04-semantic-palette.md)
- [ ] 05: Grammar guidance — [details](05-grammar-guidance.md)
