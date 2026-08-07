//! The Set page in its answering half: the sheet a waiting Set is filled in on,
//! the draft that survives leaving it, and the submit that ends the agent's
//! wait.
//!
//! The Set comes out of `tests/fixtures/set-answering.json`, which `cargo test`
//! writes from the real `/api/ui/sets/{id}` — so the form is built from the
//! Options and the labels the server really sends. The Response it sends back is
//! checked against `set-answered.json`, whose Response the same `cargo test` put
//! through the store and read out again: what the browser puts on the wire is
//! then the same Response the server records and hands back, which is the one
//! the waiting `askance ask` is given as YAML.

import { fireEvent, screen, waitFor } from "@solidjs/testing-library";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { SetView, Submitted } from "../src/api/types";
import { draftKey } from "../src/set/sheet";
import {
  answering,
  sent,
  texts,
  withHeading,
  withPostscript,
} from "./reading";
import { json } from "./serving";
import answered from "./fixtures/set-answered.json" with { type: "json" };
import waiting from "./fixtures/set-answering.json" with { type: "json" };

/// The renderer is a page's own doing and this fixture has no Diagram anyway;
/// mocked so nothing here loads megabytes of mermaid.
vi.mock("../src/set/diagrams", () => ({ drawDiagrams: () => () => {} }));

const WAITING = waiting as SetView;
const ANSWERED = answered as SetView;

/// The Response the answered fixture carries: what the server stored and handed
/// back when this same Set was answered. Filling the sheet in the same way has
/// to put exactly this on the wire.
const DECIDED =
  "Answered" in ANSWERED.standing ? ANSWERED.standing.Answered.response : null;

/// Where this Set's draft lives.
const KEY = draftKey(WAITING.id);

/// The radio for Option `n` of the question named `label`.
function option(page: ParentNode, label: string, n: number): HTMLInputElement {
  const radio = page.querySelector<HTMLInputElement>(
    `input[name="${label}-option"][value="${n}"]`,
  );
  expect(radio, `expected Option ${n} of ${label}`).toBeTruthy();
  return radio!;
}

/// The free-text field of the question named `label`.
function words(page: ParentNode, label: string): HTMLTextAreaElement {
  const field = page.querySelector<HTMLTextAreaElement>(
    `textarea[name="${label}-free-text"]`,
  );
  expect(field, `expected the free-text field of ${label}`).toBeTruthy();
  return field!;
}

/// Type into a field, the way a keystroke reaches it.
function type(field: HTMLTextAreaElement, said: string) {
  fireEvent.input(field, { target: { value: said } });
}

/// The button reading `text`, which is how both dialogs and the submit are
/// pressed.
function press(page: ParentNode, text: string) {
  const button = [...page.querySelectorAll("button")].find(
    (found) => found.textContent === text,
  );
  expect(button, `expected a button reading "${text}"`).toBeTruthy();
  fireEvent.click(button!);
}

/// Fill the sheet in exactly as the answered fixture was answered: Q1's first
/// Option, Q2's second with a word about it, Q2a skipped, Q2b answered in words,
/// Q3 skipped, and something said about the Set as a whole.
function answerLikeTheFixture(page: ParentNode) {
  fireEvent.click(option(page, "Q1", 1));
  fireEvent.click(option(page, "Q2", 2));
  type(words(page, "Q2"), "and document them in the changelog");
  type(words(page, "Q2b"), "keep them short");
  type(
    page.querySelector<HTMLTextAreaElement>("#set-comment")!,
    "Do the in-process one first; we can move it later.",
  );
}

/// One answer from the server to a submit.
const submitted = (outcome: Submitted) => json(outcome);

beforeEach(() => {
  localStorage.clear();
});

afterEach(() => {
  vi.unstubAllGlobals();
  localStorage.clear();
});

