//! The badge on the pending list, over the wait that the agent API is holding:
//! one router, an agent's long-poll on one side of it and the human's page on
//! the other.

use std::time::{Duration, Instant};

use askance_schema::QuestionSet;
use askance_server::{open_database, router_with_ui, store};
use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use leptos::prelude::LeptosOptions;
use sqlx::SqlitePool;
use tower::ServiceExt;

/// A pool over a fresh database, plus the directory keeping it alive.
async fn fresh_pool() -> (tempfile::TempDir, SqlitePool) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("askance.db")).await.unwrap();
    (dir, pool)
}

/// The UI's file locations, which these tests never reach: they ask for a
/// rendered page, not for anything under the site root.
fn options() -> LeptosOptions {
    LeptosOptions::builder()
        .output_name("askance")
        .site_root("target/site")
        .build()
}

/// One router shared by every request in a test: the page has to see the waits
/// held on the same server, so a fresh router per request would see none.
async fn fresh_app() -> (tempfile::TempDir, SqlitePool, Router) {
    let (dir, pool) = fresh_pool().await;
    let app = router_with_ui(pool.clone(), options());
    (dir, pool, app)
}

fn set(title: &str) -> QuestionSet {
    QuestionSet {
        title: title.to_owned(),
        preface: None,
        questions: Vec::new(),
        project: Some("askance".to_owned()),
        branch: Some("answering-conveniences".to_owned()),
        diff: None,
    }
}

/// Renders take turns — see the note in `set_page.rs`: two server-side renders at
/// once can deadlock inside leptos's reactive graph (leptos-rs/leptos#4673). Only
/// the page is held: the wait a test holds open on the agent API is not a render,
/// and queueing that would stop the page ever seeing it.
static ONE_RENDER_AT_A_TIME: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

async fn pending_page(app: &Router) -> String {
    let _turn = ONE_RENDER_AT_A_TIME.lock().await;

    let response = app
        .clone()
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    String::from_utf8(body.to_vec()).unwrap()
}

/// The pending page, asked for until it says `needle`. A wait takes its slot as
/// its handler starts running, which is a moment after the request opening it
/// goes in — so the page is asked again rather than once after a guessed pause.
async fn pending_page_saying(app: &Router, needle: &str) -> String {
    let deadline = Instant::now() + Duration::from_secs(5);

    loop {
        let html = pending_page(app).await;
        if html.contains(needle) {
            return html;
        }
        assert!(
            Instant::now() < deadline,
            "waited for {needle:?} in the pending page in vain:\n{html}"
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

/// Hold a wait on a Set the way the CLI does, on a task the test can drop.
fn hold_a_wait(app: &Router, set_id: i64) -> tokio::task::JoinHandle<()> {
    let app = app.clone();

    tokio::spawn(async move {
        let _held = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/v1/sets/{set_id}/response?hold=60"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await;
    })
}

/// Push a Set's creation back beyond any grace window, so a test can ask what a
/// Set nobody has waited on for a while reads as without waiting one out.
async fn backdate(pool: &SqlitePool, id: i64) {
    sqlx::query(
        "UPDATE question_sets
         SET created_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now', '-5 minutes')
         WHERE id = ?",
    )
    .bind(id)
    .execute(pool)
    .await
    .unwrap();
}

#[tokio::test]
async fn a_set_with_a_wait_held_on_it_reads_as_waiting() {
    let (_dir, pool, app) = fresh_app().await;
    let stored = store::insert_set(&pool, &set("Which window?"))
        .await
        .unwrap();
    backdate(&pool, stored.id).await;

    let waiting = hold_a_wait(&app, stored.id);

    let html = pending_page_saying(&app, "agent waiting").await;
    assert!(
        !html.contains("agent disconnected"),
        "the badge should not say both:\n{html}"
    );

    waiting.abort();
}

#[tokio::test]
async fn a_set_nothing_is_waiting_on_reads_as_disconnected() {
    let (_dir, pool, app) = fresh_app().await;
    let stored = store::insert_set(&pool, &set("Which window?"))
        .await
        .unwrap();
    backdate(&pool, stored.id).await;

    let html = pending_page(&app).await;

    assert!(
        html.contains("agent disconnected"),
        "expected the badge in the page:\n{html}"
    );
}

#[tokio::test]
async fn a_set_submitted_a_moment_ago_is_not_born_disconnected() {
    let (_dir, pool, app) = fresh_app().await;
    store::insert_set(&pool, &set("Which window?"))
        .await
        .unwrap();

    let html = pending_page(&app).await;

    assert!(
        !html.contains("agent disconnected"),
        "a Set whose agent has not opened its first wait yet is not disconnected:\n{html}"
    );
    assert!(
        html.contains("agent waiting"),
        "expected the badge in the page:\n{html}"
    );
}

#[tokio::test]
async fn a_disconnected_set_is_still_answerable() {
    let (_dir, pool, app) = fresh_app().await;
    let stored = store::insert_set(&pool, &set("Which window?"))
        .await
        .unwrap();
    backdate(&pool, stored.id).await;

    let html = pending_page(&app).await;
    assert!(html.contains("agent disconnected"));

    let answered = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/sets/{}/response", stored.id))
                .header(header::CONTENT_TYPE, "application/yaml")
                .body(Body::from("comment: answered into the void, apparently\n"))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        answered.status(),
        StatusCode::CREATED,
        "Liveness never gates an answer"
    );

    let html = pending_page(&app).await;
    assert!(
        !html.contains("Which window?"),
        "the answered Set should be off the list:\n{html}"
    );
}

#[tokio::test]
async fn a_wait_held_on_one_set_says_nothing_about_another() {
    let (_dir, pool, app) = fresh_app().await;
    let waited_on = store::insert_set(&pool, &set("the one with an agent"))
        .await
        .unwrap();
    let orphan = store::insert_set(&pool, &set("the one whose agent went"))
        .await
        .unwrap();
    backdate(&pool, waited_on.id).await;
    backdate(&pool, orphan.id).await;

    let waiting = hold_a_wait(&app, waited_on.id);

    let html = pending_page_saying(&app, "agent waiting").await;
    assert!(
        html.contains("agent disconnected"),
        "the Set nothing is waiting on keeps its own badge:\n{html}"
    );

    waiting.abort();
}
