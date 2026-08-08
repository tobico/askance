//! Closing a Set unanswered: the human declaring that nobody is ever going to
//! answer it, so it stops being something that is waiting on them.
//!
//! The one irreversible act in the whole UI, and the only one confirmed in as
//! many words. It settles the Set the way a Response does — the agent's wait
//! ends, and the Set goes into the Archive — except that there is no Response,
//! because there never was one.

import { fireEvent, waitFor } from "@solidjs/testing-library";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { Archived, SetView } from "../src/api/types";
import { draftKey } from "../src/set/sheet";
import { answering } from "./reading";
import { json } from "./serving";
import waiting from "./fixtures/set-answering.json" with { type: "json" };

vi.mock("../src/set/diagrams", () => ({ drawDiagrams: () => () => {} }));

const WAITING = waiting as SetView;
const KEY = draftKey(WAITING.id);

const archived = (outcome: Archived) => json(outcome);

/// The button reading `text`.
function press(page: ParentNode, text: string) {
  const button = [...page.querySelectorAll("button")].find(
    (found) => found.textContent === text,
  );
  expect(button, `expected a button reading "${text}"`).toBeTruthy();
  fireEvent.click(button!);
}

/// Open the standing menu — the badge is its title — and choose the one thing
/// in it, which is the offer to close the Set unanswered.
function reachForArchive(page: ParentNode) {
  const trigger = page.querySelector(".standing-trigger");
  expect(trigger, "expected the badge to open the standing menu").toBeTruthy();
  fireEvent.click(trigger!);
  press(page, "Archive unanswered");
}

beforeEach(() => {
  localStorage.clear();
});

afterEach(() => {
  vi.unstubAllGlobals();
  localStorage.clear();
});

describe("the offer to close a Set unanswered", () => {
  it("folds behind the badge saying whether anyone is still listening", async () => {
    const { page } = await answering(WAITING);

    // The badge is the menu's title, and the offer is nowhere on the page
    // until the menu is asked for: archiving is almost never the right thing
    // to do to a Set.
    const trigger = page.querySelector(".standing .standing-trigger")!;
    expect(trigger.querySelector(".liveness")!.textContent).toBe(
      "agent waiting",
    );
    expect(trigger.getAttribute("aria-expanded")).toBe("false");
    expect(page.querySelector("button.archive")).toBeNull();

    fireEvent.click(trigger);

    expect(trigger.getAttribute("aria-expanded")).toBe("true");
    expect(page.querySelector("button.archive")!.textContent).toBe(
      "Archive unanswered",
    );
  });

  it("says the agent has gone when that is what the server said", async () => {
    const { page } = await answering({
      ...WAITING,
      standing: { Waiting: "disconnected" },
    });

    const badge = page.querySelector(".standing .liveness")!;
    expect(badge.className).toBe("liveness disconnected");
    expect(badge.textContent).toBe("agent disconnected");
  });

  it("is not offered on a Set that has already settled", async () => {
    const { page } = await answering({
      ...WAITING,
      standing: { ArchivedUnanswered: "2026-08-03T09:07:11.000Z" },
    });

    expect(
      page.querySelector(".standing"),
      "nothing is waiting on a settled Set, and there is nothing left to close",
    ).toBeNull();
  });

  it("archives nothing until the human has confirmed it", async () => {
    const { page, fetching } = await answering(WAITING, archived("Closed"));

    reachForArchive(page);

    const asking = page.querySelector(".confirm")!;
    expect(asking.getAttribute("role")).toBe("dialog");
    // The one irreversible act in the UI has to be asked about as one — and it
    // has to say where the Set is going, because it is not being deleted.
    expect(asking.querySelector(".note")!.textContent).toContain(
      "cannot be undone",
    );
    expect(asking.querySelector(".note")!.textContent).toContain("Archive");
    expect(fetching, "only the Set has been fetched").toHaveBeenCalledTimes(1);

    press(page, "Keep it pending");
    expect(page.querySelector(".confirm")).toBeNull();
    expect(fetching).toHaveBeenCalledTimes(1);
  });
});

describe("closing a Set unanswered", () => {
  it("settles it as archived and files it where it can be read", async () => {
    const { page, fetching, history } = await answering(
      WAITING,
      archived("Closed"),
    );

    reachForArchive(page);
    // The dialog's own button, which is the second one reading this.
    fireEvent.click(
      page.querySelector(".confirm-actions button:last-child") as HTMLElement,
    );

    await waitFor(() => expect(fetching).toHaveBeenCalledTimes(2));
    const [path, init] = fetching.mock.calls[1]!;
    expect(path).toBe(`/api/ui/sets/${WAITING.id}/archive`);
    expect(init?.method).toBe("POST");

    // To the Archive rather than to the pending list: this Set was not
    // discarded, it was filed, and seeing it filed unanswered is the
    // confirmation that nothing was lost.
    await waitFor(() => expect(history.get()).toBe("/archive"));
  });

  it("drops the draft, which this Set can never take a Response from now", async () => {
    const { page, fetching } = await answering(WAITING, archived("Closed"));

    fireEvent.input(
      page.querySelector<HTMLTextAreaElement>("#set-comment")!,
      { target: { value: "nobody is coming back for this" } },
    );
    await waitFor(() => expect(localStorage.getItem(KEY)).toBeTruthy());

    reachForArchive(page);
    fireEvent.click(
      page.querySelector(".confirm-actions button:last-child") as HTMLElement,
    );

    await waitFor(() => expect(fetching).toHaveBeenCalledTimes(2));
    await waitFor(() => expect(localStorage.getItem(KEY)).toBeNull());
  });

  it("says why it was not archived, when it was not", async () => {
    for (const [outcome, said] of [
      ["AlreadyAnswered", "it is in the Archive as a decision"],
      ["AlreadyArchived", "This Set has already been archived."],
      ["NoSuchSet", "This Set is no longer here."],
    ] as Array<[Archived, string]>) {
      const { page, fetching } = await answering(WAITING, archived(outcome));

      reachForArchive(page);
      fireEvent.click(
        page.querySelector(".confirm-actions button:last-child") as HTMLElement,
      );

      await waitFor(() => expect(fetching).toHaveBeenCalledTimes(2));
      await waitFor(() =>
        expect(page.querySelector(".meta .error")!.textContent).toContain(
          said,
        ),
      );
    }
  });

  it("says so in the server's own wording when it did not get through", async () => {
    const { page, fetching } = await answering(
      WAITING,
      json({ error: "the Question Set could not be archived" }, 503),
    );

    reachForArchive(page);
    fireEvent.click(
      page.querySelector(".confirm-actions button:last-child") as HTMLElement,
    );

    await waitFor(() => expect(fetching).toHaveBeenCalledTimes(2));
    await waitFor(() =>
      expect(page.querySelector(".meta .error")!.textContent).toContain(
        "the Question Set could not be archived",
      ),
    );
  });
});
