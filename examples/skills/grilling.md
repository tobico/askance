# Grilling

Before building anything with a design worth arguing about, interview the human
about the plan until you both understand it the same way. A plan is a paragraph
and the thing built from it is a week, so this is the cheapest moment for a
decision to change.

## Workflow

1. **Map the decision tree.** Every choice the plan rests on, and which of them
   depend on which.
2. **Ask a branch at a time.** Put every question that doesn't depend on an
   answer you're still waiting for. A question whose wording would change with
   an answer you asked for alongside it waits for the next round.
3. **Recommend an answer to every question.** "I'd do this, because…" is easier
   to reply to than a neutral menu, and they can still say no.
4. **Wait for the answers before going deeper.** Their answers decide which
   branch is worth walking down next.
5. **Repeat until nothing load-bearing is open**, then state the agreed plan
   back and build it.

## Rules

- **Ask only about what the human can settle and you can't**: trade-offs decided
  by taste, product or cost; assumptions nothing in the repository confirms;
  scope; facts from outside the codebase.
- **If a question can be answered by exploring the codebase, explore the
  codebase instead.** A question you were supposed to answer yourself spends
  their attention on your work.
- **Batch generously.** Each round trip is expensive and they answer the whole
  batch in one sitting, so a question held back for no reason costs another
  round.
- **A question left unanswered is still open.** Silence, or a reply that skips
  it, is never agreement with your recommendation — ask it again.
- **Don't build while the answers are outstanding.** Do work the answers cannot
  invalidate, and start nothing they might throw away.
