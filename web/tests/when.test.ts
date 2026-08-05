//! Wording the one stamp the viewer is handed raw.
//!
//! The assertions are `crates/render/src/when.rs`'s own, against the same
//! stamps: the two have to agree about how a settled Set is dated, because they
//! date the same Sets — the Archive's rows on the server's side of the wire, and
//! the record of one Set on this side.

import { describe, expect, it } from "vitest";

import { settledWhen } from "../src/set/when";

describe("dating a settled Set", () => {
  it("says it to the minute, in UTC, out loud", () => {
    expect(settledWhen("2026-08-03T09:07:42.123Z")).toBe(
      "2026-08-03 09:07 UTC",
    );
  });

  it("says a stamp from another zone in UTC", () => {
    expect(settledWhen("2026-08-03T19:07:00+10:00")).toBe(
      "2026-08-03 09:07 UTC",
    );
  });

  it("hands a stamp that will not parse back as it was stored", () => {
    expect(settledWhen("  not a timestamp  ")).toBe("not a timestamp");
  });
});
