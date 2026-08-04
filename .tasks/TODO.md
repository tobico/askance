# Set-page table of contents

A table of contents for the Set page, in every standing — waiting, answered,
and archived unanswered — and always present, however small the Set. On wide
viewports it sits sticky in the left margin, mirroring the page top to bottom:
the Preface, the Diff with one nested entry per file, and the Questions with
one nested entry per top-level Question. Clicking an entry jumps to that part
of the page (smoothly, unfolding a folded Diff file first, and recording the
position in the URL hash via replaceState); the highlighted entry follows the
scroll. Below the sidebar breakpoint, a bar sticky from load names the current
section and opens the full list on tap.

The page itself gains quiet h2 headings for the Preface and the Questions
(styled like the Diff's existing one), so a jump lands somewhere visibly
named, and id anchors on every section, file, and Question — which also makes
hash deep-links work server-rendered, before any script.

## Tasks

- [ ] 01: Anchors and section headings — [details](01-anchors-and-headings.md)
- [ ] 02: Desktop sidebar with click-to-jump — [details](02-desktop-sidebar.md)
- [ ] 03: Scroll-spy highlight — [details](03-scroll-spy.md)
- [ ] 04: Mobile bar with dropdown — [details](04-mobile-bar.md)
