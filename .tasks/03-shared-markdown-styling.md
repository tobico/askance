# 03. Rendered markdown is styled once, everywhere

## What to build

Rendered markdown gets one set of styles, shared by every place that shows it.
Today the Preface carries two rules of its own, for inline code and for a
scrolling `pre`, which means a heading, a table, a blockquote or a horizontal
rule in a Preface renders as whatever the reset left behind — and after task 01
the same is true inside a Question.

Style it once and have both use it, rather than growing a second copy of the
Preface's rules for Questions. The Preface should come out of this looking no
worse than it does now, and a heading inside a Question should not shout louder
than the page's own headings around it — a Question's text is prose in a form,
not a section of the document.

Both colour schemes, and phone width, are part of the job rather than a
follow-up: this is a phone-first UI, and the stylesheet already carries a dark
scheme the new rules have to hold up in.

## Acceptance criteria

- [ ] Headings, ordered and unordered lists, tables, blockquotes, horizontal
      rules, inline code and fenced code blocks all read as themselves in a
      Preface and in a Question alike
- [ ] A wide table or a long code line scrolls sideways within its own box
      rather than widening the page around it
- [ ] Readable in both the light and the dark scheme, using the existing
      variables rather than new fixed colours
- [ ] Holds together at phone width, in the answering form and on a settled Set
- [ ] A heading inside a Question sits below the page's own headings in weight,
      not above them
