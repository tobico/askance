//! The Nudge: the open page being told the pending world moved, and looking
//! again because of it — over the server's stream, and relayed by the service
//! worker off a push (ADR-0005).
//!
//! Driven through `App` for the same reason `resuming` is — what the Nudge acts
//! on is the app's own query client, and a test that built a client of its own
//! would be asserting its own arrangement rather than the app's.
//!
//! The clock is held still throughout, except where the poll is the thing being
//! asked about: anything a page learns here it learned from a Nudge, because the
//! fallback underneath never ran.

import { render, screen, waitFor } from "@solidjs/testing-library";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { App } from "../src/App";
import type { PendingEntry, SetView } from "../src/api/types";
import { askedFor, json, serving, whenever } from "./serving";
import { worker } from "./worker";
import pending from "./fixtures/pending.json" with { type: "json" };
import answered from "./fixtures/set-answered.json" with { type: "json" };
import answering from "./fixtures/set-answering.json" with { type: "json" };

const SETS = pending as PendingEntry[];

/// The page asks about updating as well as about the Sets, and is told there is
/// nothing to update to throughout: a Nudge is never about a release, so the
/// banner's own request stays out of the counting here.
const CURRENT = whenever("/api/ui/update", json("Current"));

/// The read a Nudge is meant to cause, and the only one worth counting.
const PENDING = "/api/ui/pending";

/// One Set twice over: waiting when the page was drawn, answered from another
/// device by the time a Nudge says to look again.
const WAITING = answering as SetView;
const ANSWERED = answered as SetView;

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

/// A stand-in for `navigator.serviceWorker`, the page's end of the relay, which
/// jsdom has none of either.
class Container {
  private readonly listeners = new Set<(event: MessageEvent) => void>();

  addEventListener(
    _name: "message",
    listener: (event: MessageEvent) => void,
  ): void {
    this.listeners.add(listener);
  }

  removeEventListener(
    _name: "message",
    listener: (event: MessageEvent) => void,
  ): void {
    this.listeners.delete(listener);
  }

  /// Whether anything on the page is still listening for a relayed Nudge.
  get listening(): boolean {
    return this.listeners.size > 0;
  }

  /// One message from the worker, as the browser hands it over.
  delivers(data: unknown): void {
    for (const listener of [...this.listeners]) {
      listener(new MessageEvent("message", { data }));
    }
  }
}

/// A browser whose worker can reach this page. Defined on the real navigator
/// rather than stubbed over it, because everything else the app reads there is
/// still wanted — and taken away again after the test.
function attaches(): Container {
  const container = new Container();
  Object.defineProperty(navigator, "serviceWorker", {
    configurable: true,
    value: container,
  });
  return container;
}

/// A push arriving at the service worker and relayed on, as the worker relays
/// it: the message delivered is the worker's own — `assets/sw.js` driven with a
/// real push — so what the page is asked about is what it will really be sent
/// rather than a guess at it.
async function pushed(container: Container): Promise<void> {
  const sw = worker();
  sw.opens();
  await sw.pushes({
    id: ARRIVAL.id,
    title: ARRIVAL.title,
    project: ARRIVAL.project,
  });

  for (const message of sw.relayed) {
    container.delivers(message);
  }
}

beforeEach(() => {
  vi.useFakeTimers();
  Streaming.opened = [];
  vi.stubGlobal("EventSource", Streaming);
});

afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllGlobals();
  // The property is this test's own, over a navigator every other test shares.
  delete (navigator as { serviceWorker?: unknown }).serviceWorker;
});

