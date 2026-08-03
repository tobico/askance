# 05. Archive list

## What to build

A browsable Archive at its own route, listing every Set that has been answered,
newest first, each row opening the read-only view from task 04. The Archive is a
permanent decision log: answered Sets are never deleted, and a Set lands there
by being answered rather than by anyone filing it.

Rows carry what the pending list's rows carry — title, project, branch — plus
when the Set was answered, since in the Archive that is the date the decision was
made. No Liveness badge: nothing is waiting on an answered Set.

The two lists are reachable from each other, so the human can get from what is
waiting to what was decided and back without typing a URL. An empty Archive says
so plainly, the way the empty pending list does.

The store gains a query for answered Sets that, like the pending query, reads the
lifted columns and its Response's timestamp without deserializing bodies — a
decision log grows forever and the list is scanned, not read.

Ordering follows the pending list's reasoning: order by the answering, but break
ties by id rather than by a timestamp two Sets could share to the millisecond.

## Acceptance criteria

- [ ] A Set that has been answered appears in the Archive, newest first, with
      its title, provenance and when it was answered
- [ ] A pending Set does not appear in the Archive, and an answered Set no
      longer appears in the pending list (unchanged behaviour, asserted)
- [ ] Tapping an Archive row opens that Set's read-only view with its Response
      readable
- [ ] The Archive and the pending list link to each other
- [ ] An Archive with nothing in it renders an empty-state line, not a bare
      heading
- [ ] The listing query does not deserialize Set bodies
- [ ] Asserted on the server-rendered HTML, including the empty case
