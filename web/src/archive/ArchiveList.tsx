//! The Archive: every Question Set that has been settled, newest first — the ones
//! that were answered, and the ones the human closed unanswered because nobody
//! was ever going to answer them.
//!
//! A permanent log. A Set lands here by being answered or by being archived
//! unanswered rather than by anyone filing it, and nothing here is ever deleted —
//! which is what the rows are worded for: no Liveness badge, because nothing is
//! waiting on a settled Set, and its time in the words the pending list is
//! scanned in — an age while the settling is fresh, because the top of this
//! list is scanned the same way, and the plain date once it is not, because
//! "400d ago" says nothing about a log. The exact minute rides behind the
//! words as the tooltip. A Set that was never answered says so, because
//! reading it as a decision would be reading a decision nobody made.

import { A } from "@solidjs/router";
import { useQuery } from "@tanstack/solid-query";
import { For, Match, Show, Switch } from "solid-js";

import { listArchive } from "../api/client";
import type { ArchiveEntry } from "../api/types";

/// The Sets that have been settled, newest first.
export function ArchiveList() {
  // Read once when the page opens, unlike the pending list's ten-second refetch:
  // nothing here is waiting on the human, and a decision that has already been
  // made does not go stale while the page is being read.
  const archive = useQuery(() => ({
    queryKey: ["archive"],
    queryFn: listArchive,
  }));

  return (
    // The wrapper is the page's width: a row carries a title and a line of
    // provenance, and never wanted more room than the reading column — so on a
    // window that offers more, the whole page keeps to that and sits in the
    // middle of it.
    <div class="list-page">
      {/* The way back to what is still waiting, in the slot the set view's
          "← Pending" sits in: every page here starts with where else there is to
          go, so neither list needs a typed URL to reach the other. */}
      <A class="back" href="/">
        ← Pending
      </A>
      <div class="page-head">
        <h1>Archive</h1>
      </div>
      <Switch>
        {/* Pending rather than fetching: the fallback belongs to the first load
            alone. */}
        <Match when={archive.isPending}>
          <p class="empty">Loading…</p>
        </Match>
        <Match when={archive.isError}>
          <p class="error">
            Could not read the Archive: {archive.error?.message}
          </p>
        </Match>
        <Match when={archive.data?.length === 0}>
          {/* Both ways into the Archive, because a Set reaches it either way. */}
          <p class="empty">Nothing has been answered or archived yet.</p>
        </Match>
        <Match when={archive.data}>
          {(sets) => (
            <ul class="set-list">
              <For each={sets()}>{(entry) => <ArchiveRow entry={entry} />}</For>
            </ul>
          )}
        </Match>
      </Switch>
    </div>
  );
}

/// One line of the log: what was asked and where from, and when it was
/// settled — with, when that is what happened, the fact that it was closed
/// without ever being answered.
///
/// Built like a pending row and styled as one — the lists are read the same way,
/// and the same Set may well have been looked at in both.
function ArchiveRow(props: { entry: ArchiveEntry }) {
  // In the same words the set view uses, and in the same place a settling's
  // time sits: what a row of this log has to say first is which of the two it
  // is, because only one of them is a decision.
  const marks = () =>
    props.entry.unanswered
      ? "set-row archived-set unanswered"
      : "set-row archived-set";
  const settled = () =>
    `${props.entry.unanswered ? "archived unanswered" : "answered"} ${
      props.entry.settled_at
    }`;

  // The whole row is the link, as on the pending list: on a phone the tap target
  // should be the card, not the title inside it.
  return (
    <li class={marks()}>
      <A href={`/sets/${props.entry.id}`}>
        <span class="title">{props.entry.title}</span>
        <span class="meta">
          <Show when={props.entry.project}>
            {(project) => <span class="project">{project()}</span>}
          </Show>
          <Show when={props.entry.branch}>
            {(branch) => <span class="branch">{branch()}</span>}
          </Show>
          <span class="decided-at" title={props.entry.settled_stamp}>
            {settled()}
          </span>
        </span>
      </A>
    </li>
  );
}
