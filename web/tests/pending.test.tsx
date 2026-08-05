//! The pending list, fed by the payload the server actually writes.
//!
//! `tests/fixtures/pending.json` is a golden fixture: `cargo test` renders the
//! real `/api/ui/pending` and writes the file, so these assertions are made
//! against the endpoint's own words rather than against a mock that agrees with
//! this file by construction. When the wire shape moves, the fixture moves with
//! it and these tests are what notice.

import { screen, waitFor } from "@solidjs/testing-library";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { PendingEntry } from "../src/api/types";
import { PendingList } from "../src/pending/PendingList";
import { mount, texts } from "./listing";
import { json, serving } from "./serving";
import pending from "./fixtures/pending.json" with { type: "json" };

const SETS = pending as PendingEntry[];

/// The two Sets the fixture holds, by the badge each is there to exercise.
const LIVE = SETS.find((set) => set.liveness === "waiting")!;
const DEAD = SETS.find((set) => set.liveness === "disconnected")!;

beforeEach(() => {
  vi.useFakeTimers();
});

afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllGlobals();
});

describe("the pending list", () => {
  it("asks the server for the Sets waiting on the human", async () => {
    const fetching = serving(json(SETS));
    mount(PendingList);

    await waitFor(() => screen.getByText(LIVE.title));
    expect(fetching).toHaveBeenCalledWith(
      "/api/ui/pending",
      expect.anything(),
    );
  });

  it("draws a row per Set, with what the server said about each", async () => {
    serving(json(SETS));
    mount(PendingList);

    const row = (await waitFor(() => screen.getByText(DEAD.title))).closest("li")!;

    expect(row.querySelector(".project")!.textContent).toBe(DEAD.project);
    expect(row.querySelector(".branch")!.textContent).toBe(DEAD.branch);
    expect(row.querySelector(".age")!.textContent).toBe(DEAD.age);
    // The whole row is the tap target, so the link is the row's and not the
    // title's.
    expect(row.querySelector("a")!.getAttribute("href")).toBe(`/sets/${DEAD.id}`);
  });

  it("keeps the order it was given, which is newest ask first", async () => {
    serving(json(SETS));
    const { container } = mount(PendingList);

    await waitFor(() => screen.getByText(LIVE.title));

    // The list is scanned from the top for what has just come in, and the server
    // is what ordered it — the page must not reorder what it was handed.
    expect(texts(container, ".set-row .title")).toEqual(
      SETS.map((set) => set.title),
    );
  });

  it("offers the way through to what was already decided", async () => {
    serving(json(SETS));
    const { container } = mount(PendingList);

    await waitFor(() => screen.getByText(LIVE.title));

    const through = container.querySelector(".to-archive")!;
    expect(through.getAttribute("href")).toBe("/archive");
  });

  it("says of each Set whether an agent is still waiting on it", async () => {
    serving(json(SETS));
    mount(PendingList);

    await waitFor(() => screen.getByText(LIVE.title));

    expect(screen.getByText("agent waiting").className).toBe("liveness waiting");
    expect(screen.getByText("agent disconnected").className).toBe(
      "liveness disconnected",
    );
  });

  it("says so plainly when nothing is waiting", async () => {
    serving(json([]));
    mount(PendingList);

    await waitFor(() => screen.getByText("Nothing is waiting on you."));
    expect(screen.queryByRole("listitem")).toBeNull();
  });

  it("shows the server's own wording when the list cannot be read", async () => {
    serving(json({ error: "the pending Sets could not be read" }, 500));
    mount(PendingList);

    await waitFor(() =>
      screen.getByText(/the pending Sets could not be read/),
    );
  });

  it("picks up a Set that arrived while the page was open", async () => {
    const arrival: PendingEntry = {
      id: 7,
      title: "Whether to keep the outbound queue at all",
      project: "askance",
      branch: "outbound-retries",
      age: "just now",
      liveness: "waiting",
    };

    serving(json(SETS), json([arrival, ...SETS]));
    mount(PendingList);
    await waitFor(() => screen.getByText(LIVE.title));

    await vi.advanceTimersByTimeAsync(10_000);
    await waitFor(() => screen.getByText(arrival.title));
  });

  it("does not blink the list away while it is refetching", async () => {
    // The second answer is held open, which is the moment a fallback would
    // show: the list is stale, the request is in flight, and the human is
    // reading. Nothing may be taken off the page.
    let deliver: () => void;
    const held = new Promise<void>((resolve) => {
      deliver = resolve;
    });

    serving(
      json(SETS),
      () => held.then(() => new Response(JSON.stringify(SETS))),
    );
    mount(PendingList);
    await waitFor(() => screen.getByText(LIVE.title));

    await vi.advanceTimersByTimeAsync(10_000);

    expect(screen.getByText(LIVE.title)).toBeTruthy();
    expect(screen.queryByText("Loading…")).toBeNull();

    deliver!();
  });
});
