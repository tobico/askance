# 01. Accept all recommendations

## What to build

A button on the set view that fills every *unanswered* question with its
Recommendation, so the common case — the human agrees with the agent — is one
tap plus submit.

It is explicit and it is never automatic: nothing is selected when the page
loads, because a sleepy thumb-tap must not approve decisions that were never
read. Pressing it changes the same fields the human could have clicked, so
every Answer it fills can still be changed before submit, and pressing submit
is still a separate act.

What counts as already answered: a question carrying a selected Option **or**
free text. An Answer is a selection and/or free text, so a question with words
in it and no Option selected is answered and accept-all leaves it alone.

What it skips: questions that offer no Recommendation (no Options at all, or
Options with no ★ among them) — there is nothing for it to fill. Both
Questions and Sub-questions take part, since both carry Options.

The button is absent — not disabled — on a Set where no question anywhere has a
Recommendation, because there is nothing it could ever do there.

Keep the decision of *which* questions get filled in a pure function over the
snapshot shape the submit path already uses, so it can be unit-tested without a
browser.

## Acceptance criteria

- [ ] On a Set with Recommendations, the button appears and fills every
      question that has a ★ and is currently unanswered
- [ ] A question the human already answered — by Option or by free text — is
      left exactly as it was
- [ ] A question with no Recommendation is left unanswered; the button does not
      fill it with an arbitrary Option
- [ ] Sub-questions are filled on the same terms as Questions
- [ ] A Set where nothing has a Recommendation renders no button (asserted on
      the server-rendered HTML)
- [ ] Individual Answers can still be changed after pressing it, and submit
      remains a separate action
- [ ] Unit tests cover: fills only the unanswered, skips the ★-less, and is
      idempotent when pressed twice
