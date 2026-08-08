//! Listening for the Nudge: the contentless word from the server that the
//! pending world moved (ADR-0005).
//!
//! There is one reaction and it never varies — look again at everything the
//! page is showing. A Nudge says a Set arrived, was answered or was archived
//! without saying which, because the page would do the same thing either way,
//! and so nothing here has to decide anything.
//!
//! Nothing here has to work, either. Every Nudge is latency saved off the
//! ten-second poll underneath, and a page that gets none of them stays correct
//! at the poll's pace — which is what makes it safe for this to be a connection
//! that drops, a browser that has no `EventSource`, or a server that is being
//! restarted.

import type { QueryClient } from "@tanstack/solid-query";

/// The server's stream — see the `nudge` module on the other side of it.
const STREAM = "/api/ui/nudges";

/// Hold the stream open, looking again at every Nudge, until the returned
/// closer is called.
///
/// The reconnect is a Nudge in itself. A stream comes back from a suspended
/// PWA or a restarted server knowing nothing about what it missed, and it does
/// not need to: what happened while it was dead is read back off the server
/// rather than replayed down the wire. That the first open is not treated the
/// same way is the one distinction drawn here — the page has only just read the
/// world it is opening this over.
export function listenForNudges(queries: QueryClient): () => void {
  // Absent in a browser without server-sent events, which loses the fast path
  // and nothing else: the poll is still there and still enough.
  if (typeof EventSource === "undefined") {
    return () => {};
  }

  const stream = new EventSource(STREAM);

  let established = false;
  stream.addEventListener("open", () => {
    if (established) {
      lookAgain(queries);
    }
    established = true;
  });

  // Named, so that whatever else may one day come down this stream is not
  // mistaken for a Nudge by a page too old to know about it.
  stream.addEventListener("nudge", () => lookAgain(queries));

  return () => stream.close();
}

/// Read back everything the app is showing.
///
/// Every active query at once, rather than the ones a change was about: the
/// page listening is whichever page is open, and a Nudge does not say enough to
/// narrow it down even if it wanted to. What is not on screen is left to be
/// refetched when something mounts it.
function lookAgain(queries: QueryClient): void {
  void queries.invalidateQueries();
}
