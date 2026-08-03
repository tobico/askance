//! Submitting a Question Set: what the store keeps, and what the API refuses.

use askance_schema::{ApiError, QuestionSet, SetCreated};
use askance_server::store;
use askance_server::{open_database, router};
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use axum::response::Response;
use http_body_util::BodyExt;
use sqlx::SqlitePool;
use tower::ServiceExt;

const VALID_SET: &str = r#"
title: Storage layout for the pending list
preface: |
  We need to settle how Sets are stored before the UI lands.

  The candidates differ mainly in how much SQL the Archive view needs.
project: askance
branch: api-core-and-cli
questions:
  - label: Q1
    text: How should a Question Set be stored?
    options:
      - n: 1
        text: One JSON body column
        recommended: true
      - n: 2
        text: Fully normalised tables
  - label: Q2
    text: What identifies a Set on the wire?
    subquestions:
      - letter: a
        text: Integer rowid or UUID?
        options:
          - n: 1
            text: Integer rowid
          - n: 2
            text: UUID
      - letter: b
        text: Should the id appear in URLs?
"#;

/// A pool over a fresh database, plus the directory keeping it alive.
async fn fresh_pool() -> (tempfile::TempDir, SqlitePool) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("askance.db")).await.unwrap();
    (dir, pool)
}

async fn post_set(pool: &SqlitePool, yaml: &str) -> Response {
    router(pool.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/sets")
                .header(header::CONTENT_TYPE, "application/yaml")
                .body(Body::from(yaml.to_owned()))
                .unwrap(),
        )
        .await
        .unwrap()
}

async fn body_text(response: Response) -> String {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    String::from_utf8(bytes.to_vec()).unwrap()
}

async fn stored_set_count(pool: &SqlitePool) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM question_sets")
        .fetch_one(pool)
        .await
        .unwrap()
}

#[tokio::test]
async fn a_valid_set_is_stored_and_answered_with_its_id() {
    let (_dir, pool) = fresh_pool().await;

    let response = post_set(&pool, VALID_SET).await;

    assert_eq!(response.status(), StatusCode::CREATED);
    assert_eq!(
        response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok()),
        Some("application/yaml"),
    );

    let created: SetCreated = serde_saphyr::from_str(&body_text(response).await).unwrap();
    assert!(created.id > 0, "the server should stamp an id");
    assert!(
        created.created_at.starts_with("20"),
        "the server should stamp an RFC 3339 created_at, got {:?}",
        created.created_at
    );

    let stored = store::load_set(&pool, created.id)
        .await
        .unwrap()
        .expect("the Set should be in the store");
    assert_eq!(stored.id, created.id);
    assert_eq!(stored.created_at, created.created_at);
    assert_eq!(stored.set.title, "Storage layout for the pending list");
    assert_eq!(stored.set.questions.len(), 2);
    assert_eq!(stored.set.questions[1].subquestions.len(), 2);
    assert_eq!(stored.set.project.as_deref(), Some("askance"));
    assert_eq!(stored.set.branch.as_deref(), Some("api-core-and-cli"));
}

