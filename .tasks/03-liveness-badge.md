# 03. Liveness tracking and badge

## What to build

The pending list says, per Set, whether an agent is still waiting on it —
"agent waiting" or "agent disconnected" — so a Set whose agent died is visible
at a glance instead of being answered into the void.

Liveness is display-only (ADR-0001). Disconnection never withdraws a Set and
never stops it being answerable: the CLI reconnects through transient drops, and
only a human may archive an orphaned Set (task 06).

**Where the truth comes from.** The server knows whether a long-poll is
currently held for a Set, because it is the thing holding it. A registry of held
waits, keyed by Set id, lives beside `Submissions` in the store crate and for
the same reason: the agent API holds the waits and the UI's server functions
read them, and those are different crates. It records how many waits are held
for a Set right now and when the last one was released. Registration is guarded
so that a dropped connection — a killed CLI, a broken tailnet — releases its
slot as the handler future is dropped, not only on a tidy return.

Like `Submissions`, the registry is in-process and not persisted: after a server
restart every pending Set reads disconnected until its CLI's next reconnect,
which is a second or two away.

**The grace window.** Re-grounded against how stage 01 actually landed: the
server holds each wait up to the client's requested `hold` (the CLI asks 30s,
the server caps at 60s), and the CLI reopens immediately on a 204, backing off
1s→10s only on a transient failure. So a live agent's gap between held waits is
sub-second, and the window only has to cover the backoff ceiling. Use **30s**,
generously clear of it, so the badge never flaps while an agent is merely
between polls. A Set that has never had a wait held on it measures its window
from `created_at` instead, so a Set that is one second old is not born
"disconnected" — worst-case detection of a killed agent is therefore about one
hold plus one window.

Keep the waiting/disconnected decision a pure function of (waits held now, when
the last one was released, when the Set was created, now) so the window can be
unit-tested without sockets.

**Reaching the UI.** The registry goes into the Leptos context beside the pool
and `Submissions`, and the pending list's entries carry the Liveness the server
computed — the browser gets a verdict, not a timestamp to interpret, matching
how the age is already worded server-side.

**Staying current.** The pending list is fetched once per page load today, so it
refetches on a client-side interval of about 10s while the page is open. That
keeps the badge honest, and also refreshes ages and surfaces newly arrived Sets
before push notifications exist. The interval must not run during SSR, and must
be cleaned up when the list is unmounted.

## Acceptance criteria

- [ ] A pending Set with a wait held on it renders "agent waiting"; asserted by
      holding a real long-poll against the same router while the pending page is
      requested
- [ ] A pending Set with no wait held, past the grace window, renders "agent
      disconnected"
- [ ] A Set created a moment ago, before any wait has been opened, does not read
      disconnected
- [ ] Killing the CLI flips its Set's badge to disconnected without a manual
      reload; restarting it flips the badge back
- [ ] A dropped connection releases its slot — the badge does not stay stuck on
      "agent waiting" after a client vanishes mid-hold
- [ ] Answering a Set that reads "agent disconnected" still works and still
      wakes an agent that turns out to be alive; no Set is auto-withdrawn
- [ ] Unit tests cover the grace-window decision, including the `created_at`
      floor and the boundary either side of the window
- [ ] Existing API tests still pass with the registry wired into both the
      API-only router and the router serving the UI
