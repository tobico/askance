# 04. Per-block width

## What to build

Content width becomes a property of the block rather than of the section.
At the big-window breakpoint where the Diff alone breaks out today, the
page's main column itself takes that wide width, and the Diff's private
breakout — its own width variable, the negative-margin centering, the nav
measured off the Diff's edge — retires in favor of the one wide column.

Inside the wide column:

- The section cards — Preface body, Diff files, Question cards, the
  Postscript card — span the full wide width, borders and all.
- Prose blocks inside them pull back to the reading measure, flush left:
  paragraphs, lists, block quotes, headings.
- Free-text fields, the set-level comment box and the submit button stay at
  the measure, aligned with the prose beside them.
- Tables — the Answer Table included — take fit-content width: only what
  their columns want, up to the card, scrolling internally past that as
  they do today.
- Diagrams and code fences run the card's full width.

Below the breakpoint nothing changes anywhere: the prose column, the
sidebar arithmetic and the phone layout are untouched. The Diff keeps its
own guarantee — a line written to the code's width is still not cut off —
whatever the mechanism becomes; the nav keeps clear of the wide column the
way it keeps clear of the Diff today.

## Acceptance criteria

- [ ] At the wide breakpoint every section card spans the same wide column
      the Diff used to take alone, and the Diff's separate breakout rules
      are gone
- [ ] Prose inside wide cards reads at the measure, flush left; fields,
      comment box and submit sit at the measure
- [ ] A wide markdown table grows past the measure only as far as its
      content needs; Diagrams and fences span the card
- [ ] Below the breakpoint the rendered page is unchanged, and the nav
      never collides with the wide column above it
