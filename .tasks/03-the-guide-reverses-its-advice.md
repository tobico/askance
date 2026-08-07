# 03. The Guide reverses its advice

## What to build

The core Guide Topic stops teaching the trailing catch-all Question and
teaches the Postscript instead. This is a reversal of standing advice: the
current text says a trailing open Question "often saves a whole round trip"
and its worked example ends with `Q12 — Anything worth knowing before this
starts?` — exactly the pattern the Postscript replaces.

The new stance: a catch-all "anything else" must never be a Question;
suggested discussion topics go in the Postscript, and the set-level comment
is where "anything else" lands. A trailing open Question stays legitimate
only when it asks for something specific. Reading the Response, an absent
`comment` means the human had nothing to add — never that they skipped it.

The README's grammar section gains the field too. `gates.md` is untouched
(decided: the Postscript is orthogonal to gates).

## Acceptance criteria

- [ ] The worked example drops the catch-all `Q12` and shows `postscript:`;
      the Set-shape summary line lists the field
- [ ] The Guide states the new stance on catch-all Questions and the meaning
      of an absent `comment`, and no passage still recommends the trailing
      catch-all
- [ ] The README's grammar documentation includes `postscript`
- [ ] Guide tests pass against the new text