describe("the sheet a waiting Set is answered on", () => {
  it("is a form, and says nothing about a decision nobody has made", async () => {
    const { page } = await answering(WAITING);

    expect(page.querySelector("#set-comment")).toBeTruthy();
    for (const absent of [".answered-at", ".counter-question", ".chosen"]) {
      expect(
        page.querySelector(absent),
        `nothing has been decided here, so ${absent} does not belong`,
      ).toBeNull();
    }
  });

  it("offers every Option as a radio its question answers by", async () => {
    const { page } = await answering(WAITING);

    // Q1, Q2 and Q2a offer two Options each; Q2b and Q3 offer none.
    expect(page.querySelectorAll("input[type=radio]")).toHaveLength(6);
    for (const label of ["Q1", "Q2", "Q2a"]) {
      for (const n of [1, 2]) {
        expect(option(page, label, n).value).toBe(String(n));
      }
    }
    for (const bare of ["Q2b", "Q3"]) {
      expect(
        page.querySelector(`input[name="${bare}-option"]`),
        `${bare} offers no Options, so it has nothing to select`,
      ).toBeNull();
    }
  });

  it("marks the Recommendation and preselects nothing", async () => {
    const { page } = await answering(WAITING);

    expect(page.querySelectorAll(".option .star")).toHaveLength(1);
    expect(
      page.querySelector(".option.recommended input")!.getAttribute("value"),
      "the ★ is on Q1's second Option, which is where the agent put it",
    ).toBe("2");
    expect(
      [...page.querySelectorAll<HTMLInputElement>("input")].some(
        (radio) => radio.checked,
      ),
      "nothing may be selected on load, or an unread Recommendation could be submitted by accident",
    ).toBe(false);
  });

  it("gives every question a field and the Set one comment box", async () => {
    const { page } = await answering(WAITING);

    // Five questions — Q1, Q2, Q2a, Q2b, Q3 — plus the set-level comment.
    expect(page.querySelectorAll("textarea")).toHaveLength(6);
    expect(page.querySelector('textarea[name="set-comment"]')).toBeTruthy();
  });

  it("gives a Heading no field, because it asks nothing", async () => {
    const { page } = await answering(withHeading(WAITING));

    // The text is still drawn — it is what the Sub-questions under it are read
    // against — and it is still the anchor the nav jumps to.
    expect(page.querySelector("#q2"), "expected Q2 still on the page").toBeTruthy();
    expect(texts(page, ".ask.heading .label")).toEqual(["Q2"]);

    expect(
      page.querySelector('textarea[name="Q2-free-text"]'),
      "a Heading with a field is a blank the human has nothing to put in",
    ).toBeNull();
    expect(
      page.querySelector('input[name="Q2-option"]'),
      "and it offers no Options either — that is what makes it a Heading",
    ).toBeNull();

    // Its Sub-questions are untouched: they are what there is to answer.
    expect(page.querySelector('textarea[name="Q2b-free-text"]')).toBeTruthy();
  });

  it("sends no entry for a Heading, so nothing comes back open that was never asked", async () => {
    const { page, fetching } = await answering(
      withHeading(WAITING),
      submitted("Accepted"),
    );

    fireEvent.click(option(page, "Q1", 1));
    press(page, "Submit");
    press(page, "Send anyway");

    await waitFor(() => expect(fetching).toHaveBeenCalledTimes(2));
    const response = sent(fetching) as { answers: Array<{ label: string }> };

    expect(
      response.answers.map((answer) => answer.label),
      "everything there is to answer appears exactly once, and the Heading is " +
        "not among it — an entry for one is refused by the grammar",
    ).toEqual(["Q1", "Q2a", "Q2b", "Q3"]);
  });

  it("closes the sheet with the Postscript, wrapped around the comment box", async () => {
    const { page } = await answering(withPostscript(WAITING));

    const postscript = page.querySelector("section.postscript")!;
    expect(postscript, "expected the Postscript drawn").toBeTruthy();
    // Rendered markdown, drawn by the same rules as everything else the agent
    // wrote — a code span in it included.
    const body = postscript.querySelector(".postscript-body")!;
    expect(body.className).toContain("markdown");
    expect(body.innerHTML).toContain("<code>ops/export</code>");

    // Inside the card rather than after it, directly under the prose that is
    // inviting something into it: the two are one thing to read.
    const comment = postscript.querySelector(".set-comment")!;
    expect(comment, "expected the box inside the Postscript").toBeTruthy();
    expect(body.nextElementSibling).toBe(comment);

    // And the box itself is untouched: it says what it always said.
    const field = page.querySelector<HTMLTextAreaElement>("#set-comment")!;
    expect(field.placeholder).toBe("Other comments");
    expect(field.getAttribute("aria-label")).toBe("Other comments");
  });

  it("draws the card around the box for a Set that closed with none", async () => {
    const { page } = await answering(WAITING);

    const postscript = page.querySelector("section.postscript")!;
    expect(
      postscript,
      "the box is on every Set, so the card holding it is too",
    ).toBeTruthy();
    expect(postscript.querySelector(".postscript-body")).toBeNull();
    expect(postscript.querySelector(".set-comment")).toBeTruthy();
  });

  it("prompts every field by its placeholder and starts it one line tall", async () => {
    const { page } = await answering(WAITING);

    // Q1, Q2 and Q2a offer Options, so the neutral prompt goes on those three;
    // Q2b and Q3 offer none, and there the words are the whole answer. Each is
    // named for its own question, so that five fields in a column are five
    // different fields — to a screen reader above all, which has nothing else
    // to tell them apart by.
    for (const [label, named] of [
      ["Q1", "Q1 — Your thoughts"],
      ["Q2", "Q2 — Your thoughts"],
      ["Q2a", "Q2a — Your thoughts"],
      ["Q2b", "Q2b — Your answer"],
      ["Q3", "Q3 — Your answer"],
    ]) {
      const field = words(page, label!);
      expect(field.placeholder).toBe(named);
      // No labels left to name them, so the placeholder has to be the
      // accessible name as well — a browser is free to leave one unspoken.
      expect(field.getAttribute("aria-label")).toBe(named);
    }

    const comment = page.querySelector<HTMLTextAreaElement>("#set-comment")!;
    expect(comment.placeholder).toBe("Other comments");
    expect(comment.getAttribute("aria-label")).toBe("Other comments");

    // Every field grows from one row, and the wrapper the stylesheet measures
    // it by is around each of them.
    expect(page.querySelectorAll(".grow > textarea")).toHaveLength(6);
    expect(
      [...page.querySelectorAll("textarea")].every(
        (field) => field.getAttribute("rows") === "1",
      ),
      "an empty field is one line tall, the comment box included",
    ).toBe(true);
  });

  it("gives the wrapper the words to measure the field by", async () => {
    const { page } = await answering(WAITING);
    const field = words(page, "Q3");

    type(field, "two lines\nof it");

    expect(
      field.parentElement!.getAttribute("data-value"),
      "the stylesheet grows the field by the text the wrapper carries",
    ).toBe("two lines\nof it");
  });
});

