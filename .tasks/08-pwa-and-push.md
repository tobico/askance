# 08. PWA and push

## What to build

The device page and the push plumbing: notification permission, subscribe
and unsubscribe against the `/api/ui/` push endpoints, and service worker
registration from the SPA. The service worker, manifest and icons are kept
verbatim — they are already framework-free — and a push notification's
click-through opens `/sets/{id}` in the SPA as it does today.

## Acceptance criteria

- [ ] Subscribe and unsubscribe round-trip from the device page, recorded
      in the store
- [ ] A new Set's push notification arrives and its click lands on that
      Set's page
- [ ] The service worker, manifest and icons are unchanged files
- [ ] Push behaviour assertions from the old tests are ported (component
      level in vitest, delivery level staying in Rust)
