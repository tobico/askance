# 01. Mermaid fence fallback

## What to build

Teach the server-side markdown renderer to divert a fenced code block whose
info string is `mermaid` into the shape the client-side renderer will later
look for: a `pre` carrying exactly the class `mermaid`, containing the
escaped diagram source. The sanitizer must let that one class through on
`pre` — and nothing wider; an agent writing any other class of its own still
loses it. This applies everywhere full-block markdown renders (Preface and
Question text), because it's the one shared pipeline.

No JS yet: what ships here is the *fallback* — the readable source block a
human sees with JS off, before the script loads, or when a Diagram fails to
render. It should read as well as any unlanguaged code fence does today.

## Acceptance criteria

- [ ] A ```` ```mermaid ```` fence in a Preface or Question renders as
      `<pre class="mermaid">` with the source escaped and intact
- [ ] The sanitizer admits only the `mermaid` class on `pre`; agent-written
      classes elsewhere are still stripped (existing tests keep passing)
- [ ] Script/HTML smuggled inside a mermaid fence reaches the page as
      escaped text, covered by a test
- [ ] The block is styled like other code blocks, so the no-JS view is
      readable
