//! The worker relaying a push to the pages that are already open (ADR-0005).
//!
//! A push is the channel that survives what the Nudge stream does not: iOS
//! suspends the PWA and the stream dies with it, while the push service wakes
//! the worker anyway. So every push does two things — it shows its notification,
//! and it tells every open window to look again.
//!
//! The notification is the part that must never be traded away. The subscription
//! was made `userVisibleOnly`, and a push that showed nothing would cost it, so
//! each test here that arranges some difficulty asks about the notification too.

import { describe, expect, it } from "vitest";

import { ORIGIN, UNREADABLE, worker } from "./worker";

/// A push as the server sends it — see `Notice` in `crates/server/src/push.rs`.
const NOTICE = {
  id: 7,
  title: "Whether to keep the outbound queue at all",
  project: "askance",
};

/// What the worker posts to say the pending world moved. Named, and asserted as
/// a literal, because the page's half of this cannot import it: the worker is a
/// static file served from the site root rather than a module of the bundle.
const NUDGE = { askance: "nudge" };

describe("a push arriving at the worker", () => {
  it("nudges every window that is open", async () => {
    const sw = worker();
    const list = sw.opens();
    const set = sw.opens(`${ORIGIN}/sets/3`);

    await sw.pushes(NOTICE);

    // Both of them, because a Nudge says nothing about what changed and so
    // nothing about which page wanted to hear it.
    expect(list.posted).toEqual([NUDGE]);
    expect(set.posted).toEqual([NUDGE]);
  });

  it("still shows the notification", async () => {
    const sw = worker();
    sw.opens();

    await sw.pushes(NOTICE);

    // Even with the window open on the very list this would refresh: Apple
    // expects every web push to surface a notification, and suppressing one
    // risks the subscription on the platform actually in use.
    expect(sw.shown).toHaveLength(1);
    expect(sw.shown[0]!.title).toBe(NOTICE.title);
  });

  it("notifies and relays when the push carries nothing readable", async () => {
    const sw = worker();
    const open = sw.opens();

    await sw.pushes(UNREADABLE);

    // The body is what names the Set; that it arrived at all is the news, and
    // the news is all a Nudge ever carried.
    expect(open.posted).toEqual([NUDGE]);
    expect(sw.shown).toHaveLength(1);
  });

  it("notifies where there is no window to relay to", async () => {
    const sw = worker();

    await sw.pushes(NOTICE);

    // The ordinary case: the phone is locked and the app is closed, which is
    // the case the notification exists for.
    expect(sw.relayed).toEqual([]);
    expect(sw.shown).toHaveLength(1);
  });
});
