//! Answering a Question Set: what the API accepts as a Response, and how it
//! reaches the agent still waiting on the other end.

use std::time::{Duration, Instant};

use askance_schema::{ApiError, Response, ResponseAccepted, SetCreated};
use askance_server::store;
use askance_server::{open_database, router};
use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use sqlx::SqlitePool;
use tower::ServiceExt;

/// Two Questions, one of them with Sub-questions, so a Response has to cover
/// `Q1`, `Q2`, `Q2a` and `Q2b`.
const SET: &str = r#"
title: How should the wait end?
questions:
  - label: Q1
    text: How long should the hold window be?
    options:
      - n: 1
        text: Thirty seconds
        recommended: true
      - n: 2
        text: Five minutes
  - label: Q2
    text: What comes back when nothing has been answered yet?
    subquestions:
      - letter: a
        text: Which status?
        options:
          - n: 1
            text: 204 No Content
          - n: 2
            text: 200 with a pending document
      - letter: b
        text: Anything else worth saying in the reply?
"#;

/// A pool over a fresh database, plus the directory keeping it alive.
async fn fresh_pool() -> (tempfile::TempDir, SqlitePool) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("askance.db")).await.unwrap();
    (dir, pool)
}

/// One router shared by every request in a test: a wait held on one clone must
/// hear a submission made through another.
async fn fresh_app() -> (tempfile::TempDir, SqlitePool, Router) {
    let (dir, pool) = fresh_pool().await;
    let app = router(pool.clone());
    (dir, pool, app)
}

async fn post_set(app: &Router, yaml: &str) -> i64 {
    let response = app
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
    assert_eq!(response.status(), StatusCode::CREATED);

    let created: SetCreated = serde_saphyr::from_str(&body_text(response).await).unwrap();
    created.id
}

async fn post_response(app: &Router, set_id: i64, yaml: &str) -> axum::response::Response {
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/sets/{set_id}/response"))
                .header(header::CONTENT_TYPE, "application/yaml")
                .body(Body::from(yaml.to_owned()))
                .unwrap(),
        )
        .await
        .unwrap()
}

async fn wait_for_response(app: &Router, set_id: i64, hold: u64) -> axum::response::Response {
    app.clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/sets/{set_id}/response?hold={hold}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
}

async fn body_text(response: axum::response::Response) -> String {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    String::from_utf8(bytes.to_vec()).unwrap()
}

async fn stored_response_count(pool: &SqlitePool) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM responses")
        .fetch_one(pool)
        .await
        .unwrap()
}

/// A Response answering every question in [`SET`].
const COMPLETE: &str = "
answers:
  - label: Q1
    selected: 1
  - label: Q2a
    selected: 1
    free_text: Empty body, no ambiguity.
  - label: Q2b
    free_text: Say nothing; the status is the message.
";

#[tokio::test]
async fn a_response_omitting_a_question_is_refused_naming_it() {
    let (_dir, pool, app) = fresh_app().await;
    let id = post_set(&app, SET).await;

    let response = post_response(
        &app,
        id,
        "
answers:
  - label: Q1
    selected: 1
  - label: Q2a
    selected: 1
",
    )
    .await;

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let error: ApiError = serde_saphyr::from_str(&body_text(response).await).unwrap();
    assert!(
        error
            .violations
            .iter()
            .any(|v| v.label.as_deref() == Some("Q2b")),
        "the refusal should name the missing Q2b, got {error:?}"
    );
    assert_eq!(stored_response_count(&pool).await, 0);
}

#[tokio::test]
async fn a_response_leaving_every_question_unanswered_is_accepted() {
    let (_dir, pool, app) = fresh_app().await;
    let id = post_set(&app, SET).await;

    let response = post_response(
        &app,
        id,
        "
answers:
  - label: Q1
    unanswered: true
  - label: Q2a
    unanswered: true
  - label: Q2b
    unanswered: true
comment: |
  None of these are the real question. Why is the CLI holding the
  connection at all, rather than the agent polling?
",
    )
    .await;

    assert_eq!(response.status(), StatusCode::CREATED);
    let accepted: ResponseAccepted = serde_saphyr::from_str(&body_text(response).await).unwrap();
    assert_eq!(accepted.set_id, id);
    assert!(
        accepted.submitted_at.starts_with("20"),
        "the server should stamp an RFC 3339 submitted_at, got {:?}",
        accepted.submitted_at
    );

    let stored = store::load_response(&pool, id).await.unwrap().unwrap();
    assert!(
        stored.response.answers.iter().all(|a| !a.is_answer()),
        "a counter-question carries no Answers at all"
    );
    assert!(stored.response.comment.unwrap().contains("real question"));
}

