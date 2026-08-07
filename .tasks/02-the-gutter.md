# 02. The Gutter

## What to build

At the wide window (80rem and up), every Set-page section reserves the Gutter:
one shared width for the whole page, sized by its widest resident — the Diff's
line-number and marker columns. Content — prose, fields, whole Option cards —
starts at the content edge, the Gutter's right edge; wide markdown blocks
(tables, fences, Diagrams) bleed from the card edge; section headings keep the
column edge.

Question labels leave their inline float and hang in the Gutter beside the
question's first line. Sub-question labels stay inline with their text, and
the Sub-question rule and indent stay inside the content area.

The Answer Table's frame starts at the card edge with its radio-and-number
column sized to the Gutter, so Option text starts at the content edge — on the
sheet and the record — and the ring from task 01 still wraps the whole row,
that column included. The Diff's code consequently starts at the content edge
too.

The Gutter exists only on the Set page and only at the wide window; narrower
windows keep today's layout, inline label float included. The term is defined
in CONTEXT.md.

## Acceptance criteria

- [ ] At 80rem+, prose, question text, Option text and the Diff's code share
      one left axis at the content edge
- [ ] Question labels, the Answer Table's radio-and-number column and the
      Diff's line numbers occupy the Gutter at one shared width
- [ ] Wide markdown blocks and framed blocks start at the card edge; section
      headings keep the column edge
- [ ] Sub-questions, the list pages, and every window below 80rem are
      unchanged
