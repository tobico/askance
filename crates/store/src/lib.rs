//! SQLite persistence for Question Sets and their Responses.
//!
//! Each is kept as one JSON body — JSON rather than YAML because it preserves
//! a Preface's whitespace exactly, and the Preface is markdown the human reads
//! back verbatim. `title`, `project` and `branch` are lifted into columns
//! beside the Set so the pending list can be drawn without deserializing every
//! Set.
//!
//! The store sits below both the agent API and the web UI: the UI's server
//! functions live in the shared `askance-app` crate, which cannot reach back
//! into the server binary that links it.

use std::path::Path;

use anyhow::{Context, Result};
use askance_schema::{QuestionSet, Response, ResponseAccepted, SetCreated, ValidationError};
use sqlx::SqlitePool;
use sqlx::sqlite::SqliteConnectOptions;
use tokio::sync::broadcast;

mod waits;

pub use waits::{WaitHeld, Waits};

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

/// One row of the pending list: a Set still waiting on the human, drawn from
/// the lifted columns without touching its body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingSet {
    pub id: i64,
    pub created_at: String,
    pub title: String,
    pub project: Option<String>,
    pub branch: Option<String>,
}

/// One row of the Archive: a Set that has been answered, drawn from the lifted
/// columns and its Response's stamp without touching either body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchivedSet {
    pub id: i64,
    pub answered_at: String,
    pub title: String,
    pub project: Option<String>,
    pub branch: Option<String>,
}

/// Word that a Set has just been answered, so a wait held on it can end
/// without going back to the store to look.
///
/// It lives beside the store for the same reason the store is its own crate:
/// the agent API's long-poll and the web UI's submit are in different crates,
/// and the two have to meet at the same channel or a browser submit would
/// leave a waiting agent hanging until its hold window closed.
#[derive(Debug, Clone)]
pub struct Submissions(broadcast::Sender<i64>);

impl Submissions {
    /// A channel that will hold `backlog` announcements for a listener that
    /// falls behind.
    pub fn new(backlog: usize) -> Self {
        let (announcements, _) = broadcast::channel(backlog);
        Self(announcements)
    }

    /// Hear about Sets answered from now on. Subscribe before reading the
    /// store, so a Response landing between the two wakes the wait instead of
    /// slipping past it.
    pub fn subscribe(&self) -> broadcast::Receiver<i64> {
        self.0.subscribe()
    }
}

/// What became of a submitted Response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Submission {
    /// Stored as the Set's answer, and everyone waiting on it has been told.
    Accepted(ResponseAccepted),

    /// There is no Set with that id to answer.
    NoSuchSet,

    /// The Response does not resolve the Set, so it is not an answer to it.
    Invalid(ValidationError),

    /// The Set was answered before this Response arrived. A Set is answered
    /// once, and the first Response stands.
    AlreadyAnswered,
}

/// Answer a Set: check the Response resolves it, store it, and wake whoever is
/// waiting.
///
/// This is the one path a Response takes, whether it came from the human's
/// browser or from `curl`, so a wait ends the same way either way.
pub async fn submit_response(
    pool: &SqlitePool,
    submissions: &Submissions,
    set_id: i64,
    response: &Response,
) -> Result<Submission> {
    let Some(stored) = load_set(pool, set_id).await? else {
        return Ok(Submission::NoSuchSet);
    };

    if let Err(invalid) = response.validate(&stored.set) {
        return Ok(Submission::Invalid(invalid));
    }

    let Some(accepted) = insert_response(pool, set_id, response).await? else {
        return Ok(Submission::AlreadyAnswered);
    };

    // A send error means nobody is listening, which is the ordinary case — the
    // agent may well be between polls.
    let _ = submissions.0.send(set_id);

    Ok(Submission::Accepted(accepted))
}

/// Open the SQLite database at `path`, creating the file if it is absent and
/// bringing its schema up to date.
pub async fn open_database(path: &Path) -> Result<SqlitePool> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating database directory {}", parent.display()))?;
    }

    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true);

    let pool = SqlitePool::connect_with(options)
        .await
        .with_context(|| format!("opening database {}", path.display()))?;

    apply_schema(&pool).await?;

    Ok(pool)
}

/// Bring an opened database up to the shape the server expects. Safe to run
/// against a database that already has it.
async fn apply_schema(pool: &SqlitePool) -> Result<()> {
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

/// The Sets still waiting on the human, newest first.
///
/// Ordered by id rather than `created_at`: the id is handed out in submission
/// order, so it says "newest" without two Sets stamped in the same millisecond
/// coming back in an arbitrary order.
pub async fn pending_sets(pool: &SqlitePool) -> Result<Vec<PendingSet>> {
    /// The lifted columns in the order the query below selects them.
    type Row = (i64, String, String, Option<String>, Option<String>);

    let rows: Vec<Row> = sqlx::query_as(
        "SELECT s.id, s.created_at, s.title, s.project, s.branch
         FROM question_sets s
         LEFT JOIN responses r ON r.set_id = s.id
         WHERE r.set_id IS NULL
         ORDER BY s.id DESC",
    )
    .fetch_all(pool)
    .await
    .context("listing the pending Question Sets")?;

    Ok(rows
        .into_iter()
        .map(|(id, created_at, title, project, branch)| PendingSet {
            id,
            created_at,
            title,
            project,
            branch,
        })
        .collect())
}

/// The Sets in the Archive, newest decision first.
///
/// Ordered by when each Set was answered rather than by when it was asked: in
/// the Archive the day the decision was made is what the log is read along. Two
/// Responses can share a stamp to the millisecond, so the id — handed out in
/// submission order, and unique — breaks the tie.
///
/// Like the pending list, drawn from the lifted columns alone: the Archive is
/// permanent and only grows, and a list is scanned rather than read.
pub async fn archived_sets(pool: &SqlitePool) -> Result<Vec<ArchivedSet>> {
    /// The columns in the order the query below selects them.
    type Row = (i64, String, String, Option<String>, Option<String>);

    let rows: Vec<Row> = sqlx::query_as(
        "SELECT s.id, r.submitted_at, s.title, s.project, s.branch
         FROM question_sets s
         JOIN responses r ON r.set_id = s.id
         ORDER BY r.submitted_at DESC, s.id DESC",
    )
    .fetch_all(pool)
    .await
    .context("listing the archived Question Sets")?;

    Ok(rows
        .into_iter()
        .map(|(id, answered_at, title, project, branch)| ArchivedSet {
            id,
            answered_at,
            title,
            project,
            branch,
        })
        .collect())
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
