//! Archiving a Set unanswered: what the human's page offers, what it does to the
//! two lists, and what everyone still holding on to the Set is told.
//!
//! One router throughout, as the binary serves it: the archiving goes in through
//! the UI's own endpoint — the only way in, because only a human may archive a
//! Set — and the agent waiting on the REST endpoint has to hear about it there.

use std::time::{Duration, Instant};

use askance_app::set_view::{Archived, Submitted};
use askance_schema::{Answer, Question, QuestionOption, QuestionSet, Response};
use askance_server::{open_database, router_with_ui, store};
use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use leptos::prelude::LeptosOptions;
use sqlx::SqlitePool;
use tower::ServiceExt;

/// The UI's file locations, which these tests never reach: they ask for a
/// rendered page, not for anything under the site root.
fn options() -> LeptosOptions {
    LeptosOptions::builder()
        .output_name("askance")
        .site_root("target/site")
        .build()
}

/// One router shared by every request in a test, over a fresh database: a wait
/// held on one clone has to hear an archiving made through another.
async fn fresh_app() -> (tempfile::TempDir, SqlitePool, Router) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("askance.db")).await.unwrap();
    let app = router_with_ui(pool.clone(), options());
    (dir, pool, app)
}

/// A Set with something to read on it either way: a Preface, and a Question whose
/// Options an archived Set still has to show.
fn set(title: &str) -> QuestionSet {
    QuestionSet {
        title: title.to_owned(),
        preface: Some("The agent asking this has since been killed.\n".to_owned()),
        questions: vec![Question {
            label: "Q1".to_owned(),
            text: "Where should the request counter live?".to_owned(),
            options: vec![
                QuestionOption {
                    n: 1,
                    text: "In-process, per instance.".to_owned(),
                    recommended: false,
                },
                QuestionOption {
                    n: 2,
                    text: "In Redis, shared across instances.".to_owned(),
                    recommended: true,
                },
            ],
            subquestions: Vec::new(),
        }],
        project: Some("askance".to_owned()),
        branch: Some("answering-conveniences".to_owned()),
        diff: None,
    }
}

/// A Response resolving [`set`], for the tests about a Set that was answered
/// rather than orphaned.
fn decided() -> Response {
    Response {
        answers: vec![Answer {
            label: "Q1".to_owned(),
            selected: Some(1),
            free_text: None,
            unanswered: false,
        }],
        comment: None,
    }
}

/// Renders take turns — see the note in `set_page.rs`: two server-side renders at
/// once can deadlock inside leptos's reactive graph (leptos-rs/leptos#4673). Only
/// the pages are held; a wait on the agent API is not a render, and queueing that
/// would stop the page ever seeing it.
static ONE_RENDER_AT_A_TIME: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

async fn body_text(response: axum::response::Response) -> String {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    String::from_utf8(bytes.to_vec()).unwrap()
}

async fn page(app: &Router, path: &str) -> String {
    let _turn = ONE_RENDER_AT_A_TIME.lock().await;

    let response = app
        .clone()
        .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    body_text(response).await
}

/// Archive a Set the way the human's browser does: the UI's own endpoint, over
/// JSON. The agent API has no route for this at all.
async fn archive_from_the_browser(app: &Router, id: i64) -> Archived {
    let http = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/ui/archive-set")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::json!({ "id": id }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    let status = http.status();
    let body = body_text(http).await;
    assert_eq!(status, StatusCode::OK, "the archiving failed: {body}");

    serde_json::from_str(&body).unwrap_or_else(|err| panic!("reading {body:?}: {err}"))
}

/// Submit a Response the way the page does, rather than through the agent-facing
/// YAML endpoint.
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

