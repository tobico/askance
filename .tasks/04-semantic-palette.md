# 04. Semantic palette

## What to build

Give gate diagrams a vocabulary for marking the delta without hardcoding
colours that clash with one scheme. The stylesheet defines three semantic
node classes an agent can put on Diagram nodes — `new`, `modified`,
`removed` — themed to match the Diff's own add/change/remove colours in both
light and dark. An agent tags nodes (e.g. `class parser new` in flowchart
syntax); the viewer keeps the colours coherent and visually rhyming with the
Diff below.

Document the whole capability in the README's wire-format section: that
```` ```mermaid ```` fences in Prefaces and Question text render as
Diagrams, that they degrade to source, and the three semantic classes with
a short example.

## Acceptance criteria

- [ ] Nodes tagged `new` / `modified` / `removed` take the matching Diff
      colours in both schemes
- [ ] Untagged nodes keep the base theme
- [ ] README wire-format section documents mermaid fences, the degrade
      behaviour, and the semantic classes with an example
