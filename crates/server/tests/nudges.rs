//! The Nudge stream: telling the open viewer pages that the pending world
//! moved.
//!
//! Read off the wire rather than off the channel behind it, because what has to
//! hold is that a page listening over HTTP hears each durable change — the
//! three that move a Set on or off the pending list, and nothing else. A Nudge
//! is contentless, so every assertion here is about *how many* arrived and
//! *when*, never about what one said.

use std::time::Duration;

use askance_schema::SetCreated;
use askance_server::{open_database, router};
use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use tower::ServiceExt;

const SET: &str = r#"
title: Rate limiting for the public API
project: askance
branch: nudge
questions:
  - label: Q1
    text: Where should the request counter live?
    options:
      - n: 1
        text: In-process, per instance.
      - n: 2
        text: In Redis, shared across instances.
"#;

/// A Response resolving every question of [`SET`].
fn decided() -> serde_json::Value {
    serde_json::json!({
        "answers": [{ "label": "Q1", "selected": 2, "unanswered": false }],
    })
}

/// How long a test will wait for a Nudge it expects: generous, because it is
/// only ever paid when the assertion is about to fail.
const PATIENCE: Duration = Duration::from_secs(5);

/// How long to keep listening after everything expected has arrived, to catch a
/// Nudge that should not have been sent at all.
const SETTLING: Duration = Duration::from_millis(300);

/// One router over a fresh database, shared by every request in a test: a page
/// listening through one clone has to hear a Set posted through another.
async fn fresh_app() -> (tempfile::TempDir, Router) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("askance.db")).await.unwrap();
    (dir, router(pool))
}

/// An open page, listening on the Nudge stream.
struct Listening {
    body: Body,

    /// What has been read off the stream and is not a whole frame yet. SSE
    /// frames are not the chunks they arrive in.
    buffered: String,
}

impl Listening {
    /// Open the stream the way a page does. Returns once the response is in
    /// hand, which is after the handler has subscribed — so anything the test
    /// does next is something this page is listening for.
    async fn open(app: &Router) -> Self {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/ui/nudges")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response
                .headers()
                .get(header::CONTENT_TYPE)
                .and_then(|kind| kind.to_str().ok()),
            Some("text/event-stream"),
        );

        Self {
            body: response.into_body(),
            buffered: String::new(),
        }
    }

    /// The next whole frame off the stream, whatever kind it is.
    async fn frame(&mut self) -> String {
        loop {
            if let Some(end) = self.buffered.find("\n\n") {
                return self.buffered.drain(..end + 2).collect();
            }

            let chunk = self
                .body
                .frame()
                .await
                .expect("the stream ended")
                .unwrap()
                .into_data()
                .expect("the stream carries data frames");

            self.buffered.push_str(std::str::from_utf8(&chunk).unwrap());
        }
    }

    /// The next Nudge, past the keep-alives that are the stream's other
    /// traffic.
    async fn nudge(&mut self) {
        let waited_for = tokio::time::timeout(PATIENCE, async {
            loop {
                let frame = self.frame().await;
                if frame.starts_with("event: nudge") {
                    return;
                }
            }
        });

        waited_for.await.expect("waited for a Nudge in vain");
    }

    /// Insist that nothing more is coming.
    async fn nothing_more(&mut self) {
        let arrived = tokio::time::timeout(SETTLING, async {
            loop {
                let frame = self.frame().await;
                if frame.starts_with("event: nudge") {
                    return frame;
                }
            }
        })
        .await;

        assert!(arrived.is_err(), "an unwanted Nudge arrived: {arrived:?}");
    }
}

async fn body_text(response: axum::response::Response) -> String {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    String::from_utf8(bytes.to_vec()).unwrap()
}

/// Send a Set the way the CLI does, and return the id the agent then waits on.
async fn post_set(app: &Router) -> i64 {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/sets")
                .header(header::CONTENT_TYPE, "application/yaml")
                .body(Body::from(SET))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    let created: SetCreated = serde_saphyr::from_str(&body_text(response).await).unwrap();
    created.id
}

/// Tell the viewer's namespace something, the way a browser would.
async fn post(app: &Router, path: &str, body: serde_json::Value) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(path)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK, "POST {path} failed");
}

#[tokio::test]
async fn a_set_arriving_nudges_the_open_pages() {
    let (_dir, app) = fresh_app().await;
    let mut page = Listening::open(&app).await;

    post_set(&app).await;

    page.nudge().await;
    page.nothing_more().await;
}

#[tokio::test]
async fn a_response_landing_nudges_the_open_pages() {
    let (_dir, app) = fresh_app().await;
    let id = post_set(&app).await;

    // Opened after the Set exists, so the only durable change it can hear about
    // is the answering.
    let mut page = Listening::open(&app).await;

    post(&app, &format!("/api/ui/sets/{id}/response"), decided()).await;

    page.nudge().await;
    page.nothing_more().await;
}

#[tokio::test]
async fn a_response_from_the_agents_half_nudges_the_open_pages_too() {
    let (_dir, app) = fresh_app().await;
    let id = post_set(&app).await;
    let mut page = Listening::open(&app).await;

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/sets/{id}/response"))
                .header(header::CONTENT_TYPE, "application/yaml")
                .body(Body::from("answers:\n  - label: Q1\n    selected: 2\n"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    page.nudge().await;
    page.nothing_more().await;
}

#[tokio::test]
async fn a_set_archived_unanswered_nudges_the_open_pages() {
    let (_dir, app) = fresh_app().await;
    let id = post_set(&app).await;
    let mut page = Listening::open(&app).await;

    post(
        &app,
        &format!("/api/ui/sets/{id}/archive"),
        serde_json::json!({}),
    )
    .await;

    page.nudge().await;
    page.nothing_more().await;
}

#[tokio::test]
async fn an_agents_wait_opening_and_closing_nudges_nobody() {
    let (_dir, app) = fresh_app().await;
    let id = post_set(&app).await;
    let mut page = Listening::open(&app).await;

    // A whole wait, from the agent taking its slot in the registry to the hold
    // window closing on it: the Liveness badge moves twice and the pending
    // world does not move at all.
    let held = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/sets/{id}/response?hold=1"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(held.status(), StatusCode::NO_CONTENT);

    page.nothing_more().await;
}

#[tokio::test]
async fn every_open_page_hears_every_nudge() {
    let (_dir, app) = fresh_app().await;

    let mut phone = Listening::open(&app).await;
    let mut laptop = Listening::open(&app).await;

    let id = post_set(&app).await;
    post(&app, &format!("/api/ui/sets/{id}/response"), decided()).await;

    for page in [&mut phone, &mut laptop] {
        page.nudge().await;
        page.nudge().await;
        page.nothing_more().await;
    }
}

#[tokio::test]
async fn the_stream_says_something_while_nothing_is_happening() {
    let (_dir, app) = fresh_app().await;
    let mut page = Listening::open(&app).await;

    // The keep-alive interval is a quarter-minute of real time, which is a
    // quarter-minute nobody should spend watching a clock: from here the test
    // is on the runtime's clock, which jumps to the next timer the moment
    // there is nothing else to do.
    tokio::time::pause();

    let frame = page.frame().await;

    assert!(
        frame.starts_with(':'),
        "a quiet stream should have carried a keep-alive, not {frame:?}",
    );
}
