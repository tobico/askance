//! The push identity and the devices that asked to be told.
//!
//! Two things, both kept in SQLite beside the Sets: the VAPID keypair every push
//! this server sends is signed with, and one subscription per browser that has
//! asked to be notified.
//!
//! The keypair is generated the first time the database is opened rather than
//! configured by hand, so there is no key ceremony to perform. Regenerating it
//! would silently invalidate every subscription stored against it — the push
//! services reject a push signed by a key the subscription was not created for —
//! so nothing here offers to.
//!
//! The private key never leaves this crate's callers on the server: the browser
//! is handed the public key alone, and only so it can name this server when it
//! subscribes.

use anyhow::{Context, Result};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use p256::SecretKey;
use p256::elliptic_curve::rand_core::OsRng;
use p256::elliptic_curve::sec1::ToEncodedPoint;
use sqlx::SqlitePool;

/// The server's push identity, in the encodings the two sides want it in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VapidKeys {
    /// The uncompressed SEC1 point, base64url without padding: exactly what
    /// `PushManager.subscribe` takes as its `applicationServerKey`.
    pub public_key: String,

    /// The 32-byte scalar, base64url without padding — the form a VAPID signer
    /// expects a private key in. Server-side only.
    pub private_key: String,
}

/// One browser that has asked to be told, as it described itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PushSubscription {
    /// The push service's URL for this device, and its identity here: a browser
    /// handing back an endpoint already stored is the same device again, not a
    /// second one.
    pub endpoint: String,

    /// The device's public key, which a push to it is encrypted for.
    pub p256dh: String,

    /// The device's authentication secret, which that encryption also needs.
    pub auth: String,
}

/// What became of storing a subscription.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Subscribing {
    /// Stored: this device will be pushed to. Either it is newly subscribed, or
    /// it re-subscribed and its keys were replaced with these.
    Stored,

    /// Refused: the endpoint or one of the keys was missing. All three are
    /// needed to send anything, so storing it would only mean keeping a device
    /// on the list that no push could ever reach.
    Incomplete,
}

