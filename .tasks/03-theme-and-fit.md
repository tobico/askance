# 03. Theme and fit

## What to build

Make Diagrams belong to the page in both colour schemes and on a phone.

Drive mermaid's `base` theme from the stylesheet's existing CSS variables at
render time, so diagrams sit on the same palette as everything else in both
light and dark. The UI themes purely by `prefers-color-scheme`, and mermaid
picks its theme at init — so when the scheme flips mid-session, re-render
the Diagrams from their kept source rather than leaving stale colours.

Rendered SVGs scale to the container width rather than overflowing or
scrolling: at-a-glance means seeing the whole shape at once, and the
pressure to keep diagrams small enough to stay legible lands on the
authoring guidance (task 05), not on viewer chrome.

## Acceptance criteria

- [ ] A Diagram is legible and palette-coherent in light and in dark
- [ ] Flipping the OS colour scheme while the page is open re-renders
      Diagrams to match
- [ ] A wide Diagram scales down to fit the viewport width on a phone, with
      no horizontal page scroll
- [ ] Reduced-motion preferences stay respected
