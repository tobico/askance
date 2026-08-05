//! Drawing the Diagrams on a Set's page.
//!
//! The renderer itself is a stand-in throughout: what is under test is which
//! blocks are drawn, what happens to one that will not draw, and that a page
//! whose colours change is drawn again — none of which needs mermaid to be here
//! to be asked.

import { waitFor } from "@solidjs/testing-library";
import { afterEach, describe, expect, it, vi } from "vitest";

import { drawDiagrams } from "../src/set/diagrams";
// The stylesheet as text: the two rules asserted at the bottom are about a drawn
// diagram, which is an SVG mermaid wrote and no component renders, so there is
// nothing to query them off.
import stylesheet from "../src/main.css?raw";

/// A page carrying the source blocks the markdown renderer leaves behind —
/// escaped, exactly as the server wrote them.
function page(...sources: string[]) {
  document.body.innerHTML = sources
    .map((source) => `<pre class="mermaid">${source}</pre>`)
    .join("");
}

/// The colour scheme, as something a test can flip. jsdom answers the media
/// query but has no way to change its mind, and following a change of scheme is
/// half of what this module does.
function scheme() {
  const listeners = new Set<() => void>();
  const query = {
    matches: false,
    addEventListener: (_: string, listen: () => void) => listeners.add(listen),
    removeEventListener: (_: string, listen: () => void) =>
      listeners.delete(listen),
  };

  vi.stubGlobal("matchMedia", () => query);

  return {
    flip() {
      query.matches = !query.matches;
      for (const listen of [...listeners]) listen();
    },
    watched: () => listeners.size,
  };
}

/// The page's palette, as something a test can read back out of what mermaid was
/// told. jsdom resolves no custom property, so `getComputedStyle` is stood in for:
/// every `--name` asked for answers with a marker naming itself, which is what
/// lets an assertion say *which* variable a theme colour was spent from.
///
/// It also records the asking, so the second half of the pairing can be checked:
/// every property the renderer reaches for has to be one the stylesheet defines.
function palette() {
  const asked = new Set<string>();

  vi.stubGlobal("getComputedStyle", () => ({
    getPropertyValue: (name: string) => {
      asked.add(name);
      return ` spent(${name}) `;
    },
    fontFamily: "the page's own type",
  }));

  return {
    asked: () => [...asked],
    /// What a variable comes out as once the renderer has trimmed it.
    spent: (name: string) => `spent(${name})`,
  };
}

/// A stand-in for the renderer: what it was told about the page, what it was
/// asked to draw, and whatever `drawing` makes of each source — `null` for the
/// two ways mermaid does not draw, which it never tells apart.
function renderer(drawing: (text: string) => string | null) {
  const configured: Array<Record<string, unknown>> = [];
  const asked: Array<{ id: string; text: string }> = [];

  const bundle = () =>
    Promise.resolve({
      initialize(config: Record<string, unknown>) {
        configured.push(config);
      },
      render(id: string, text: string) {
        asked.push({ id, text });
        const svg = drawing(text);
        return svg === null
          ? Promise.reject(new Error("will not draw"))
          : Promise.resolve({ svg });
      },
    });

  return { bundle, configured, asked };
}

/// A drawing of whatever it was handed, so a test can tell one apart from
/// another without a renderer in the room.
function drawn(text: string): string {
  return `<svg data-source="${text.trim()}"></svg>`;
}

afterEach(() => {
  vi.unstubAllGlobals();
  document.body.innerHTML = "";
});

