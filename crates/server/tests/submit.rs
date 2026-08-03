//! Answering a Set from the browser: what the UI's submit does to a Set, and
//! to the agent still waiting on the other end of the API.
//!
//! The page's own field-gathering is unit-tested where it lives. What is worth
//! proving here is that the Response the page builds goes through the same path
//! `curl` does — that a browser submit ends a real held wait, and not merely
//! that a row appeared in the store.

use std::time::{Duration, Instant};

use askance_app::set_view::Submitted;
use askance_schema::{Answer, Response, SetCreated};
use askance_server::{open_database, router_with_ui};
use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use leptos::prelude::LeptosOptions;
use tower::ServiceExt;

/// Two Questions, one with Sub-questions, so a Response has to account for
/// `Q1`, `Q2`, `Q2a` and `Q2b` — the shape the page has to get right.
const SET: &str = r#"
title: Rate limiting for the public API
questions:
  - label: Q1
    text: Where should the request counter live?
    options:
      - n: 1
        text: In-process, per instance.
      - n: 2
        text: In Redis, shared across instances.
        recommended: true
  - label: Q2
    text: How should a throttled client be told to back off?
    subquestions:
      - letter: a
        text: What should Retry-After say?
        options:
          - n: 1
            text: The exact number of seconds.
      - letter: b
        text: Anything else about the headers?
"#;

/// Every question in [`SET`], in the order a Response has to account for them.
const QUESTIONS: [&str; 4] = ["Q1", "Q2", "Q2a", "Q2b"];

/// The UI's file locations, which these tests never reach.
fn options() -> LeptosOptions {
    LeptosOptions::builder()
        .output_name("askance")
        .site_root("target/site")
        .build()
}

/// One router shared by every request in a test — the API routes and the UI
/// over the same store and the same channel, exactly as the binary serves them.
/// A wait held on one clone must hear a submit made through another.
async fn fresh_app() -> (tempfile::TempDir, Router) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("askance.db")).await.unwrap();
    (dir, router_with_ui(pool, options()))
}

async fn body_text(response: axum::response::Response) -> String {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    String::from_utf8(bytes.to_vec()).unwrap()
}

/// Send a Set the way the CLI does, and return the id the agent then waits on.
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

/// Submit a Response the way the page does: the UI's server function, over
/// JSON, rather than the agent-facing YAML endpoint.
async fn submit_from_the_browser(app: &Router, id: i64, response: &Response) -> Submitted {
    let args = serde_json::json!({ "id": id, "response": response });

    let http = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/ui/submit-response")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_vec(&args).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    let status = http.status();
    let body = body_text(http).await;
    assert_eq!(status, StatusCode::OK, "the submit failed: {body}");

    serde_json::from_str(&body).unwrap_or_else(|err| panic!("reading {body:?}: {err}"))
}

/// Open a wait on a Set the way the CLI does, held for `hold` seconds.
async fn wait_for_response(app: &Router, id: i64, hold: u64) -> axum::response::Response {
    app.clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/sets/{id}/response?hold={hold}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
}

async fn page(app: &Router, path: &str) -> String {
    let response = app
        .clone()
        .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    body_text(response).await
}

fn answered(label: &str, selected: Option<u32>, free_text: Option<&str>) -> Answer {
    Answer {
        label: label.to_owned(),
        selected,
        free_text: free_text.map(str::to_owned),
        unanswered: false,
    }
}

fn left_open(label: &str) -> Answer {
    Answer {
        label: label.to_owned(),
        selected: None,
        free_text: None,
        unanswered: true,
    }
}

/// What the page builds when the human answers `Q1` and confirms the warning
/// about the three they left alone.
fn confirmed_with_three_left_open() -> Response {
    Response {
        answers: vec![
            answered("Q1", Some(2), None),
            left_open("Q2"),
            left_open("Q2a"),
            left_open("Q2b"),
        ],
        comment: None,
    }
}

/// A Set with every question left open and only a comment: the human answering
/// the ask with an ask of their own.
fn a_counter_question() -> Response {
    Response {
        answers: QUESTIONS.iter().map(|label| left_open(label)).collect(),
        comment: Some("Neither — why is this not just a cache in front?".to_owned()),
    }
}

