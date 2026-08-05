//! The small YAML envelopes the API answers with, kept beside the Set types so
//! the CLI reads back exactly what the server writes.

use serde::{Deserialize, Serialize};

use crate::validate::Violation;

// The refusal shape is the whole server's, viewer included — see `Response` for
// why the emitter is gated.
#[cfg(feature = "typescript")]
use ts_rs::TS;

/// What `POST /api/v1/sets` returns once a Set is stored: the identity the
/// server stamped on it. The CLI waits on `id`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetCreated {
    pub id: i64,

    /// When the server accepted the Set, RFC 3339.
    pub created_at: String,
}

/// What `POST /api/v1/sets/{id}/response` returns once a Response is stored.
/// The human's device gets this; the agent gets the Response itself, off the
/// wait.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResponseAccepted {
    /// The Set that has just been answered.
    pub set_id: i64,

    /// When the server accepted the Response, RFC 3339.
    pub submitted_at: String,
}

/// What the API returns when it refuses a request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct ApiError {
    /// One line saying what was refused.
    pub error: String,

    /// The grammar violations behind the refusal, each naming its question.
    /// Empty when the request failed for some other reason.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub violations: Vec<Violation>,
}

impl ApiError {
    pub fn new(error: impl Into<String>) -> Self {
        Self {
            error: error.into(),
            violations: Vec::new(),
        }
    }

    pub fn with_violations(error: impl Into<String>, violations: Vec<Violation>) -> Self {
        Self {
            error: error.into(),
            violations,
        }
    }
}
