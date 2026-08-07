//! What the agent closed the Set with, drawn above the box the human writes
//! their own closing word in.
//!
//! The same kind of thing as the Preface and drawn the same way — the agent's
//! markdown, rendered by the server, with a Diagram in it drawn like any other.
//! What differs is what it is for: the Preface opens the page, and this
//! introduces the comment box under it, so it carries no heading and the table
//! of contents does not offer a way to it. It is read on the way to the box, not
//! jumped to.
//!
//! Here rather than in either page half because both halves draw it: the sheet a
//! waiting Set is filled in on, and the record a settled one is read as. One
//! component means the two cannot drift into drawing it differently.

import type { JSX } from "solid-js";
import { Show } from "solid-js";

/// The Set's Postscript, and nothing at all for a Set that closed without one —
/// which the server says by sending none, whitespace included.
export function Postscript(props: { html: string | null }): JSX.Element {
  return (
    <Show when={props.html}>
      {(html) => (
        <section class="postscript">
          {/* Marked as rendered markdown, so the agent's headings, lists and
              code get the same rules here as they get in the Preface — the
              section around it is all that is the Postscript's own. */}
          <div class="postscript-body markdown" innerHTML={html()} />
        </section>
      )}
    </Show>
  );
}
