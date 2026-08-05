//! The two lists' rows as the viewer receives them: what is waiting on the
//! human, and what has already been settled.
//!
//! Neither carries a timestamp. The age of a pending Set and the day a settled
//! one was decided are worded on the server — see [`crate::when`] — because it
//! is the side with the clock, and because the rows are the one place a date
//! library would otherwise have to exist on both sides of the wire.

use askance_schema::Liveness;
use serde::{Deserialize, Serialize};

#[cfg(feature = "typescript")]
use ts_rs::TS;

/// One row of the pending list.
///
/// The Liveness arrives already decided, like the age: the registry of held
/// waits is the server's, and this way the viewer draws a badge rather than
/// working one out.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct PendingEntry {
    pub id: i64,
    pub title: String,
    pub project: Option<String>,
    pub branch: Option<String>,
    pub age: String,
    pub liveness: Liveness,
}

/// One row of the Archive.
///
/// No Liveness, because nothing is waiting on a settled Set — and a date rather
/// than an age, because this is a permanent log of decisions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct ArchiveEntry {
    pub id: i64,
    pub title: String,
    pub project: Option<String>,
    pub branch: Option<String>,
    pub settled_at: String,

    /// Whether it got here without a Response — archived unanswered by the
    /// human, rather than decided.
    pub unanswered: bool,
}