describe("the Nudge stream", () => {
  it("listens on the server's stream for as long as the app is running", () => {
    window.history.pushState({}, "", "/");
    serving(CURRENT, json(SETS));
    const { unmount } = render(() => <App />);

    expect(stream().url).toBe("/api/ui/nudges");

    unmount();

    // Nothing is left holding a connection open behind a page that is gone.
    expect(stream().closed).toBe(true);
  });

  it("shows the Set a Nudge is about without waiting on the poll", async () => {
    window.history.pushState({}, "", "/");
    const fetching = serving(CURRENT, json(SETS), json([ARRIVAL, ...SETS]));
    render(() => <App />);
    await waitFor(() => screen.getByText(SETS[0]!.title));
    stream().opens();

    stream().nudges();

    await waitFor(() => screen.getByText(ARRIVAL.title));
    // The clock never moved, so the ten-second poll never ran: the second read
    // is the Nudge's doing and nothing else's.
    expect(askedFor(fetching, PENDING)).toBe(2);
  });

  it("reads everything back when a dropped stream reconnects", async () => {
    window.history.pushState({}, "", "/");
    // Answered from another device while the stream was dead — so what the page
    // has to catch up on is a Set leaving the list, which no Nudge arrived to
    // say. The reconnect is the whole of the news.
    serving(CURRENT, json([ARRIVAL, ...SETS]), json(SETS));
    render(() => <App />);
    stream().opens();
    await waitFor(() => screen.getByText(ARRIVAL.title));

    stream().opens();

    await waitFor(() => expect(screen.queryByText(ARRIVAL.title)).toBeNull());
  });

  it("asks for nothing when the stream first opens", async () => {
    window.history.pushState({}, "", "/");
    const fetching = serving(CURRENT, json(SETS));
    render(() => <App />);
    await waitFor(() => screen.getByText(SETS[0]!.title));

    stream().opens();
    await vi.advanceTimersByTimeAsync(0);

    // The page has just read the world; opening the stream it reads the world
    // over is not news that the world moved.
    expect(askedFor(fetching, PENDING)).toBe(1);
  });

  it("leaves the poll running underneath it", async () => {
    window.history.pushState({}, "", "/");
    const fetching = serving(CURRENT, json(SETS));
    render(() => <App />);
    await waitFor(() => screen.getByText(SETS[0]!.title));
    stream().opens();

    await vi.advanceTimersByTimeAsync(10_000);

    // The stream is the fast path, never the only one: a page that cannot have
    // one at all still keeps up, ten seconds at a time.
    expect(askedFor(fetching, PENDING)).toBe(2);
  });
});

describe("a Nudge relayed by the worker", () => {
  it("shows the Set the push was about, with no stream to hear it on", async () => {
    window.history.pushState({}, "", "/");
    // The list as the server would answer for it now, which the arrival changes
    // under the open page exactly as it does in the world.
    let listed = SETS;
    const fetching = serving(CURRENT, () => json(listed)());
    const container = attaches();
    render(() => <App />);
    await waitFor(() => screen.getByText(SETS[0]!.title));
    const read = askedFor(fetching, PENDING);

    listed = [ARRIVAL, ...SETS];
    await pushed(container);

    await waitFor(() => screen.getByText(ARRIVAL.title));
    // One read more than the page had already done, and not one beyond it: the
    // stream never opened — which is what a suspended PWA leaves behind — and
    // the clock never moved, so the poll never ran either. The push is the whole
    // of how this page found out.
    expect(askedFor(fetching, PENDING)).toBe(read + 1);
  });

  it("reads everything back, exactly as a Nudge on the stream does", async () => {
    window.history.pushState({}, "", `/sets/${WAITING.id}`);
    serving(CURRENT, json(WAITING), json(ANSWERED));
    const container = attaches();
    const { container: page } = render(() => <App />);
    // The badge and the menu under it belong to a Set still waiting: the page
    // as it was drawn.
    await waitFor(() => expect(page.querySelector(".standing")).toBeTruthy());

    await pushed(container);

    // A relayed Nudge is as contentless as a streamed one — it says a Set
    // arrived, not which — so the reaction is the same either way: everything
    // this page is showing, which here is a Set answered elsewhere in the
    // meantime.
    await waitFor(() => expect(page.querySelector(".answered-at")).toBeTruthy());
  });

  it("ignores a message that is not a Nudge", async () => {
    window.history.pushState({}, "", "/");
    const fetching = serving(CURRENT, json(SETS));
    const container = attaches();
    render(() => <App />);
    await waitFor(() => screen.getByText(SETS[0]!.title));
    const read = askedFor(fetching, PENDING);

    container.delivers({ askance: "something else" });
    container.delivers("a message from somewhere else entirely");
    container.delivers(null);
    await vi.advanceTimersByTimeAsync(0);

    // Whatever else may one day be posted to a page, by this worker or another,
    // is not a Nudge until it says so.
    expect(askedFor(fetching, PENDING)).toBe(read);
  });

  it("stops listening when the app goes", async () => {
    window.history.pushState({}, "", "/");
    serving(CURRENT, json(SETS));
    const container = attaches();
    const { unmount } = render(() => <App />);
    await waitFor(() => screen.getByText(SETS[0]!.title));
    expect(container.listening).toBe(true);

    unmount();

    // Nothing is left holding a query client that has no page to refresh.
    expect(container.listening).toBe(false);
  });

  it("shrugs where the browser has no worker at all", async () => {
    window.history.pushState({}, "", "/");
    serving(CURRENT, json(SETS));

    // No `attaches()`: jsdom has no `navigator.serviceWorker`, which is the same
    // absence a browser without service workers presents. The page loses the
    // relay and nothing else.
    expect(() => render(() => <App />)).not.toThrow();
    await waitFor(() => screen.getByText(SETS[0]!.title));
  });
});
