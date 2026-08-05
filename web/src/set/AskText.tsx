//! What one question asks: the name it answers to, then its text as the server
//! rendered it.
//!
//! Shared by the form and the record — a settled Set is read for what was asked,
//! so it is asked in the same words and the same markup.

import type { JSX } from "solid-js";

/// A `div` rather than a `p`: the agent's markdown can be as blocky as a list, a
/// table or a fenced block, and none of those may live inside a paragraph. The
/// rendered markdown is boxed inside it so the label can stay a child of its own
/// rather than being swallowed by the markup beside it.
export function AskText(props: { name: string; html: string }): JSX.Element {
  return (
    <div class="text">
      <span class="label">{props.name}</span>
      <div class="markdown" innerHTML={props.html} />
    </div>
  );
}
