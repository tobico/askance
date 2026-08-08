//! Wording the one stamp the viewer is handed raw.
//!
//! The assertions are `crates/render/src/when.rs`'s own, against the same
//! stamps: the two have to agree about how a settled Set is worded, because
//! they word the same settlings — the Archive's rows on the server's side of
//! the wire, and the record of one Set on this side.

import { describe, expect, it } from "vitest";

import { settledAge, utcStamp } from "../src/set/when";

/// The clock held still, so the ages under test never move.
const NOW = Date.parse("2026-08-03T12:00:00.000Z");

describe("wording a settled Set", () => {
  it("ages a fresh settling in the roughest unit that still says something", () => {
    expect(settledAge("2026-08-03T11:59:31.000Z", NOW)).toBe("just now");
    expect(settledAge("2026-08-03T11:52:00.000Z", NOW)).toBe("8m ago");
    expect(settledAge("2026-08-03T09:00:00.000Z", NOW)).toBe("3h ago");
    expect(settledAge("2026-07-28T12:30:00.000Z", NOW)).toBe("5d ago");
  });

  it("dates a settling a week old instead", () => {
    expect(settledAge("2026-07-27T11:00:00.000Z", NOW)).toBe("2026-07-27");
    expect(settledAge("2025-01-15T09:07:00.000Z", NOW)).toBe("2025-01-15");
  });

  it("dates it in UTC, whatever zone stamped it", () => {
    // 01:00+10:00 is still the previous day in UTC.
    expect(settledAge("2026-01-01T01:00:00+10:00", NOW)).toBe("2025-12-31");
  });

  it("hands a stamp that will not parse back as it was stored", () => {
    expect(settledAge("  not a timestamp  ", NOW)).toBe("not a timestamp");
  });
});

describe("the exact stamp behind the words", () => {
  it("says it to the minute, in UTC, out loud", () => {
    expect(utcStamp("2026-08-03T09:07:42.123Z")).toBe("2026-08-03 09:07 UTC");
  });

  it("says a stamp from another zone in UTC", () => {
    expect(utcStamp("2026-08-03T19:07:00+10:00")).toBe("2026-08-03 09:07 UTC");
  });

  it("hands a stamp that will not parse back as it was stored", () => {
    expect(utcStamp("  not a timestamp  ")).toBe("not a timestamp");
  });
});
