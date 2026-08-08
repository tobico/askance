# 04. The worker relays

## What to build

The service worker, on receiving any push, additionally posts a Nudge to
every open Askance window — and still always shows the notification, since
Apple expects every web push to surface one and suppression risks the
subscription (ADR-0005). The page treats a worker-relayed Nudge exactly as it
treats a stream Nudge: invalidate every active query.

This is the second channel of the redundancy the design chose: the stream is
instant over the tailnet but dies when iOS suspends the PWA, while a push
survives backgrounding but transits Apple's push service and needs
notifications enabled. Each covers the other's gap.

## Acceptance criteria

- [ ] A push refreshes an open pending list even when the SSE stream is
      severed
- [ ] The notification still shows in every case it shows today
- [ ] A worker-relayed Nudge and a stream Nudge produce the same reaction in
      the page
- [ ] Worker and viewer tests cover the relay end to end
