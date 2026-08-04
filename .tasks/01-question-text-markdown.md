# 01. A Question's text renders as markdown

## What to build

The text of a Question and of a Sub-question renders as markdown on the Set
page, in every standing — waiting, answered, and archived unanswered. An agent
that writes a question as a bulleted list, a fenced code block, a table or a
blockquote gets one; the `Qn` label it answers to still sits at the head of it.

This is the task that opens the seam. The Set the browser receives currently
carries the schema's Questions exactly as the agent sent them; it should instead
carry view types belonging to the page, with the markdown already rendered by
the server on the way out. That keeps the parser where the Preface's and the
Diff's already are, and out of the wasm bundle. The form is built from the
labels, the Option numbers and the Recommendation flags, so those travel on the
view types unchanged — a Sub-question still nests one level under its Question,
and still answers by its parent's label plus its letter.

Block markdown cannot live inside a `<p>`, so the element a Question's text sits
in has to stop being one. Keep the class it is found by, so the stylesheet and
the tests move with it rather than being rewritten around it.

Option text is untouched here — it stays plain until task 02.

## Acceptance criteria

- [ ] A Question or Sub-question written as a bulleted list, a fenced code
      block, or a GFM table renders as that, with its label still at its head
- [ ] Markdown that would act on the page is dropped — a script, an event
      handler, or a `javascript:` link in a Question cannot reach the browser,
      on the same terms the Preface is already held to
- [ ] The answering form is unchanged in behaviour: every Question and
      Sub-question gets its fields in Set order, Options still submit by
      number, the Recommendation is still marked and still not preselected,
      and the accept-all offer still appears exactly when a Recommendation
      exists
- [ ] An answered Set and an archived-unanswered one render their Questions the
      same way the form did
- [ ] No markdown parser is compiled into the browser half
- [ ] Rendered markdown in a Question is legible even before task 03 — a list
      indents, a code span is monospace, nothing shows raw markup
