# 01. Answer Table on the sheet, end to end

## What to build

A Set whose Question (or Sub-question) declares `columns` — a list of axis
headers, inline markdown — and whose Options each carry `cells` — one cell
per axis, inline markdown — is drawn on the answering sheet with the table
itself as the selectable Options, in the place the radio list draws today.
The presence of `columns` is what makes an Answer Table; a question without
it is untouched, and the radio list stays exactly as it is for those.

The declared data travels the same road as everything else the agent writes:
the schema accepts the new fields from the Set YAML, the server renders each
column header and cell to inline HTML (the same flattening an Option's text
gets), and the view the browser receives carries them structured — no table
parsing or DOM surgery on the client.

The table's shape, row by row:

- First column: the radio and the Option's number, with an empty header.
- Second column: the Option's `text`, headed by the fixed word **Option**.
- Then one column per declared axis, headed by the agent's words.
- A trailing ★ column, drawn only when the question carries a
  Recommendation, with an empty header; ★ on the recommended row.

Interaction is exactly the list's: the whole row is the tap target, a tap
selects, a tap on the selected row clears, arrow keys move the selection,
and the selected row takes the accent treatment the selected Option card
takes today. The Option's `text` is the radio's accessible name. The
free-text field below the question is unchanged, and the Response a
table-mode question produces is indistinguishable from a list-mode one —
draft persistence included.

## Acceptance criteria

- [ ] A Set YAML declaring `columns` and `cells` round-trips schema →
      rendered view → sheet, and the question draws as an Answer Table; the
      same Set without `columns` draws today's radio list
- [ ] Selecting a row, clearing it, and submitting produce the same Response
      entries as the equivalent list-mode question, and a draft restores the
      selection
- [ ] The ★ column appears only when some Option is recommended; header
      cells are as specified (empty, Option, axes, empty)
- [ ] A Sub-question with `columns` gets the same table; markdown in headers
      and cells renders inline (code spans survive, blocks flatten)
- [ ] The radio's accessible name is the Option's `text`
