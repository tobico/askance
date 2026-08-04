# Markdown handling

Agents write markdown whether or not anyone asked them to, and the Set page
only renders it in the Preface. Question text, Sub-question text and Option
text reach the page as the agent's source, so a bulleted question arrives as a
run of hyphens and a quoted command arrives wearing its backticks. This feature
renders those three as markdown too — the whole of it in a Question, inline
markup only in an Option, where a block would break the radio's row apart.

The rendering stays on the server, as the Preface's and the Diff's already do:
the Set the browser receives stops carrying the schema's Questions and starts
carrying view types of the page's own, with the HTML already made. No markdown
parser reaches the wasm bundle.

Only what the agent writes is markdown. The human's free-text Answers and the
set-level comment stay plain, keeping the line breaks they were typed with; the
Set title and the pending and archive list rows stay plain too. The wire format
and `askance-schema` are untouched, and the waiting agent still receives raw
text.

Note for whoever merges `set-page-toc`: that branch builds its nav lines out of
`question.text`, which task 01 takes off the Set the page receives. The nav
wants a plain-text form of the Question text, and adding it belongs to that
merge rather than to these tasks.

## Tasks

- [ ] 01: A Question's text renders as markdown — [details](01-question-text-markdown.md)
- [ ] 02: An Option's text renders as inline markdown — [details](02-option-text-inline.md)
- [ ] 03: Rendered markdown is styled once, everywhere — [details](03-shared-markdown-styling.md)
- [ ] 04: The grammar says so — [details](04-document-the-grammar.md)
