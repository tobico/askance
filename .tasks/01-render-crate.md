# 01. Extract the render crate

## What to build

Move the server-side rendering of agent-supplied content — markdown to
sanitized HTML, Diff syntax highlighting, and the building of the view types
the pages consume — out of the Leptos app crate into a new `askance-render`
crate (`crates/render`), together with its unit tests. The Leptos app then
depends on the new crate and keeps serving byte-identical output. This is
the piece of the old viewer that survives the rewrite: the JSON API task
builds on it without touching anything Leptos.

## Acceptance criteria

- [ ] `crates/render` owns markdown rendering, highlighting, and diff view
      building, with their existing unit tests moved along and green
- [ ] The Leptos app consumes the new crate; `cargo test` is green across
      the workspace
- [ ] The served UI is unchanged — same rendered HTML as before the move
