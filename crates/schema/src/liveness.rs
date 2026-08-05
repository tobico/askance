//! Whether an agent is still on the other end of a Question Set.

use serde::{Deserialize, Serialize};

// Drawn by the viewer, so its TypeScript comes from here — see `Response`.
#[cfg(feature = "typescript")]
use ts_rs::TS;

/// What the pending list says about a Set: whether an agent is currently
/// waiting on it, or nothing is holding a wait any more.
///
/// Display state only (ADR-0001). A disconnected Set is still answerable and is
/// never withdrawn on its own — the CLI reconnects through transient drops, and
/// only a human may archive a Set whose agent is really gone.
///
/// It is a verdict rather than a timestamp because the server has the clock and
/// the registry of held waits; the browser only draws what it is told.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
#[serde(rename_all = "snake_case")]
pub enum Liveness {
    /// A wait is held on the Set, or one was held recently enough that its
    /// agent is taken to be between polls.
    Waiting,

    /// Nothing has held a wait on the Set for long enough that its agent is
    /// taken to be gone.
    Disconnected,
}
