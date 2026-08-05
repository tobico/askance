//! The one date the viewer is handed raw.
//!
//! Everywhere else a time reaches the browser already said in words, because the
//! server has the clock and the calendar — see `crates/render/src/when.rs`. A
//! Set's own standing is the exception: the stamp travels with the Response it
//! belongs to, so the wording happens here instead, in the same words and to the
//! same rules.

/// When a Set was settled, to the minute and in UTC.
///
/// UTC rather than the reader's own zone, and said out loud: the server is the
/// only one of the two that has a clock in this arrangement, and a bare "14:32"
/// that turns out to be somewhere else's afternoon is worse than an hour the
/// reader has to convert.
///
/// A timestamp that will not parse is handed back as it was stored: what the
/// store holds is more use to whoever has to explain it than nothing at all.
export function settledWhen(settledAt: string): string {
  const when = new Date(settledAt);

  if (Number.isNaN(when.getTime())) {
    return settledAt.trim();
  }

  return (
    `${when.getUTCFullYear()}-${two(when.getUTCMonth() + 1)}-${two(when.getUTCDate())}` +
    ` ${two(when.getUTCHours())}:${two(when.getUTCMinutes())} UTC`
  );
}

function two(part: number): string {
  return String(part).padStart(2, "0");
}