describe("the Diagrams on a page", () => {
  it("draws every source block, in place and unescaped", async () => {
    scheme();
    const { bundle, asked } = renderer(drawn);
    page("graph LR;\n  client--&gt;api;\n", "graph TD;\n  a--&gt;b;\n");

    drawDiagrams({ bundle });

    // The figures rather than the asking: a block is replaced once the drawing
    // of it comes back, which is a turn after the renderer was handed it.
    //
    // A `div` in place of the `pre`, because the stylesheet washes and boxes
    // every `pre` in rendered markdown as the code it usually is.
    await waitFor(() =>
      expect(document.querySelectorAll("div.diagram")).toHaveLength(2),
    );

    // `textContent` is what takes the escaping back off, so the renderer is
    // handed what the agent wrote rather than what the page holds.
    expect(asked[0]!.text).toBe("graph LR;\n  client-->api;\n");
    expect(asked[1]!.text).toBe("graph TD;\n  a-->b;\n");

    const figures = document.querySelectorAll("div.diagram");
    expect(figures[0]!.innerHTML).toContain('data-source="graph LR;');
    expect(document.querySelectorAll("pre.mermaid")).toHaveLength(0);
  });

  it("names each drawing on the page differently", async () => {
    scheme();
    const { bundle, asked } = renderer(drawn);
    page("graph LR;\n  a--&gt;b;\n", "graph LR;\n  a--&gt;b;\n");

    drawDiagrams({ bundle });
    await waitFor(() => expect(asked).toHaveLength(2));

    // Mermaid stamps the id it is handed all the way through the SVG it gives
    // back, so two drawings on one page may not share one.
    expect(asked[0]!.id).not.toBe(asked[1]!.id);
  });

  it("leaves a diagram it cannot draw as the source the agent wrote", async () => {
    scheme();
    const { bundle, asked } = renderer((text) =>
      text.includes("not a diagram") ? null : drawn(text),
    );
    page("not a diagram at all\n", "graph LR;\n  a--&gt;b;\n");

    drawDiagrams({ bundle });
    await waitFor(() => expect(asked).toHaveLength(2));
    await waitFor(() => expect(document.querySelector("div.diagram")).toBeTruthy());

    const left = document.querySelectorAll("pre.mermaid");
    expect(left).toHaveLength(1);
    expect(left[0]!.textContent).toBe("not a diagram at all\n");

    // The fallback is silent: a human reads the source the agent wrote rather
    // than mermaid's complaint about it.
    for (const complaint of ["Syntax error", "error in text", "mermaid-error"]) {
      expect(document.body.innerHTML).not.toContain(complaint);
    }
  });

  it("leaves every block standing when the renderer never arrives", async () => {
    scheme();
    page("graph LR;\n  a--&gt;b;\n");

    drawDiagrams({ bundle: () => Promise.reject(new Error("never landed")) });

    // Nothing to wait for, so the assertion is that nothing happens: a page of
    // readable source blocks is the right page to be left with.
    await Promise.resolve();
    expect(document.querySelectorAll("pre.mermaid")).toHaveLength(1);
    expect(document.querySelector("div.diagram")).toBeNull();
  });

  it("tells the renderer that nothing in a diagram may run or be complained about", async () => {
    scheme();
    const { bundle, configured } = renderer(drawn);
    page("graph LR;\n  a--&gt;b;\n");

    drawDiagrams({ bundle });
    await waitFor(() => expect(configured).toHaveLength(1));

    // Every diagram here was written by an agent, so the labels are sanitized
    // and the click handlers and inline styles a diagram can ask for refused;
    // the deciding of what is drawn is ours; and a diagram mermaid cannot parse
    // must not be drawn as its own error graphic, because the source block is
    // the error state.
    expect(configured[0]).toMatchObject({
      startOnLoad: false,
      securityLevel: "strict",
      suppressErrorRendering: true,
    });
  });

  it("draws what drew again when the colour scheme flips", async () => {
    const colours = scheme();
    const { bundle, asked } = renderer((text) =>
      text.includes("not a diagram") ? null : drawn(text),
    );
    page("graph LR;\n  a--&gt;b;\n", "not a diagram at all\n");

    drawDiagrams({ bundle });
    await waitFor(() => expect(asked).toHaveLength(2));

    colours.flip();

    // The one that drew, and only that one: the other is still a source block,
    // and the stylesheet themes those along with the rest of the page.
    await waitFor(() => expect(asked).toHaveLength(3));
    expect(asked[2]!.text).toBe("graph LR;\n  a-->b;\n");
    expect(document.querySelectorAll("div.diagram")).toHaveLength(1);
  });

  it("stops watching the colour scheme once the page has gone", async () => {
    const colours = scheme();
    const { bundle, asked } = renderer(drawn);
    page("graph LR;\n  a--&gt;b;\n");

    const stop = drawDiagrams({ bundle });
    await waitFor(() => expect(asked).toHaveLength(1));

    stop();
    expect(colours.watched()).toBe(0);

    colours.flip();
    await Promise.resolve();
    expect(asked).toHaveLength(1);
  });
});

