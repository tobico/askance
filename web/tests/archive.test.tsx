//! The Archive: the permanent log of every settled Set, fed by the payload the
//! server actually writes.
//!
//! `tests/fixtures/archive.json` is a golden fixture, like the pending list's:
//! `cargo test` renders the real `/api/ui/archive` and writes the file, so these
//! assertions are made against the endpoint's own words rather than against a
//! mock that agrees with this file by construction.

import { fireEvent, render, screen, waitFor } from "@solidjs/testing-library";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { App } from "../src/App";
import type { ArchiveEntry, PendingEntry } from "../src/api/types";
import { ArchiveList } from "../src/archive/ArchiveList";
import { mount, texts } from "./listing";
import { json, serving } from "./serving";
import archive from "./fixtures/archive.json" with { type: "json" };
import pending from "./fixtures/pending.json" with { type: "json" };

const SETS = archive as ArchiveEntry[];
const PENDING = pending as PendingEntry[];

/// The two Sets the fixture holds, by the way each of them got here: one that was
/// answered, and one nobody was ever going to answer.
const DECIDED = SETS.find((set) => !set.unanswered)!;
const ORPHANED = SETS.find((set) => set.unanswered)!;

beforeEach(() => {
  vi.useFakeTimers();
});

afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllGlobals();
  // The one test that mounts the whole app navigates the document's own URL, and
  // the next test starts at the top of the app.
  window.history.pushState({}, "", "/");
});

describe("the Archive", () => {
  it("asks the server for the Sets that have been settled", async () => {
    const fetching = serving(json(SETS));
    mount(ArchiveList);

    await waitFor(() => screen.getByText(DECIDED.title));
    expect(fetching).toHaveBeenCalledWith("/api/ui/archive", expect.anything());
  });

  it("draws a row per Set, with what the server said about each", async () => {
    serving(json(SETS));
    mount(ArchiveList);

    const row = (await waitFor(() => screen.getByText(DECIDED.title))).closest(
      "li",
    )!;

    expect(row.querySelector(".project")!.textContent).toBe(DECIDED.project);
    expect(row.querySelector(".branch")!.textContent).toBe(DECIDED.branch);
    // The whole card is the tap target here as on the pending list: the lists
    // are read the same way, and the same Set is looked at in both.
    expect(row.querySelector("a")!.getAttribute("href")).toBe(
      `/sets/${DECIDED.id}`,
    );
  });

  it("words each settling as the server did, and badges nothing as waiting", async () => {
    serving(json(SETS));
    const { container } = mount(ArchiveList);

    await waitFor(() => screen.getByText(DECIDED.title));

    // As the server worded it — an age while the settling is fresh, the plain
    // date once it is not — with the exact minute riding behind the words as
    // the tooltip. The fixture holds one of each.
    const row = screen.getByText(DECIDED.title).closest("li")!;
    expect(row.querySelector(".decided-at")!.textContent).toContain(
      DECIDED.settled_at,
    );
    expect(row.querySelector(".decided-at")!.getAttribute("title")).toBe(
      DECIDED.settled_stamp,
    );
    const fresh = screen.getByText(ORPHANED.title).closest("li")!;
    expect(fresh.querySelector(".decided-at")!.textContent).toContain(
      ORPHANED.settled_at,
    );
    expect(fresh.querySelector(".decided-at")!.getAttribute("title")).toBe(
      ORPHANED.settled_stamp,
    );
    expect(
      container.querySelector(".liveness"),
      "nothing is waiting on a settled Set, so there is no Liveness to badge",
    ).toBeNull();
    expect(container.querySelector(".age")).toBeNull();
  });

  it("says which of them was never answered by anybody", async () => {
    serving(json(SETS));
    mount(ArchiveList);

    await waitFor(() => screen.getByText(ORPHANED.title));

    // In the same words the set view uses, and in the same place a decision's
    // date sits: what a row of this log has to say first is which of the two it
    // is, because only one of them is a decision.
    const orphaned = screen.getByText(ORPHANED.title).closest("li")!;
    expect(orphaned.className).toBe("set-row archived-set unanswered");
    expect(orphaned.querySelector(".decided-at")!.textContent).toBe(
      `archived unanswered ${ORPHANED.settled_at}`,
    );

    const decided = screen.getByText(DECIDED.title).closest("li")!;
    expect(decided.className).toBe("set-row archived-set");
    expect(decided.querySelector(".decided-at")!.textContent).toBe(
      `answered ${DECIDED.settled_at}`,
    );
  });

  it("keeps the order it was given, which is newest settlement first", async () => {
    serving(json(SETS));
    const { container } = mount(ArchiveList);

    await waitFor(() => screen.getByText(DECIDED.title));

    // The log is read along the settling, and the server is what ordered it —
    // the page must not reorder what it was handed.
    expect(texts(container, ".set-row .title")).toEqual(
      SETS.map((set) => set.title),
    );
  });

  it("says so plainly when nothing has been settled yet", async () => {
    serving(json([]));
    mount(ArchiveList);

    // Both ways into the Archive, because a Set reaches it either way.
    await waitFor(() =>
      screen.getByText("Nothing has been answered or archived yet."),
    );
    expect(screen.queryByRole("listitem")).toBeNull();
  });

  it("shows the server's own wording when the log cannot be read", async () => {
    serving(json({ error: "the Archive could not be read" }, 503));
    mount(ArchiveList);

    await waitFor(() => screen.getByText(/the Archive could not be read/));
  });

  it("reads the log once and does not poll it", async () => {
    const fetching = serving(json(SETS));
    mount(ArchiveList);
    await waitFor(() => screen.getByText(DECIDED.title));

    // Unlike the pending list's ten-second refetch: nothing here is waiting on
    // the human, and a decision that has already been made does not go stale
    // while the page is open.
    await vi.advanceTimersByTimeAsync(60_000);
    expect(fetching).toHaveBeenCalledTimes(1);
  });
});

describe("the way between the two lists", () => {
  it("goes both ways, and stays in the app", async () => {
    // Through the app's own routes rather than a stand-in for them: what this
    // asks is whether `/archive` is a page at all, and whether the link on each
    // list reaches the other one.
    window.history.pushState({}, "", "/archive");
    // The Archive, the pending list, and the Archive again: coming back to a
    // list reads it afresh, which is three answers and not two.
    serving(json(SETS), json(PENDING), json(SETS));
    render(() => <App />);

    // Each list is recognised by a Set only it holds: the fixtures share a
    // title, as the same Set answered would, and a page is not identified by
    // one.
    await waitFor(() => screen.getByText(ORPHANED.title));

    fireEvent.click(screen.getByText("← Pending"));

    await waitFor(() => expect(window.location.pathname).toBe("/"));
    // Waited for rather than read: the URL changes as the route does, and the
    // list it landed on is a fetch behind that.
    await waitFor(() => screen.getByText(PENDING[0]!.title));

    fireEvent.click(screen.getByText("Archive →"));

    await waitFor(() => expect(window.location.pathname).toBe("/archive"));
    await waitFor(() => screen.getByText(ORPHANED.title));
  });
});
