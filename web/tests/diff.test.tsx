//! The Diff on a Set's page: the evidence for what the agent is asking about,
//! rendered and highlighted by the server and folded per file by the browser.
//!
//! The payload every Set here is drawn from comes out of `tests/fixtures/`,
//! which `cargo test` writes from the real `/api/ui/sets/{id}` — so the markup
//! being injected, the anchors on it and the paths beside it are the server's
//! own answers rather than a mock's agreement with this file.

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { SetView } from "../src/api/types";
import { reading, texts } from "./reading";
import answered from "./fixtures/set-answered.json" with { type: "json" };
import answering from "./fixtures/set-answering.json" with { type: "json" };

/// The Set with a Diff attached, and one without: two files, one of them a Rust
/// file the server has highlighted.
const WAITING = answering as SetView;
const UNDIFFED = answered as SetView;

/// Where the wrap setting is kept — the key `src/device.ts` writes, asked for
/// here by the name a browser would find it under.
const WRAP = "askance.diff-wrap";

beforeEach(() => {
  localStorage.clear();
});

afterEach(() => {
  vi.unstubAllGlobals();
  localStorage.clear();
});

/// The Diff section of a page, which every test here has to have found.
function diffOf(page: ParentNode): HTMLElement {
  const diff = page.querySelector<HTMLElement>("section.diff");
  expect(diff, "expected the Diff section").toBeTruthy();
  return diff as HTMLElement;
}

describe("the attached Diff", () => {
  it("is put in as the server rendered it, one fold per file", async () => {
    const diff = diffOf(await reading(WAITING));

    const folds = diff.querySelectorAll<HTMLDetailsElement>("details.diff-file");
    expect(folds, "one fold per file, whatever git knew of it").toHaveLength(2);
    expect([...folds].map((fold) => fold.id)).toEqual(["diff-1", "diff-2"]);
    expect(texts(diff, "details.diff-file .diff-path")).toEqual([
      "src/limits.rs",
      "notes.txt",
    ]);
  });

  it("keeps the colouring the server put on it", async () => {
    // The browser gets no diff parser and no highlighter: both are the
    // server's, and this is the whole of what arrives.
    const diff = diffOf(await reading(WAITING));

    expect(diff.querySelector(".diff-line.add")).toBeTruthy();
    expect(diff.querySelector(".diff-line.del")).toBeTruthy();
    expect(
      diff.querySelector("span[class^='tok-']"),
      "expected the Rust file highlighted server-side",
    ).toBeTruthy();
  });

  it("is named by a heading a jump can land on", async () => {
    const diff = diffOf(await reading(WAITING));

    expect(diff.id).toBe("diff");
    expect(diff.querySelector("h2.section-heading")!.textContent).toBe("Diff");
  });

  it("folds and unfolds each file", async () => {
    const diff = diffOf(await reading(WAITING));
    const [first] = diff.querySelectorAll<HTMLDetailsElement>("details.diff-file");

    // Open as the server wrote them: a Diff is evidence, and evidence nobody
    // can see until they open it is not being shown.
    expect(first!.open).toBe(true);

    // The fold is the browser's own `details`, so what a test can ask is that
    // the page is still one — a summary to press, and the file's lines inside
    // it.
    expect(first!.querySelector("summary .diff-path")!.textContent).toBe(
      "src/limits.rs",
    );
    first!.open = false;
    expect(first!.open, "and it takes being shut again").toBe(false);
  });

  it("shows none of its chrome on a Set that has no Diff", async () => {
    const page = await reading(UNDIFFED);

    expect(page.querySelector("section.diff")).toBeNull();
    expect(page.querySelector("#diff")).toBeNull();
    expect(page.querySelector("#diff-1")).toBeNull();
    expect(
      texts(page, "h2.section-heading"),
      "with no Diff there is no heading to draw either — the closing section " +
        "is headed for the box it holds, this Set having no Postscript",
    ).toEqual(["Preface", "Questions", "Comment"]);
    expect(
      page.querySelector(".switch"),
      "and nowhere for word wrap to belong: it governs a Diff, and there is none",
    ).toBeNull();
  });
});

describe("word wrap", () => {
  /// The switch beside the Diff's heading.
  function wrapSwitch(page: ParentNode): HTMLInputElement {
    const found = page.querySelector<HTMLInputElement>(
      "section.diff .switch input",
    );
    expect(
      found,
      "expected the wrap switch beside the Diff's heading",
    ).toBeTruthy();
    return found as HTMLInputElement;
  }

  it("is offered beside the Diff heading, and off until it is asked for", async () => {
    const page = await reading(WAITING);
    const flip = wrapSwitch(page);

    expect(flip.getAttribute("role")).toBe("switch");
    expect(flip.checked).toBe(false);
    expect(diffOf(page).className).toBe("diff");
  });

  it("wraps the Diff, and remembers it for the next one", async () => {
    const page = await reading(WAITING);

    wrapSwitch(page).click();

    expect(
      diffOf(page).className,
      "wrapping is a class and nothing more: the Diff arrived rendered",
    ).toBe("diff wrapped");
    expect(
      localStorage.getItem(WRAP),
      "the setting governs every Diff, so it is the device that remembers it",
    ).toBe("on");
  });

  it("comes back on for a device that asked for it", async () => {
    localStorage.setItem(WRAP, "on");

    const page = await reading(WAITING);

    expect(diffOf(page).className).toBe("diff wrapped");
    expect(wrapSwitch(page).checked).toBe(true);
  });

  it("leaves nothing behind when it is turned back off", async () => {
    localStorage.setItem(WRAP, "on");
    const page = await reading(WAITING);

    wrapSwitch(page).click();

    expect(diffOf(page).className).toBe("diff");
    expect(
      localStorage.getItem(WRAP),
      "the absence is already the default, so off is nothing kept",
    ).toBeNull();
  });

  it("costs the page nothing when the device has no storage to remember it in", async () => {
    // A browser that refuses storage costs the human their settings and nothing
    // else: the Diff is still drawn, and the switch still works for this visit.
    vi.spyOn(Storage.prototype, "getItem").mockImplementation(() => {
      throw new Error("refused");
    });
    vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => {
      throw new Error("refused");
    });

    const page = await reading(WAITING);
    expect(diffOf(page).className).toBe("diff");

    wrapSwitch(page).click();
    expect(diffOf(page).className).toBe("diff wrapped");

    vi.restoreAllMocks();
  });
});
