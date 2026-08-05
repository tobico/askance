//! Drawing the Diagrams on a Set's page.
//!
//! The renderer itself is a stand-in throughout: what is under test is which
//! blocks are drawn, what happens to one that will not draw, and that a page
//! whose colours change is drawn again — none of which needs mermaid to be here
//! to be asked.

import { waitFor } from "@solidjs/testing-library";
import { afterEach, describe, expect, it, vi } from "vitest";

import { drawDiagrams } from "../src/set/diagrams";

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
