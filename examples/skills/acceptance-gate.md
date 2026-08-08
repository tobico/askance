# Acceptance gate

Finished work stops for a human yes. Before you commit, land a branch, push,
deploy — anything hard to take back — put the work to the human and wait for
approval.

## Workflow

1. **Stop at the gate.** Finish the work, then stop before the irreversible
   step. Not after it, and not while asking.
2. **Say what you did.** Files changed, behaviour delivered, tests added,
   acceptance criteria met — and what happens the moment they say yes. They are
   deciding without having watched you work, so your summary and the diff are
   all they have.
3. **Ask one question: proceed, or not.** A gate is a yes or a no, not a design
   discussion — anything else worth deciding was a question before the work
   started.
4. **Wait.** Do nothing further until an answer arrives, however long it takes.
5. **Proceed only on an explicit yes.** If changes are asked for, make them and
   put the same gate again.

## Fail closed

Only an explicit approval opens the gate. Everything else leaves it shut:

- **No answer to the gate itself** — shut, even where everything around it was
  answered.
- **A question back, or a partial answer** — a counter-question, not approval.
  Answer it, then put the same gate again.
- **Anything ambiguous** — shut. Ask again rather than resolving it in your own
  favour.

A gate that fails closed costs one more round trip. A gate that fails open
commits or ships work nobody approved.

## Don't route around it

If you cannot reach the human, say what failed and what is now waiting on them,
then stop. A gate you couldn't put is not a gate that was passed, and your own
recommendation is not their answer.
