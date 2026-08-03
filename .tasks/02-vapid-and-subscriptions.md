# 02. VAPID identity and subscriptions

## What to build

The server's push identity and the record of which devices want notifications —
everything task 03's toggle needs to talk to, without any UI yet.

On first run the server generates a P-256 keypair and stores it in SQLite beside
the Sets; on every later run it reads the same one back. This is the whole key
ceremony: nothing to generate by hand, nothing to configure. The browser needs
the public key in the base64url form `PushManager.subscribe` expects, so hand it
out already encoded.

A subscription is what a browser hands back after subscribing: an endpoint URL and
the two keys (`p256dh` and `auth`) a push is encrypted for. Store one row per
endpoint URL, and let a repeat of the same endpoint replace what is stored rather
than add to it — a device that re-enables notifications, or whose subscription is
refreshed by the browser, must not end up notified twice.

Regenerating the keypair would silently invalidate every stored subscription, so
it is not something this task offers.

## Acceptance criteria

- [ ] A fresh database gets a keypair on first run; restarting the server against
      it reuses the same one, and the public key it hands out is stable across
      restarts
- [ ] The public key is delivered base64url-encoded from the uncompressed point,
      which is what `PushManager.subscribe` accepts as `applicationServerKey`
- [ ] Storing a subscription twice with the same endpoint leaves exactly one row,
      with the keys from the later submission
- [ ] Two different endpoints are two subscriptions
- [ ] A submission missing an endpoint or either key is refused, rather than
      stored as a subscription no push can be sent to
