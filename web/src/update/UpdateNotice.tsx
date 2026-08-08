//! The Update Notice: the banner above the pending list, saying that a newer
//! Askance has been released than the one serving the page.
//!
//! It informs and nothing else (CONTEXT.md). The link to the instructions is the
//! whole of what it offers — nothing here installs anything, and nothing here
//! changes the server. Nor can it be dismissed: it stands until the server is
//! updated, at which point the server stops saying there is anything to update
//! to and the banner goes with it.
//!
//! Everything but a release waiting is drawn as nothing at all — a server that
//! is current, one that could not find out, and the moment before either has
//! been said. A page that cannot reach the endpoint is the pending list exactly
//! as it was.

import { useQuery } from "@tanstack/solid-query";
import type { JSX } from "solid-js";
import { Show } from "solid-js";

import { updateNotice } from "../api/client";

/// Where the human is told how to update: the README's own section, which is
/// the one place the instructions are kept current.
const UPDATING = "https://github.com/tobico/askance#updating";

/// How often the open page asks whether a release has appeared, in milliseconds.
///
/// Hourly, where the list beside it asks every ten seconds: a Set can arrive at
/// any moment and a release cannot. The server polls GitHub once a day and holds
/// the verdict, so asking oftener would only be reading the same memory again.
const REFRESH = 60 * 60 * 1000;

/// The banner, when there is a release to name.
export function UpdateNotice(): JSX.Element {
  const notice = useQuery(() => ({
    queryKey: ["update"],
    queryFn: updateNotice,
    refetchInterval: REFRESH,
    // Coming back to the pending list from a Set is not new information about
    // what has been released, so a fresh mount reads what the last one learned.
    staleTime: REFRESH,
  }));

  // The version waiting, and `null` for every other answer there is — including
  // the ones that are not answers: a request in flight and one that failed.
  const version = (): string | null => {
    const said = notice.data;
    return said && said !== "Current" ? said.Available.version : null;
  };

  return (
    <Show when={version()}>
      {(version) => (
        <aside class="update-notice">
          <span class="release">Askance {version()} has been released.</span>
          {/* Out to the README rather than over the page: the human is in the
              middle of a list of things waiting on them, and reading how to
              update should not be leaving it. */}
          <a href={UPDATING} target="_blank" rel="noreferrer">
            How to update →
          </a>
        </aside>
      )}
    </Show>
  );
}
