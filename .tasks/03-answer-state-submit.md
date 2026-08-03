# 03. Answer state + submit with unanswered warning

## What to build

Make the set view answerable end-to-end: form state for every Question and
Sub-question, and a submit that posts the Response, unblocks the genuinely
waiting CLI, and returns to the pending list.

The Response must match stage 01's explicitness rules exactly (they are
enforced server-side against the Set): every Question **and** Sub-question
appears in `answers` exactly once — either as an Answer (a `selected` Option
number and/or non-empty `free_text`) or as an explicit `unanswered: true`
marker; never both, never neither. The UI builds the marker entries itself
for anything the human left untouched.

Submit warns but never blocks: when Questions are Unanswered, a confirmation
(client-side dialog — we have hydration) lists them by name before the
Response goes out. Submitting with zero Answers plus only a set-level comment
is a legitimate counter-question flow, not an error — the same warning path
covers it.

The submit path must end in the same place stage 01's endpoint does:
validation against the Set, one-Response-per-Set (first one stands), and
waking the held long-poll waits via the server's broadcast channel — however
the UI reaches it (server function or the existing endpoint), a waiting CLI
must actually wake.

## Acceptance criteria

- [ ] Submitting with unanswered questions shows a warning naming each of
      them; confirming sends those as `unanswered: true`
- [ ] A zero-Answer submit with only a set-level comment round-trips to the
      CLI
- [ ] A successful submit unblocks a genuinely waiting CLI (real long-poll,
      not just a stored row) and navigates back to the pending list with the
      Set gone
- [ ] Submitting to an already-answered Set surfaces the conflict rather
      than silently discarding the Response