describe("selecting an Option", () => {
  it("selects the one clicked, and moves with a second click", async () => {
    const { page } = await answering(WAITING);

    fireEvent.click(option(page, "Q1", 1));
    expect(option(page, "Q1", 1).checked).toBe(true);

    fireEvent.click(option(page, "Q1", 2));
    expect(option(page, "Q1", 2).checked).toBe(true);
    expect(option(page, "Q1", 1).checked).toBe(false);
  });

  it("clears the question when the click lands on the one already selected", async () => {
    const { page } = await answering(WAITING);

    // The one gesture a radio group has no way of its own to make, and the one
    // that thinking better of a Recommendation wants.
    fireEvent.click(option(page, "Q1", 2));
    fireEvent.click(option(page, "Q1", 2));

    expect(option(page, "Q1", 2).checked).toBe(false);
  });

  it("keeps each question's Options to itself", async () => {
    const { page } = await answering(WAITING);

    fireEvent.click(option(page, "Q1", 1));
    fireEvent.click(option(page, "Q2", 2));

    expect(option(page, "Q1", 1).checked).toBe(true);
    expect(option(page, "Q2", 2).checked).toBe(true);
  });
});

describe("submitting a Response", () => {
  it("sends the Set answered in the browser as the Response the server records", async () => {
    const { page, fetching, history } = await answering(
      WAITING,
      submitted("Accepted"),
    );

    answerLikeTheFixture(page);
    press(page, "Submit");
    // Q2a offered Options and was left alone, so the warning stands between the
    // human and the send.
    press(page, "Send anyway");

    await waitFor(() => expect(fetching).toHaveBeenCalledTimes(2));
    const [path, init] = fetching.mock.calls[1]!;
    expect(path).toBe(`/api/ui/sets/${WAITING.id}/response`);
    expect(init?.method).toBe("POST");
    // The Response the answered fixture carries, which is what the store took
    // and what the agent was handed as YAML.
    expect(sent(fetching)).toEqual(DECIDED);

    // Back to the pending list, where the Set's absence is the confirmation
    // that the agent has its answer.
    await waitFor(() => expect(history.get()).toBe("/"));
  });

  it("sends a Set with every choice made without asking anything first", async () => {
    const { page, fetching } = await answering(WAITING, submitted("Accepted"));

    for (const label of ["Q1", "Q2", "Q2a"]) {
      fireEvent.click(option(page, label, 1));
    }
    press(page, "Submit");

    expect(
      page.querySelector(".confirm"),
      "no offered choice was overlooked, so there is nothing to warn about",
    ).toBeNull();
    await waitFor(() => expect(fetching).toHaveBeenCalledTimes(2));
  });

  it("names every offered choice being left open, and lets the human go back", async () => {
    const { page, fetching } = await answering(WAITING, submitted("Accepted"));

    fireEvent.click(option(page, "Q1", 1));
    press(page, "Submit");

    const warning = page.querySelector(".confirm")!;
    expect(warning.getAttribute("role")).toBe("dialog");
    expect(
      texts(warning, ".unanswered li"),
      "Q2b and Q3 offered nothing, so skipping them is not warned about",
    ).toEqual(["Q2", "Q2a"]);

    press(page, "Keep answering");
    expect(page.querySelector(".confirm")).toBeNull();
    expect(fetching, "nothing was sent").toHaveBeenCalledTimes(1);
  });

  it("sends a comment and nothing else as the counter-question it is", async () => {
    const { page, fetching } = await answering(WAITING, submitted("Accepted"));

    type(
      page.querySelector<HTMLTextAreaElement>("#set-comment")!,
      "Neither, really — why not cache it upstream?",
    );
    press(page, "Submit");
    press(page, "Send anyway");

    await waitFor(() => expect(fetching).toHaveBeenCalledTimes(2));
    expect(sent(fetching)).toEqual({
      answers: ["Q1", "Q2", "Q2a", "Q2b", "Q3"].map((label) => ({
        label,
        unanswered: true,
      })),
      comment: "Neither, really — why not cache it upstream?",
    });
  });

  it("says it is sending, and says so once", async () => {
    const { page, fetching } = await answering(WAITING, submitted("Accepted"));

    for (const label of ["Q1", "Q2", "Q2a"]) {
      fireEvent.click(option(page, label, 1));
    }
    press(page, "Submit");

    const button = page.querySelector<HTMLButtonElement>(".submit button")!;
    expect(button.textContent).toBe("Sending…");
    expect(button.disabled).toBe(true);

    await waitFor(() => expect(fetching).toHaveBeenCalledTimes(2));
  });
});

