//! The table of contents down a Set's margin: the way around a page whose Diff
//! can run to thousands of lines.
//!
//! One nav in two shapes — the sidebar in the wide margin, and the bar with the
//! list under it below that width — so which of the two the reader gets is the
//! stylesheet's answer to how wide their window is. These tests ask about the
//! one list both are drawn from, the jump that lands on a folded file, and the
//! highlight that follows the reader down the page.
//!
//! What the browser knows about where the page is scrolled to, jsdom has none
//! of: the scroll-spy's observer is stood in for, so a test says what has
//! crossed the reading line rather than laying out a page to make it happen.
//! What is being asked is what the page does with the answer.

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { SetView } from "../src/api/types";
import { reading, texts } from "./reading";
import answered from "./fixtures/set-answered.json" with { type: "json" };
import answering from "./fixtures/set-answering.json" with { type: "json" };
import archived from "./fixtures/set-archived.json" with { type: "json" };

const WAITING = answering as SetView;

/// The two settled standings, each given the Diff the waiting fixture carries:
/// a Set is read for what it asked about however it stands, so the nav has the
/// same work to do on all three.
const ANSWERED: SetView = { ...(answered as SetView), diff: WAITING.diff };
const ARCHIVED: SetView = { ...(archived as SetView), diff: WAITING.diff };

/// A Set with neither of the two sections a Set can be without.
const BARE: SetView = { ...WAITING, preface_html: null, diff: null };

/// What one crossing of the reading line says, as the spy is told it.
type Crossing = { target: { id: string }; isIntersecting: boolean };

/// One page's scroll-spy, stood in for: what it was told to watch, and the way
/// to tell it something has crossed the line.
type Spy = {
  observing: string[];
  disconnected: boolean;
  margin: string | undefined;
  cross(started: Record<string, boolean>): void;
};

/// The spies the page makes while a test runs, newest last. Stubbed for the
/// whole file: a page that made a real observer here would watch a document
/// jsdom never lays out, and say nothing ever.
function watching(): Spy[] {
  const spies: Spy[] = [];

  class Stub {
    private readonly told: (crossings: Crossing[]) => void;
    private readonly spy: Spy;

    constructor(
      told: (crossings: Crossing[]) => void,
      watch?: { rootMargin?: string },
    ) {
      this.told = told;
      this.spy = {
        observing: [],
        disconnected: false,
        margin: watch?.rootMargin,
        cross: (started) => {
          this.told(
            Object.entries(started).map(([id, isIntersecting]) => ({
              target: { id },
              isIntersecting,
            })),
          );
        },
      };
      spies.push(this.spy);
    }

    observe(target: Element) {
      this.spy.observing.push(target.id);
    }

    unobserve() {}

    disconnect() {
      this.spy.disconnected = true;
    }

    takeRecords() {
      return [];
    }
  }

  vi.stubGlobal("IntersectionObserver", Stub);
  return spies;
}

let spies: Spy[];

beforeEach(() => {
  spies = watching();
  // jsdom lays nothing out and so scrolls nothing; what the jump is worth is
  // that it asked the browser to take the reader there. Asked for no motion —
  // the stub reads as `prefers-reduced-motion: reduce` — the jump asks with
  // `scrollIntoView`, which is the ask these tests can see: the animated ask
  // works from geometry, and jsdom has none to give it.
  Element.prototype.scrollIntoView = vi.fn();
  vi.stubGlobal("matchMedia", () => ({ matches: true }));
  localStorage.clear();
});

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
  localStorage.clear();
});

/// The page makes two observers, told apart by the reading line: the
/// scroll-spy watches with the margin that puts the line near the bottom of
/// the window, and the floating header's watcher of the first heading watches
/// with none.

/// The page's scroll-spy — the last one made, since a test reads one page at a
/// time.
function spy(): Spy {
  const spying = spies.filter((made) => made.margin !== undefined).at(-1);
  expect(spying, "expected the page to be following the reader").toBeTruthy();
  return spying as Spy;
}