#[tokio::test]
async fn a_multi_line_preface_survives_the_store_byte_for_byte() {
    let (_dir, pool) = fresh_pool().await;
    let submitted = "First paragraph, with a trailing space. \n\
                     \n\
                     Second paragraph with `code` and a \"quote\".\n\
                     \x20   an indented line\n";

    let response = post_set(
        &pool,
        "
title: Round trip
preface: |
  First paragraph, with a trailing space.\x20

  Second paragraph with `code` and a \"quote\".
      an indented line
questions:
  - label: Q1
    text: Does it survive?
",
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    let created: SetCreated = serde_saphyr::from_str(&body_text(response).await).unwrap();
    let stored = store::load_set(&pool, created.id).await.unwrap().unwrap();

    assert_eq!(stored.set.preface.as_deref(), Some(submitted));
}

#[tokio::test]
async fn a_diff_is_stored_opaquely() {
    let (_dir, pool) = fresh_pool().await;

    let response = post_set(
        &pool,
        "
title: Please review
diff: |
  diff --git a/notes.md b/notes.md
  @@ -1 +1,2 @@
   first
  +second
questions:
  - label: Q1
    text: Land it?
",
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    let created: SetCreated = serde_saphyr::from_str(&body_text(response).await).unwrap();
    let stored = store::load_set(&pool, created.id).await.unwrap().unwrap();

    assert_eq!(
        stored.set.diff.as_deref(),
        Some("diff --git a/notes.md b/notes.md\n@@ -1 +1,2 @@\n first\n+second\n")
    );
}

#[tokio::test]
async fn three_levels_of_nesting_are_refused_naming_the_sub_question() {
    let (_dir, pool) = fresh_pool().await;

    let response = post_set(
        &pool,
        "
title: Too deep
questions:
  - label: Q7
    text: Which storage layout?
    subquestions:
      - letter: a
        text: Rowid or UUID?
        subquestions:
          - letter: i
            text: And how wide?
",
    )
    .await;

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let error: ApiError = serde_saphyr::from_str(&body_text(response).await).unwrap();
    assert!(
        error
            .violations
            .iter()
            .any(|v| v.label.as_deref() == Some("Q7a")),
        "the refusal should name Q7a, got {error:?}"
    );
    assert_eq!(stored_set_count(&pool).await, 0);
}

#[tokio::test]
async fn two_recommended_options_are_refused_naming_the_question() {
    let (_dir, pool) = fresh_pool().await;

    let response = post_set(
        &pool,
        "
title: Two stars
questions:
  - label: Q3
    text: Which one?
    options:
      - n: 1
        text: This
        recommended: true
      - n: 2
        text: That
        recommended: true
",
    )
    .await;

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let error: ApiError = serde_saphyr::from_str(&body_text(response).await).unwrap();
    assert!(
        error
            .violations
            .iter()
            .any(|v| v.label.as_deref() == Some("Q3")),
        "the refusal should name Q3, got {error:?}"
    );
    assert_eq!(stored_set_count(&pool).await, 0);
}

#[tokio::test]
async fn an_empty_title_is_refused() {
    let (_dir, pool) = fresh_pool().await;

    let response = post_set(
        &pool,
        "
title: '  '
questions:
  - label: Q1
    text: Ship it?
",
    )
    .await;

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let error: ApiError = serde_saphyr::from_str(&body_text(response).await).unwrap();
    assert!(
        error.violations.iter().any(|v| v.message.contains("title")),
        "the refusal should mention the title, got {error:?}"
    );
    assert_eq!(stored_set_count(&pool).await, 0);
}

#[tokio::test]
async fn a_missing_title_is_refused_as_malformed() {
    let (_dir, pool) = fresh_pool().await;

    let response = post_set(
        &pool,
        "
questions:
  - label: Q1
    text: Ship it?
",
    )
    .await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let error: ApiError = serde_saphyr::from_str(&body_text(response).await).unwrap();
    assert!(
        error.error.contains("title"),
        "the refusal should mention the missing title, got {error:?}"
    );
    assert_eq!(stored_set_count(&pool).await, 0);
}

#[tokio::test]
async fn a_body_that_is_not_yaml_is_refused_as_malformed() {
    let (_dir, pool) = fresh_pool().await;

    let response = post_set(&pool, "{{{ not yaml").await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(stored_set_count(&pool).await, 0);
}

#[tokio::test]
async fn sets_are_stamped_with_distinct_ids() {
    let (_dir, pool) = fresh_pool().await;

    let first: SetCreated =
        serde_saphyr::from_str(&body_text(post_set(&pool, VALID_SET).await).await).unwrap();
    let second: SetCreated =
        serde_saphyr::from_str(&body_text(post_set(&pool, VALID_SET).await).await).unwrap();

    assert_ne!(first.id, second.id);
    assert_eq!(stored_set_count(&pool).await, 2);
}

#[tokio::test]
async fn loading_an_unknown_set_finds_nothing() {
    let (_dir, pool) = fresh_pool().await;

    assert!(store::load_set(&pool, 404).await.unwrap().is_none());
}

#[tokio::test]
async fn the_store_keeps_the_pending_list_columns_alongside_the_body() {
    let (_dir, pool) = fresh_pool().await;
    let created: SetCreated =
        serde_saphyr::from_str(&body_text(post_set(&pool, VALID_SET).await).await).unwrap();

    let (title, project, branch): (String, Option<String>, Option<String>) =
        sqlx::query_as("SELECT title, project, branch FROM question_sets WHERE id = ?")
            .bind(created.id)
            .fetch_one(&pool)
            .await
            .unwrap();

    assert_eq!(title, "Storage layout for the pending list");
    assert_eq!(project.as_deref(), Some("askance"));
    assert_eq!(branch.as_deref(), Some("api-core-and-cli"));
}

#[tokio::test]
async fn the_schema_is_applied_to_an_existing_database() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("askance.db");

    let pool = open_database(&path).await.unwrap();
    let set = QuestionSet::from_yaml(VALID_SET).unwrap();
    let created = store::insert_set(&pool, &set).await.unwrap();
    pool.close().await;

    let pool = open_database(&path).await.unwrap();
    let stored = store::load_set(&pool, created.id).await.unwrap().unwrap();
    assert_eq!(stored.set, set);
}
