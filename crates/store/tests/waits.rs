//! The registry of held waits: taking a slot, and giving it back however the
//! wait ends.

use askance_schema::Liveness;
use askance_store::Waits;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

/// A Set created long enough ago that its own grace window is closed: what these
/// tests are asking about is the waits held on it, not its age.
const CREATED: &str = "2026-08-03T12:00:00.000Z";

fn at(stamp: &str) -> OffsetDateTime {
    OffsetDateTime::parse(stamp, &Rfc3339).unwrap()
}

/// Long after the Set was created, so nothing but a held or lately released wait
/// can make it read as waiting.
fn later() -> OffsetDateTime {
    at(CREATED) + time::Duration::hours(1)
}

#[test]
fn a_held_wait_reads_as_waiting_and_a_released_one_stops() {
    let waits = Waits::new();

    {
        let _held = waits.hold(7);
        assert_eq!(waits.liveness(7, CREATED, later()), Liveness::Waiting);
    }

    assert_eq!(waits.liveness(7, CREATED, later()), Liveness::Disconnected);
}

#[test]
fn a_set_nothing_has_ever_waited_on_measures_its_own_age() {
    let waits = Waits::new();

    assert_eq!(
        waits.liveness(404, CREATED, at("2026-08-03T12:00:01.000Z")),
        Liveness::Waiting,
        "a Set submitted a moment ago is on its way to its first wait"
    );
    assert_eq!(
        waits.liveness(404, CREATED, later()),
        Liveness::Disconnected,
        "an agent that never opened a wait has had its window"
    );
}

#[test]
fn one_agent_reconnecting_does_not_release_another_agents_slot() {
    let waits = Waits::new();

    let first = waits.hold(7);
    let second = waits.hold(7);

    drop(first);
    assert_eq!(
        waits.liveness(7, CREATED, later()),
        Liveness::Waiting,
        "a slot is still held, so the Set is still being waited on"
    );

    drop(second);
    assert_eq!(waits.liveness(7, CREATED, later()), Liveness::Disconnected);
}

/// A vanished client does not let its handler return — the future holding the
/// slot is dropped where it stands, mid-await. That is the ending a badge would
/// otherwise stay stuck on "agent waiting" after.
#[tokio::test]
async fn a_wait_dropped_mid_hold_releases_its_slot() {
    let waits = Waits::new();

    let holding = tokio::spawn({
        let waits = waits.clone();
        async move {
            let _held = waits.hold(7);
            std::future::pending::<()>().await;
        }
    });

    // Let the spawned wait reach its await with the slot taken.
    tokio::task::yield_now().await;
    assert_eq!(waits.liveness(7, CREATED, later()), Liveness::Waiting);

    holding.abort();
    assert!(
        holding.await.unwrap_err().is_cancelled(),
        "the wait should have been dropped rather than returned"
    );

    assert_eq!(
        waits.liveness(7, CREATED, later()),
        Liveness::Disconnected,
        "a dropped wait should not leave its slot held"
    );
}
