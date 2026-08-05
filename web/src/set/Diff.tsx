//! The attached Diff: what the agent is asking about, shown rather than
//! described.
//!
//! It arrives as HTML the server already rendered — parsed per file,
//! highlighted, and anchored `diff-1`, `diff-2`, … in the order the paths
//! beside it name — so there is nothing here to render and no diff parser in
//! the browser at all. The folds are the browser's own `details`, which is why
//! they work without a line of script.

import type { JSX } from "solid-js";

import { Switch } from "../Switch";
import type { DiffView } from "../api/types";

/// The attached Diff, and the one setting that governs how it is read.
///
/// The wrap switch sits beside the heading rather than in a settings page
/// somewhere, because this is the only place its answer is visible — and it
/// governs every Diff, not this one, which is why it is remembered on the
/// device instead of per Set.
///
/// Wrapping is a class and nothing more: the Diff arrived rendered, so there is
/// nothing here to render again and the stylesheet is the whole of the change.
export function Diff(props: {
  diff: DiffView;
  wrapped: boolean;
  flip: (on: boolean) => void;
}): JSX.Element {
  return (
    <section class={props.wrapped ? "diff wrapped" : "diff"} id="diff">
      <div class="section-head">
        <h2 class="section-heading">Diff</h2>
        <Switch label="Word wrap" on={props.wrapped} flip={props.flip} />
      </div>
      {/* The per-file anchors are stamped by the renderer, since this arrives
          already rendered. */}
      <div class="diff-files" innerHTML={props.diff.html} />
    </section>
  );
}
