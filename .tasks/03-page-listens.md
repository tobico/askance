# 03. The page listens

## What to build

The viewer holds an open `EventSource` on the Nudge stream for as long as it
is running. Any Nudge invalidates every active query — the reaction is always
"look again", never anything per-event — so a Set submitted while the pending
list is on screen appears without polling delay, and a Set answered or
archived from another device updates its open page the same way.

A stream that drops (the PWA suspended, the server restarted) reconnects on
its own, and the reconnect itself triggers the same catch-up invalidation:
whatever happened while the stream was dead is fetched, not replayed. The
10-second poll stays untouched as the fallback for a stream that cannot be
had at all.

## Acceptance criteria

- [ ] A Set submitted while the pending list is open appears without waiting
      on the poll
- [ ] Reconnecting after a dropped stream refetches active queries
- [ ] The poll still runs at its existing interval
- [ ] Viewer tests cover both the Nudge reaction and the reconnect catch-up