/// The floating header's watcher of the page's first heading — the last one
/// made.
function headingSpy(): Spy {
  const spying = spies.filter((made) => made.margin === undefined).at(-1);
  expect(
    spying,
    "expected the header to be watching the first heading",
  ).toBeTruthy();
  return spying as Spy;
}

/// The nav, which every page has.
function navOf(page: ParentNode): HTMLElement {
  const nav = page.querySelector<HTMLElement>("nav.contents");
  expect(nav, "expected a table of contents in the page").toBeTruthy();
  return nav as HTMLElement;
}

/// Every jump the nav offers, in the order it lists them.
function jumps(nav: ParentNode): string[] {
  return [...nav.querySelectorAll("a.contents-link")].map(
    (link) => link.getAttribute("href") ?? "",
  );
}

/// The nav's line jumping to this id.
function line(nav: ParentNode, anchor: string): HTMLAnchorElement {
  const found = nav.querySelector<HTMLAnchorElement>(`a[href="#${anchor}"]`);
  expect(found, `expected a line jumping to #${anchor}`).toBeTruthy();
  return found as HTMLAnchorElement;
}

/// Where the nav says the reader is: the jump the highlighted line points at.
///
/// Read off the marked line rather than out of a class list, because what the
/// highlight is worth is which part of the page it names.
function highlighted(nav: ParentNode): string {
  const lit = nav.querySelectorAll("a.contents-link.contents-here");
  expect(lit, "exactly one line is ever the highlight").toHaveLength(1);
  return lit[0]!.getAttribute("href")!.replace("#", "");
}

/// What the bar says it is at — the whole of the nav on a narrow viewport,
/// until it is tapped.
function barSays(nav: ParentNode): string {
  const said = nav.querySelector(".contents-bar-name");
  expect(said, "expected a bar naming where the reader is").toBeTruthy();
  return said!.textContent ?? "";
}

describe("the table of contents", () => {
  it("mirrors the page top to bottom", async () => {
    const nav = navOf(await reading(WAITING));

    expect(
      jumps(nav),
      "the nav lists the sections in the order the page has them",
    ).toEqual([
      "#preface",
      "#diff",
      "#diff-1",
      "#diff-2",
      "#questions",
      "#q1",
      "#q2",
      "#q3",
      "#postscript",
    ]);
    expect(
      nav.querySelector('a[href="#q2a"]'),
      "a Sub-question scrolls into view with its parent, so it is not listed " +
        "separately",
    ).toBeNull();
    expect(
      line(nav, "q1").textContent,
      "a Question is listed by its label and its own words",
    ).toContain("Q1");
    expect(line(nav, "q1").textContent).toContain(
      "Where should the request counter live?",
    );
  });

  it("names a markdown Question by its words alone", async () => {
    // The nav is a line of text in a narrow column, and the page draws that
    // same Question as blocks: the two are rendered from the one source, and
    // this is the seam where they meet.
    const nav = navOf(await reading(WAITING));

    expect(
      line(nav, "q1").textContent,
      "the nav wants the words, with the list flattened into the line",
    ).toContain(
      "Where should the request counter live? in-process, per instance in " +
        "redis, shared across instances",
    );
    expect(line(nav, "q2").textContent, "a fenced block is words too").toContain(
      "How should a throttled client be told to back off? fn allowance()",
    );
    expect(
      nav.querySelectorAll("p, ul, pre, code"),
      "the nav is text, so the markup the Question is drawn with has no place " +
        "in it",
    ).toHaveLength(0);
  });

  it("names the Diff's files in Diff order, and in full on hover", async () => {
    const nav = navOf(await reading(WAITING));

    // The paths travel with the Set rather than being read back out of the
    // rendered Diff, and the nth of them has to be what the nth fold shows.
    expect(line(nav, "diff-1").textContent).toBe("src/limits.rs");
    expect(line(nav, "diff-2").textContent).toBe("notes.txt");
    expect(
      line(nav, "diff-1").title,
      "the column is narrow, so the whole of a cut path is readable here",
    ).toBe("src/limits.rs");
  });

  it("lists only the sections the Set has", async () => {
    const nav = navOf(await reading(BARE));

    expect(
      jumps(nav),
      "the Questions are the one section every Set has",
    ).toEqual(["#questions", "#q1", "#q2", "#q3", "#postscript"]);
  });

  it("is one nav, one bar and one list on every standing", async () => {
    for (const set of [WAITING, ANSWERED, ARCHIVED]) {
      const page = await reading(set);

      // Which of the bar and the sidebar the reader gets is the stylesheet's
      // business at a width, so there is no second copy to fall out of step.
      expect(page.querySelectorAll("nav.contents")).toHaveLength(1);
      expect(page.querySelectorAll("button.contents-bar")).toHaveLength(1);
      expect(page.querySelectorAll("ol.contents-sections")).toHaveLength(1);

      // A Set is read for what it asked about however it stands.
      for (const jump of ["#preface", "#diff-1", "#questions", "#q1"]) {
        expect(jumps(navOf(page)), `expected ${jump}`).toContain(jump);
      }
    }
  });
});

