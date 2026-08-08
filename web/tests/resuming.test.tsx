//! Coming back to the app after it was away — the PWA reopened, the phone
//! unlocked, the tab refocused. The document becoming visible again is the one
//! signal that reliably fires on an iOS PWA resume, and every open page reads
//! itself afresh off it.
//!
//! Driven through `App` rather than through a page on its own, because what is
//! being asked about is the app's own query client: the refetch is that
//! client's doing, and a test that built a client of its own would only be
//! asking about the client it built. Both pages are reached on their real
//! routes, for the same reason.

import { render, screen, waitFor } from "@solidjs/testing-library";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { App } from "../src/App";
import type { PendingEntry, SetView } from "../src/api/types";
import { json, serving } from "./serving";
import pending from "./fixtures/pending.json" with { type: "json" };
import answered from "./fixtures/set-answered.json" with { type: "json" };
import answering from "./fixtures/set-answering.json" with { type: "json" };

const SETS = pending as PendingEntry[];

/// One Set twice over: waiting when the app went away, answered from another
/// device by the time it came back.
const WAITING = answering as SetView;
const ANSWERED = answered as SetView;

/// What arrived while nobody was looking — the Set the human must not have to
/// wait ten seconds to be told about.
const ARRIVAL: PendingEntry = {
  id: 7,
  title: "Whether to keep the outbound queue at all",
  project: "askance",
  branch: "outbound-retries",
  age: "just now",
  created_stamp: "2026-08-03 09:17 UTC",
  liveness: "waiting",
};

beforeEach(() => {
  // The poll is the fallback these tests are here to get ahead of, so the clock
  // is held still: anything a page learns here, it learned from coming back.
  vi.useFakeTimers();
});

afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllGlobals();
  // The state is the instance's own, over the one every other test reads off
  // the prototype — so it goes when the test does.
  delete (document as { visibilityState?: DocumentVisibilityState })
    .visibilityState;
});

/// The app going where the human put it, as the browser says so: the state
/// first, because a listener reads it rather than the event.
///
/// Dispatched on the document and left to bubble, which is where the browser
/// fires it and how it reaches a listener on the window.
function showing(state: DocumentVisibilityState): void {
  Object.defineProperty(document, "visibilityState", {
    configurable: true,
    get: () => state,
  });
  document.dispatchEvent(new Event("visibilitychange", { bubbles: true }));
}

/// Away and back again — the whole of a resume, which is two transitions and
/// not one.
function reopened(): void {
  showing("hidden");
  showing("visible");
}

describe("coming back to the app", () => {
  it("shows the Set that arrived while it was away", async () => {
    window.history.pushState({}, "", "/");
    const fetching = serving(json(SETS), json([ARRIVAL, ...SETS]));
    render(() => <App />);
    await waitFor(() => screen.getByText(SETS[0]!.title));

    reopened();

    await waitFor(() => screen.getByText(ARRIVAL.title));
    // The clock never moved, so the ten-second poll never ran: the second read
    // is the one coming back asked for.
    expect(fetching).toHaveBeenCalledTimes(2);
  });

  it("catches up the Set whose page was open when it went away", async () => {
    window.history.pushState({}, "", `/sets/${WAITING.id}`);
    serving(json(WAITING), json(ANSWERED));
    const { container } = render(() => <App />);
    // The badge and the menu under it belong to a Set still waiting: this is
    // the page as it was left.
    await waitFor(() => expect(container.querySelector(".standing")).toBeTruthy());

    reopened();

    // Answered from another device in the meantime, so the page the human comes
    // back to is the record rather than the form they left.
    await waitFor(() =>
      expect(container.querySelector(".answered-at")).toBeTruthy(),
    );
  });

  it("asks for nothing while the app is away", async () => {
    window.history.pushState({}, "", "/");
    const fetching = serving(json(SETS));
    render(() => <App />);
    await waitFor(() => screen.getByText(SETS[0]!.title));

    showing("hidden");
    await vi.advanceTimersByTimeAsync(0);

    // Going away is not coming back: the phone pays for a fetch, and there is
    // nobody there to read what it brings.
    expect(fetching).toHaveBeenCalledTimes(1);
  });
});
