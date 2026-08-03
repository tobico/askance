# 04. Answered Sets render read-only

## What to build

Opening a Set that already has a Response shows what was decided instead of a
form. This is the Archive's detail view, built first because the Archive is
nothing but a list pointing at it — and because it closes a real hole found
while re-grounding: today an answered Set still draws every radio and textarea,
and pressing submit gets "this Set had already been answered".

The read-only view keeps the Set's own material — title, provenance, Preface,
Diff, every Question and Sub-question in the order asked — and renders each
question's outcome beside it: the Option the human selected (marked as chosen,
alongside the agent's Recommendation), whatever they wrote, or the fact that the
question went back Unanswered. The set-level comment shows when there is one,
and the page says when the Response was submitted.

A Set answered with zero Answers and only a comment is a legitimate Response —
a counter-question — and has to read as one rather than as an empty page.

There is no submit, no accept-all and no draft interaction here: the Response is
delivered and a Set is answered once, so the page offers nothing to press. The
Set's existing draft, if one somehow survives, is not restored into it.

Which view the page shows is decided server-side from whether the Set has a
Response, so the browser never flashes a form for a Set that is already
answered.

## Acceptance criteria

- [ ] Opening an answered Set renders its Answers, not a form: no radios, no
      textareas, no submit button
- [ ] Each answered question shows its selected Option and/or free text; the
      selection is distinguishable from the agent's Recommendation
- [ ] Questions that went back Unanswered are shown as Unanswered rather than
      omitted or blank
- [ ] The set-level comment and the submission time are shown when present
- [ ] A Response with zero Answers and only a comment renders as a
      counter-question, not as an empty page
- [ ] An unanswered Set is unaffected — it still renders the answerable form
      exactly as before, and the existing set-view tests still pass
- [ ] Asserted on the server-rendered HTML for both an answered and an
      unanswered Set