describe("a submit that did not land", () => {
  /// Fill in enough to send without a warning, then send it.
  async function refused(outcome: Submitted) {
    const { page, fetching } = await answering(WAITING, submitted(outcome));
    for (const label of ["Q1", "Q2", "Q2a"]) {
      fireEvent.click(option(page, label, 1));
    }
    press(page, "Submit");
    await waitFor(() => expect(fetching).toHaveBeenCalledTimes(2));
    await waitFor(() => expect(page.querySelector(".submit .error")).toBeTruthy());
    return page;
  }

  it("says the Set had already been answered", async () => {
    const page = await refused("AlreadyAnswered");

    expect(page.querySelector(".submit .error")!.textContent).toContain(
      "The first Response stands",
    );
  });

  it("says the Set was archived, which closed it for good", async () => {
    const page = await refused("Archived");

    expect(page.querySelector(".submit .error")!.textContent).toContain(
      "archived unanswered",
    );
  });

  it("says the Set is no longer here", async () => {
    const page = await refused("NoSuchSet");

    expect(page.querySelector(".submit .error")!.textContent).toBe(
      "This Set is no longer here.",
    );
  });

  it("carries back what the server said the Response failed to resolve", async () => {
    const page = await refused({ Rejected: ["Q2b is unaccounted for"] });

    expect(page.querySelector(".submit .error")!.textContent).toContain(
      "Q2b is unaccounted for",
    );
  });

  it("says so in the server's own wording when it did not get through", async () => {
    const { page, fetching } = await answering(
      WAITING,
      json({ error: "the Response could not be taken" }, 503),
    );
    for (const label of ["Q1", "Q2", "Q2a"]) {
      fireEvent.click(option(page, label, 1));
    }
    press(page, "Submit");

    await waitFor(() => expect(fetching).toHaveBeenCalledTimes(2));
    await waitFor(() =>
      expect(page.querySelector(".submit .error")!.textContent).toContain(
        "the Response could not be taken",
      ),
    );
  });
});

