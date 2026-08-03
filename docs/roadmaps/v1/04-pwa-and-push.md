# 04. PWA + push

## Goal

Askance installed on the phone as a PWA over Tailscale HTTPS; when an agent
submits a Question Set, the phone gets one push notification, and tapping it
opens that Set ready to answer.

## Decisions in force

- **HTTPS via `tailscale serve`** (PLAN.md “Frontend”) — the server keeps
  binding plain HTTP on localhost; `tailscale serve` terminates TLS with the
  `ts.net` cert. This provides the secure context that service workers and
  Web Push require. The tool is never exposed to the public internet.
- **Web Push, one notification per new Set, no reminders** (grilling session
  Q11) — deep-links to the Set. Reminder nudges were explicitly deferred.
- **VAPID keys auto-generated on first run and stored in SQLite** — zero
  manual key ceremony. Note: Web Push delivery goes out through the browser
  vendors' push services, so the *server* needs outbound internet; the
  inbound surface stays tailnet-only.
- Push subscriptions (per device/browser) are stored server-side; expired or
  rejected subscriptions are pruned on send failure.

## Proposed tasks (provisional)

1. **Installable PWA** — manifest, icons, service worker with a minimal
   offline/cache story.
   - Passes installability checks over the `ts.net` HTTPS URL; opens
     standalone on the phone
2. **Push subscription flow** — UI toggle to enable notifications on this
   device; subscription stored; VAPID keys auto-generated.
   - Fresh DB gets keys on first run; re-enabling on the same device doesn't
     duplicate subscriptions
3. **Send on Set arrival** — push to all subscriptions when a Set is created;
   deep link to the Set; prune dead subscriptions.
   - CLI-submitted set produces exactly one notification per subscribed
     device; tapping opens that Set
4. **Tailscale serve setup docs** — the `tailscale serve` invocation/config
   and any PWA-over-tailscale caveats, in the README.

## Re-verify at start

- Whether `tailscale serve` buffers or times out the long-poll/streaming
  endpoints (the phone UI and CLI may now both traverse it) — flagged in
  stage 01's brief too.
- Service-worker story in current Leptos (asset pipeline, hydration
  interplay) — check what current Leptos PWA examples do.
- Rust Web Push crate health (`web-push` crate maintenance status) at
  implementation time.
- Assumes stages 02 (UI to deep-link into) and ideally 03 (drafts interact
  with notification-driven re-entry) have landed.
