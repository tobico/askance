//! What a device talks to when it asks to be notified: the server's public key,
//! and where the subscription the browser hands back is sent.
//!
//! There is no UI here yet — this is what the control on the pending list will
//! call.

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

/// A subscription as `PushManager.subscribe` describes one, flattened to the
/// three things a push needs: where to send it, and the two keys it is
/// encrypted for.
///
/// Flattened rather than passed through as the browser's own JSON, because the
/// nesting it uses — `keys.p256dh`, `keys.auth` — is the browser's shape and not
/// something the server has any reason to learn.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Subscription {
    pub endpoint: String,
    pub p256dh: String,
    pub auth: String,
}

/// The public half of the server's VAPID keypair, base64url-encoded from the
/// uncompressed point — what `PushManager.subscribe` takes as its
/// `applicationServerKey`.
///
/// The private half stays on the server: this is only how a browser names the
/// server it is subscribing to.
///
/// The path is spelled out rather than left to the macro's default so it is
/// legible in a log beside `/api/v1/`, which the agents use.
#[server(prefix = "/api/ui", endpoint = "push-key")]
pub async fn push_public_key() -> Result<String, ServerFnError> {
    let pool: sqlx::SqlitePool = expect_context();

    let keys = askance_store::vapid_keys(&pool)
        .await
        .map_err(|err| ServerFnError::new(format!("{err:#}")))?;

    Ok(keys.public_key)
}

/// What became of a device asking to be notified.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Subscribed {
    /// This device will be told about a Set from now on. It is stored once
    /// however many times it subscribes.
    Stored,

    /// Refused: the browser handed over a subscription with no endpoint, or
    /// missing a key. Nothing could ever be sent to it, so nothing was stored.
    Incomplete,
}

/// Take a device's subscription, so a Set arriving can reach it.
#[server(prefix = "/api/ui", endpoint = "subscribe-push", input = server_fn::codec::Json)]
pub async fn subscribe_push(subscription: Subscription) -> Result<Subscribed, ServerFnError> {
    use askance_store::{PushSubscription, Subscribing};

    let pool: sqlx::SqlitePool = expect_context();

    let subscribing = askance_store::store_subscription(
        &pool,
        &PushSubscription {
            endpoint: subscription.endpoint,
            p256dh: subscription.p256dh,
            auth: subscription.auth,
        },
    )
    .await
    .map_err(|err| ServerFnError::new(format!("{err:#}")))?;

    Ok(match subscribing {
        Subscribing::Stored => Subscribed::Stored,
        Subscribing::Incomplete => Subscribed::Incomplete,
    })
}
