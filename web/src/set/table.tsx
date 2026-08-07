//! The shape of an Answer Table, shared by the sheet and the record.
//!
//! The two draw different bodies — one holds radios and the other holds what was
//! decided — but they are one table, and a record whose columns did not line up
//! with the sheet's would be a different reading of the same declaration. So the
//! columns are settled once, here, and both take them from it.

import type { JSX } from "solid-js";
import { For, Show } from "solid-js";

import type { OptionView } from "../api/types";

/// Whether a ★ column is drawn at all: only where there is a Recommendation to
/// mark. A column that could only ever be empty is a column of nothing, and one
/// narrower table is one less thing to scan past on a phone.
export function starred(options: OptionView[]): boolean {
  return options.some((option) => option.recommended);
}

/// The head of an Answer Table: the columns fixed around the agent's, in the
/// order the rows below fill them — the Option's number first, its text next
/// under the one word this page supplies, then the axes, and the ★ last.
///
/// The first column's header and the ★'s are empty on purpose. Neither is an
/// axis, and a word over either would read as one more thing the agent said.
export function Head(props: {
  columns: string[];
  starred: boolean;
}): JSX.Element {
  return (
    <thead>
      <tr>
        <th />
        <th scope="col">Option</th>
        <For each={props.columns}>
          {(column) => <th scope="col" class="markdown" innerHTML={column} />}
        </For>
        <Show when={props.starred}>
          <th />
        </Show>
      </tr>
    </thead>
  );
}
