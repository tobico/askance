//! The Nudge stream: the open page being told the pending world moved, and
//! looking again because of it.
//!
//! Driven through `App` for the same reason `resuming` is — what the Nudge acts
//! on is the app's own query client, and a test that built a client of its own
//! would be asserting its own arrangement rather than the app's.
//!
//! The clock is held still throughout, except where the poll is the thing being
//! asked about: anything a page learns here it learned from the stream, because
//! the fallback underneath never ran.

import { render, screen, waitFor } from "@solidjs/testing-library";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { App } from "../src/App";
import type { PendingEntry } from "../src/api/types";
import { json, serving } from "./serving";
import pending from "./fixtures/pending.json" with { type: "json" };

const SETS = pending as PendingEntry[];

/// The Set the stream is there to be immediate about — submitted while the
/// human is looking straight at the list it belongs on.
const ARRIVAL: PendingEntry = {
  id: 7,
  title: "Whether to keep the outbound queue at all",
  project: "askance",
  branch: "outbound-retries",
  age: "just now",
  created_stamp: "2026-08-03 09:17 UTC",
  liveness: "waiting",
};

/// A stand-in for the browser's `EventSource`, which jsdom has none of — and
/// which a test would want its own of anyway, having no other way to put a
/// Nudge on the wire or to sever the connection carrying it.
class Streaming {
  /// Every stream the app has opened, newest last.
  static opened: Streaming[] = [];

  private readonly listeners = new Map<string, Array<(event: Event) => void>>();
  closed = false;

  constructor(readonly url: string) {
    Streaming.opened.push(this);
  }

  addEventListener(name: string, listener: (event: Event) => void): void {
    this.listeners.set(name, [...(this.listeners.get(name) ?? []), listener]);
  }

  close(): void {
    this.closed = true;
  }

  /// What the browser does when the connection is established — on the first
  /// one and on every reconnect after it, which is the whole of how a page
  /// finds out it was away.
  opens(): void {
    this.fire("open");
  }

  /// One Nudge, as the server writes it: a named event carrying nothing worth
  /// reading.
  nudges(): void {
    this.fire("nudge");
  }

  private fire(name: string): void {
    for (const listener of this.listeners.get(name) ?? []) {
      listener(new Event(name));
    }
  }
}

/// The stream the app opened, which there is always exactly one of: the app
/// opens it on mount and holds it for as long as it is running.
function stream(): Streaming {
  const opened = Streaming.opened.at(-1);
  if (!opened) {
    throw new Error("the app opened no stream");
  }
  return opened;
}

beforeEach(() => {
  vi.useFakeTimers();
  Streaming.opened = [];
  vi.stubGlobal("EventSource", Streaming);
});

afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllGlobals();
});

describe("the Nudge stream", () => {
  it("listens on the server's stream for as long as the app is running", () => {
    window.history.pushState({}, "", "/");
    serving(json(SETS));
    const { unmount } = render(() => <App />);

    expect(stream().url).toBe("/api/ui/nudges");

    unmount();

    // Nothing is left holding a connection open behind a page that is gone.
    expect(stream().closed).toBe(true);
  });

  it("shows the Set a Nudge is about without waiting on the poll", async () => {
    window.history.pushState({}, "", "/");
    const fetching = serving(json(SETS), json([ARRIVAL, ...SETS]));
    render(() => <App />);
    await waitFor(() => screen.getByText(SETS[0]!.title));
    stream().opens();

    stream().nudges();

    await waitFor(() => screen.getByText(ARRIVAL.title));
    // The clock never moved, so the ten-second poll never ran: the second read
    // is the Nudge's doing and nothing else's.
    expect(fetching).toHaveBeenCalledTimes(2);
  });

  it("reads everything back when a dropped stream reconnects", async () => {
    window.history.pushState({}, "", "/");
    // Answered from another device while the stream was dead — so what the page
    // has to catch up on is a Set leaving the list, which no Nudge arrived to
    // say. The reconnect is the whole of the news.
    serving(json([ARRIVAL, ...SETS]), json(SETS));
    render(() => <App />);
    stream().opens();
    await waitFor(() => screen.getByText(ARRIVAL.title));

    stream().opens();

    await waitFor(() => expect(screen.queryByText(ARRIVAL.title)).toBeNull());
  });

  it("asks for nothing when the stream first opens", async () => {
    window.history.pushState({}, "", "/");
    const fetching = serving(json(SETS));
    render(() => <App />);
    await waitFor(() => screen.getByText(SETS[0]!.title));

    stream().opens();
    await vi.advanceTimersByTimeAsync(0);

    // The page has just read the world; opening the stream it reads the world
    // over is not news that the world moved.
    expect(fetching).toHaveBeenCalledTimes(1);
  });

  it("leaves the poll running underneath it", async () => {
    window.history.pushState({}, "", "/");
    const fetching = serving(json(SETS));
    render(() => <App />);
    await waitFor(() => screen.getByText(SETS[0]!.title));
    stream().opens();

    await vi.advanceTimersByTimeAsync(10_000);

    // The stream is the fast path, never the only one: a page that cannot have
    // one at all still keeps up, ten seconds at a time.
    expect(fetching).toHaveBeenCalledTimes(2);
  });
});