#[tokio::test]
async fn a_wait_opened_before_submission_receives_the_response() {
    let (_dir, _pool, app) = fresh_app().await;
    let id = post_set(&app, SET).await;

    let waiting = tokio::spawn({
        let app = app.clone();
        async move { wait_for_response(&app, id, 30).await }
    });

    // Long enough for the wait to be parked on the notification rather than
    // still reading the store.
    tokio::time::sleep(Duration::from_millis(150)).await;

    let submitted = post_response(&app, id, COMPLETE).await;
    assert_eq!(submitted.status(), StatusCode::CREATED);

    let delivered = tokio::time::timeout(Duration::from_secs(5), waiting)
        .await
        .expect("the held wait should be woken by the submission")
        .unwrap();

    assert_eq!(delivered.status(), StatusCode::OK);
    let response: Response = serde_saphyr::from_str(&body_text(delivered).await).unwrap();
    assert_eq!(response.answers.len(), 3);
    assert_eq!(response.answers[0].selected, Some(1));
}

#[tokio::test]
async fn a_wait_opened_after_submission_receives_the_response_immediately() {
    let (_dir, _pool, app) = fresh_app().await;
    let id = post_set(&app, SET).await;
    assert_eq!(
        post_response(&app, id, COMPLETE).await.status(),
        StatusCode::CREATED
    );

    let started = Instant::now();
    let delivered = wait_for_response(&app, id, 30).await;

    assert_eq!(delivered.status(), StatusCode::OK);
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "an already-answered Set should not be held at all"
    );
    assert_eq!(
        delivered
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok()),
        Some("application/yaml"),
    );

    let response: Response = serde_saphyr::from_str(&body_text(delivered).await).unwrap();
    assert_eq!(response.answers[2].label, "Q2b");
    assert_eq!(
        response.answers[2].free_text.as_deref(),
        Some("Say nothing; the status is the message.")
    );
}

#[tokio::test]
async fn a_wait_on_an_unanswered_set_gives_up_when_the_hold_window_closes() {
    let (_dir, _pool, app) = fresh_app().await;
    let id = post_set(&app, SET).await;

    let started = Instant::now();
    let response = wait_for_response(&app, id, 1).await;
    let held = started.elapsed();

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    assert!(
        held >= Duration::from_millis(900),
        "the wait should have been held for the window, but returned after {held:?}"
    );
    assert!(
        held < Duration::from_secs(10),
        "the wait should not hang past its window, but took {held:?}"
    );
    assert!(body_text(response).await.is_empty());
}

#[tokio::test]
async fn a_hold_of_zero_makes_the_wait_a_plain_poll() {
    let (_dir, _pool, app) = fresh_app().await;
    let id = post_set(&app, SET).await;

    let started = Instant::now();
    let response = wait_for_response(&app, id, 0).await;

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    assert!(started.elapsed() < Duration::from_secs(1));
}

#[tokio::test]
async fn a_second_response_is_refused_and_the_first_one_stands() {
    let (_dir, pool, app) = fresh_app().await;
    let id = post_set(&app, SET).await;
    assert_eq!(
        post_response(&app, id, COMPLETE).await.status(),
        StatusCode::CREATED
    );

    let second = post_response(
        &app,
        id,
        "
answers:
  - label: Q1
    selected: 2
  - label: Q2a
    selected: 2
  - label: Q2b
    unanswered: true
",
    )
    .await;

    assert_eq!(second.status(), StatusCode::CONFLICT);
    assert_eq!(stored_response_count(&pool).await, 1);

    let stored = store::load_response(&pool, id).await.unwrap().unwrap();
    assert_eq!(stored.response.answers[0].selected, Some(1));
}

