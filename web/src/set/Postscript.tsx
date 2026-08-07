//! What the agent closed the Set with, and the human's closing word under it,
//! in one card.
//!
//! The same kind of thing as the Preface and drawn the same way — the agent's
//! markdown, rendered by the server, with a Diagram in it drawn like any other,
//! under a heading of its own that the table of contents offers a way to. What
//! differs is what it is for: the Preface opens the page and this closes it.
//!
//! The heading is named for what the section holds rather than always for the
//! Postscript, because a Set that closed without one still ends here: with no
//! Postscript there is only the box, and `Comment` is what that is.
//!
//! The comment goes *inside* the card rather than after it, because the two are
//! one exchange — what the agent raised and what the human made of it — and a
//! box that trailed the card read as a separate thing that happened to follow
//! it. That is why the card is this section and not the body inside it: the
//! body is filled from the server as raw HTML, so nothing can be placed within
//! it.
//!
//! Here rather than in either page half because both halves draw it: the sheet a
//! waiting Set is filled in on, and the record a settled one is read as. One
//! component means the two cannot drift into drawing it differently.

import type { JSX } from "solid-js";
import { Show } from "solid-js";

import { closing } from "./outline";

/// The Set's Postscript, wrapped around whatever closes it — the field on the
/// sheet, the comment that came back on the record.
///
/// A Set that closed without a Postscript still gets the card, holding only
/// what is passed in: the box is on every Set, so every Set ends the same way.
/// The markdown is what the server sends, and a Set with none sends none,
/// whitespace included.
export function Postscript(props: {
  html: string | null;
  children?: JSX.Element;
}): JSX.Element {
  return (
    <section class="postscript" id="postscript">
      {/* Named and anchored like the Preface: the heading is what a jump from
          the table of contents lands on, and the id is what it jumps to — see
          `closing` in `outline.ts`, which names it by the same rule. */}
      <h2 class="section-heading">{closing(props.html)}</h2>
      <div class="postscript-card">
        <Show when={props.html}>
          {(html) => (
            /* Marked as rendered markdown, so the agent's headings, lists and
               code get the same rules here as they get in the Preface — the
               card around it is all that is the Postscript's own. */
            <div class="postscript-body markdown" innerHTML={html()} />
          )}
        </Show>
        {props.children}
      </div>
    </section>
  );
}
