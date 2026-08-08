//! The one date the viewer is handed raw.
//!
//! Everywhere else a time reaches the browser already said in words, because the
//! server has the clock and the calendar — see `crates/render/src/when.rs`. A
//! Set's own standing is the exception: the stamp travels with the Response it
//! belongs to, so the wording happens here instead, in the same words and to the
//! same rules.

/// How many days a settling stays relative before it is said as a date: a week,
/// past which counting days stops meaning anything to a reader. The same week
/// `crates/render/src/when.rs` draws the line at, because the Archive's rows
/// and this page word the same settlings.
const FRESH_DAYS = 7;

/// When a Set was settled: an age while the settling is fresh, and the plain
/// date once it is not — `settled_age` on this side of the wire.
///
/// `now` is a parameter rather than read here so the page can hand in the same
/// ticking clock its redraws are driven by — and so a test can hold it still.
///
/// A timestamp that will not parse is handed back as it was stored: what the
/// store holds is more use to whoever has to explain it than nothing at all.
export function settledAge(settledAt: string, now: number): string {
  const when = new Date(settledAt);

  if (Number.isNaN(when.getTime())) {
    return settledAt.trim();
  }

  const seconds = Math.floor((now - when.getTime()) / 1000);
  if (seconds >= FRESH_DAYS * 24 * 60 * 60) {
    return (
      `${when.getUTCFullYear()}-${two(when.getUTCMonth() + 1)}` +
      `-${two(when.getUTCDate())}`
    );
  }

  if (seconds < 60) {
    return "just now";
  }

  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) {
    return `${minutes}m ago`;
  }

  const hours = Math.floor(minutes / 60);
  if (hours < 24) {
    return `${hours}h ago`;
  }

  return `${Math.floor(hours / 24)}d ago`;
}

/// A stamp said exactly, to the minute and in UTC — the tooltip behind the
/// worded time.
///
/// UTC rather than the reader's own zone, and said out loud: the server is the
/// only one of the two that has a clock in this arrangement, and a bare "14:32"
/// that turns out to be somewhere else's afternoon is worse than an hour the
/// reader has to convert.
///
/// A timestamp that will not parse is handed back as it was stored: what the
/// store holds is more use to whoever has to explain it than nothing at all.
export function utcStamp(stamp: string): string {
  const when = new Date(stamp);

  if (Number.isNaN(when.getTime())) {
    return stamp.trim();
  }

  return (
    `${when.getUTCFullYear()}-${two(when.getUTCMonth() + 1)}-${two(when.getUTCDate())}` +
    ` ${two(when.getUTCHours())}:${two(when.getUTCMinutes())} UTC`
  );
}

function two(part: number): string {
  return String(part).padStart(2, "0");
}
