# 04. The grammar says so

## What to build

An agent writing a Question Set has to be able to tell from the documentation
what it may write in markdown and what it may not. The README's Question Set
grammar marks the Preface as markdown and says nothing about the rest, which was
true before this feature and is not now.

Say which fields are markdown, and how much of it each gets: the whole of it in
the Preface and in Question and Sub-question text, inline markup only in an
Option. Say the other half too — that the human's own words come back as plain
text, and that the Set title and the list rows are plain — because a fact the
documentation leaves out is one an agent will guess at.

`CONTEXT.md` defines Preface, Question and Option; the Preface's entry already
says markdown, and the other two now want a word about it where the terms are
defined.

## Acceptance criteria

- [ ] The README's Question Set grammar states which fields are markdown and
      the block-versus-inline distinction between a Question and an Option
- [ ] It also says what is not markdown — the Set title, and the human's own
      words coming back in a Response
- [ ] `CONTEXT.md`'s Question and Option entries say it where the terms are
      defined, in the register the rest of the file is written in
- [ ] Nothing in the documentation claims a behaviour the code does not have
