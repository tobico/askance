# 01. Selected-row ring

## What to build

The Answer Table's selected row takes one 2px accent ring around the whole
row — top edge included — matching the selected Option card's treatment, while
the borders between its columns keep the normal edge colour. Today the accent
is painted per cell and the collapsed-border tie is lost on the top edge and
won in the wrong places between columns; the fix must not depend on winning
collapse ties. Applies alike to the answering sheet (the row with the checked
radio) and the archived record (the chosen row), keeping the accent wash.

## Acceptance criteria

- [ ] A selected row on the sheet shows the accent ring on all four sides,
      including the top, at the Option card's 2px weight
- [ ] Borders between columns inside the selected row stay the normal edge
      colour
- [ ] The record's chosen row renders the same ring
- [ ] The keyboard focus indicator remains visible and distinguishable