describe("the highlight", () => {
  it("is on the first section before anything has scrolled", async () => {
    const nav = navOf(await reading(WAITING));

    expect(
      highlighted(nav),
      "a page nobody has scrolled reads as being at the top of it",
    ).toBe("preface");
    expect(
      nav.querySelectorAll(".contents-within"),
      "the quiet mark is on the section the highlight is inside, and at the " +
        "top of the page there is none",
    ).toHaveLength(0);
    expect(line(nav, "preface").getAttribute("aria-current")).toBe("location");
  });

  it("starts on whatever section the Set starts with", async () => {
    const nav = navOf(await reading({ ...WAITING, preface_html: null }));

    expect(
      highlighted(nav),
      "with no Preface the page opens on the Diff, and the first line of the " +
        "nav is the Diff's",
    ).toBe("diff");
  });

  it("follows the reader to the last part of the page to have begun", async () => {
    const nav = navOf(await reading(WAITING));

    spy().cross({ preface: true, diff: true, "diff-1": true });

    expect(highlighted(nav), "the file is where they are").toBe("diff-1");
    expect(
      line(nav, "diff").className,
      "and the Diff only says they are in it",
    ).toContain("contents-within");
    expect(
      line(nav, "diff").getAttribute("aria-current"),
      "the section around them is not where they are",
    ).toBeNull();
  });

  it("watches every anchored part of the page, from a line a tenth down it", async () => {
    navOf(await reading(WAITING));

    expect(spy().observing).toEqual([
      "preface",
      "diff",
      "diff-1",
      "diff-2",
      "questions",
      "q1",
      "q2",
      "q3",
      "postscript",
    ]);
    expect(
      spy().margin,
      "what is under the reading line is what the reader has in front of them, " +
        "and a section long scrolled past still counts as started",
    ).toBe("100000px 0px -90% 0px");
  });

  it("goes back to the top when the reader scrolls back up", async () => {
    const nav = navOf(await reading(WAITING));

    spy().cross({ preface: true, diff: true, "diff-1": true });
    spy().cross({ diff: false, "diff-1": false });

    expect(highlighted(nav)).toBe("preface");
  });

  it("is let go of when the page is", async () => {
    await reading(WAITING);
    const following = spy();
    // The next page's mount takes the last one down.
    await reading(WAITING);

    expect(
      following.disconnected,
      "an observer still watching a page that has gone is a scroll away from " +
        "trouble",
    ).toBe(true);
  });
});

