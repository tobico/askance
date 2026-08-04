// Draws the Diagrams on a page: the client-side half of the one carve-out from
// rendering everything on the server (ADR 0002), and the only script of our own
// the browser runs outside the wasm.
//
// It and the mermaid bundle beside it are named in the page only when the
// server's rendering of that page turned up a Diagram, so a Set without one
// pays neither.
//
// What it looks for is what the markdown renderer left behind: a `pre.mermaid`
// holding its own source, escaped. That block is also the fallback — for a
// browser with no JS, for a bundle that never arrived, and for a diagram that
// will not draw — so a diagram this script cannot render is one it leaves
// alone, and the human reads the source the agent wrote rather than mermaid's
// complaint about it.
(() => {
  "use strict";

  // The block the renderer wrote, and what a drawn one becomes.
  const SOURCE = "pre.mermaid";
  const DRAWN = "diagram";

  // The bundle's own script tag, which the page names so that this script can
  // wait for it. Both tags are deferred, so on a page load mermaid is already
  // here by the time this runs; a Set reached by a link inside the app has both
  // appended at once instead, and promises nothing about which lands first.
  const BUNDLE = "mermaid-bundle";

  // Do this once mermaid is here to do it with — and never, if it turns out not
  // to be coming. A bundle that failed to load fires no `load` event, which
  // leaves every source block standing, which is the right page to be left with.
  function withMermaid(draw) {
    if (window.mermaid) return draw(window.mermaid);

    const bundle = document.getElementById(BUNDLE);
    if (!bundle) return;

    bundle.addEventListener("load", () => window.mermaid && draw(window.mermaid), {
      once: true,
    });
  }

  async function drawAll(mermaid) {
    // `strict` because every diagram here was written by an agent and is
    // therefore untrusted: it is what has mermaid sanitize the labels it draws
    // and refuse the click handlers and inline styles a diagram can ask for.
    //
    // `startOnLoad` off because the deciding is ours. Mermaid's own pass would
    // find these same blocks on `window.onload` and replace an unparseable one
    // with an error graphic, and the source block is the error state.
    //
    // `suppressErrorRendering` for the half of that mermaid does even when it is
    // asked one diagram at a time: a diagram it cannot parse it draws as a bomb
    // captioned "Syntax error in text", and it draws it into the document before
    // reporting the failure, so leaving the source block alone afterwards is not
    // enough to keep the page quiet. With this on, it reports and draws nothing.
    mermaid.initialize({
      startOnLoad: false,
      securityLevel: "strict",
      suppressErrorRendering: true,
    });

    const sources = document.querySelectorAll(SOURCE);

    // In turn rather than all at once: mermaid measures a diagram by laying it
    // out in the document, and it has one place it does that in.
    for (let n = 0; n < sources.length; n++) {
      await draw(mermaid, sources[n], n);
    }
  }

  async function draw(mermaid, source, n) {
    let drawn;

    try {
      // The id is mermaid's handle on the diagram while it lays it out, and it
      // ends up stamped through the SVG that comes back, so each one gets its
      // own. `textContent` un-escapes the source back to what the agent wrote.
      const rendered = await mermaid.render(`diagram-${n}`, source.textContent);
      drawn = rendered.svg;
    } catch (_) {
      // Unparseable, or mermaid gave up drawing it. The source block stays
      // exactly as it came.
      return;
    }

    // A `div` in place of the `pre`, because the stylesheet washes and boxes
    // every `pre` in rendered markdown as the code it usually is, and a drawn
    // diagram is not that.
    const figure = document.createElement("div");
    figure.className = DRAWN;
    figure.innerHTML = drawn;
    source.replaceWith(figure);
  }

  withMermaid(drawAll);
})();
