# 02. The viewer draws it

## What to build

The viewer renders the Postscript directly above the set-level comment box,
wherever that box (or the submitted comment) is shown: the answering sheet,
and the answered and archived-unanswered views. It is rendered markdown like
the Preface — mermaid fences draw as Diagrams — but it introduces the
comment rather than opening the page.

The comment box itself is untouched: its "Other comments" placeholder and
aria-label stay exactly as they are, with or without a Postscript above
them. A Set without a Postscript renders exactly as today.

## Acceptance criteria

- [ ] On the answering sheet, a Set with a Postscript shows it as rendered
      markdown immediately above the comment textarea; the placeholder and
      aria-label are unchanged
- [ ] On an answered Set's page, the Postscript appears above the submitted
      comment (and above the spot where the comment would be, when the human
      sent none)
- [ ] A Set without a Postscript draws no extra section anywhere
- [ ] Web tests cover the sheet and the answered view, with and without a
      Postscript
