//! What became of the two things the human can do to a Set: answer it, or close
//! it unanswered.
//!
//! Both are named outcomes rather than status codes, because every one of them
//! is something the viewer has to say in words — a Set answered from another
//! device, one archived in another tab, a Response the page should never have
//! built. None of them is an error the human can act on by trying again, and
//! none of them is silent.

use serde::{Deserialize, Serialize};

#[cfg(feature = "typescript")]
use ts_rs::TS;

/// What became of the human's Response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum Submitted {
    /// Stored as the Set's answer; whoever was waiting has been woken.
    Accepted,

    /// The Set was answered before this Response arrived — the first stands,
    /// and this one was discarded.
    AlreadyAnswered,

    /// There is no such Set, though there was one when the page loaded.
    NoSuchSet,

    /// The Set was archived unanswered before this Response arrived — from
    /// another device, or another tab. Archiving closes a Set for good, so it
    /// cannot also become an answered one.
    Archived,

    /// The Response does not resolve the Set. The viewer builds Responses that
    /// do, so this is a bug rather than something the human can fix — but it
    /// is carried back and shown rather than swallowed.
    Rejected(Vec<String>),
}

/// What became of the human closing a Set unanswered.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum Archived {
    /// Closed: the Set is off the pending list and in the Archive, and a CLI
    /// still holding a wait on it has been told.
    Closed,

    /// It was answered before this arrived, so it is in the Archive as a
    /// decision. Nothing was changed — a decision is not something to close.
    AlreadyAnswered,

    /// It had already been archived, from another device or another tab.
    AlreadyArchived,

    /// There is no such Set, though there was one when the page loaded.
    NoSuchSet,
}
