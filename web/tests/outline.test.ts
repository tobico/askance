//! The shape of the table of contents, away from the nav that draws it.
//!
//! All of this is worked out from the Set rather than from the page, which is
//! why it can be asked here at all: the sections a Set has, the ids they are
//! reached by, and where the highlight belongs among them. The nav and the
//! scroll-spy read the same two lists, so a disagreement between them would be
//! a highlight sitting on a line that is not where the reader is — and that is
//! what these pin.

import { describe, expect, it } from "vitest";

import type { QuestionView, SetView } from "../src/api/types";
import {
  anchor,
  inside,
  lit,
  mark,
  outline,
  shortened,
  spied,
  standsFor,
} from "../src/set/outline";

/// A Set carrying one of everything the nav lists: a Preface, a Diff of two
/// files, and two Questions — one of which has a Sub-question, which the nav
/// does not list.
///
/// Hand-built rather than a fixture: what is being asked here is what the
/// outline makes of the fields, so the fields are what a test wants to set.
function everySection(): SetView {
  return {
    id: 1,
    title: "Rate limiting for the public API",
    project: "askance",
    branch: "solid-viewer",
    preface_html: "<p>no rate limit</p>",
    diff: {
      html: "<details id=\"diff-1\"></details><details id=\"diff-2\"></details>",
      paths: ["src/limits.rs", "notes.txt"],
    },
    questions: [
      question("Q1", "Where should the request counter live?"),
      {
        ...question("Q2", "How should a throttled client be told?"),
        subquestions: [
          {
            name: "Q2a",
            text_html: "<p>What should Retry-After say?</p>",
            options: [],
          },
        ],
      },
    ],
    diagrams: false,
    standing: { Waiting: "waiting" },
  };
}

function question(name: string, words: string): QuestionView {
  return {
    ask: { name, text_html: `<p>${words}</p>`, options: [] },
    subquestions: [],
    nav_text: words,
  };
}

/// The ids of the watched parts of the page, in the order they are watched in.
function anchors(set: SetView): string[] {
  return spied(outline(set)).map((watched) => watched.anchor);
}

describe("the id a Question is reached by", () => {
  it("is its label, lowercased", () => {
    expect(anchor("Q3", 3)).toBe("q3");
    expect(anchor(" Q12 ", 12), "a padded label is still that one").toBe("q12");
  });

  it("is made into one from a label an id cannot hold", () => {
    expect(
      anchor("Q 7.a", 7),
      "a label is the agent's own string, and an id takes less than one does",
    ).toBe("q-7-a");
    expect(
      anchor("...", 4),
      "a label that makes no id at all falls back to the Question's place",
    ).toBe("q4");
  });
});

describe("a file's path in the contents", () => {
  it("is shown whole when there is room for it", () => {
    expect(shortened("crates/app/src/diff.rs")).toBe("crates/app/src/diff.rs");
  });

  it("keeps its filename when it is too long for the line", () => {
    expect(
      shortened("crates/app/src/set_view.rs"),
      "the end of a path is what is being looked for, so the cut takes from " +
        "the front — and lands on a directory boundary, so what is left is " +
        "still a path",
    ).toBe("…/app/src/set_view.rs");
    expect(
      shortened("crates/server/tests/deeply/nested/set_page.rs"),
      "as much of the path as the line has room for, not just the file",
    ).toBe("…/nested/set_page.rs");
  });

  it("is cut into when the filename alone is longer than the line", () => {
    expect(
      shortened("assets/a-very-long-name-for-one-file.webmanifest"),
      "no boundary leaves a tail that fits, so the extension is kept over " +
        "the start of the stem",
    ).toBe("…or-one-file.webmanifest");
  });
});

