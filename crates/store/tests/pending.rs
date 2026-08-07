//! The pending list's query: which Sets are still waiting on the human, in
//! the order they should be shown, drawn from the lifted columns alone.

use askance_schema::{QuestionSet, Response};
use askance_store::{insert_response, insert_set, open_database, pending_sets};
use sqlx::SqlitePool;

/// A pool over a fresh database, plus the directory keeping it alive.
async fn fresh_pool() -> (tempfile::TempDir, SqlitePool) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("askance.db")).await.unwrap();
    (dir, pool)
}

fn set(title: &str) -> QuestionSet {
    QuestionSet {
        title: title.to_owned(),
        preface: None,
        questions: Vec::new(),
        postscript: None,
        project: Some("askance".to_owned()),
        branch: Some("answering-web-ui".to_owned()),
        diff: None,
    }
}

#[tokio::test]
async fn a_stored_set_is_pending_with_its_lifted_columns() {
    let (_dir, pool) = fresh_pool().await;

    let created = insert_set(&pool, &set("Storage layout for the pending list"))
        .await
        .unwrap();

    let pending = pending_sets(&pool).await.unwrap();

    assert_eq!(pending.len(), 1);
    let entry = &pending[0];
    assert_eq!(entry.id, created.id);
    assert_eq!(entry.created_at, created.created_at);
    assert_eq!(entry.title, "Storage layout for the pending list");
    assert_eq!(entry.project.as_deref(), Some("askance"));
    assert_eq!(entry.branch.as_deref(), Some("answering-web-ui"));
}

#[tokio::test]
async fn pending_sets_come_back_newest_first() {
    let (_dir, pool) = fresh_pool().await;

    for title in ["oldest", "middle", "newest"] {
        insert_set(&pool, &set(title)).await.unwrap();
    }

    let titles: Vec<String> = pending_sets(&pool)
        .await
        .unwrap()
        .into_iter()
        .map(|entry| entry.title)
        .collect();

    assert_eq!(titles, ["newest", "middle", "oldest"]);
}

#[tokio::test]
async fn an_answered_set_is_no_longer_pending() {
    let (_dir, pool) = fresh_pool().await;

    let answered = insert_set(&pool, &set("already answered")).await.unwrap();
    insert_set(&pool, &set("still waiting")).await.unwrap();

    insert_response(&pool, answered.id, &Response::default())
        .await
        .unwrap()
        .expect("the Set had no Response yet");

    let titles: Vec<String> = pending_sets(&pool)
        .await
        .unwrap()
        .into_iter()
        .map(|entry| entry.title)
        .collect();

    assert_eq!(titles, ["still waiting"]);
}

#[tokio::test]
async fn nothing_stored_means_nothing_pending() {
    let (_dir, pool) = fresh_pool().await;

    assert!(pending_sets(&pool).await.unwrap().is_empty());
}
