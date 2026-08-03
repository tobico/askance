//! Who is still waiting: the registry of held waits, and the window that turns
//! it into what the pending list says.
//!
//! The server knows whether a long-poll is currently held for a Set, because it
//! is the thing holding it. This lives beside [`Settlements`](crate::Settlements)
//! for the same reason that does: the agent API holds the waits and the web UI's
//! server functions read them, and those are different crates that can only meet
//! in this one.
//!
//! Also like `Settlements`, the registry is in-process and not persisted. After
//! a server restart every pending Set reads disconnected until its CLI's next
//! reconnect, which is a second or two away.
//!
//! Liveness is display-only (ADR-0001): nothing here withdraws a Set, refuses an
//! answer to one, or is consulted when a Response is taken.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use askance_schema::Liveness;
use time::format_description::well_known::Rfc3339;
use time::{Duration, OffsetDateTime};

/// How long after the last wait was released a Set still reads as waiting.
///
/// The CLI asks for a 30s hold and reopens the moment it gets a 204, so a live
/// agent's gap between held waits is sub-second; the window only has to cover
/// the 10s ceiling its backoff climbs to after a transient failure. 30s is
/// generously clear of that, so the badge never flaps while an agent is merely
/// between polls — at the cost of taking about one hold plus one window to
/// notice an agent that has really been killed.
const GRACE: Duration = Duration::seconds(30);

/// Which Sets have a wait held on them right now, and when the last one was
/// released for those that no longer do.
///
/// One small entry per Set that has ever been waited on, kept for as long as the
/// process runs: it is what a released wait is remembered by, and one human's
/// Sets is not a quantity worth sweeping.
#[derive(Debug, Clone, Default)]
pub struct Waits(Arc<Mutex<HashMap<i64, Held>>>);

/// The waiting on one Set.
#[derive(Debug, Default)]
struct Held {
    /// How many waits are held on it at this moment. More than one is ordinary
    /// enough — a second agent, or a human with `curl` — so the slots are
    /// counted rather than flagged.
    holders: usize,

    /// When the last of them was released, if any has been.
    released_at: Option<OffsetDateTime>,
}

impl Waits {
    pub fn new() -> Self {
        Self::default()
    }

    /// Take a slot for a wait held on `set_id`, released as the returned guard
    /// is dropped.
    ///
    /// Releasing is the guard's job and the guard's alone: a killed CLI or a
    /// broken tailnet drops the handler's future rather than letting it return,
    /// and being dropped is the one thing that ending has in common with a tidy
    /// one. So hold the guard — `let _held = …`, never `let _ = …`, which would
    /// release the slot on the spot.
    #[must_use = "the slot is released as soon as the guard is dropped"]
    pub fn hold(&self, set_id: i64) -> WaitHeld {
        self.entries().entry(set_id).or_default().holders += 1;

        WaitHeld {
            waits: self.clone(),
            set_id,
        }
    }

    /// What the pending list should say about this Set, given its `created_at`
    /// as the store holds it.
    pub fn liveness(&self, set_id: i64, created_at: &str, now: OffsetDateTime) -> Liveness {
        let entries = self.entries();
        let held = entries.get(&set_id);

        verdict(
            held.map_or(0, |held| held.holders),
            held.and_then(|held| held.released_at),
            OffsetDateTime::parse(created_at, &Rfc3339).ok(),
            now,
        )
    }

    /// Give a slot back, remembering when the last one went.
    fn release(&self, set_id: i64) {
        if let Some(held) = self.entries().get_mut(&set_id) {
            held.holders = held.holders.saturating_sub(1);
            if held.holders == 0 {
                held.released_at = Some(OffsetDateTime::now_utc());
            }
        }
    }

