# Layered viewer freshness: poll, SSE, and push-relayed Nudges

The pending page went stale — a new Set could sit unseen for the length of a
poll interval while the page was open, and indefinitely after the iOS PWA was
backgrounded or the phone locked, because refetch-on-focus was deliberately
off. We layer three mechanisms rather than picking one: the existing 10-second
poll stays, the server broadcasts a Nudge over an SSE stream to open pages,
and the service worker relays every push it receives to open pages as the same
Nudge. A `visibilitychange` refetch (plus SSE reconnect catch-up) covers PWA
resume. The channels fail differently on iOS — the SSE connection is instant
over the tailnet but dies when the PWA is suspended; a push survives
backgrounding but transits Apple's push service and needs notifications
enabled; the poll needs nothing but outlives neither gap — so each covers the
others' failure modes, and deleting any one of them would regress staleness in
a way nobody notices quickly.

## Considered Options

- **SSE alone** — instant while open, works with notifications off, but blind
  through iOS suspension until the reconnect lands.
- **Push relay alone** — no new server surface, but useless with
  notifications off and dependent on Apple's delivery.
- **Faster polling** — no new moving parts, but trades battery and network
  for latency that still isn't "immediate".

## Consequences

- A Nudge is contentless and the reaction is always the same: invalidate all
  active queries. Typed events wait until some page wants a per-event
  reaction.
- The server broadcasts only durable changes — Set created, Response
  submitted, Set archived. Liveness transitions stay with the poll: the
  waiting/disconnected verdict cycles with the agent's long-poll rather than
  changing at clean moments.
- The push notification always shows, even when the app is focused on the
  pending list: Apple expects every web push to surface one, and suppression
  risks the subscription on the platform actually in use.
- This overturns the earlier decision that "coming back to a tab is not new
  information about a Set" (`refetchOnWindowFocus` off in `App.tsx`) — for an
  installed PWA, coming back is precisely when the world has moved.
