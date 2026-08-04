# 04. Mobile bar with dropdown

## What to build

The ToC below the sidebar breakpoint — phones, and narrow desktop windows. A
bar sticky from the moment the page loads names the current section, driven
by the same scroll-spy as the sidebar: the first entry's name at page top,
then whichever section the reader is in. Tapping the bar drops down the full
ToC — the same entries, nesting, and truncation as the sidebar; tapping an
entry jumps (same behaviour: smooth scroll, unfold a folded file, hash via
replaceState) and closes the list; tapping outside it closes without
jumping.

Jumped-to sections must land clear of the bar rather than underneath it
(scroll-margin on the anchors), and this must not disturb the wide layout,
where no bar exists. The bar and the sidebar are exclusive: exactly one of
the two is present at any viewport width, on every standing.

## Acceptance criteria

- [ ] On a narrow viewport the bar is sticky from load, shows the first
      entry's name at the top, and follows the scroll thereafter
- [ ] Tapping the bar opens the full list; tapping an entry jumps, updates
      the hash, and closes; tapping away closes without moving
- [ ] A jump lands the section heading visibly below the bar, not hidden
      behind it; on wide viewports nothing changed
- [ ] Exactly one of bar and sidebar is present at any width, on waiting,
      answered, and archived-unanswered Sets alike
