# 06. Manual archive of an orphaned Set

## What to build

The human can archive a Set that will never be answered — the agent died, the
work moved on — so the pending list stays a list of things actually waiting on
them. Only a human may do this: a disconnected agent is never enough, because
the CLI reconnects through transient drops (ADR-0001).

Archiving is offered on the set view of a pending Set, beside its Liveness, so
the decision is made with the Set's own Questions and the "agent disconnected"
badge in front of the human rather than from a list row. It goes through a
confirmation — this closes the Set for good, and it is the one action here that
cannot be taken back.

**What archiving means.** An archived Set leaves the pending list and appears in
the Archive, visibly distinguished from an answered one: it carries no Response,
and the Archive must say that it was archived unanswered rather than showing it
as a decision that was made. Its detail view is read-only like an answered Set's
— the Questions and the Preface stay readable forever — and it offers no way to
answer it.

**What the waiting agent is told.** Archiving is the human declaring the Set
closed, so a CLI that is somehow still holding a wait on it is told rather than
left polling a Set nobody will ever answer: the wait endpoint answers `410 Gone`
for an archived, unanswered Set, and the CLI treats 410 as fatal — it prints that
the Set was archived unanswered and exits, instead of retrying it as a transient
failure the way it treats other unexpected statuses today. Submitting a Response
to an archived Set is refused on the same grounds, through the one submit path
both the browser and the API go through, so an archived Set cannot also become an
answered one.

**Storage.** Re-grounded during planning: schema setup is `CREATE TABLE IF NOT
EXISTS` only, with no migration machinery, and `question_sets` is `STRICT`. So
archiving gets its own table keyed by Set id with the time it was archived —
mirroring how the Response table hangs off a Set — and no existing table is
altered. The pending query gains a join that excludes archived Sets; the Archive
query from task 05 widens to cover both answered and archived-unanswered Sets,
ordered by whichever event settled each one.

## Acceptance criteria

- [ ] The set view of a pending Set offers to archive it, alongside its Liveness
- [ ] The action is confirmed before it takes effect, and the confirmation says
      it cannot be undone
- [ ] An archived Set disappears from the pending list and appears in the
      Archive, marked as archived unanswered and distinguishable from an
      answered Set
- [ ] An archived Set's detail view is read-only: its Questions and Preface are
      readable, and there is no way to answer it
- [ ] A wait held on a Set that is then archived ends with `410 Gone`
- [ ] The CLI treats 410 as fatal: it says the Set was archived unanswered and
      exits non-zero rather than retrying
- [ ] Submitting a Response to an archived Set is refused, from the browser and
      from the API alike
- [ ] An answered Set cannot be archived-unanswered, and archiving does not
      touch or delete anything already in the Archive
- [ ] Store tests cover the pending/Archive queries either side of archiving;
      API tests cover the 410 and the refused submit