#[tokio::test]
async fn answering_an_unknown_set_is_not_found() {
    let (_dir, pool, app) = fresh_app().await;

    let submitted = post_response(&app, 404, COMPLETE).await;
    assert_eq!(submitted.status(), StatusCode::NOT_FOUND);
    assert_eq!(stored_response_count(&pool).await, 0);

    let waited = wait_for_response(&app, 404, 30).await;
    assert_eq!(waited.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn an_answer_naming_no_question_in_the_set_is_refused() {
    let (_dir, pool, app) = fresh_app().await;
    let id = post_set(&app, SET).await;

    let response = post_response(
        &app,
        id,
        "
answers:
  - label: Q1
    selected: 1
  - label: Q2a
    selected: 1
  - label: Q2b
    unanswered: true
  - label: Q9
    selected: 1
",
    )
    .await;

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let error: ApiError = serde_saphyr::from_str(&body_text(response).await).unwrap();
    assert!(
        error
            .violations
            .iter()
            .any(|v| v.label.as_deref() == Some("Q9")),
        "the refusal should name Q9, got {error:?}"
    );
    assert_eq!(stored_response_count(&pool).await, 0);
}

#[tokio::test]
async fn a_body_that_is_not_yaml_is_refused_as_malformed() {
    let (_dir, pool, app) = fresh_app().await;
    let id = post_set(&app, SET).await;

    let response = post_response(&app, id, "{{{ not yaml").await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(stored_response_count(&pool).await, 0);
}

#[tokio::test]
async fn a_multi_line_comment_survives_the_store_byte_for_byte() {
    let (_dir, pool, app) = fresh_app().await;
    let id = post_set(&app, SET).await;

    let response = post_response(
        &app,
        id,
        "
answers:
  - label: Q1
    selected: 1
  - label: Q2a
    selected: 1
  - label: Q2b
    unanswered: true
comment: |
  Two paragraphs, the second with `code` and a \"quote\".

      an indented line
",
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    let stored = store::load_response(&pool, id).await.unwrap().unwrap();
    assert_eq!(
        stored.response.comment.as_deref(),
        Some("Two paragraphs, the second with `code` and a \"quote\".\n\n    an indented line\n")
    );
}

#[tokio::test]
async fn a_wait_opened_on_a_set_that_was_closed_unanswered_is_told_straight_away() {
    let (_dir, pool, app) = fresh_app().await;
    let id = post_set(&app, SET).await;
    close_unanswered(&pool, id).await;

    let waited = wait_for_response(&app, id, 30).await;

    assert_eq!(
        waited.status(),
        StatusCode::GONE,
        "there is nothing to hold a connection open for",
    );
    assert!(
        body_text(waited).await.contains("archived unanswered"),
        "the reply has to say why there is nothing coming",
    );
}

#[tokio::test]
async fn a_response_to_a_set_that_was_closed_unanswered_is_refused() {
    let (_dir, pool, app) = fresh_app().await;
    let id = post_set(&app, SET).await;
    close_unanswered(&pool, id).await;

    // A Response that does resolve the Set, so what refuses it is the Set being
    // closed and nothing else.
    let refused = post_response(&app, id, COMPLETE).await;

    assert_eq!(
        refused.status(),
        StatusCode::GONE,
        "an archived Set cannot also become an answered one",
    );
    assert_eq!(stored_response_count(&pool).await, 0);
}

/// Archive a Set unanswered, through the store rather than through the viewer's
/// endpoint: only a human may close a Set, and how they did it is not what these
/// two are about. Its own channel, because no wait is being held over one here.
async fn close_unanswered(pool: &SqlitePool, id: i64) {
    store::archive_set(pool, &store::Settlements::new(1), id)
        .await
        .unwrap();
}

#[tokio::test]
async fn a_response_outlives_a_restart() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("askance.db");

    let pool = open_database(&path).await.unwrap();
    let app = router(pool.clone());
    let id = post_set(&app, SET).await;
    assert_eq!(
        post_response(&app, id, COMPLETE).await.status(),
        StatusCode::CREATED
    );
    pool.close().await;

    let pool = open_database(&path).await.unwrap();
    let delivered = wait_for_response(&router(pool), id, 30).await;

    assert_eq!(delivered.status(), StatusCode::OK);
}