/// Bring an opened database up to the shape push needs, and perform the key
/// ceremony — which is to say, generate the keypair if this is the first run.
///
/// Doing it as the database is opened rather than when the first browser asks
/// means a keypair that cannot be generated or stored stops the server on the
/// spot, instead of failing a tap on the phone.
pub(crate) async fn apply_schema(pool: &SqlitePool) -> Result<()> {
    // A single row, pinned to id 1 by the check: there is one push identity, and
    // a second one would mean subscriptions signed by whichever was read first.
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS vapid_keys (
             id          INTEGER PRIMARY KEY CHECK (id = 1),
             created_at  TEXT NOT NULL,
             public_key  TEXT NOT NULL,
             private_key TEXT NOT NULL
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the vapid_keys table")?;

    // The endpoint is the primary key, so a device re-subscribing replaces its
    // row rather than adding one — see [`store_subscription`].
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS push_subscriptions (
             endpoint      TEXT PRIMARY KEY,
             p256dh        TEXT NOT NULL,
             auth          TEXT NOT NULL,
             subscribed_at TEXT NOT NULL
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the push_subscriptions table")?;

    vapid_keys(pool).await?;

    Ok(())
}

/// The server's push identity, generated and stored if the database has none.
///
/// Every later call — and every later run of the server — reads the same one
/// back, so the public key handed to a browser is the key the push it eventually
/// receives was signed with.
pub async fn vapid_keys(pool: &SqlitePool) -> Result<VapidKeys> {
    if let Some(keys) = stored_keys(pool).await? {
        return Ok(keys);
    }

    let fresh = generate();

    // Inserted only if the row is not there, and then read back rather than
    // returned: two callers racing on a fresh database must not end up handing
    // out two different public keys, so the loser uses the winner's.
    sqlx::query(
        "INSERT INTO vapid_keys (id, created_at, public_key, private_key)
         VALUES (1, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), ?, ?)
         ON CONFLICT (id) DO NOTHING",
    )
    .bind(&fresh.public_key)
    .bind(&fresh.private_key)
    .execute(pool)
    .await
    .context("storing the VAPID keypair")?;

    stored_keys(pool)
        .await?
        .context("the VAPID keypair was gone the moment after it was stored")
}

/// Record that a device wants to be notified, or replace what is recorded for it.
///
/// The endpoint is the device's identity: a browser that re-enables
/// notifications, or whose subscription the browser has refreshed, comes back
/// with the same endpoint and must not end up notified twice.
pub async fn store_subscription(
    pool: &SqlitePool,
    subscription: &PushSubscription,
) -> Result<Subscribing> {
    let complete = [
        &subscription.endpoint,
        &subscription.p256dh,
        &subscription.auth,
    ]
    .iter()
    .all(|field| !field.trim().is_empty());

    if !complete {
        return Ok(Subscribing::Incomplete);
    }

    sqlx::query(
        "INSERT INTO push_subscriptions (endpoint, p256dh, auth, subscribed_at)
         VALUES (?, ?, ?, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
         ON CONFLICT (endpoint) DO UPDATE SET
             p256dh        = excluded.p256dh,
             auth          = excluded.auth,
             subscribed_at = excluded.subscribed_at",
    )
    .bind(&subscription.endpoint)
    .bind(&subscription.p256dh)
    .bind(&subscription.auth)
    .execute(pool)
    .await
    .context("storing the push subscription")?;

    Ok(Subscribing::Stored)
}

/// Forget a device, so that nothing more is sent to it.
///
/// Idempotent, and deliberately silent about whether there was a row to delete:
/// what the caller is asking for is that this endpoint not be notified, and
/// afterwards it is not — whether it was already gone (a device turning
/// notifications off twice, one whose subscription the browser replaced) says
/// nothing worth acting on.
pub async fn forget_subscription(pool: &SqlitePool, endpoint: &str) -> Result<()> {
    sqlx::query("DELETE FROM push_subscriptions WHERE endpoint = ?")
        .bind(endpoint)
        .execute(pool)
        .await
        .context("forgetting the push subscription")?;

    Ok(())
}

/// Every device that has asked to be told, oldest subscription first.
///
/// Ordered so that a list of them reads the same way twice; nothing about a push
/// depends on which device is sent to first.
pub async fn push_subscriptions(pool: &SqlitePool) -> Result<Vec<PushSubscription>> {
    let rows: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT endpoint, p256dh, auth FROM push_subscriptions
         ORDER BY subscribed_at, endpoint",
    )
    .fetch_all(pool)
    .await
    .context("listing the push subscriptions")?;

    Ok(rows
        .into_iter()
        .map(|(endpoint, p256dh, auth)| PushSubscription {
            endpoint,
            p256dh,
            auth,
        })
        .collect())
}

/// The stored keypair, or `None` before there is one.
async fn stored_keys(pool: &SqlitePool) -> Result<Option<VapidKeys>> {
    let row: Option<(String, String)> =
        sqlx::query_as("SELECT public_key, private_key FROM vapid_keys WHERE id = 1")
            .fetch_optional(pool)
            .await
            .context("loading the VAPID keypair")?;

    Ok(row.map(|(public_key, private_key)| VapidKeys {
        public_key,
        private_key,
    }))
}

/// A fresh P-256 keypair, encoded as [`VapidKeys`] describes.
fn generate() -> VapidKeys {
    let secret = SecretKey::random(&mut OsRng);

    VapidKeys {
        // Uncompressed rather than compressed: `applicationServerKey` is defined
        // as the 65-byte form, and a browser handed the 33-byte one refuses it.
        public_key: URL_SAFE_NO_PAD.encode(secret.public_key().to_encoded_point(false).as_bytes()),
        private_key: URL_SAFE_NO_PAD.encode(secret.to_bytes()),
    }
}
