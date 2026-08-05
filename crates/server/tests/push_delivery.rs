//! Telling the devices that a Question Set has arrived.
//!
//! Against a push service stood up in-process rather than against a mock: what
//! is worth proving is the request that actually goes out — one per device,
//! encrypted for that device's own keys — and what the server does with the
//! answer it gets back. A push service only ever speaks to us in status codes,
//! so a fake that returns them is the whole of the contract.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use askance_schema::SetCreated;
use askance_server::{open_database, router, store};
use axum::Router;
use axum::body::{Body, Bytes};
use axum::extract::{Path, State};
use axum::http::{HeaderMap, Request, StatusCode, header};
use axum::routing::post;
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use http_body_util::BodyExt;
use p256::SecretKey;
use p256::elliptic_curve::rand_core::OsRng;
use p256::elliptic_curve::sec1::ToEncodedPoint;
use sqlx::SqlitePool;
use tower::ServiceExt;
use web_push_native::Auth;

const SET: &str = r#"
title: Rate limiting for the public API
project: askance
branch: pwa-and-push
questions:
  - label: Q1
    text: Where should the request counter live?
"#;

/// How long a test will wait for something that happens behind the response:
/// generous, because it is only ever paid when the assertion is about to fail.
const PATIENCE: Duration = Duration::from_secs(5);

/// How long to keep watching after everything expected has arrived, to catch a
/// push that should not have been sent at all.
const SETTLING: Duration = Duration::from_millis(300);

/// One push, as the push service received it.
#[derive(Debug, Clone)]
struct Received {
    /// The last segment of the path it was posted to, which is the device it
    /// was meant for.
    device: String,
    authorization: String,
    content_encoding: String,
    body: Vec<u8>,
}

/// A device as its browser would have described it, plus the private half the
/// browser would have kept — which is what lets a test read a push back.
struct Device {
    name: String,
    endpoint: String,
    secret: SecretKey,
    auth: Vec<u8>,
}

impl Device {
    /// A device whose push service will answer `verdict` for it.
    fn new(service: &str, verdict: &str, name: &str) -> Self {
        // Deterministic rather than random, so a failure names the same device
        // twice running. Sixteen bytes because that is what the auth secret is.
        let auth: Vec<u8> = name.bytes().cycle().take(16).collect();

        Self {
            name: name.to_owned(),
            endpoint: format!("{service}/{verdict}/{name}"),
            secret: SecretKey::random(&mut OsRng),
            auth,
        }
    }

    /// The public half, in the encoding `PushManager.subscribe` hands back.
    fn p256dh(&self) -> String {
        URL_SAFE_NO_PAD.encode(self.secret.public_key().to_encoded_point(false).as_bytes())
    }

    fn auth(&self) -> String {
        URL_SAFE_NO_PAD.encode(&self.auth)
    }

    /// Read a push meant for this device, as its service worker would.
    fn read(&self, push: &Received) -> serde_json::Value {
        let plain = web_push_native::decrypt(
            push.body.clone(),
            &self.secret,
            &Auth::clone_from_slice(&self.auth),
        )
        .expect("a push for this device has to decrypt with this device's keys");

        serde_json::from_slice(&plain).expect("the notice has to be JSON")
    }
}

/// A push service on a loopback port, and the pushes it has taken.
///
/// The path says what it should answer: `take` accepts, `gone` and `missing`
/// are the two ways a service says a subscription is finished with, and `flaky`
/// is a service having a bad day.
async fn push_service() -> (String, Arc<Mutex<Vec<Received>>>) {
    let received = Arc::new(Mutex::new(Vec::new()));

    let app = Router::new()
        .route("/{verdict}/{device}", post(take))
        .with_state(received.clone());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

    (format!("http://{address}"), received)
}

