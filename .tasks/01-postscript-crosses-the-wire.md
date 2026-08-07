# 01. The Postscript crosses the wire

## What to build

The Question Set grammar gains an optional `postscript` field: markdown the
agent closes the Set with, carried through the store and delivered to the
viewer as rendered HTML on the Set's view — alongside the Preface, which is
the model for everything about it. The Preface keeps its name (decided:
the pre-/post- pairing is enough, no rename).

A Set sent with a Postscript arrives in the view JSON with the Postscript
rendered; one sent without is byte-for-byte what it is today. Lockstep
upgrade is accepted: `deny_unknown_fields` stays, no version negotiation.

## Acceptance criteria

- [ ] A Set whose YAML carries `postscript` parses, validates, and round-trips
      to YAML with the field intact (block scalar for multi-line, like the
      Preface)
- [ ] An empty or whitespace-only Postscript comes out of the view as absent,
      exactly as the Preface does
- [ ] The Set view carries the Postscript as rendered, sanitized HTML, and the
      view's diagram flag is true when the Postscript alone holds a mermaid
      fence
- [ ] The generated TypeScript types include the new view field
- [ ] `examples/questions.yaml` exercises the field and the example-parsing
      test passes
