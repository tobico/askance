//! Turning notifications on for one device: the key a browser subscribes
//! against, the subscription it hands back, and what the server made of it.
//!
//! Per device, and never anything the viewer remembers: what the switch says is
//! read out of the browser on every load, because an installed app reopened a
//! week later must not offer to enable what is already enabled.

use serde::{Deserialize, Serialize};

#[cfg(feature = "typescript")]
use ts_rs::TS;

/// The public half of the server's VAPID keypair, base64url-encoded from the
/// uncompressed point — what `PushManager.subscribe` takes as its
/// `applicationServerKey`.
///
/// The private half stays on the server: this is only how a browser names the
/// server it is subscribing to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct PushKey {
    pub key: String,
}

/// A subscription as `PushManager.subscribe` describes one, flattened to the
/// three things a push needs: where to send it, and the two keys it is
/// encrypted for.
///
/// Flattened rather than passed through as the browser's own JSON, because the
/// nesting it uses — `keys.p256dh`, `keys.auth` — is the browser's shape and not
/// something the server has any reason to learn.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct Subscription {
    pub endpoint: String,
    pub p256dh: String,
    pub auth: String,
}

/// What became of a device asking to be notified.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum Subscribed {
    /// This device will be told about a Set from now on. It is stored once
    /// however many times it subscribes.
    Stored,

    /// Refused: the browser handed over a subscription with no endpoint, or
    /// missing a key. Nothing could ever be sent to it, so nothing was stored.
    Incomplete,
}

/// A device asking not to be told any more, named by its endpoint — which is the
/// only name a subscription has.
///
/// An endpoint the server never stored is not an error: a browser can drop its
/// own subscription without the server having heard of it, and afterwards what
/// was asked for holds either way — nothing is sent there.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct Unsubscribe {
    pub endpoint: String,
}