describe("the parts of the page the spy watches", () => {
  it("are every anchored one of them, in page order", () => {
    expect(
      anchors(everySection()),
      "each section's own heading, then whatever is under it — which is the " +
        "order the page has them in, and what the highlight moves along",
    ).toEqual(["preface", "diff", "diff-1", "diff-2", "questions", "q1", "q2"]);
  });

  it("each know which section they are in", () => {
    // What the floating header reads to name where the reader is: the section
    // for everything under one, and nothing for a section's own heading, which
    // is in nothing but itself.
    expect(spied(outline(everySection())).map(inside)).toEqual([
      null,
      null,
      "Diff",
      "Diff",
      null,
      "Questions",
      "Questions",
    ]);
  });

  it("are the Diff's own where word wrap is offered", () => {
    // The switch follows the section and not the line, so it stays put as the
    // reader moves from the Diff's heading down through its files rather than
    // appearing and vanishing per file.
    expect(
      spied(outline(everySection())).map((watched) => watched.section === "diff"),
      "the Diff's heading and both its files, and nothing else",
    ).toEqual([false, true, true, true, false, false, false]);
  });

  it("offer it nowhere on a Set with no Diff", () => {
    const set = { ...everySection(), diff: null };

    expect(
      spied(outline(set)).every((watched) => watched.section !== "diff"),
      "with no Diff there is nowhere the switch belongs",
    ).toBe(true);
  });

  it("leave out a section the Set does not have", () => {
    const set = { ...everySection(), preface_html: null, diff: null };

    expect(
      anchors(set),
      "the Questions are the one section every Set has",
    ).toEqual(["questions", "q1", "q2"]);
  });

  it("each carry the name the nav gives them", () => {
    const said = spied(outline(everySection())).map((watched) => [
      watched.label,
      watched.text,
    ]);

    expect(
      said,
      "the bar reads a line out by the same name the sidebar shows it under, " +
        "because the two are one list — a section by its name, a file by its " +
        "path, and a Question by its label and its words",
    ).toEqual([
      [null, "Preface"],
      [null, "Diff"],
      [null, "src/limits.rs"],
      [null, "notes.txt"],
      [null, "Questions"],
      ["Q1", "Where should the request counter live?"],
      ["Q2", "How should a throttled client be told?"],
    ]);
  });

  it("are each set in the face their kind of name wants", () => {
    expect(
      spied(outline(everySection())).map((watched) => watched.kind),
      "so a path in the bar is set as the Diff sets it, and the two read as " +
        "the same name",
    ).toEqual([
      "contents-section",
      "contents-section",
      "contents-path",
      "contents-path",
      "contents-section",
      "contents-question",
      "contents-question",
    ]);
  });

  it("name a path the bar reads out as the nav cuts it", () => {
    const set = everySection();
    set.diff = { html: "", paths: ["crates/app/src/set_view.rs"] };

    expect(
      spied(outline(set))[2]!.text,
      "the bar shows the same one line the sidebar does, cut the same way",
    ).toBe("…/app/src/set_view.rs");
  });
});

describe("where the highlight belongs", () => {
  it("is the last part of the page to have begun", () => {
    expect(
      lit([true, true, true, false, false]),
      "the third is the one being read: the two before it are above the " +
        "reader, and the two after have not begun",
    ).toBe(2);
  });

  it("is the first section while nothing has begun at all", () => {
    expect(
      lit([false, false, false]),
      "nothing has begun above the reading line, so the reader is at the top " +
        "— which reads as the first section rather than as nowhere",
    ).toBe(0);
    expect(
      lit([]),
      "and the same before the spy has said anything at all, which is the " +
        "page as it is first drawn",
    ).toBe(0);
  });
});

describe("what a line of the nav answers for", () => {
  it("reaches down through whatever is under it", () => {
    const sections = outline(everySection());
    const watched = spied(sections);

    expect(
      sections.map((section) => standsFor(watched, section)),
      "the Preface has nothing under it, while the Diff reaches through both " +
        "its files and the Questions through both Questions — which is how a " +
        "lit file marks the Diff it is in without taking the highlight from it",
    ).toEqual([
      { at: 0, through: 0 },
      { at: 1, through: 3 },
      { at: 4, through: 6 },
    ]);
  });

  it("is marked where the reader is, and quietly on the section around it", () => {
    // The Diff of a Set whose Preface is watched first: its own heading, then
    // its two files.
    const diff = { at: 1, through: 3 };

    expect(
      mark(diff, 1),
      "at the Diff's own heading, above its first file, the Diff is where the " +
        "reader is",
    ).toBe("at");
    expect(
      mark(diff, 2),
      "and once they are in one of its files, the file is the highlight and " +
        "the Diff only says they are in it",
    ).toBe("within");
    expect(mark(diff, 3)).toBe("within");
    expect(
      mark(diff, 4),
      "past the last file the Diff is behind them and unmarked",
    ).toBeNull();
    expect(mark(diff, 0), "as it is before they reach it").toBeNull();

    const file = { at: 2, through: 2 };
    expect(mark(file, 2)).toBe("at");
    expect(
      mark(file, 3),
      "a file has nothing under it, so it is the highlight or it is nothing",
    ).toBeNull();
  });
});
