//! The pending list: the Question Sets still waiting on the human, newest
//! first.

import { A } from "@solidjs/router";
import { useQuery } from "@tanstack/solid-query";
import { For, Match, Show, Switch } from "solid-js";

import { listPending } from "../api/client";
import type { Liveness, PendingEntry } from "../api/types";
import { Notifications } from "../push/Notifications";

/// How often the open page refetches the list, in milliseconds.
///
/// The badge has to keep up with an agent that has died, and while the page is
/// asking it also keeps the ages honest and brings a newly arrived Set into
/// view — which is what stands in for push notifications until there are any.
const REFRESH = 10_000;

/// What the badge says. The word that colours it is the Liveness itself, which
/// is the class on the span.
///
/// Worded about the agent rather than about the connection: what the human
/// wants to know before answering is whether anyone is still on the other end.
///
/// Shared with the set view, which badges the Set it is about to be archived
/// from: the same state has to read as the same words wherever it is met.
export const BADGE: Record<Liveness, string> = {
  waiting: "agent waiting",
  disconnected: "agent disconnected",
};

/// The Sets still waiting on the human, newest first.
export function PendingList() {
  const pending = useQuery(() => ({
    queryKey: ["pending"],
    queryFn: listPending,
    // The query keeps the last list while the next one is in flight, so a
    // refetch every ten seconds swaps rows rather than blinking the page
    // through a loading state the human is trying to read past.
    refetchInterval: REFRESH,
  }));

  return (
    <>
      {/* The way through to what was already decided, in the slot the set
          view's "← Pending" sits in: every page here starts with where else
          there is to go, so neither list needs a typed URL to reach the
          other. */}
      <A class="to-archive" href="/archive">
        Archive →
      </A>
      {/* On the title's line rather than buried in a settings page: this is the
          one page open often enough to be where the human notices that the
          phone is not being told, and a switch is small enough to live in the
          space the heading was leaving empty anyway. */}
      <div class="page-head">
        <h1>Pending</h1>
        <Notifications />
      </div>
      <Switch>
        {/* Pending rather than fetching: the fallback belongs to the first
            load alone. */}
        <Match when={pending.isPending}>
          <p class="empty">Loading…</p>
        </Match>
        <Match when={pending.isError}>
          <p class="error">
            Could not read the pending Sets: {pending.error?.message}
          </p>
        </Match>
        <Match when={pending.data?.length === 0}>
          <p class="empty">Nothing is waiting on you.</p>
        </Match>
        <Match when={pending.data}>
          {(sets) => (
            <ul class="set-list">
              <For each={sets()}>{(entry) => <PendingRow entry={entry} />}</For>
            </ul>
          )}
        </Match>
      </Switch>
    </>
  );
}

function PendingRow(props: { entry: PendingEntry }) {
  // The whole row is the link: on a phone the tap target should be the card,
  // not the title inside it.
  return (
    <li class="set-row pending-set">
      <A href={`/sets/${props.entry.id}`}>
        <span class="title">{props.entry.title}</span>
        <span class="meta">
          <Show when={props.entry.project}>
            {(project) => <span class="project">{project()}</span>}
          </Show>
          <Show when={props.entry.branch}>
            {(branch) => <span class="branch">{branch()}</span>}
          </Show>
          <span class={`liveness ${props.entry.liveness}`}>
            {BADGE[props.entry.liveness]}
          </span>
          <span class="age">{props.entry.age}</span>
        </span>
      </A>
    </li>
  );
}
