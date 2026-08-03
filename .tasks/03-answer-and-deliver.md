# 03. Answer and deliver

## What to build

The Response comes back and reaches the waiter. Two endpoints:

**Submission** — accepts a Response (YAML) for a Set: per Question and
Sub-question either an Answer (`selected` and/or `free_text`) or an explicit
`unanswered: true`, plus an optional set-level `comment`. The invariant is
explicitness, not completeness: every question in the Set must appear one
way or the other. A Response omitting any question is rejected naming the
missing label; a Response with zero Answers plus a comment is valid (the
counter-question case). A Set that already has a Response cannot receive a
second one. Response types and this validation live in `schema` (the CLI
prints Responses in task 04, and stage 02's UI builds them).

**Wait (long-poll)** — a request that returns the Response for a Set. If the
Response already exists it returns immediately; otherwise the connection is
held until submission or a server-side hold window elapses (a bounded hold
that tells the client "nothing yet, poll again" — the client owns retry
per ADR-0001; there is no expiry, waiting is indefinite across polls).

## Acceptance criteria

- [ ] A Response omitting one of the Set's Questions is rejected, naming the
      missing label; nothing is stored
- [ ] A Response marking Questions `unanswered: true` is accepted, including
      the zero-Answers-plus-comment case
- [ ] A long-poll opened before submission receives the Response when it is
      submitted; one opened after receives it immediately
- [ ] A long-poll on an unanswered Set returns a "nothing yet" indication
      after the hold window rather than hanging forever
- [ ] A second Response for the same Set is rejected
