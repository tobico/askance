//! The Archive as the human's browser gets it: server-rendered, the decision
//! log of every Set that has been answered.

use askance_schema::{Answer, Question, QuestionOption, QuestionSet, Response};
use askance_server::{open_database, router_with_ui, store};
use axum::body::Body;
use axum::http::{Request, StatusCode};
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

fn set(title: &str) -> QuestionSet {
    QuestionSet {
        title: title.to_owned(),
        preface: None,
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

/// A Response resolving [`set`]: one Option chosen, with a word about the Set.
fn decided() -> Response {
    Response {
        answers: vec![Answer {
            label: "Q1".to_owned(),
            selected: Some(1),
            free_text: None,
            unanswered: false,
        }],
        comment: Some("Do the in-process one first; we can move it later.".to_owned()),
    }
}

/// Renders take turns — see the note in `set_page.rs`: two server-side renders at
/// once can deadlock inside leptos's reactive graph (leptos-rs/leptos#4673), and
/// asking for one page at a time costs these tests nothing.
static ONE_RENDER_AT_A_TIME: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

async fn page(pool: &SqlitePool, path: &str) -> String {
    let _turn = ONE_RENDER_AT_A_TIME.lock().await;

    let response = router_with_ui(pool.clone(), options())
        .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    String::from_utf8(body.to_vec()).unwrap()
}

/// Store a Set and answer it, which is the one way a Set reaches the Archive.
/// Comes back with its id and the time the Response landed.
async fn answered(pool: &SqlitePool, title: &str, response: &Response) -> (i64, String) {
    let stored = store::insert_set(pool, &set(title)).await.unwrap();
    let accepted = store::insert_response(pool, stored.id, response)
        .await
        .unwrap()
        .expect("a freshly stored Set has no Response yet");

    (stored.id, accepted.submitted_at)
}

#[tokio::test]
async fn an_answered_set_is_rendered_into_the_archive_with_the_day_it_was_decided() {
    let (_dir, pool) = fresh_pool().await;
    let (_, submitted_at) = answered(&pool, "Rate limiting for the public API", &decided()).await;

    let html = page(&pool, "/archive").await;

    assert!(
        html.contains("Rate limiting for the public API"),
        "expected the title in the page:\n{html}"
    );
    assert!(html.contains("askance"), "expected the project in the page");
    assert!(
        html.contains("answering-conveniences"),
        "expected the branch in the page"
    );
    // Dated rather than aged: in the Archive this is when the decision was made,
    // and "3h ago" stops meaning anything by the following week.
    assert!(
        html.contains(&submitted_at[..10]) && html.contains(" UTC"),
        "expected {submitted_at} dated on the page:\n{html}"
    );
}

#[tokio::test]
async fn nothing_in_the_archive_is_badged_as_waiting_on_anyone() {
    let (_dir, pool) = fresh_pool().await;
    answered(&pool, "Rate limiting for the public API", &decided()).await;

    let html = page(&pool, "/archive").await;

    assert!(
        !html.contains("liveness"),
        "nothing is waiting on an answered Set, so there is no Liveness to badge:\n{html}"
    );
}

#[tokio::test]
async fn a_pending_set_is_not_in_the_archive_and_an_answered_one_is_not_pending() {
    let (_dir, pool) = fresh_pool().await;
    answered(&pool, "already answered", &decided()).await;
    store::insert_set(&pool, &set("still waiting"))
        .await
        .unwrap();

    let archive = page(&pool, "/archive").await;
    assert!(archive.contains("already answered"));
    assert!(
        !archive.contains("still waiting"),
        "a Set lands in the Archive by being answered:\n{archive}"
    );

    let pending = page(&pool, "/").await;
    assert!(pending.contains("still waiting"));
    assert!(
        !pending.contains("already answered"),
        "an answered Set is off the pending list:\n{pending}"
    );
}

#[tokio::test]
async fn the_archive_is_rendered_newest_decision_first() {
    let (_dir, pool) = fresh_pool().await;
    // Answered in the opposite order to the asking: the Archive is read along
    // the answering, not the asking.
    let (older, _) = answered(&pool, "decided second", &decided()).await;
    let (newer, _) = answered(&pool, "decided first", &decided()).await;
    for (id, submitted_at) in [
        (older, "2026-08-03T17:00:00.000Z"),
        (newer, "2026-08-03T09:00:00.000Z"),
    ] {
        sqlx::query("UPDATE responses SET submitted_at = ? WHERE set_id = ?")
            .bind(submitted_at)
            .bind(id)
            .execute(&pool)
            .await
            .unwrap();
    }

    let html = page(&pool, "/archive").await;

    let newer = html.find("decided second").expect("the later decision");
    let older = html.find("decided first").expect("the earlier decision");
    assert!(newer < older, "expected the newer decision first:\n{html}");
}

#[tokio::test]
async fn each_archive_row_opens_the_sets_response() {
    let (_dir, pool) = fresh_pool().await;
    let (id, _) = answered(&pool, "Rate limiting for the public API", &decided()).await;

    let html = page(&pool, "/archive").await;
    assert!(
        html.contains(&format!(r#"href="/sets/{id}""#)),
        "expected the row to link to the Set's own page:\n{html}"
    );

    // And that page is the read-only record, with the Response readable on it.
    let set_page = page(&pool, &format!("/sets/{id}")).await;
    assert!(
        set_page.contains(r#"class="questions decided""#),
        "expected the answered Set's record:\n{set_page}"
    );
    assert!(
        set_page.contains("Do the in-process one first; we can move it later."),
        "expected the Response readable:\n{set_page}"
    );
}

#[tokio::test]
async fn the_archive_and_the_pending_list_link_to_each_other() {
    let (_dir, pool) = fresh_pool().await;

    let archive = page(&pool, "/archive").await;
    assert!(
        archive.contains(r#"href="/""#),
        "expected the way back to what is waiting:\n{archive}"
    );

    let pending = page(&pool, "/").await;
    assert!(
        pending.contains(r#"href="/archive""#),
        "expected the way through to what was decided:\n{pending}"
    );
}

#[tokio::test]
async fn an_empty_archive_says_so() {
    let (_dir, pool) = fresh_pool().await;
    store::insert_set(&pool, &set("still waiting"))
        .await
        .unwrap();

    let html = page(&pool, "/archive").await;

    assert!(
        html.contains("Nothing has been answered or archived yet."),
        "expected the empty state rather than a bare heading, and one that covers \
         both ways into the Archive:\n{html}"
    );
}