#[tokio::test]
async fn a_submit_from_the_browser_ends_a_wait_that_is_genuinely_being_held() {
    let (_dir, app) = fresh_app().await;
    let id = post_set(&app, SET).await;

    // The agent's wait goes up first and finds nothing, so the only thing that
    // can end it is word of the submit — not a row it happened to read.
    let agent = tokio::spawn({
        let app = app.clone();
        async move { wait_for_response(&app, id, 30).await }
    });
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert!(
        !agent.is_finished(),
        "the wait should still be held: nothing has answered the Set yet",
    );

    let submitted = Instant::now();
    let outcome = submit_from_the_browser(&app, id, &confirmed_with_three_left_open()).await;
    assert_eq!(outcome, Submitted::Accepted);

    let waited = agent.await.unwrap();
    let woken_in = submitted.elapsed();

    assert_eq!(waited.status(), StatusCode::OK);
    assert!(
        woken_in < Duration::from_secs(5),
        "the wait should have been woken by the submit, not left to time out; \
         it took {woken_in:?} of its 30s hold",
    );

    // And what the agent gets is the Response the human confirmed, unanswered
    // markers and all.
    let response = Response::from_yaml(&body_text(waited).await).unwrap();
    assert_eq!(response, confirmed_with_three_left_open());
}

#[tokio::test]
async fn every_question_left_alone_reaches_the_agent_marked_unanswered() {
    let (_dir, app) = fresh_app().await;
    let id = post_set(&app, SET).await;

    submit_from_the_browser(&app, id, &confirmed_with_three_left_open()).await;

    let response =
        Response::from_yaml(&body_text(wait_for_response(&app, id, 0).await).await).unwrap();

    let open: Vec<&str> = response
        .answers
        .iter()
        .filter(|answer| answer.unanswered)
        .map(|answer| answer.label.as_str())
        .collect();
    assert_eq!(
        open,
        ["Q2", "Q2a", "Q2b"],
        "the questions the human confirmed leaving open have to say so out loud",
    );
}

#[tokio::test]
async fn a_comment_and_no_answers_at_all_round_trips_to_the_agent() {
    let (_dir, app) = fresh_app().await;
    let id = post_set(&app, SET).await;

    // A counter-question is a legitimate reply, not a Response to refuse.
    let outcome = submit_from_the_browser(&app, id, &a_counter_question()).await;
    assert_eq!(outcome, Submitted::Accepted);

    let response =
        Response::from_yaml(&body_text(wait_for_response(&app, id, 0).await).await).unwrap();
    assert_eq!(response, a_counter_question());
}

#[tokio::test]
async fn an_answered_set_is_gone_from_the_pending_list() {
    let (_dir, app) = fresh_app().await;
    let id = post_set(&app, SET).await;

    assert!(
        page(&app, "/")
            .await
            .contains("Rate limiting for the public API"),
        "the Set should be waiting on the human before it is answered",
    );

    submit_from_the_browser(&app, id, &confirmed_with_three_left_open()).await;

    // Where the page navigates to once the submit lands. The Set's absence is
    // the confirmation that the agent has its answer.
    let pending = page(&app, "/").await;
    assert!(
        !pending.contains("Rate limiting for the public API"),
        "an answered Set is not waiting on anyone:\n{pending}",
    );
}

#[tokio::test]
async fn a_second_submit_is_told_the_set_was_already_answered() {
    let (_dir, app) = fresh_app().await;
    let id = post_set(&app, SET).await;

    submit_from_the_browser(&app, id, &confirmed_with_three_left_open()).await;

    // Two devices, or one page left open in a second tab. The second Response
    // is refused rather than quietly dropped, and the first one stands.
    let outcome = submit_from_the_browser(&app, id, &a_counter_question()).await;
    assert_eq!(outcome, Submitted::AlreadyAnswered);

    let response =
        Response::from_yaml(&body_text(wait_for_response(&app, id, 0).await).await).unwrap();
    assert_eq!(
        response,
        confirmed_with_three_left_open(),
        "the first Response is the Set's answer",
    );
}

#[tokio::test]
async fn a_response_that_misses_a_question_comes_back_naming_it() {
    let (_dir, app) = fresh_app().await;
    let id = post_set(&app, SET).await;

    // The page builds Responses that account for every question, so this is a
    // bug rather than something the human can do — which is exactly why it has
    // to be visible instead of swallowed.
    let incomplete = Response {
        answers: vec![answered("Q1", Some(1), None)],
        comment: None,
    };

    let Submitted::Rejected(violations) = submit_from_the_browser(&app, id, &incomplete).await
    else {
        panic!("a Response missing three questions should not have been taken");
    };

    for missing in ["Q2", "Q2a", "Q2b"] {
        assert!(
            violations
                .iter()
                .any(|violation| violation.contains(missing)),
            "expected {missing} named among {violations:?}",
        );
    }
}
