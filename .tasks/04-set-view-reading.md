# 04. Set view as a record

## What to build

The Set page (`/sets/{id}`) in its reading half: title, project and branch,
the Preface fragment injected as received, every Question in order with its
Options, Sub-questions and Recommendation marks, and the Set's standing —
an answered or archived Set renders as the record of what was decided
(selected Options, free text, Unanswered marks). The answering form is the
later task; the Diff is its own task too.

Mermaid becomes an ordinary pnpm dependency, dynamically imported only when
the Set carries a Diagram, so diagram-free pages pay nothing; a Diagram that
cannot render degrades to its readable source, as the glossary promises.

## Acceptance criteria

- [ ] Fixtures exercising the full question grammar render correctly:
      Options with and without a Recommendation, mixed nodes carrying both
      Options and Sub-questions, bare questions with neither
- [ ] Answered and archived Sets render as records, including Unanswered
      marks and the set-level comment
- [ ] Mermaid is loaded only when the Set's Diagram flag says so, and
      failure degrades to the source block
- [ ] The reading-side behaviour assertions from the old set-page tests are
      ported to vitest and green