async fn take(
    State(received): State<Arc<Mutex<Vec<Received>>>>,
    Path((verdict, device)): Path<(String, String)>,
    headers: HeaderMap,
    body: Bytes,
) -> StatusCode {
    let said = |name: header::HeaderName| {
        headers
            .get(name)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
            .to_owned()
    };

    received.lock().unwrap().push(Received {
        device,
        authorization: said(header::AUTHORIZATION),
        content_encoding: said(header::CONTENT_ENCODING),
        body: body.to_vec(),
    });

    match verdict.as_str() {
        "gone" => StatusCode::GONE,
        "missing" => StatusCode::NOT_FOUND,
        "flaky" => StatusCode::SERVICE_UNAVAILABLE,
        _ => StatusCode::CREATED,
    }
}

/// An address nothing is listening on: bound to claim a free port, then dropped.
async fn nowhere() -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    drop(listener);
    format!("http://{address}")
}

/// One router over a fresh database, as the binary serves it: the Set arrives
/// through the agents' half, and the devices it is pushed to were put on the list
/// through the viewer's.
async fn fresh_app() -> (tempfile::TempDir, SqlitePool, Router) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("askance.db")).await.unwrap();
    let app = router(pool.clone());
    (dir, pool, app)
}

async fn body_text(response: axum::response::Response) -> String {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    String::from_utf8(bytes.to_vec()).unwrap()
}

/// Put a device on the list, the way the page does.
async fn subscribe(app: &Router, device: &Device) {
    let args = serde_json::json!({
        "endpoint": device.endpoint,
        "p256dh": device.p256dh(),
        "auth": device.auth(),
    });

    let http = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/ui/push/subscribe")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_vec(&args).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(http.status(), StatusCode::OK, "subscribing {}", device.name);
}

/// Send a Set the way the CLI does, and return what the agent was told.
async fn post_set(app: &Router, yaml: &str) -> (StatusCode, SetCreated) {
    let http = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/sets")
                .header(header::CONTENT_TYPE, "application/yaml")
                .body(Body::from(yaml.to_owned()))
                .unwrap(),
        )
        .await
        .unwrap();

    let status = http.status();
    let body = body_text(http).await;
    let created =
        serde_saphyr::from_str(&body).unwrap_or_else(|err| panic!("reading {body:?}: {err}"));

    (status, created)
}

/// Answer a Set the way the viewer does.
async fn answer(app: &Router, id: i64) {
    let response = serde_json::json!({
        "answers": [{ "label": "Q1", "free_text": "In Redis." }],
    });

    let http = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/ui/sets/{id}/response"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_vec(&response).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    let status = http.status();
    let body = body_text(http).await;
    assert_eq!(status, StatusCode::OK, "answering failed: {body}");
}

/// Close a Set unanswered the way the viewer does.
async fn archive(app: &Router, id: i64) {
    let http = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/ui/sets/{id}/archive"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();

    let status = http.status();
    let body = body_text(http).await;
    assert_eq!(status, StatusCode::OK, "archiving failed: {body}");
}

