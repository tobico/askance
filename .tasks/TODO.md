# PWA and push

Askance installed on the phone as a PWA over Tailscale HTTPS, so a Question Set
arriving from an agent reaches the human without the pending list being open. One
push notification per new Set, deep-linking to it; no reminders. VAPID keys are
generated on first run and stored in SQLite, so there is no key ceremony to
perform.

`tailscale serve` terminates TLS with the `ts.net` certificate, which is what
gives service workers and Web Push the secure context they require. The server
keeps binding plain HTTP and is never exposed to the public internet — though
sending a push does need outbound internet, because delivery goes through the
browser vendors' push services.

Roadmap stage: [04: PWA + push](docs/roadmaps/v1/04-pwa-and-push.md)

## Tasks

- [x] 01: Installable app shell — [details](01-app-shell.md)
- [x] 02: VAPID identity and subscriptions — [details](02-vapid-and-subscriptions.md)
- [x] 03: Enable notifications on this device — [details](03-enable-notifications.md)
- [x] 04: Push on Set arrival — [details](04-push-on-set-arrival.md)
- [ ] 05: Serving over Tailscale, and the install — [details](05-tailscale-and-install.md)