describe("a jump from the contents", () => {
  it("takes the reader to the file, unfolding it first", async () => {
    const page = await reading(WAITING);
    const fold = page.querySelector<HTMLDetailsElement>("#diff-2")!;
    fold.open = false;

    line(navOf(page), "diff-2").click();

    expect(
      fold.open,
      "landing on a closed fold is landing on nothing",
    ).toBe(true);
    expect(fold.scrollIntoView).toHaveBeenCalled();
  });

  it("leaves a hash to copy, and no step to come back through", async () => {
    const page = await reading(WAITING);
    const replacing = vi.spyOn(history, "replaceState");
    const pressing = vi.spyOn(history, "pushState");

    line(navOf(page), "q2").click();

    expect(replacing).toHaveBeenCalledWith(null, "", "#q2");
    expect(
      pressing,
      "moving around a page is not somewhere to come back to",
    ).not.toHaveBeenCalled();
  });

  it("holds the highlight where it landed until the reader scrolls by hand", async () => {
    const nav = navOf(await reading(WAITING));

    line(nav, "q3").click();
    expect(
      highlighted(nav),
      "the page cannot always bring a section to the top, so the highlight " +
        "goes where the reader asked to be",
    ).toBe("q3");

    // The spy answering for wherever the scroll ran out does not move it.
    spy().cross({ preface: true, diff: true });
    expect(highlighted(nav)).toBe("q3");

    // And the reader taking over puts it straight back on where the page is.
    window.dispatchEvent(new Event("wheel"));
    expect(highlighted(nav)).toBe("diff");
  });

  it("is left to the browser when the reader asked for a tab", async () => {
    const page = await reading(WAITING);
    const fold = page.querySelector<HTMLDetailsElement>("#diff-1")!;
    fold.open = false;

    // A modified click is the reader asking their browser for a tab or a
    // window, which is the browser's business and not ours.
    line(navOf(page), "diff-1").dispatchEvent(
      new MouseEvent("click", { bubbles: true, cancelable: true, metaKey: true }),
    );

    expect(fold.open).toBe(false);
    expect(fold.scrollIntoView).not.toHaveBeenCalled();
  });
});

describe("the bar", () => {
  it("names the line the nav has highlighted", async () => {
    const nav = navOf(await reading(WAITING));

    expect(barSays(nav)).toContain("Preface");
    expect(
      highlighted(nav),
      "which is the same line the sidebar marks — one scroll-spy answers for both",
    ).toBe("preface");

    spy().cross({ preface: true, diff: true, "diff-1": true });
    expect(
      barSays(nav),
      "and a file by the same cut path the sidebar shows it under",
    ).toBe("src/limits.rs");
  });

  it("names whatever section the Set starts with", async () => {
    const nav = navOf(await reading({ ...WAITING, preface_html: null }));

    expect(barSays(nav)).toContain("Diff");
    expect(barSays(nav)).not.toContain("Preface");
  });

  it("arrives shut, with the entries in the page all the same", async () => {
    const nav = navOf(await reading(WAITING));
    const bar = nav.querySelector("button.contents-bar")!;

    expect(bar.getAttribute("aria-expanded")).toBe("false");
    expect(bar.getAttribute("aria-controls")).toBe("contents-list");
    expect(nav.className, "nothing has opened it yet").toBe("contents");
    expect(
      nav.querySelector("#contents-list"),
      "the same list the sidebar draws, so opening the bar has nothing to fetch",
    ).toBeTruthy();
  });

  it("brings the list down, and puts it away again", async () => {
    const nav = navOf(await reading(WAITING));
    const bar = nav.querySelector<HTMLButtonElement>("button.contents-bar")!;

    bar.click();
    expect(nav.className).toBe("contents contents-open");
    expect(bar.getAttribute("aria-expanded")).toBe("true");

    bar.click();
    expect(nav.className).toBe("contents");
  });

  it("puts the list away on Escape", async () => {
    const nav = navOf(await reading(WAITING));
    nav.querySelector<HTMLButtonElement>("button.contents-bar")!.click();

    document.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape" }));

    expect(
      nav.className,
      "a list drawn over the page has to be dismissible from the keyboard",
    ).toBe("contents");
  });

  it("puts the list away on a tap beside it, and presses nothing", async () => {
    const nav = navOf(await reading(WAITING));
    nav.querySelector<HTMLButtonElement>("button.contents-bar")!.click();

    const backdrop = nav.querySelector<HTMLElement>(".contents-backdrop");
    expect(
      backdrop,
      "the tap taking the list back must not also press something on the page " +
        "underneath",
    ).toBeTruthy();
    expect(backdrop!.getAttribute("aria-hidden")).toBe("true");

    backdrop!.click();
    expect(nav.className).toBe("contents");
    expect(nav.querySelector(".contents-backdrop")).toBeNull();
  });

  it("puts itself away when a line is pressed", async () => {
    const nav = navOf(await reading(WAITING));
    nav.querySelector<HTMLButtonElement>("button.contents-bar")!.click();

    line(nav, "q1").click();

    expect(nav.className, "the list has done what it was opened for").toBe(
      "contents",
    );
  });
});

