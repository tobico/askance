# 05. Diff viewer

## What to build

The Diff on the Set page: the server-rendered, syntax-highlighted HTML
injected into the page, with per-file folding and the table of contents —
jumping to a file unfolds it before landing, and scrolling tracks which
section the reader has reached. Behaviour parity with the Leptos viewer,
driven by the Diff view's paths-plus-HTML payload rather than by parsing
the markup.

## Acceptance criteria

- [ ] The Diff renders from the fragment HTML with highlighting intact, and
      files fold and unfold
- [ ] The ToC jumps to a file (unfolding it first) and tracks the reader's
      position on scroll, matching today's behaviour
- [ ] A Set without a Diff shows none of the Diff chrome
