# Answer Table and per-block width

A Question's Options can be declared with tabular data — `columns` naming the
trade-off axes on the Question or Sub-question, `cells` on each Option — and
the viewer then draws the table itself as the selectable Options: radio and
number in the first column, the Option's `text` as the row's leading content
cell, the whole row as the tap target. The old pattern of a comparison table
in the Question's text with a radio list repeating it below disappears.

Alongside it, content width becomes per-block instead of per-section: at the
wide breakpoint `main` itself takes the wide column the Diff used to break
out into alone, section cards span it fully, prose inside pulls back to the
reading measure, and tables, Diagrams and code fences get past the measure.

## Tasks

- [x] 01: Answer Table on the sheet, end to end — [details](01-answer-table-sheet.md)
- [x] 02: The record reads the same table — [details](02-record-view.md)
- [x] 03: Malformed declarations refuse the Set — [details](03-refuse-malformed.md)
- [ ] 04: Per-block width — [details](04-per-block-width.md)
- [ ] 05: The Guide teaches the declaration — [details](05-guide.md)