describe("the floating header", () => {
  /// What the header says, which is a breadcrumb rather than one line: the
  /// sidebar beside it already says both halves with two highlights.
  function headerOf(page: ParentNode): HTMLElement {
    const header = page.querySelector<HTMLElement>(".page-header");
    expect(header, "expected the floating header").toBeTruthy();
    return header as HTMLElement;
  }

  it("stays out of the way until the first heading has been scrolled past", async () => {
    const page = await reading(WAITING);
    const header = headerOf(page);

    // At the top of the page it would be naming a heading the reader can see
    // right under it, and saying nothing.
    expect(header.querySelector(".page-header-chrome")).toBeNull();

    headingSpy().cross({ preface: false });
    expect(header.querySelector(".page-header-chrome")).toBeTruthy();

    // And scrolling back up puts it away again.
    headingSpy().cross({ preface: true });
    expect(header.querySelector(".page-header-chrome")).toBeNull();
  });

  it("names where the reader is, and says it once", async () => {
    const page = await reading(WAITING);
    const header = headerOf(page);
    headingSpy().cross({ preface: false });

    expect(header.textContent).toContain("Preface");

    spy().cross({ preface: true, diff: true, "diff-1": true });
    expect(
      texts(header, ".page-header-section"),
      "a file is named under the section it is in",
    ).toEqual(["Diff"]);
    expect(header.querySelector(".page-header-name")!.textContent).toBe(
      "src/limits.rs",
    );
    expect(
      header.querySelector(".page-header-where")!.getAttribute("aria-hidden"),
      "the sidebar's own line already carries aria-current, and a second copy " +
        "of the answer is a second thing to hear",
    ).toBe("true");
  });

  it("offers word wrap while the reader is in the Diff, and nowhere else", async () => {
    const page = await reading(WAITING);
    const header = headerOf(page);
    headingSpy().cross({ preface: false });

    expect(header.querySelector(".switch")).toBeNull();

    spy().cross({ preface: true, diff: true });
    expect(
      header.querySelector(".switch"),
      "the switch follows the section, so it stays put from the Diff's heading " +
        "down through its files",
    ).toBeTruthy();

    spy().cross({ "diff-1": true, "diff-2": true });
    expect(header.querySelector(".switch")).toBeTruthy();

    spy().cross({ questions: true, q1: true });
    expect(header.querySelector(".switch")).toBeNull();
  });

  it("is two views of the one setting, with the switch beside the Diff", async () => {
    const page = await reading(WAITING);
    headingSpy().cross({ preface: false });
    spy().cross({ preface: true, diff: true });

    const beside = page.querySelector<HTMLInputElement>(
      "section.diff .switch input",
    )!;
    const floating = page.querySelector<HTMLInputElement>(
      ".page-header .switch input",
    )!;

    floating.click();

    expect(page.querySelector("section.diff")!.className).toBe("diff wrapped");
    expect(
      beside.checked,
      "flipping either has to move the other: they are one setting",
    ).toBe(true);
  });

  it("offers nothing on a Set with no Diff", async () => {
    const page = await reading(BARE);
    headingSpy().cross({ questions: false });

    expect(headerOf(page).querySelector(".switch")).toBeNull();
  });
});
