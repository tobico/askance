# 02. An Option's text renders as inline markdown

## What to build

An Option's text renders with inline markup — emphasis, code spans, links,
strikethrough — and nothing that would break the row it sits in. An Option is
one line beside a radio, its number and possibly its ★, and the whole row is the
tap target; a paragraph or a list emitted inside that label would split it.

So the markdown renderer needs a second way of being asked: render this as
inline content only, with the wrapping paragraph gone and block structure
flattened rather than emitted. Sanitized on exactly the same terms as everything
else — an Option is agent-supplied text like any other.

Both views draw it: the radio row on a Set still waiting, and the read-back row
on a settled Set, where "chosen" and the ★ still have to read beside the Option
they belong to.

## Acceptance criteria

- [ ] An Option quoting a command in backticks renders it as a code span, and
      tapping anywhere on the row still selects that Option
- [ ] Emphasis, links and strikethrough render; a link in an Option is
      sanitized like any other, and a `javascript:` one is dropped
- [ ] An Option whose text contains block markdown — a list, a heading, a
      fenced block — renders as a single row with the markup flattened, not as
      a broken-apart label
- [ ] On a settled Set every Option still reads with its number, its ★ where the
      agent recommended it, and "chosen" where the human picked it
- [ ] Submitting is unaffected: Options are still answered by number
