//! The push identity and the record of which devices asked to be told: that the
//! keypair survives a restart, that it is in the encoding the browser wants, and
//! that a device re-subscribing stays one device.

use askance_store::{
    PushSubscription, Subscribing, open_database, push_subscriptions, store_subscription,
    vapid_keys,
};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use sqlx::SqlitePool;

/// A pool over a fresh database, plus the directory keeping it alive.
async fn fresh_pool() -> (tempfile::TempDir, SqlitePool) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("askance.db")).await.unwrap();
    (dir, pool)
}

/// A subscription as a browser hands one back: a push service URL and the two
/// keys a push is encrypted for.
fn subscription(endpoint: &str, p256dh: &str, auth: &str) -> PushSubscription {
    PushSubscription {
        endpoint: endpoint.to_owned(),
        p256dh: p256dh.to_owned(),
        auth: auth.to_owned(),
    }
}

#[tokio::test]
async fn a_fresh_database_gets_a_keypair_on_first_run() {
    let (_dir, pool) = fresh_pool().await;

    let keys = vapid_keys(&pool).await.unwrap();

    assert!(!keys.public_key.is_empty());
    assert!(!keys.private_key.is_empty());
}

#[tokio::test]
async fn restarting_against_the_same_database_reuses_the_same_keypair() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("askance.db");

    let pool = open_database(&path).await.unwrap();
    let first = vapid_keys(&pool).await.unwrap();
    pool.close().await;

    let pool = open_database(&path).await.unwrap();
    let second = vapid_keys(&pool).await.unwrap();

    assert_eq!(
        first, second,
        "regenerating would invalidate every stored subscription"
    );
}

#[tokio::test]
async fn asking_twice_in_one_run_gets_the_same_keypair() {
    let (_dir, pool) = fresh_pool().await;

    assert_eq!(
        vapid_keys(&pool).await.unwrap(),
        vapid_keys(&pool).await.unwrap()
    );
}

#[tokio::test]
async fn the_public_key_is_the_uncompressed_point_base64url_encoded() {
    let (_dir, pool) = fresh_pool().await;

    let keys = vapid_keys(&pool).await.unwrap();
    let point = URL_SAFE_NO_PAD
        .decode(&keys.public_key)
        .expect("the public key should be base64url without padding");

    assert_eq!(point.len(), 65, "an uncompressed P-256 point is 65 bytes");
    assert_eq!(point[0], 0x04, "an uncompressed point is tagged 0x04");
    p256::PublicKey::from_sec1_bytes(&point)
        .expect("what PushManager.subscribe is given has to be a point on P-256");
}

#[tokio::test]
async fn the_private_key_is_the_scalar_the_public_key_belongs_to() {
    let (_dir, pool) = fresh_pool().await;

    let keys = vapid_keys(&pool).await.unwrap();
    let scalar = URL_SAFE_NO_PAD
        .decode(&keys.private_key)
        .expect("the private key should be base64url without padding");
    let secret = p256::SecretKey::from_slice(&scalar)
        .expect("the stored private key has to be a P-256 secret key");

    let public = URL_SAFE_NO_PAD.decode(&keys.public_key).unwrap();
    assert_eq!(
        p256::PublicKey::from_sec1_bytes(&public).unwrap(),
        secret.public_key(),
        "a push signed with the private key must verify against the public one"
    );
}

#[tokio::test]
async fn a_subscription_is_stored() {
    let (_dir, pool) = fresh_pool().await;

    let device = subscription("https://push.example/aaa", "p256dh-aaa", "auth-aaa");
    assert_eq!(
        store_subscription(&pool, &device).await.unwrap(),
        Subscribing::Stored
    );

    assert_eq!(push_subscriptions(&pool).await.unwrap(), [device]);
}

#[tokio::test]
async fn the_same_endpoint_twice_is_one_subscription_with_the_later_keys() {
    let (_dir, pool) = fresh_pool().await;

    store_subscription(
        &pool,
        &subscription("https://push.example/aaa", "p256dh-old", "auth-old"),
    )
    .await
    .unwrap();

    let refreshed = subscription("https://push.example/aaa", "p256dh-new", "auth-new");
    assert_eq!(
        store_subscription(&pool, &refreshed).await.unwrap(),
        Subscribing::Stored
    );

    assert_eq!(
        push_subscriptions(&pool).await.unwrap(),
        [refreshed],
        "a device that re-enables notifications must not be notified twice"
    );
}

#[tokio::test]
async fn two_endpoints_are_two_subscriptions() {
    let (_dir, pool) = fresh_pool().await;

    let phone = subscription("https://push.example/phone", "p256dh-phone", "auth-phone");
    let laptop = subscription(
        "https://push.example/laptop",
        "p256dh-laptop",
        "auth-laptop",
    );

    store_subscription(&pool, &phone).await.unwrap();
    store_subscription(&pool, &laptop).await.unwrap();

    let stored = push_subscriptions(&pool).await.unwrap();
    assert_eq!(stored.len(), 2);
    assert!(stored.contains(&phone));
    assert!(stored.contains(&laptop));
}

#[tokio::test]
async fn a_subscription_missing_an_endpoint_or_a_key_is_refused() {
    let (_dir, pool) = fresh_pool().await;

    for incomplete in [
        subscription("", "p256dh-aaa", "auth-aaa"),
        subscription("https://push.example/aaa", "", "auth-aaa"),
        subscription("https://push.example/aaa", "p256dh-aaa", ""),
        subscription("   ", "p256dh-aaa", "auth-aaa"),
    ] {
        assert_eq!(
            store_subscription(&pool, &incomplete).await.unwrap(),
            Subscribing::Incomplete,
            "no push could ever be sent to {incomplete:?}"
        );
    }

    assert_eq!(push_subscriptions(&pool).await.unwrap(), []);
}
