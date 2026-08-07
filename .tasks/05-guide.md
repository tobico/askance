# 05. The Guide teaches the declaration

## What to build

The Guide's core is the only place an agent discovers the Answer Table, so
it is amended — about ten lines, no new Topic:

- The Set-shape line of the CLI contract gains the new fields, so the shape
  an agent serializes is complete at a glance.
- The existing "prefer a comparison table" bullet becomes the declaration's
  own: state that a question's `columns` and its Options' `cells` declare
  the table, that the Option's `text` is the row's leading cell, and show
  a compact YAML example — compact enough that the core's reading cost
  barely moves.

The wording should make the old pattern quietly obsolete: an agent who
would have written a markdown table into the Question's text and echoed it
as a list is steered to the declaration instead.

## Acceptance criteria

- [ ] `askance guide` (the core) shows the amended Set-shape line and the
      declaration example, and no other Topic is added
- [ ] The example round-trips: pasted into a Set, it produces an Answer
      Table
- [ ] Existing guide-related tests (if any) pass with the amendment
