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

  // The whole of how the page decides which colours it is in — there is no
  // switch and no stored preference, so this is also the only warning of a
  // change. A `MediaQueryList` answers live, so this is asked rather than read.
  const dark = window.matchMedia("(prefers-color-scheme: dark)");

  // Each diagram that drew: the figure standing where its source block was, the
  // source itself — which the block took with it when it went — and the number
  // the drawing is named by. Kept because the colour scheme can flip after a
  // diagram is drawn, and drawing it again is the only way to follow.
  const drawn = [];

  // Mermaid stamps the id it is handed all the way through the SVG it gives
  // back, so no two drawings on the page may share one — and for the moment
  // between a redraw finishing and the drawing it replaces going away, the pair
  // are both here. The generation is what tells them apart.
  let generation = 0;

  function id(n) {
    return `diagram-${generation}-${n}`;
  }

  // Mermaid measures a diagram by laying it out in the document, and it has one
  // place it does that in — so a pass over the page waits for the pass before it
  // rather than overlapping with it. Both ends of the chain matter: the flip can
  // land while the first drawing is still going on, and the pass it asks for has
  // nothing to redraw until that one has finished and said what it drew.
  let pass = Promise.resolve();

  function queue(work) {
    // The same either way: a pass that threw is finished too, and the next one is
    // owed its turn regardless.
    pass = pass.then(work, work);
  }

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

  // The three classes an agent can put on a node to say what it did to the thing
  // the node stands for, in the colours the Diff below marks the same three
  // things in — so the picture and the patch read as one account of the delta
  // rather than two.
  //
  // Handed to mermaid as CSS of its own rather than written in the stylesheet
  // beside every other rule about the page, for a reason that is entirely
  // mermaid's: what it is given it namespaces under the id of the drawing it is
  // drawing — `.node.new rect` becomes `#diagram-0-1 .node.new rect` — and an id
  // out-ranks any selector a stylesheet could write about a node from outside.
  // The colours are still the stylesheet's; only the rules that spend them are
  // here.
  //
  // Each mark is the Diff's own pattern for a line: the wash behind it and the
  // saturated ink at its edge. `modified` is the one the Diff has no colour of
  // its own for — it marks lines, and a changed line there is an added one
  // beside a removed one — so it takes the wash that means "look at this",
  // outlined in the accent to tell it from a note, which is filled the same way.
  function marks(ink) {
    // What the base theme fills and outlines, because which of them a node is
    // drawn as is the diagram's business and not this script's.
    const shapes = ["rect", "circle", "ellipse", "polygon", "path"];

    const mark = (name, wash, edge) => {
      const selector = shapes
        .map((shape) => `.node.${name} ${shape}`)
        .join(", ");

      return `${selector} { fill: ${wash}; stroke: ${edge}; }`;
    };

    // Nothing about a node nobody tagged: every selector above is qualified by
    // the class it marks, so an untagged one is left to the theme.
    return [
      mark("new", ink("--added-wash"), ink("--added")),
      mark("modified", ink("--marked"), ink("--accent")),
      mark("removed", ink("--removed-wash"), ink("--removed")),
    ].join("\n");
  }

  // What mermaid should draw with, read off the document every time it is asked
  // for rather than written out here. The stylesheet is the only thing that knows
  // what colour the page is — it has two schemes and picks between them by media
  // query, and nothing tells this script which one won — so asking the document
  // is both how a diagram comes out on the page's own palette and how it comes
  // out on the right one of the two.
  //
  // `base` because it is mermaid's only theme that is all overrides: every other
  // one brings a palette with it, and a second palette on the page is the thing
  // this is here to avoid. `darkMode` is not a colour but it decides the
  // direction mermaid derives the shades it still works out for itself, and on
  // this page that is the same question the query above answers.
  function palette() {
    const page = getComputedStyle(document.documentElement);
    const ink = (name) => page.getPropertyValue(name).trim();

    return {
      theme: "base",
      themeCSS: marks(ink),
      themeVariables: {
        darkMode: dark.matches,

        // Behind the drawing: a Diagram is inside a card, never on the paper.
        background: ink("--card"),

        // A node. The fenced block's wash, so a Diagram and the code around it
        // are washed alike; the softer ink for the outline, because the hairline
        // the page draws boxes with is too faint to hold a shape at this size;
        // and the page's own ink for the label inside.
        primaryColor: ink("--code-wash"),
        primaryBorderColor: ink("--ink-soft"),
        primaryTextColor: ink("--ink"),

        // The two shades mermaid reaches for when one fill is not enough. Given
        // shades of the page rather than left alone: unset, the base theme
        // arrives at them by rotating the first fill's hue, which is how a colour
        // that is on no palette at all ends up on the page.
        secondaryColor: ink("--hunk"),
        secondaryTextColor: ink("--ink"),
        tertiaryColor: ink("--card"),
        tertiaryTextColor: ink("--ink"),

        // Everything drawn between the nodes, and every label that is not inside
        // one. An edge label lands on top of the line it belongs to, so it needs
        // the page behind it to be read at all.
        lineColor: ink("--ink-soft"),
        textColor: ink("--ink"),
        edgeLabelBackground: ink("--card"),

        // A subgraph: the wash the Diff's hunks get, boxed in the page's own
        // hairline. A grouping should read as a grouping and not as one more node.
        clusterBkg: ink("--hunk"),
        clusterBorder: ink("--edge"),

        // A note is the one thing in a diagram that is there to be looked at,
        // which is the one thing `--marked` is the colour for.
        noteBkgColor: ink("--marked"),
        noteBorderColor: ink("--edge"),
        noteTextColor: ink("--ink"),

        // The page's own type, asked for rather than named, so there is one place
        // the font stack is written down. A label in a node is not prose and reads
        // better a notch down from it — mermaid wants that as a length, and it is
        // the same notch down the stylesheet takes its smaller text.
        fontFamily: getComputedStyle(document.body).fontFamily,
        fontSize: "14px",
      },
    };
  }

  // Mermaid settles its theme at init and bakes it into the SVG it hands back, so
  // every pass over the page opens by telling it what the page looks like now.
  function configure(mermaid) {
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
      ...palette(),
    });
  }

  async function drawAll(mermaid) {
    configure(mermaid);

    const sources = document.querySelectorAll(SOURCE);

    // In turn rather than all at once, for the reason the queue exists.
    for (let n = 0; n < sources.length; n++) {
      await draw(mermaid, sources[n], n);
    }
  }

  async function draw(mermaid, source, n) {
    // `textContent` un-escapes the source back to what the agent wrote. It is
    // kept as well as drawn: the block holding it is about to be replaced, and a
    // redraw has nowhere else to read it from.
    const text = source.textContent;
    const svg = await rendered(mermaid, text, n);

    // Unparseable, or mermaid gave up drawing it. The source block stays exactly
    // as it came, and there is nothing here to redraw later either.
    if (svg === null) return;

    // A `div` in place of the `pre`, because the stylesheet washes and boxes
    // every `pre` in rendered markdown as the code it usually is, and a drawn
    // diagram is not that.
    const figure = document.createElement("div");
    figure.className = DRAWN;
    figure.innerHTML = svg;
    source.replaceWith(figure);

    drawn.push({ figure, text, n });
  }

  // Every diagram that drew, drawn again for the scheme the page is in now. The
  // ones that did not are still source blocks, and the stylesheet themes those
  // along with the rest of the page, so they need nothing from here.
  async function redrawAll(mermaid) {
    configure(mermaid);
    generation += 1;

    for (const diagram of drawn) {
      const svg = await rendered(mermaid, diagram.text, diagram.n);

      // A redraw that fails leaves the drawing that is already there: it is in
      // last scheme's colours, which is a worse page than this one and a much
      // better one than a hole where the diagram was.
      if (svg !== null) diagram.figure.innerHTML = svg;
    }
  }

  // Mermaid's drawing step, and the one place its two ways of not drawing —
  // refusing the source, and failing on it — become the same nothing.
  async function rendered(mermaid, text, n) {
    try {
      const { svg } = await mermaid.render(id(n), text);
      return svg;
    } catch (_) {
      return null;
    }
  }

  withMermaid((mermaid) => {
    // Listening before the first pass rather than after it, so that a flip
    // arriving while the page is still being drawn is a flip like any other.
    dark.addEventListener("change", () => queue(() => redrawAll(mermaid)));
    queue(() => drawAll(mermaid));
  });
})();
