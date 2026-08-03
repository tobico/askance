//! The Archive's query: which Sets have been answered, in the order the
//! decision log should show them, drawn from the lifted columns and the
//! Response's stamp alone.

use askance_schema::{QuestionSet, Response};
use askance_store::{archived_sets, insert_response, insert_set, open_database};
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
        project: Some("askance".to_owned()),
        branch: Some("answering-conveniences".to_owned()),
        diff: None,
    }
}

/// Store a Set and answer it, which is the one way a Set reaches the Archive.
async fn answer(pool: &SqlitePool, title: &str) -> i64 {
    let stored = insert_set(pool, &set(title)).await.unwrap();
    insert_response(pool, stored.id, &Response::default())
        .await
        .unwrap()
        .expect("the Set had no Response yet");
    stored.id
}

/// Say when a Set was answered. The store stamps a Response as it takes it, so
/// a test wanting two answerings a working day apart has to say so itself.
async fn stamp(pool: &SqlitePool, set_id: i64, submitted_at: &str) {
    sqlx::query("UPDATE responses SET submitted_at = ? WHERE set_id = ?")
        .bind(submitted_at)
        .bind(set_id)
        .execute(pool)
        .await
        .unwrap();
}

/// The Archive's titles, in the order it lists them.
async fn titles(pool: &SqlitePool) -> Vec<String> {
    archived_sets(pool)
        .await
        .unwrap()
        .into_iter()
        .map(|entry| entry.title)
        .collect()
}

#[tokio::test]
async fn an_answered_set_is_archived_with_its_lifted_columns_and_the_time_it_was_answered() {
    let (_dir, pool) = fresh_pool().await;

    let created = insert_set(&pool, &set("Where the request counter lives"))
        .await
        .unwrap();
    let accepted = insert_response(&pool, created.id, &Response::default())
        .await
        .unwrap()
        .unwrap();

    let archived = archived_sets(&pool).await.unwrap();

    assert_eq!(archived.len(), 1);
    let entry = &archived[0];
    assert_eq!(entry.id, created.id);
    assert_eq!(entry.title, "Where the request counter lives");
    assert_eq!(entry.project.as_deref(), Some("askance"));
    assert_eq!(entry.branch.as_deref(), Some("answering-conveniences"));
    assert_eq!(
        entry.answered_at, accepted.submitted_at,
        "in the Archive the date that matters is the day the decision was made",
    );
}

#[tokio::test]
async fn a_set_still_waiting_is_not_in_the_archive() {
    let (_dir, pool) = fresh_pool().await;

    answer(&pool, "already answered").await;
    insert_set(&pool, &set("still waiting")).await.unwrap();

    assert_eq!(
        titles(&pool).await,
        ["already answered"],
        "a Set lands in the Archive by being answered, not by anyone filing it",
    );
}

#[tokio::test]
async fn the_archive_is_ordered_by_the_answering() {
    let (_dir, pool) = fresh_pool().await;

    // Answered in the opposite order to the asking, and far enough apart to be
    // told apart: ordering by the answering is the only way to get this back,
    // since ordering by the asking would invert it.
    let first = answer(&pool, "asked first").await;
    let second = answer(&pool, "asked second").await;
    stamp(&pool, second, "2026-08-03T09:00:00.000Z").await;
    stamp(&pool, first, "2026-08-03T17:00:00.000Z").await;

    assert_eq!(titles(&pool).await, ["asked first", "asked second"]);
}

#[tokio::test]
async fn two_sets_answered_in_the_same_millisecond_are_still_ordered() {
    let (_dir, pool) = fresh_pool().await;

    let older = answer(&pool, "the older ask").await;
    let newer = answer(&pool, "the newer ask").await;
    // A stamp is only good to the millisecond, so two Responses can share one.
    // The id was handed out in submission order and cannot.
    sqlx::query("UPDATE responses SET submitted_at = '2026-08-03T12:00:00.000Z'")
        .execute(&pool)
        .await
        .unwrap();
    assert!(older < newer);

    assert_eq!(titles(&pool).await, ["the newer ask", "the older ask"]);
}

#[tokio::test]
async fn listing_the_archive_does_not_read_a_single_set_body() {
    let (_dir, pool) = fresh_pool().await;

    // A body no deserializer would take. The lifted columns beside it are all
    // the Archive is drawn from, so listing it has to come back regardless —
    // a decision log grows forever and the list is scanned, not read.
    sqlx::query(
        "INSERT INTO question_sets (created_at, title, project, branch, body)
         VALUES ('2026-08-03T12:00:00.000Z', 'unreadable', NULL, NULL, 'not json')",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO responses (set_id, submitted_at, body)
         VALUES (1, '2026-08-03T12:01:00.000Z', 'not json either')",
    )
    .execute(&pool)
    .await
    .unwrap();

    assert_eq!(titles(&pool).await, ["unreadable"]);
}

#[tokio::test]
async fn nothing_answered_means_an_empty_archive() {
    let (_dir, pool) = fresh_pool().await;

    insert_set(&pool, &set("still waiting")).await.unwrap();

    assert!(archived_sets(&pool).await.unwrap().is_empty());
}
