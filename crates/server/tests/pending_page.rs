//! The pending list as the human's browser gets it: server-rendered, from the
//! same binary that took the Set in.

use askance_schema::QuestionSet;
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

/// The UI's file locations, which this test never reaches: it asks for a
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
        questions: Vec::new(),
        project: Some("askance".to_owned()),
        branch: Some("answering-web-ui".to_owned()),
        diff: None,
    }
}

async fn pending_page(pool: &SqlitePool) -> String {
    let response = router_with_ui(pool.clone(), options())
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    String::from_utf8(body.to_vec()).unwrap()
}

#[tokio::test]
async fn a_submitted_set_is_rendered_into_the_pending_list() {
    let (_dir, pool) = fresh_pool().await;
    store::insert_set(&pool, &set("Storage layout for the pending list"))
        .await
        .unwrap();

    let html = pending_page(&pool).await;

    assert!(
        html.contains("Storage layout for the pending list"),
        "expected the title in the page:\n{html}"
    );
    assert!(html.contains("askance"), "expected the project in the page");
    assert!(
        html.contains("answering-web-ui"),
        "expected the branch in the page"
    );
    assert!(
        html.contains("just now"),
        "expected the age in the page:\n{html}"
    );
}

#[tokio::test]
async fn the_pending_list_is_rendered_newest_first() {
    let (_dir, pool) = fresh_pool().await;
    for title in ["the older ask", "the newer ask"] {
        store::insert_set(&pool, &set(title)).await.unwrap();
    }

    let html = pending_page(&pool).await;

    let newer = html.find("the newer ask").expect("the newer Set is listed");
    let older = html.find("the older ask").expect("the older Set is listed");
    assert!(newer < older, "expected the newer Set first:\n{html}");
}

#[tokio::test]
async fn an_answered_set_is_not_rendered() {
    let (_dir, pool) = fresh_pool().await;
    let answered = store::insert_set(&pool, &set("already answered"))
        .await
        .unwrap();
    store::insert_set(&pool, &set("still waiting"))
        .await
        .unwrap();
    store::insert_response(&pool, answered.id, &Default::default())
        .await
        .unwrap()
        .expect("the Set had no Response yet");

    let html = pending_page(&pool).await;

    assert!(html.contains("still waiting"));
    assert!(
        !html.contains("already answered"),
        "an answered Set should be off the list:\n{html}"
    );
}

#[tokio::test]
async fn an_empty_pending_list_says_so() {
    let (_dir, pool) = fresh_pool().await;

    let html = pending_page(&pool).await;

    assert!(
        html.contains("Nothing is waiting on you."),
        "expected the empty state:\n{html}"
    );
}
