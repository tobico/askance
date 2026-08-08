//! The two lists' rows as the viewer receives them: what is waiting on the
//! human, and what has already been settled.
//!
//! Neither carries a timestamp. A row's times are worded on the server — see
//! [`crate::when`] — because it is the side with the clock, and because the
//! rows are the one place a date library would otherwise have to exist on both
//! sides of the wire. Each worded time travels with the exact minute behind
//! it, for the tooltip on the words.

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

    /// The minute the Set arrived, exactly — the tooltip behind the age.
    pub created_stamp: String,

    pub liveness: Liveness,
}

/// One row of the Archive.
///
/// No Liveness, because nothing is waiting on a settled Set. Its time is
/// worded like the pending list's — an age while the settling is fresh, and
/// the plain date once it is not, because this is a permanent log and old ages
/// stop meaning anything.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct ArchiveEntry {
    pub id: i64,
    pub title: String,
    pub project: Option<String>,
    pub branch: Option<String>,
    pub settled_at: String,

    /// The minute it was settled, exactly — the tooltip behind the words.
    pub settled_stamp: String,

    /// Whether it got here without a Response — archived unanswered by the
    /// human, rather than decided.
    pub unanswered: bool,
}
