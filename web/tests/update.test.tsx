//! The Update Notice: the banner the pending page draws when the server says a
//! newer Askance has been released than the one serving it.
//!
//! Mounted through the pending list rather than on its own, because where it
//! sits is half of what it is: a banner above the list, in the reading column
//! the list is in. What the server answers with is `UpdateNotice` from
//! `src/api/types.ts`, which `cargo test` writes out of the Rust the endpoint
//! fills in — so a payload written here that the server would never send does
//! not typecheck.

import { screen, waitFor } from "@solidjs/testing-library";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { PendingEntry, UpdateNotice } from "../src/api/types";
import { PendingList } from "../src/pending/PendingList";
import { mount } from "./listing";
import { json, serving, whenever } from "./serving";
import pending from "./fixtures/pending.json" with { type: "json" };

const SETS = pending as PendingEntry[];
const A_SET = SETS[0]!;

/// Where the README says how to update. Stage 06 of the public-release roadmap
/// writes that section; the anchor is the one agreed with it.
const UPDATING = "https://github.com/tobico/askance#updating";

/// What the server says about updating, whenever the page asks.
const saying = (notice: UpdateNotice) =>
  whenever("/api/ui/update", json(notice));

/// The list itself, which every test here serves the same way: the banner is
/// what is under test, and the rows are only what it has to sit above.
const list = () => whenever("/api/ui/pending", json(SETS));

const banner = () => document.querySelector(".update-notice");

beforeEach(() => {
  vi.useFakeTimers();
});

afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllGlobals();
});

describe("the Update Notice", () => {
  it("asks the server whether there is a newer Askance", async () => {
    const fetching = serving(list(), saying("Current"));
    mount(PendingList);

    await waitFor(() => screen.getByText(A_SET.title));
    expect(fetching).toHaveBeenCalledWith("/api/ui/update", expect.anything());
  });

  it("names the release when the server says there is one", async () => {
    serving(list(), saying({ Available: { version: "0.4.0" } }));
    mount(PendingList);

    await waitFor(() => expect(banner()).not.toBeNull());
    expect(banner()!.textContent).toContain("0.4.0");
  });

  it("links the updating instructions, and offers nothing that installs", async () => {
    serving(list(), saying({ Available: { version: "0.4.0" } }));
    const { container } = mount(PendingList);

    await waitFor(() => expect(banner()).not.toBeNull());

    // The link is the whole of what the Notice offers: nothing here changes the
    // server, and nothing installs on the human's behalf.
    const links = [...banner()!.querySelectorAll("a")];
    expect(links.map((link) => link.getAttribute("href"))).toEqual([UPDATING]);
    expect(banner()!.querySelectorAll("button, input, form")).toHaveLength(0);
    expect(container.querySelectorAll("button, input, form")).toHaveLength(
      // Only the pending list's own: the notifications switch, which was there
      // before the banner and is not the banner's.
      container.querySelectorAll(".notifications button, .notifications input")
        .length,
    );
  });

  it("stands above the list, in the column the list is read in", async () => {
    serving(list(), saying({ Available: { version: "0.4.0" } }));
    const { container } = mount(PendingList);

    await waitFor(() => expect(banner()).not.toBeNull());

    const page = container.querySelector(".list-page")!;
    const rows = page.querySelector(".set-list")!;
    // Inside the page's column rather than beside it, and before the rows.
    expect(banner()!.parentElement).toBe(page);
    expect(
      banner()!.compareDocumentPosition(rows) &
        Node.DOCUMENT_POSITION_FOLLOWING,
    ).toBeTruthy();
  });

  it("draws nothing when the server says there is nothing to update to", async () => {
    serving(list(), saying("Current"));
    mount(PendingList);

    await waitFor(() => screen.getByText(A_SET.title));
    expect(banner()).toBeNull();
  });

  it("draws nothing when the server could not be asked", async () => {
    serving(
      list(),
      whenever(
        "/api/ui/update",
        json({ error: "the update check could not be read" }, 500),
      ),
    );
    mount(PendingList);

    // The list is exactly as it was: a page that cannot reach the endpoint says
    // nothing about updating, and nothing about the failure either.
    await waitFor(() => screen.getByText(A_SET.title));
    expect(banner()).toBeNull();
    expect(screen.queryByText(/update/i)).toBeNull();
  });

  it("draws nothing while the answer is still in flight", async () => {
    let deliver: () => void;
    const held = new Promise<void>((resolve) => {
      deliver = resolve;
    });

    serving(
      list(),
      whenever("/api/ui/update", () =>
        held.then(
          () =>
            new Response(
              JSON.stringify({ Available: { version: "0.4.0" } }),
            ),
        ),
      ),
    );
    mount(PendingList);

    await waitFor(() => screen.getByText(A_SET.title));
    expect(banner()).toBeNull();

    deliver!();
    await waitFor(() => expect(banner()).not.toBeNull());
  });

  it("asks at its own cadence rather than the list's", async () => {
    const fetching = serving(list(), saying("Current"));
    mount(PendingList);
    await waitFor(() => screen.getByText(A_SET.title));

    const asking = () =>
      fetching.mock.calls.filter(([path]) => path === "/api/ui/update").length;
    expect(asking()).toBe(1);

    // The list has refetched several times over by now; a release cannot arrive
    // in the ten seconds a Set can, and the server is answering out of a memory
    // it refreshes daily.
    await vi.advanceTimersByTimeAsync(60_000);
    expect(asking()).toBe(1);
  });
});