    /// A poisoned lock means something panicked while holding it, and nothing
    /// under it can: recover rather than spread the panic, because a badge is
    /// not worth taking a page down over.
    fn entries(&self) -> MutexGuard<'_, HashMap<i64, Held>> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// A wait held on one Set, released as it is dropped.
pub struct WaitHeld {
    waits: Waits,
    set_id: i64,
}

impl Drop for WaitHeld {
    fn drop(&mut self) {
        self.waits.release(self.set_id);
    }
}

/// Whether an agent is to be taken as waiting, from how many waits are held on
/// its Set right now, when the last one was released, when the Set was created,
/// and the time now.
///
/// The times are wall-clock so that the release of a wait and the creation of a
/// Set — one measured as it happens, one read back out of SQLite — can be
/// compared with each other at all. A clock stepped backwards can therefore hold
/// a badge on "agent waiting" for longer than it should, which costs display and
/// nothing else.
fn verdict(
    holders: usize,
    released_at: Option<OffsetDateTime>,
    created_at: Option<OffsetDateTime>,
    now: OffsetDateTime,
) -> Liveness {
    if holders > 0 {
        return Liveness::Waiting;
    }

    // A Set that has never had a wait held on it measures the window from its
    // creation instead, so a Set one second old is not born disconnected: its
    // agent has just submitted it and is on its way to the first wait.
    let Some(since) = released_at.or(created_at) else {
        // No wait ever held, and a `created_at` that will not parse: there is
        // nothing left to measure, so claim nothing on the agent's behalf.
        return Liveness::Disconnected;
    };

    if now - since < GRACE {
        Liveness::Waiting
    } else {
        Liveness::Disconnected
    }
}

#[cfg(test)]
mod tests {
    use super::{GRACE, verdict};
    use askance_schema::Liveness;
    use time::OffsetDateTime;
    use time::format_description::well_known::Rfc3339;

    fn at(stamp: &str) -> OffsetDateTime {
        OffsetDateTime::parse(stamp, &Rfc3339).unwrap()
    }

    /// The time now in every test below, so the fixtures read as "so long ago".
    fn now() -> OffsetDateTime {
        at("2026-08-03T12:00:00.000Z")
    }

    #[test]
    fn a_set_with_a_wait_held_on_it_is_waiting_whatever_the_clock_says() {
        // Created and last released long enough ago to be disconnected on the
        // window alone: a held wait is the end of the question.
        assert_eq!(
            verdict(1, Some(now() - GRACE * 10), Some(now() - GRACE * 10), now()),
            Liveness::Waiting
        );
    }

    #[test]
    fn a_wait_released_inside_the_window_is_still_waiting() {
        let released = now() - (GRACE - time::Duration::seconds(1));

        assert_eq!(
            verdict(0, Some(released), Some(now() - GRACE * 10), now()),
            Liveness::Waiting,
            "an agent between polls should not flap the badge"
        );
    }

    #[test]
    fn a_wait_released_at_the_edge_of_the_window_is_disconnected() {
        assert_eq!(
            verdict(0, Some(now() - GRACE), None, now()),
            Liveness::Disconnected
        );
        assert_eq!(
            verdict(
                0,
                Some(now() - GRACE - time::Duration::seconds(1)),
                None,
                now()
            ),
            Liveness::Disconnected
        );
    }

    #[test]
    fn a_set_no_wait_has_reached_yet_measures_the_window_from_its_creation() {
        assert_eq!(
            verdict(0, None, Some(now() - time::Duration::seconds(1)), now()),
            Liveness::Waiting,
            "a Set a second old should not be born disconnected"
        );
        assert_eq!(
            verdict(0, None, Some(now() - GRACE), now()),
            Liveness::Disconnected,
            "an agent that never opened a wait has had its window"
        );
    }

    #[test]
    fn the_release_of_a_wait_outlives_the_creation_of_its_set() {
        // An old Set whose agent is polling: what matters is the last release,
        // not how long the Set has been sitting there.
        assert_eq!(
            verdict(0, Some(now()), Some(now() - GRACE * 100), now()),
            Liveness::Waiting
        );
    }

    #[test]
    fn nothing_to_measure_reads_as_disconnected() {
        assert_eq!(verdict(0, None, None, now()), Liveness::Disconnected);
    }
}
