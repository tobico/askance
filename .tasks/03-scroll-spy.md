# 03. Scroll-spy highlight

## What to build

The sidebar entry for the section under the reader's eyes stays highlighted
as they scroll. An IntersectionObserver in the hydrate half watches the
anchored sections and moves a highlight class through the nav; the
server-rendered page carries no highlight (or the first entry's, so the top
of the page reads correctly before hydration), and the spy takes over when
the wasm arrives.

At the very top of the page the first entry is the highlighted one — the top
counts as being in the first section. Exactly one entry is highlighted at a
time; a file entry highlighting also reads as being within the Diff, and a
Question entry within the Questions, without both levels fighting for the
highlight. Scrolling never touches the URL.

## Acceptance criteria

- [ ] Scrolling a long Set moves the highlight through Preface, each file,
      each Question in page order, and back again scrolling up
- [ ] At page load, before any scrolling, the first entry is highlighted
- [ ] Jumping via a click leaves the clicked entry highlighted once the
      scroll settles
- [ ] The URL is untouched by scrolling; reduced-motion users still get a
      correct highlight
