//! How a time is worded for the human, on the side of the wire that has the
//! clock.
//!
//! Both of these arrive at the viewer already said in words rather than as a
//! timestamp: the server has the clock and the calendar, and this way the
//! viewer needs neither a date library nor an opinion about the reader's
//! timezone to draw a list.
//!
//! Which of the two a place gets is a question of what is being read. A Set
//! still waiting is scanned, so it is aged — how long it has been sitting
//! there is the whole of what the row has to say about time. A settled Set is a
//! decision in a permanent log, so it is dated: "3h ago" stops meaning anything
//! by the following week.

use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

/// How long ago `created_at` was, in the roughest unit that still says
/// something: a pending list is scanned, not read.
///
/// A timestamp that will not parse is not worth failing the list over — it is
/// still useful without an age — so it comes back empty.
pub fn relative_age(created_at: &str, now: OffsetDateTime) -> String {
    let Ok(then) = OffsetDateTime::parse(created_at, &Rfc3339) else {
        return String::new();
    };

    let seconds = (now - then).whole_seconds();
    if seconds < 60 {
        return "just now".to_owned();
    }

    let minutes = seconds / 60;
    if minutes < 60 {
        return format!("{minutes}m ago");
    }

    let hours = minutes / 60;
    if hours < 24 {
        return format!("{hours}h ago");
    }

    format!("{}d ago", hours / 24)
}

/// When a Set was settled, to the minute and in UTC.
///
/// UTC rather than the reader's own zone, and said out loud: the server is the
/// only one of the two that has a clock in this arrangement, and a bare
/// "14:32" that turns out to be somewhere else's afternoon is worse than an
/// hour the reader has to convert.
///
/// A timestamp that will not parse is handed back as it was stored: what the
/// store holds is more use to whoever has to explain it than nothing at all.
pub fn settled_when(settled_at: &str) -> String {
    let Ok(when) = OffsetDateTime::parse(settled_at, &Rfc3339) else {
        return settled_at.trim().to_owned();
    };

    let when = when.to_offset(time::UtcOffset::UTC);
    format!(
        "{}-{:02}-{:02} {:02}:{:02} UTC",
        when.year(),
        u8::from(when.month()),
        when.day(),
        when.hour(),
        when.minute(),
    )
}

#[cfg(test)]
mod tests {
    use super::{relative_age, settled_when};
    use time::OffsetDateTime;
    use time::format_description::well_known::Rfc3339;

    fn at(stamp: &str) -> OffsetDateTime {
        OffsetDateTime::parse(stamp, &Rfc3339).unwrap()
    }

    #[test]
    fn ages_are_worded_in_the_roughest_useful_unit() {
        let now = at("2026-08-03T12:00:00.000Z");

        assert_eq!(relative_age("2026-08-03T11:59:31.000Z", now), "just now");
        assert_eq!(relative_age("2026-08-03T11:52:00.000Z", now), "8m ago");
        assert_eq!(relative_age("2026-08-03T09:00:00.000Z", now), "3h ago");
        assert_eq!(relative_age("2026-07-31T12:00:00.000Z", now), "3d ago");
    }

    #[test]
    fn an_unparseable_stamp_costs_only_its_age() {
        let now = at("2026-08-03T12:00:00.000Z");

        assert_eq!(relative_age("not a timestamp", now), "");
    }

    #[test]
    fn a_settled_set_is_dated_to_the_minute_in_utc() {
        assert_eq!(
            settled_when("2026-08-03T09:07:42.123Z"),
            "2026-08-03 09:07 UTC"
        );
    }

    #[test]
    fn a_stamp_from_another_zone_is_said_in_utc() {
        assert_eq!(
            settled_when("2026-08-03T19:07:00+10:00"),
            "2026-08-03 09:07 UTC"
        );
    }

    #[test]
    fn a_stamp_that_will_not_parse_is_handed_back_as_it_was_stored() {
        assert_eq!(settled_when("  not a timestamp  "), "not a timestamp");
    }
}