describe("the draft between visits", () => {
  it("keeps what the human has put in, as they put it in", async () => {
    const { page } = await answering(WAITING);

    fireEvent.click(option(page, "Q1", 2));
    type(words(page, "Q2"), "only for writes");
    type(
      page.querySelector<HTMLTextAreaElement>("#set-comment")!,
      "back in an hour",
    );

    await waitFor(() => expect(localStorage.getItem(KEY)).toBeTruthy());
    expect(JSON.parse(localStorage.getItem(KEY)!)).toEqual({
      filled: [
        { label: "Q1", selected: 2, free_text: "" },
        { label: "Q2", selected: null, free_text: "only for writes" },
        { label: "Q2a", selected: null, free_text: "" },
        { label: "Q2b", selected: null, free_text: "" },
        { label: "Q3", selected: null, free_text: "" },
      ],
      comment: "back in an hour",
    });
  });

  it("comes back on the next visit exactly as it was left", async () => {
    const { page } = await answering(WAITING);
    fireEvent.click(option(page, "Q1", 2));
    type(words(page, "Q2"), "only for writes");
    await waitFor(() => expect(localStorage.getItem(KEY)).toBeTruthy());

    const { page: again } = await answering(WAITING);

    expect(option(again, "Q1", 2).checked).toBe(true);
    expect(words(again, "Q2").value).toBe("only for writes");
    expect(
      words(again, "Q2").parentElement!.getAttribute("data-value"),
      "a restored draft arrives at the right height, not one line tall",
    ).toBe("only for writes");
  });

  it("is dropped again when the sheet is emptied", async () => {
    const { page } = await answering(WAITING);

    type(words(page, "Q3"), "for now");
    await waitFor(() => expect(localStorage.getItem(KEY)).toBeTruthy());

    type(words(page, "Q3"), "");
    await waitFor(() =>
      expect(
        localStorage.getItem(KEY),
        "a draft of nothing would only ever restore as nothing",
      ).toBeNull(),
    );
  });

  it("is discarded whole when it no longer describes the Set", async () => {
    localStorage.setItem(
      KEY,
      JSON.stringify({
        filled: [{ label: "Q1", selected: 1, free_text: "" }],
        comment: "from a Set that has since changed",
      }),
    );

    const { page } = await answering(WAITING);

    expect(
      option(page, "Q1", 1).checked,
      "the Set as the agent sent it wins over a draft that no longer describes it",
    ).toBe(false);
    expect(page.querySelector<HTMLTextAreaElement>("#set-comment")!.value).toBe(
      "",
    );
    expect(
      localStorage.getItem(KEY),
      "stale here is stale on the next visit too",
    ).toBeNull();
  });

  it("is gone once the Response has been taken", async () => {
    const { page, fetching } = await answering(WAITING, submitted("Accepted"));

    answerLikeTheFixture(page);
    await waitFor(() => expect(localStorage.getItem(KEY)).toBeTruthy());

    press(page, "Submit");
    press(page, "Send anyway");

    await waitFor(() => expect(fetching).toHaveBeenCalledTimes(2));
    await waitFor(() => expect(localStorage.getItem(KEY)).toBeNull());
  });

  it("is gone when the Set turns out to be settled already", async () => {
    const { page, fetching } = await answering(
      WAITING,
      submitted("AlreadyAnswered"),
    );

    answerLikeTheFixture(page);
    await waitFor(() => expect(localStorage.getItem(KEY)).toBeTruthy());

    press(page, "Submit");
    press(page, "Send anyway");

    await waitFor(() => expect(fetching).toHaveBeenCalledTimes(2));
    // This Set will take no Response from here, so a half-filled sheet would
    // only resurface stale and unsendable on some later visit.
    await waitFor(() => expect(localStorage.getItem(KEY)).toBeNull());
  });

  it("stands when the Response was refused, because it is the only copy", async () => {
    const { page, fetching } = await answering(
      WAITING,
      submitted({ Rejected: ["Q2b is unaccounted for"] }),
    );

    answerLikeTheFixture(page);
    await waitFor(() => expect(localStorage.getItem(KEY)).toBeTruthy());

    press(page, "Submit");
    press(page, "Send anyway");

    await waitFor(() => expect(fetching).toHaveBeenCalledTimes(2));
    await screen.findByText(/Q2b is unaccounted for/);
    expect(localStorage.getItem(KEY)).toBeTruthy();
  });
});
