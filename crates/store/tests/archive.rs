//! The Archive's query: which Sets have been settled — answered, or archived
//! unanswered by the human — in the order the log should show them, and drawn
//! from the lifted columns and the settling stamps alone.
//!
//! Archiving is here too, on both sides of it: what leaves the pending list, what
//! arrives in the Archive, and what an archived Set will no longer accept.

use askance_schema::{QuestionSet, Response};
use askance_store::{
    Archiving, Settled, Settlements, Submission, archive_set, archived_sets, insert_response,
    insert_set, open_database, pending_sets, submit_response,
};
use sqlx::SqlitePool;

/// A pool over a fresh database, plus the directory keeping it alive.
async fn fresh_pool() -> (tempfile::TempDir, SqlitePool) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("askance.db")).await.unwrap();
    (dir, pool)
}

/// The channel a settling is announced on. Nothing here listens: what these
/// tests are about is what the store did, and the announcement is the server's
/// business.
fn settlements() -> Settlements {
    Settlements::new(4)
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

/// The pending list's titles, in the order it lists them.
async fn pending(pool: &SqlitePool) -> Vec<String> {
    pending_sets(pool)
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
        entry.settled_at, accepted.submitted_at,
        "in the Archive the date that matters is the day the decision was made",
    );
    assert_eq!(entry.settled, Settled::Answered);
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

#[tokio::test]
async fn archiving_a_set_moves_it_off_the_pending_list_and_into_the_archive_unanswered() {
    let (_dir, pool) = fresh_pool().await;
    let orphan = insert_set(&pool, &set("the one whose agent died"))
        .await
        .unwrap();
    insert_set(&pool, &set("still waiting")).await.unwrap();

    assert_eq!(
        pending(&pool).await,
        ["still waiting", "the one whose agent died"],
        "both are waiting on the human before either is closed",
    );

    let archiving = archive_set(&pool, &settlements(), orphan.id).await.unwrap();
    let Archiving::Archived(archived) = archiving else {
        panic!("a Set nobody has answered should archive: {archiving:?}");
    };
    assert_eq!(archived.set_id, orphan.id);

    assert_eq!(
        pending(&pool).await,
        ["still waiting"],
        "the point of archiving is that the list stops claiming the Set is waiting",
    );

    let filed = archived_sets(&pool).await.unwrap();
    assert_eq!(filed.len(), 1);
    assert_eq!(filed[0].id, orphan.id);
    assert_eq!(filed[0].title, "the one whose agent died");
    assert_eq!(
        filed[0].settled,
        Settled::ArchivedUnanswered,
        "it carries no Response, and the Archive has to say so rather than show a decision",
    );
    assert_eq!(
        filed[0].settled_at, archived.archived_at,
        "an archived Set is dated by the archiving, as an answered one is by its Response",
    );
}

#[tokio::test]
async fn an_answered_set_cannot_be_archived_unanswered() {
    let (_dir, pool) = fresh_pool().await;
    let decided = answer(&pool, "already answered").await;
    let before = archived_sets(&pool).await.unwrap();

    assert_eq!(
        archive_set(&pool, &settlements(), decided).await.unwrap(),
        Archiving::AlreadyAnswered,
        "a decision is not an orphan, and archiving is not a way to unmake one",
    );
    assert_eq!(
        archived_sets(&pool).await.unwrap(),
        before,
        "and nothing already in the Archive was touched",
    );
}

#[tokio::test]
async fn an_archived_set_will_not_take_a_response() {
    let (_dir, pool) = fresh_pool().await;
    let orphan = insert_set(&pool, &set("closed unanswered")).await.unwrap();
    archive_set(&pool, &settlements(), orphan.id).await.unwrap();

    assert_eq!(
        submit_response(&pool, &settlements(), orphan.id, &Response::default())
            .await
            .unwrap(),
        Submission::Archived,
        "archiving closes a Set for good: it must not also become an answered one",
    );

    let filed = archived_sets(&pool).await.unwrap();
    assert_eq!(filed.len(), 1, "and it is in the Archive once: {filed:?}");
    assert_eq!(filed[0].settled, Settled::ArchivedUnanswered);
}

#[tokio::test]
async fn archiving_a_set_twice_leaves_the_first_archiving_standing() {
    let (_dir, pool) = fresh_pool().await;
    let orphan = insert_set(&pool, &set("closed unanswered")).await.unwrap();

    let Archiving::Archived(first) = archive_set(&pool, &settlements(), orphan.id).await.unwrap()
    else {
        panic!("the first archiving should have taken");
    };

    // Two devices, or one page left open in a second tab.
    assert_eq!(
        archive_set(&pool, &settlements(), orphan.id).await.unwrap(),
        Archiving::AlreadyArchived,
    );

    let filed = archived_sets(&pool).await.unwrap();
    assert_eq!(filed.len(), 1);
    assert_eq!(
        filed[0].settled_at, first.archived_at,
        "the Set was closed once, at the time it was closed",
    );
}

#[tokio::test]
async fn there_is_nothing_to_archive_about_a_set_that_does_not_exist() {
    let (_dir, pool) = fresh_pool().await;

    assert_eq!(
        archive_set(&pool, &settlements(), 404).await.unwrap(),
        Archiving::NoSuchSet,
    );
    assert!(archived_sets(&pool).await.unwrap().is_empty());
}

#[tokio::test]
async fn the_archive_reads_along_the_settling_whichever_settled_each_set() {
    let (_dir, pool) = fresh_pool().await;

    let decided = answer(&pool, "answered at nine").await;
    let orphan = insert_set(&pool, &set("archived at noon")).await.unwrap();
    let also_decided = answer(&pool, "answered at five").await;
    archive_set(&pool, &settlements(), orphan.id).await.unwrap();

    stamp(&pool, decided, "2026-08-03T09:00:00.000Z").await;
    stamp(&pool, also_decided, "2026-08-03T17:00:00.000Z").await;
    sqlx::query("UPDATE archivings SET archived_at = '2026-08-03T12:00:00.000Z'")
        .execute(&pool)
        .await
        .unwrap();

    assert_eq!(
        titles(&pool).await,
        ["answered at five", "archived at noon", "answered at nine"],
        "the two kinds of settling are one log, ordered by when each Set was closed",
    );
}
