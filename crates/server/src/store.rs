//! SQLite persistence for Question Sets.
//!
//! A Set is kept as one JSON body — JSON rather than YAML because it preserves
//! a Preface's whitespace exactly, and the Preface is markdown the human reads
//! back verbatim. `title`, `project` and `branch` are lifted into columns
//! beside it so the pending list can be drawn without deserializing every Set.

use anyhow::{Context, Result};
use askance_schema::{QuestionSet, SetCreated};
use sqlx::SqlitePool;

/// A Set as the store holds it: the agent's Set plus the identity the server
/// stamped on it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredSet {
    pub id: i64,
    pub created_at: String,
    pub set: QuestionSet,
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