describe("the colours a Diagram is drawn in", () => {
  /// What mermaid was told before it drew, over a stood-in palette.
  async function told() {
    scheme();
    const ink = palette();
    const { bundle, configured } = renderer(drawn);
    page("graph LR;\n  a--&gt;b;\n");

    drawDiagrams({ bundle });
    await waitFor(() => expect(configured).toHaveLength(1));

    return { config: configured[0]!, ink };
  }

  it("is mermaid's one theme that brings no palette of its own", async () => {
    const { config } = await told();

    // `base` is the only mermaid theme that is all overrides. Every other one
    // arrives with a palette, and a second palette on the page is the thing this
    // is here to avoid.
    expect(config.theme).toBe("base");
  });

  it("is the page's, read off the document rather than written out again", async () => {
    const { config, ink } = await told();
    const variables = config.themeVariables as Record<string, unknown>;

    // Each of these is spent from a named variable rather than from a literal, so
    // a diagram cannot drift from the page it sits on — including in the dark
    // scheme, which the stylesheet is the only thing that knows about.
    expect(variables.background).toBe(ink.spent("--card"));
    expect(variables.primaryColor).toBe(ink.spent("--code-wash"));
    expect(variables.primaryTextColor).toBe(ink.spent("--ink"));
    expect(variables.lineColor).toBe(ink.spent("--ink-soft"));
    expect(variables.clusterBkg).toBe(ink.spent("--hunk"));
    expect(variables.clusterBorder).toBe(ink.spent("--edge"));
    expect(variables.noteBkgColor).toBe(ink.spent("--marked"));

    // The type too, so the font stack is written down in one place.
    expect(variables.fontFamily).toBe("the page's own type");
  });

  it("marks a tagged node in the Diff's own colours", async () => {
    const { config, ink } = await told();
    const css = config.themeCSS as string;

    // The three classes an agent puts on a node, and the pair of variables each
    // spends: the wash behind the node and the saturated ink around it, which is
    // the Diff's own pattern for a line it added or removed. `modified` is the one
    // the Diff has no colour for — it marks lines, and a changed line there is an
    // added one beside a removed one — so it takes the page's "look at this" wash,
    // outlined in the accent.
    for (const [tag, wash, edge] of [
      ["new", "--added-wash", "--added"],
      ["modified", "--marked", "--accent"],
      ["removed", "--removed-wash", "--removed"],
    ] as const) {
      const mark = css
        .split("\n")
        .find((rule) => rule.includes(`.node.${tag} `));

      expect(mark, `a \`${tag}\` node should be marked`).toBeTruthy();
      expect(mark).toContain(`fill: ${ink.spent(wash)}`);
      expect(mark).toContain(`stroke: ${ink.spent(edge)}`);
    }

    // Every selector is qualified by the class it marks, so a node nobody tagged
    // is left exactly as the theme drew it.
    expect(css).not.toMatch(/(^|[\s,]) *\.node +[a-z]+ *\{/);
  });

  it("spends nothing the stylesheet does not define", async () => {
    const { ink } = await told();

    // The other half of the pairing above: a variable the renderer reaches for and
    // the stylesheet has never heard of resolves to nothing, and mermaid then
    // derives a colour of its own by rotating a hue — which is how a colour that
    // is on no palette at all ends up on the page.
    for (const property of ink.asked()) {
      expect(
        stylesheet.includes(`${property}:`),
        `${property} should be a variable the stylesheet defines`,
      ).toBe(true);
    }
  });

  it("has a value for every mark in each of the page's two schemes", async () => {
    const { ink } = await told();

    // The marks are the one part of the theme handed over as CSS, and a scheme
    // that left one of them undefined would draw that mark as nothing at all.
    for (const property of ink.asked()) {
      expect(
        stylesheet.split(`${property}:`).length - 1,
        `${property} should be defined in both schemes`,
      ).toBeGreaterThanOrEqual(2);
    }
  });
});

describe("a drawn Diagram on the page", () => {
  /// The declarations of the block `selector` opens, which is what a rule about a
  /// drawn diagram has to be read out of: the SVG is mermaid's and there is no
  /// component to query it off.
  function block(selector: string): string {
    const opened = stylesheet.indexOf(`${selector} {`);
    expect(opened, `the stylesheet should have a \`${selector}\` rule`).not.toBe(
      -1,
    );

    return stylesheet.slice(opened, stylesheet.indexOf("}", opened));
  }

  it("fits the width it is given", () => {
    const svg = block(".markdown .diagram svg");

    // At a glance means the whole shape at once, so a diagram too wide for a phone
    // scales down to fit rather than scrolling sideways inside a box — and never
    // widens the page, which is the failure a narrow viewport shows first.
    expect(svg).toContain("max-width: 100%");
    // The height follows the width, which it only does if the height mermaid wrote
    // onto the SVG is overridden.
    expect(svg).toContain("height: auto");
  });

  it("holds still for anyone who asked it to", () => {
    // A mermaid diagram can ask for animated edges, and the animation arrives
    // inside the SVG in a stylesheet of mermaid's own — so this is the one place
    // in the file where turning something off has to out-rank an author.
    const reduced = stylesheet.slice(
      stylesheet.indexOf("@media (prefers-reduced-motion: reduce)"),
    );

    expect(reduced).toContain(".diagram");
    expect(reduced).toContain("animation: none !important");
  });
});
