//! What the viewer is told about updating: whether a newer Askance has been
//! released than the one serving the page.
//!
//! The Update Notice informs and nothing else — nothing is installed on the
//! human's behalf — so the whole of what crosses the wire is a verdict and, when
//! there is one, the version to name in the banner.
//!
//! The server is the side that asks GitHub, and it asks once a day; this is only
//! what it concluded. A server that has not managed to find out says the same
//! thing as one that is already current, because there is nothing for the human
//! to do about either.

use serde::{Deserialize, Serialize};

#[cfg(feature = "typescript")]
use ts_rs::TS;

/// Whether there is a newer Askance to update to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum UpdateNotice {
    /// Nothing to update to. This server is running the latest release, or it
    /// has not been able to find out — a poll that failed is no news rather
    /// than an alarm, and the two look the same from here.
    Current,

    /// A newer release exists. Named, because a banner that cannot say which
    /// version is waiting leaves the human nothing to check against.
    Available { version: String },
}