/// Hold a wait on a Set the way the CLI does, on a task the test can await.
fn hold_a_wait(app: &Router, set_id: i64) -> tokio::task::JoinHandle<axum::response::Response> {
    let app = app.clone();

    tokio::spawn(async move {
        app.oneshot(
            Request::builder()
                .uri(format!("/api/v1/sets/{set_id}/response?hold=30"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
    })
}

/// Push a Set's creation back beyond the grace window, so it reads as the orphan
/// it is standing in for without a test waiting one out.
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
async fn the_set_view_of_a_pending_set_offers_to_archive_it_beside_its_liveness() {
    let (_dir, pool, app) = fresh_app().await;
    let stored = store::insert_set(&pool, &set("the one whose agent died"))
        .await
        .unwrap();
    backdate(&pool, stored.id).await;

    let html = page(&app, &format!("/sets/{}", stored.id)).await;

    assert!(
        html.contains("agent disconnected"),
        "the decision is made with the badge in front of the human:\n{html}"
    );
    assert!(
        html.contains("Archive unanswered"),
        "expected the archive action on the set view:\n{html}"
    );

    // The confirmation it opens is the browser's — it is not drawn until the
    // human presses this, exactly as the warning before a submit is not. What it
    // has to say is held to where it lives, in `set_view`'s own tests.
}

#[tokio::test]
async fn an_answered_set_offers_no_archiving_because_there_is_nothing_orphaned_about_it() {
    let (_dir, pool, app) = fresh_app().await;
    let stored = store::insert_set(&pool, &set("already answered"))
        .await
        .unwrap();
    submit_from_the_browser(&app, stored.id, &decided()).await;

    let html = page(&app, &format!("/sets/{}", stored.id)).await;

    for absent in ["Archive unanswered", "liveness"] {
        assert!(
            !html.contains(absent),
            "an answered Set is settled and nothing is waiting on it, so {absent} \
             has no business on the page:\n{html}"
        );
    }
}

#[tokio::test]
async fn archiving_takes_a_set_off_the_pending_list_and_files_it_unanswered() {
    let (_dir, pool, app) = fresh_app().await;
    let orphan = store::insert_set(&pool, &set("the one whose agent died"))
        .await
        .unwrap();
    let waiting = store::insert_set(&pool, &set("still waiting"))
        .await
        .unwrap();

    assert_eq!(
        archive_from_the_browser(&app, orphan.id).await,
        Archived::Closed
    );

    let pending = page(&app, "/").await;
    assert!(
        !pending.contains("the one whose agent died"),
        "an archived Set is not waiting on anyone:\n{pending}"
    );
    assert!(
        pending.contains("still waiting"),
        "and the Sets that are still waiting are untouched:\n{pending}"
    );

    let archive = page(&app, "/archive").await;
    assert!(
        archive.contains("the one whose agent died"),
        "it was filed, not discarded:\n{archive}"
    );
    assert!(
        archive.contains("archived unanswered"),
        "the Archive has to say it was never answered, not show it as a decision:\n{archive}"
    );
    assert!(
        archive.contains(&format!(r#"href="/sets/{}""#, orphan.id)),
        "and it is still readable:\n{archive}"
    );

    // Left alone in the store as well as on the page.
    assert_eq!(
        store::pending_sets(&pool)
            .await
            .unwrap()
            .into_iter()
            .map(|entry| entry.id)
            .collect::<Vec<_>>(),
        [waiting.id],
    );
}

#[tokio::test]
async fn an_answered_set_is_still_filed_as_the_decision_it_is() {
    let (_dir, pool, app) = fresh_app().await;
    let stored = store::insert_set(&pool, &set("already answered"))
        .await
        .unwrap();
    submit_from_the_browser(&app, stored.id, &decided()).await;

    assert_eq!(
        archive_from_the_browser(&app, stored.id).await,
        Archived::AlreadyAnswered,
        "archiving unanswered is for a Set nobody will answer, not a way to unmake a decision",
    );

    let archive = page(&app, "/archive").await;
    assert!(
        archive.contains("already answered"),
        "expected the decision still in the Archive:\n{archive}"
    );
    assert!(
        !archive.contains("archived unanswered"),
        "and still filed as answered:\n{archive}"
    );
}

#[tokio::test]
async fn an_archived_sets_page_is_the_ask_kept_readable_with_nothing_to_press() {
    let (_dir, pool, app) = fresh_app().await;
    let orphan = store::insert_set(&pool, &set("the one whose agent died"))
        .await
        .unwrap();
    archive_from_the_browser(&app, orphan.id).await;

    let html = page(&app, &format!("/sets/{}", orphan.id)).await;

    assert!(
        html.contains("Archived unanswered"),
        "the page has to say what became of it:\n{html}"
    );
    assert!(
        html.contains("The agent asking this has since been killed."),
        "expected the Preface still readable:\n{html}"
    );
    assert!(
        html.contains("Where should the request counter live?")
            && html.contains("In Redis, shared across instances."),
        "expected the Questions and their Options still readable:\n{html}"
    );
    for absent in ["<input", "<textarea"] {
        assert!(
            !html.contains(absent),
            "an archived Set can never be answered, so {absent} has no business \
             on the page:\n{html}"
        );
    }
    // The nav's bar is a button, and the only one an archived Set has: a way
    // around the record rather than anything that acts on it. Counted rather than
    // excused, so a button that does act on the Set still fails this.
    assert_eq!(
        html.matches("<button").count(),
        1,
        "expected the nav's bar and nothing else to press:\n{html}"
    );
    assert!(
        html.contains(r#"class="contents-bar""#),
        "and that one button to be the nav's:\n{html}"
    );
    assert!(
        !html.contains("the agent was told this one is still open"),
        "nobody was told anything: there was no Response to tell them with:\n{html}"
    );
    assert!(
        html.contains(r#"href="/archive""#),
        "an archived Set is read from the Archive, so that is the way back:\n{html}"
    );
}

#[tokio::test]
async fn a_wait_held_on_a_set_that_is_then_archived_ends_with_410() {
    let (_dir, pool, app) = fresh_app().await;
    let orphan = store::insert_set(&pool, &set("the one whose agent lingered"))
        .await
        .unwrap();

    // The wait goes up first and finds nothing, so the only thing that can end it
    // is word of the archiving.
    let agent = hold_a_wait(&app, orphan.id);
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert!(
        !agent.is_finished(),
        "the wait should still be held: the Set is not settled yet",
    );

    let archived = Instant::now();
    archive_from_the_browser(&app, orphan.id).await;

    let waited = agent.await.unwrap();
    let told_in = archived.elapsed();

    assert_eq!(
        waited.status(),
        StatusCode::GONE,
        "a Set nobody will ever answer is not 'nothing yet'",
    );
    assert!(
        told_in < Duration::from_secs(5),
        "the wait should have been ended by the archiving, not left to time out; \
         it took {told_in:?} of its 30s hold",
    );
    assert!(
        body_text(waited).await.contains("archived unanswered"),
        "the reply has to say why the wait is over",
    );
}

#[tokio::test]
async fn a_wait_opened_on_an_already_archived_set_is_told_straight_away() {
    let (_dir, pool, app) = fresh_app().await;
    let orphan = store::insert_set(&pool, &set("closed unanswered"))
        .await
        .unwrap();
    archive_from_the_browser(&app, orphan.id).await;

    let waited = hold_a_wait(&app, orphan.id).await.unwrap();

    assert_eq!(
        waited.status(),
        StatusCode::GONE,
        "there is nothing to hold a connection open for",
    );
}

#[tokio::test]
async fn a_response_to_an_archived_set_is_refused_from_the_api_and_from_the_browser() {
    let (_dir, pool, app) = fresh_app().await;
    let orphan = store::insert_set(&pool, &set("closed unanswered"))
        .await
        .unwrap();
    archive_from_the_browser(&app, orphan.id).await;

    // A Response that does resolve the Set, so what refuses it is the Set being
    // closed and nothing else.
    let curled = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/sets/{}/response", orphan.id))
                .header(header::CONTENT_TYPE, "application/yaml")
                .body(Body::from(
                    "answers:\n  - label: Q1\n    selected: 1\ncomment: answered into the void\n",
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        curled.status(),
        StatusCode::GONE,
        "an archived Set cannot also become an answered one",
    );

    assert_eq!(
        submit_from_the_browser(&app, orphan.id, &decided()).await,
        Submitted::Archived,
        "and the browser goes through the same path, so it is refused the same way",
    );

    let archive = page(&app, "/archive").await;
    assert!(
        archive.contains("archived unanswered"),
        "the Set is what it was: closed, unanswered:\n{archive}"
    );
}
