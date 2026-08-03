//! What the browser talks to when a device asks to be notified: the endpoint
//! that hands out the server's public key, and the one that takes the
//! subscription the push manager hands back.
//!
//! Through the router the binary serves, because that is the only way to know
//! the UI's endpoints are reachable at the paths the page will call.

use askance_app::push::Subscribed;
use askance_server::{open_database, router_with_ui, store};
use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use leptos::prelude::LeptosOptions;
use sqlx::SqlitePool;
use tower::ServiceExt;

/// The UI's file locations, which these tests never reach: they call endpoints,
/// not pages.
fn options() -> LeptosOptions {
    LeptosOptions::builder()
        .output_name("askance")
        .site_root("target/site")
        .build()
}

/// One router over a fresh database, as the binary serves it.
async fn fresh_app() -> (tempfile::TempDir, SqlitePool, Router) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("askance.db")).await.unwrap();
    let app = router_with_ui(pool.clone(), options());
    (dir, pool, app)
}

async fn body_text(response: axum::response::Response) -> String {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    String::from_utf8(bytes.to_vec()).unwrap()
}

/// Ask for the public key the way the page will.
async fn public_key(app: &Router) -> String {
    let http = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/ui/push-key")
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let status = http.status();
    let body = body_text(http).await;
    assert_eq!(status, StatusCode::OK, "asking for the key failed: {body}");

    serde_json::from_str(&body).unwrap_or_else(|err| panic!("reading {body:?}: {err}"))
}

/// Hand over a subscription the way the page will, over JSON.
async fn subscribe(app: &Router, endpoint: &str, p256dh: &str, auth: &str) -> Subscribed {
    let args = serde_json::json!({
        "subscription": { "endpoint": endpoint, "p256dh": p256dh, "auth": auth },
    });

    let http = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/ui/subscribe-push")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_vec(&args).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    let status = http.status();
    let body = body_text(http).await;
    assert_eq!(status, StatusCode::OK, "subscribing failed: {body}");

    serde_json::from_str(&body).unwrap_or_else(|err| panic!("reading {body:?}: {err}"))
}

#[tokio::test]
async fn the_page_is_handed_the_stored_public_key() {
    let (_dir, pool, app) = fresh_app().await;

    let handed_out = public_key(&app).await;

    assert_eq!(
        handed_out,
        store::vapid_keys(&pool).await.unwrap().public_key
    );
    assert_eq!(
        handed_out,
        public_key(&app).await,
        "the key a device subscribed against has to keep being the same key"
    );
}

#[tokio::test]
async fn the_private_key_never_leaves_the_server() {
    let (_dir, pool, app) = fresh_app().await;

    let private = store::vapid_keys(&pool).await.unwrap().private_key;

    assert!(!public_key(&app).await.contains(&private));
}

#[tokio::test]
async fn a_device_asking_to_be_told_is_stored() {
    let (_dir, pool, app) = fresh_app().await;

    assert_eq!(
        subscribe(
            &app,
            "https://push.example/phone",
            "p256dh-phone",
            "auth-phone"
        )
        .await,
        Subscribed::Stored
    );

    let stored = store::push_subscriptions(&pool).await.unwrap();
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].endpoint, "https://push.example/phone");
    assert_eq!(stored[0].p256dh, "p256dh-phone");
    assert_eq!(stored[0].auth, "auth-phone");
}

#[tokio::test]
async fn a_device_subscribing_again_stays_one_device() {
    let (_dir, pool, app) = fresh_app().await;

    subscribe(&app, "https://push.example/phone", "p256dh-old", "auth-old").await;
    subscribe(&app, "https://push.example/phone", "p256dh-new", "auth-new").await;

    let stored = store::push_subscriptions(&pool).await.unwrap();
    assert_eq!(stored.len(), 1, "one device, one notification");
    assert_eq!(stored[0].p256dh, "p256dh-new");
}

#[tokio::test]
async fn a_subscription_with_nothing_to_send_to_is_refused() {
    let (_dir, pool, app) = fresh_app().await;

    assert_eq!(
        subscribe(&app, "", "p256dh-phone", "auth-phone").await,
        Subscribed::Incomplete
    );
    assert_eq!(
        subscribe(&app, "https://push.example/phone", "p256dh-phone", "").await,
        Subscribed::Incomplete
    );

    assert!(store::push_subscriptions(&pool).await.unwrap().is_empty());
}
