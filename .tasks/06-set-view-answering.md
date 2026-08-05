# 06. Set view answering

## What to build

The answering half of the Set page: selecting Options on Questions and
Sub-questions, free text per Question, the set-level comment, and Unanswered
as the explicit state of anything skipped. Drafts persist in localStorage
keyed per Set, surviving reload and clearing on submit. Submitting delivers
the Response through the `/api/ui/` mutation and settles the waiting agent;
archiving an orphaned Set settles it without a Response. Mutations go
through TanStack Query like everything else.

## Acceptance criteria

- [ ] A full-grammar Set answered in the browser reaches a waiting
      `askance ask` process as the same Response YAML as today
- [ ] Drafts survive a reload and are gone after submit
- [ ] Skipped Questions arrive as Unanswered, and the set-level comment
      travels
- [ ] Archiving settles the Set as archived
- [ ] The form behaviour assertions from the old set-page tests are ported
      to vitest and green
