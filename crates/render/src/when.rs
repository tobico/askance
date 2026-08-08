//! How a time is worded for the human, on the side of the wire that has the
//! clock.
//!
//! The lists' stamps arrive at the viewer already said in words rather than as
//! timestamps: the server has the clock and the calendar, and this way the
//! viewer needs neither a date library nor an opinion about the reader's
//! timezone to draw a list.
//!
//! Every stamp is worded the same way: relative while it is fresh, because a
//! list is scanned and "3h ago" is what a scan wants — and once it is a week
//! old, the plain date, because "400d ago" says nothing and the Archive is a
//! permanent log. The exact minute is never lost either way: it travels beside
//! the words, for the tooltip on them.

use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

/// How many days a stamp stays relative before it is said as a date: a week,
/// past which counting days stops meaning anything to a reader.
const FRESH_DAYS: i64 = 7;

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

/// When a Set was settled: an age while the settling is fresh, and the plain
/// date once it is not.
///
/// Relative for the first week because the top of the Archive is scanned like
/// the pending list, and a decision made this morning is found by how recent it
/// is. Dated past that because ages stop meaning anything at that distance —
/// and dated in UTC, like the exact stamp beside it.
///
/// A timestamp that will not parse is handed back as it was stored: what the
/// store holds is more use to whoever has to explain it than nothing at all.
pub fn settled_age(settled_at: &str, now: OffsetDateTime) -> String {
    let Ok(then) = OffsetDateTime::parse(settled_at, &Rfc3339) else {
        return settled_at.trim().to_owned();
    };

    if (now - then).whole_days() < FRESH_DAYS {
        return relative_age(settled_at, now);
    }

    let date = then.to_offset(time::UtcOffset::UTC).date();
    format!(
        "{}-{:02}-{:02}",
        date.year(),
        u8::from(date.month()),
        date.day(),
    )
}

/// A stamp said exactly, to the minute and in UTC — the tooltip behind every
/// worded time.
///
/// UTC rather than the reader's own zone, and said out loud: the server is the
/// only one of the two that has a clock in this arrangement, and a bare
/// "14:32" that turns out to be somewhere else's afternoon is worse than an
/// hour the reader has to convert.
///
/// A timestamp that will not parse is handed back as it was stored: what the
/// store holds is more use to whoever has to explain it than nothing at all.
pub fn utc_stamp(stamp: &str) -> String {
    let Ok(when) = OffsetDateTime::parse(stamp, &Rfc3339) else {
        return stamp.trim().to_owned();
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
    use super::{relative_age, settled_age, utc_stamp};
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
    fn a_freshly_settled_set_is_aged() {
        let now = at("2026-08-03T12:00:00.000Z");

        assert_eq!(settled_age("2026-08-03T11:52:00.000Z", now), "8m ago");
        assert_eq!(settled_age("2026-08-03T09:00:00.000Z", now), "3h ago");
        assert_eq!(settled_age("2026-07-28T12:30:00.000Z", now), "5d ago");
    }

    #[test]
    fn a_settling_a_week_old_is_dated_instead() {
        let now = at("2026-08-03T12:00:00.000Z");

        assert_eq!(settled_age("2026-07-27T11:00:00.000Z", now), "2026-07-27");
        assert_eq!(settled_age("2025-01-15T09:07:00.000Z", now), "2025-01-15");
    }

    #[test]
    fn a_settling_is_dated_in_utc_whatever_zone_stamped_it() {
        let now = at("2026-08-03T12:00:00.000Z");

        // 01:00+10:00 is still the previous day in UTC.
        assert_eq!(settled_age("2026-01-01T01:00:00+10:00", now), "2025-12-31");
    }

    #[test]
    fn an_unparseable_settling_is_handed_back_as_it_was_stored() {
        let now = at("2026-08-03T12:00:00.000Z");

        assert_eq!(settled_age("  not a timestamp  ", now), "not a timestamp");
    }

    #[test]
    fn the_exact_stamp_is_said_to_the_minute_in_utc() {
        assert_eq!(
            utc_stamp("2026-08-03T09:07:42.123Z"),
            "2026-08-03 09:07 UTC"
        );
    }

    #[test]
    fn a_stamp_from_another_zone_is_said_in_utc() {
        assert_eq!(
            utc_stamp("2026-08-03T19:07:00+10:00"),
            "2026-08-03 09:07 UTC"
        );
    }

    #[test]
    fn a_stamp_that_will_not_parse_is_handed_back_as_it_was_stored() {
        assert_eq!(utc_stamp("  not a timestamp  "), "not a timestamp");
    }
}
