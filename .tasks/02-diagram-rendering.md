# 02. Diagram rendering

## What to build

Turn the fallback block into a rendered Diagram, paying the JS cost only
where a Diagram exists.

Vendor mermaid's single-file build (the ESM distribution is code-split into
dozens of chunks, so the single-file IIFE build is the vendoring target) into
`assets/` as a committed file, following the committed-icons precedent — a
build needs nothing but cargo. Add `tools/update-mermaid.sh` that fetches a
pinned version from npm, drops it into `assets/`, and records the version so
updates are one command.

The server already renders the markdown, so it knows whether the page has a
Diagram: detect that while loading a Set (Preface and all Question texts) and
emit the script tags in the SSR HTML only then — the browser starts fetching
immediately, and diagram-free pages ship zero JS, keeping the carve-out from
ADR 0002 as narrow as the ADR promises.

A small hand-written init script (second vendored asset, ours) drives
mermaid once loaded: find the `pre.mermaid` nodes, render each with
`securityLevel: 'strict'` (the source is agent-authored and untrusted), and
swap in the SVG. A diagram that fails to parse or render keeps its source
block untouched — the task-01 fallback is the error state, silently.

## Acceptance criteria

- [ ] A Set whose Preface has a valid mermaid fence shows a rendered SVG
      diagram in the viewer
- [ ] A Set with no Diagram serves a page with no mermaid script tags
- [ ] An invalid mermaid fence still shows its readable source block, with
      no error artefacts injected into the page
- [ ] Rendering runs at mermaid's strict security level
- [ ] `tools/update-mermaid.sh` reproduces the committed bundle at the
      pinned version