/// Wait until the push service has taken `count` pushes, then hand them over.
async fn pushes(received: &Arc<Mutex<Vec<Received>>>, count: usize) -> Vec<Received> {
    let deadline = tokio::time::Instant::now() + PATIENCE;

    loop {
        {
            let taken = received.lock().unwrap();
            if taken.len() >= count {
                return taken.clone();
            }
            if tokio::time::Instant::now() >= deadline {
                panic!("waited for {count} pushes and {} arrived", taken.len());
            }
        }

        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

/// Give anything still on its way time to arrive, for an assertion about a push
/// that should never be sent.
async fn settle() {
    tokio::time::sleep(SETTLING).await;
}

/// Wait until the stored devices are `endpoints`, in some order.
async fn subscriptions_settle_to(pool: &SqlitePool, endpoints: &[&str]) {
    let deadline = tokio::time::Instant::now() + PATIENCE;
    let wanted: std::collections::BTreeSet<&str> = endpoints.iter().copied().collect();

    loop {
        let stored = store::push_subscriptions(pool).await.unwrap();
        let have: std::collections::BTreeSet<&str> =
            stored.iter().map(|one| one.endpoint.as_str()).collect();

        if have == wanted {
            return;
        }

        assert!(
            tokio::time::Instant::now() < deadline,
            "the stored devices settled at {have:?}, not {wanted:?}",
        );

        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

#[tokio::test]
async fn a_set_arriving_is_pushed_once_to_every_device() {
    let (service, received) = push_service().await;
    let (_dir, pool, app) = fresh_app().await;

    let phone = Device::new(&service, "take", "phone");
    let laptop = Device::new(&service, "take", "laptop");
    subscribe(&app, &phone).await;
    subscribe(&app, &laptop).await;

    let (status, created) = post_set(&app, SET).await;
    assert_eq!(status, StatusCode::CREATED);

    let taken = pushes(&received, 2).await;
    settle().await;
    assert_eq!(
        received.lock().unwrap().len(),
        2,
        "one notification per device, and no more",
    );

    let key = store::vapid_keys(&pool).await.unwrap().public_key;

    for device in [&phone, &laptop] {
        let push = taken
            .iter()
            .find(|push| push.device == device.name)
            .unwrap_or_else(|| panic!("{} was not pushed to", device.name));

        assert_eq!(push.content_encoding, "aes128gcm");
        assert!(
            push.authorization.starts_with("vapid t=")
                && push.authorization.ends_with(&format!("k={key}")),
            "the push has to be signed by this server's identity: {:?}",
            push.authorization,
        );

        // Decrypted with the device's own keys, which is the proof that it was
        // encrypted for that device and not merely addressed to it.
        let notice = device.read(push);
        assert_eq!(notice["id"], created.id);
        assert_eq!(notice["title"], "Rate limiting for the public API");
        assert_eq!(notice["project"], "askance");
    }
}

#[tokio::test]
async fn the_set_is_stored_even_when_every_push_fails() {
    let (_dir, pool, app) = fresh_app().await;

    let unreachable = nowhere().await;
    subscribe(&app, &Device::new(&unreachable, "take", "phone")).await;

    let (status, created) = post_set(&app, SET).await;

    assert_eq!(
        status,
        StatusCode::CREATED,
        "a push service that cannot be reached costs a notification, not the Set",
    );
    assert!(store::set_exists(&pool, created.id).await.unwrap());

    // And the device stays on the list: nothing said it was gone.
    settle().await;
    assert_eq!(store::push_subscriptions(&pool).await.unwrap().len(), 1);
}

#[tokio::test]
async fn a_device_the_push_service_has_finished_with_is_dropped() {
    let (service, received) = push_service().await;
    let (_dir, pool, app) = fresh_app().await;

    let gone = Device::new(&service, "gone", "gone");
    let missing = Device::new(&service, "missing", "missing");
    let flaky = Device::new(&service, "flaky", "flaky");
    let phone = Device::new(&service, "take", "phone");

    for device in [&gone, &missing, &flaky, &phone] {
        subscribe(&app, device).await;
    }

    post_set(&app, SET).await;
    pushes(&received, 4).await;

    // The two the service said were finished, and only those: a 503 is a
    // notification lost, not a device that has gone away.
    subscriptions_settle_to(&pool, &[&flaky.endpoint, &phone.endpoint]).await;
}

#[tokio::test]
async fn answering_or_archiving_a_set_sends_nothing() {
    let (service, received) = push_service().await;
    let (_dir, _pool, app) = fresh_app().await;

    subscribe(&app, &Device::new(&service, "take", "phone")).await;

    let (_, answered) = post_set(&app, SET).await;
    let (_, closed) = post_set(&app, SET).await;
    pushes(&received, 2).await;

    answer(&app, answered.id).await;
    archive(&app, closed.id).await;
    settle().await;

    assert_eq!(
        received.lock().unwrap().len(),
        2,
        "one notification per new Set — no reminders and no follow-ups",
    );
}
