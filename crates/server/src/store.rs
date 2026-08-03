//! SQLite persistence for Question Sets and their Responses.
//!
//! Each is kept as one JSON body — JSON rather than YAML because it preserves
//! a Preface's whitespace exactly, and the Preface is markdown the human reads
//! back verbatim. `title`, `project` and `branch` are lifted into columns
//! beside the Set so the pending list can be drawn without deserializing every
//! Set.

use anyhow::{Context, Result};
use askance_schema::{QuestionSet, Response, ResponseAccepted, SetCreated};
use sqlx::SqlitePool;

/// A Set as the store holds it: the agent's Set plus the identity the server
/// stamped on it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredSet {
    pub id: i64,
    pub created_at: String,
    pub set: QuestionSet,
}

/// A Response as the store holds it: the human's reply plus when it landed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredResponse {
    pub set_id: i64,
    pub submitted_at: String,
    pub response: Response,
}

/// Bring an opened database up to the shape the server expects. Safe to run
/// against a database that already has it.
pub(crate) async fn apply_schema(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS question_sets (
             id         INTEGER PRIMARY KEY AUTOINCREMENT,
             created_at TEXT NOT NULL,
             title      TEXT NOT NULL,
             project    TEXT,
             branch     TEXT,
             body       TEXT NOT NULL
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the question_sets table")?;

    // One Response per Set, enforced by the primary key rather than by a
    // read-then-write the second submitter could slip through.
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS responses (
             set_id       INTEGER PRIMARY KEY REFERENCES question_sets(id),
             submitted_at TEXT NOT NULL,
             body         TEXT NOT NULL
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the responses table")?;

    Ok(())
}

/// Store a Set, stamping it with an id and a creation time.
///
/// The Set is expected to have been validated already — the store is not where
/// the question grammar is enforced.
pub async fn insert_set(pool: &SqlitePool, set: &QuestionSet) -> Result<SetCreated> {
    let body = serde_json::to_string(set).context("serialising the Question Set")?;

    // SQLite stamps the time as it assigns the id, so both come from one place.
    // `%f` gives seconds to the millisecond, making the whole string RFC 3339.
    let (id, created_at): (i64, String) = sqlx::query_as(
        "INSERT INTO question_sets (created_at, title, project, branch, body)
         VALUES (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), ?, ?, ?, ?)
         RETURNING id, created_at",
    )
    .bind(&set.title)
    .bind(&set.project)
    .bind(&set.branch)
    .bind(body)
    .fetch_one(pool)
    .await
    .context("storing the Question Set")?;

    Ok(SetCreated { id, created_at })
}

/// Read a Set back, or `None` if no Set has that id.
pub async fn load_set(pool: &SqlitePool, id: i64) -> Result<Option<StoredSet>> {
    let row: Option<(i64, String, String)> =
        sqlx::query_as("SELECT id, created_at, body FROM question_sets WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await
            .with_context(|| format!("loading Question Set {id}"))?;

    let Some((id, created_at, body)) = row else {
        return Ok(None);
    };

    let set = serde_json::from_str(&body)
        .with_context(|| format!("deserialising stored Question Set {id}"))?;

    Ok(Some(StoredSet {
        id,
        created_at,
        set,
    }))
}

/// Whether a Set with this id exists, without paying to deserialize it.
pub async fn set_exists(pool: &SqlitePool, id: i64) -> Result<bool> {
    let found: Option<(i64,)> = sqlx::query_as("SELECT 1 FROM question_sets WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
        .with_context(|| format!("looking for Question Set {id}"))?;

    Ok(found.is_some())
}

/// Store the Response to a Set, stamping it with a submission time.
///
/// `None` means the Set already has a Response: a Set is answered once, and
/// the first Response stands.
///
/// The Response is expected to have been validated against its Set already.
pub async fn insert_response(
    pool: &SqlitePool,
    set_id: i64,
    response: &Response,
) -> Result<Option<ResponseAccepted>> {
    let body = serde_json::to_string(response).context("serialising the Response")?;

    let row: Option<(String,)> = sqlx::query_as(
        "INSERT INTO responses (set_id, submitted_at, body)
         VALUES (?, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), ?)
         ON CONFLICT (set_id) DO NOTHING
         RETURNING submitted_at",
    )
    .bind(set_id)
    .bind(body)
    .fetch_optional(pool)
    .await
    .with_context(|| format!("storing the Response to Question Set {set_id}"))?;

    Ok(row.map(|(submitted_at,)| ResponseAccepted {
        set_id,
        submitted_at,
    }))
}

/// Read a Set's Response back, or `None` if it has not been answered yet.
pub async fn load_response(pool: &SqlitePool, set_id: i64) -> Result<Option<StoredResponse>> {
    let row: Option<(String, String)> =
        sqlx::query_as("SELECT submitted_at, body FROM responses WHERE set_id = ?")
            .bind(set_id)
            .fetch_optional(pool)
            .await
            .with_context(|| format!("loading the Response to Question Set {set_id}"))?;

    let Some((submitted_at, body)) = row else {
        return Ok(None);
    };

    let response = serde_json::from_str(&body)
        .with_context(|| format!("deserialising the stored Response to Question Set {set_id}"))?;

    Ok(Some(StoredResponse {
        set_id,
        submitted_at,
        response,
    }))
}
