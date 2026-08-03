# 04. Push on Set arrival

## What to build

An agent submits a Question Set and the phone buzzes: one notification per
subscribed device, and tapping it opens that Set ready to answer.

A stored Set is sent to every subscription — encrypted for that subscription's
keys, signed with the server's VAPID identity from task 02 — carrying enough to
draw the notification (the Set's title, its project) and the id to open. The
service worker shows it and, on a tap, focuses an already-open Askance if there is
one and navigates it to that Set, or opens the app there if there is not.

Sending must never be what fails a submission. The agent's `POST /api/v1/sets`
answers with the id as soon as the Set is stored; pushes go out behind that, and a
push service being unreachable costs a notification, not the Set. Delivery goes out
through the browser vendors' push services, so this is the one thing in Askance
that needs outbound internet — the inbound surface stays tailnet-only.

Push services report a subscription that is gone for good (`404`, `410`), which is
the only word we get that a device has uninstalled the app or had its subscription
expire. Prune those, and leave everything else alone — a timeout or a `503` is a
notification lost, not a dead device.

Dependencies: `web-push-native` builds the encrypted request and `reqwest`
(rustls) sends it, both async alongside the rest of the server. The VAPID JWT is
signed with `p256` directly rather than through a JWT crate — it is one ES256
signature over two claims, and `p256` is already here for the keypair.

## Acceptance criteria

- [ ] A Set submitted by the CLI produces exactly one notification per subscribed
      device, whose text identifies the Set
- [ ] Tapping the notification opens that Set's page, and reuses an already-open
      Askance rather than stacking another window
- [ ] The submission still answers `201` with the Set's id when every push fails,
      and the failure is logged rather than swallowed
- [ ] A subscription the push service rejects as gone is deleted; one that fails
      transiently is kept
- [ ] Covered by a test that stands a local push endpoint up in-process, so the
      per-subscription count, the pruning and the transient case are asserted
      against real requests rather than a mock
- [ ] Answering or archiving a Set sends nothing — one notification per new Set,
      no reminders and no follow-ups
