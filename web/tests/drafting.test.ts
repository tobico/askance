//! The answer sheet away from the page it is filled in on: what a sheet full of
//! fields adds up to, which of them the submit warns about, and what survives a
//! round trip through `localStorage`.
//!
//! Asked of the three functions rather than of a rendered page: the arithmetic of
//! a Response is the one part of answering that has nothing to do with how the
//! page draws it.

import { describe, expect, it } from "vitest";

import {
  clicked,
  drafted,
  draftKey,
  empty,
  restorable,
  unanswered,
} from "../src/set/sheet";
import type { Draft, Filled } from "../src/set/sheet";

function filled(
  label: string,
  selected: number | null,
  free_text: string,
): Filled {
  return { label, selected, free_text };
}

/// A sheet part-way filled in, as the human might leave it.
function partWay(): Draft {
  return {
    filled: [
      filled("Q1", 2, ""),
      filled("Q2", null, "only for writes"),
      filled("Q2a", null, ""),
    ],
    comment: "back in an hour",
  };
}

describe("the Response a sheet adds up to", () => {
  it("marks a question the human left alone as unanswered", () => {
    const response = drafted([filled("Q1", null, "")], "");

    // Written the way the schema writes one: a field with nothing in it is left
    // out rather than sent empty, which is the shape the CLI and the agent API
    // have always used.
    expect(response.answers[0]).toEqual({ label: "Q1", unanswered: true });
  });

  it("does not take whitespace for an answer", () => {
    const response = drafted([filled("Q1", null, "  \n ")], "  ");

    expect(response.answers[0]!.unanswered).toBe(true);
    expect(
      response.comment,
      "a blank comment is no comment",
    ).toBeUndefined();
  });

  it("takes an Option, or words, or both", () => {
    const response = drafted(
      [
        filled("Q1", 2, ""),
        filled("Q2", null, "the second one, but only for writes"),
        filled("Q3", 1, " with a caveat "),
      ],
      "",
    );

    expect(response.answers[0]!.selected).toBe(2);
    expect(response.answers[1]!.free_text).toBe(
      "the second one, but only for writes",
    );
    expect(response.answers[2]!.selected).toBe(1);
    expect(
      response.answers[2]!.free_text,
      "free text is trimmed before it goes out",
    ).toBe("with a caveat");

    expect(
      response.answers.every((answer) => answer.unanswered === undefined),
      "an entry is an Answer or the marker, never both",
    ).toBe(true);
  });

  it("gives every question an entry, in the order it was asked", () => {
    const response = drafted(
      [
        filled("Q1", 1, ""),
        filled("Q2", null, ""),
        filled("Q2a", null, "no"),
        filled("Q2b", null, ""),
      ],
      "",
    );

    expect(response.answers.map((answer) => answer.label)).toEqual([
      "Q1",
      "Q2",
      "Q2a",
      "Q2b",
    ]);
  });

  it("is a whole counter-question when it carries a comment and nothing else", () => {
    const said = "Neither, really — why not cache it upstream?";
    const response = drafted(
      [filled("Q1", null, ""), filled("Q2", null, "")],
      said,
    );

    expect(
      response.answers.every((answer) => answer.unanswered),
      "nothing was answered, and every question still has to say so",
    ).toBe(true);
    expect(response.comment).toBe(said);
  });
});

describe("the warning before a submit", () => {
  it("names every multiple-choice question being left open", () => {
    const response = drafted(
      [
        filled("Q1", 1, ""),
        filled("Q2", null, ""),
        filled("Q2a", null, "no"),
        filled("Q2b", null, "   "),
      ],
      "",
    );

    expect(unanswered(response, ["Q1", "Q2", "Q2a", "Q2b"])).toEqual([
      "Q2",
      "Q2b",
    ]);
  });

  it("says nothing about a free-text question left open", () => {
    const response = drafted(
      [filled("Q1", null, ""), filled("Q2", null, ""), filled("Q3", null, "")],
      "",
    );

    expect(
      unanswered(response, ["Q2"]),
      "Q1 and Q3 offered no Options, so skipping them is not warned about",
    ).toEqual(["Q2"]);
  });

  it("warns again about a question that was cleared", () => {
    // Clearing is not a third state: it puts the Question back exactly where it
    // was before anything was picked, warning and all.
    const response = drafted([filled("Q1", clicked(1, 1), "")], "");

    expect(response.answers[0]!.unanswered).toBe(true);
    expect(
      unanswered(response, ["Q1"]),
      "an Option cleared is an Option not chosen, and the submit says so",
    ).toEqual(["Q1"]);
  });
});

describe("clicking an Option", () => {
  it("clears the question when it lands on the one already selected", () => {
    expect(clicked(2, 2)).toBeNull();
  });

  it("selects any other", () => {
    expect(clicked(null, 2), "the first click on a question").toBe(2);
    expect(clicked(1, 2), "changing which one").toBe(2);
  });
});

describe("a draft between visits", () => {
  it("comes back exactly as it was left", () => {
    const draft = partWay();

    expect(
      restorable(JSON.stringify(draft), ["Q1", "Q2", "Q2a"]),
      "every Option, every word and the comment survive the round trip",
    ).toEqual(draft);
  });

  it("restores only what the human put there", () => {
    const draft: Draft = {
      filled: [filled("Q1", null, ""), filled("Q2", 1, "")],
      comment: "",
    };

    const restored = restorable(JSON.stringify(draft), ["Q1", "Q2"])!;
    expect(
      restored.filled[0]!.selected,
      "a question the human left alone comes back untouched, not answered for them",
    ).toBeNull();
    expect(restored.filled[1]!.selected).toBe(1);
  });

  it("is discarded whole when its questions are not this Set's", () => {
    const body = JSON.stringify(partWay());

    expect(
      restorable(body, ["Q1", "Q2", "Q2b"]),
      "a Sub-question that was renamed makes the whole draft stale",
    ).toBeNull();
    expect(
      restorable(body, ["Q1", "Q2"]),
      "and so does a Set that has since lost a question",
    ).toBeNull();
    expect(
      restorable(body, ["Q2", "Q1", "Q2a"]),
      "the order is the order the Set asked them in, not a set of names",
    ).toBeNull();
  });

  it("is discarded when it will not parse", () => {
    expect(restorable('{"filled": [', ["Q1"])).toBeNull();
    expect(
      restorable('{"answers": [], "comment": null}', ["Q1"]),
      "a body from some other shape of draft is no more usable than a truncated one",
    ).toBeNull();
    expect(
      restorable('{"filled": [{"label": "Q1"}], "comment": ""}', ["Q1"]),
      "and neither is one whose fields are missing",
    ).toBeNull();
    expect(restorable(null, ["Q1"]), "nor is nothing at all").toBeNull();
  });

  it("is not worth keeping when the sheet is empty", () => {
    const nothing: Draft = {
      filled: [filled("Q1", null, ""), filled("Q2", null, "  \n")],
      comment: "   ",
    };
    expect(empty(nothing), "whitespace is not an answer here either").toBe(
      true,
    );

    expect(empty(partWay())).toBe(false);
    expect(
      empty({
        filled: [filled("Q1", null, "")],
        comment: "why not cache it upstream?",
      }),
      "a comment on its own is a draft: it is a whole counter-question",
    ).toBe(false);
  });

  it("is kept per Set, so two answered in turn do not share one", () => {
    expect(draftKey(7)).not.toBe(draftKey(8));
  });
});
