//! The Nudge: telling the open viewer pages that the pending world moved.
//!
//! A Nudge is contentless (ADR-0005). It says that a Set arrived, was answered
//! or was archived, and never which of the three, because the page's reaction
//! is the same either way — look again at everything it is showing. Nothing
//! here has to be got right for correctness: a Nudge that never lands costs
//! latency, and the ten-second poll underneath collects the change anyway.
//!
//! The three durable changes reach this stream over two channels rather than
//! one. A settlement — a Response taken, or a Set closed unanswered — is
//! already announced on [`Settlements`](askance_store::Settlements) from inside
//! the store, on the single path the browser's submit and the agent's both
//! take, so listening to it here is what makes it impossible to Nudge about a
//! settlement from one namespace and silently not from the other. Only a Set
//! arriving needs a channel of its own.
//!
//! Liveness stays out of it, and so stays with the poll: the
//! waiting/disconnected verdict cycles with the agent's long-poll rather than
//! changing at clean moments, so there is no moment here to broadcast.

use std::convert::Infallible;
use std::time::Duration;

use axum::extract::State;
use axum::response::Sse;
use axum::response::sse::{Event, KeepAlive};
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::{Stream, StreamExt};

use crate::AppState;

/// How many Sets may arrive before a page that is behind is told it is. It
/// costs nothing to be told: a Nudge carries nothing, so a page that missed a
/// burst of them is nudged once for the whole burst and reads the world back
/// as it now is.
const CREATION_BACKLOG: usize = 16;

/// How often the stream says something into the quiet. Anything between the
/// page and the server may close a connection that has gone silent, and a
/// reconnect is the slow way to find out the world moved — this is what keeps
/// the stream open rather than reopened.
const KEEP_ALIVE: Duration = Duration::from_secs(15);

/// Word that a Question Set has just been created.
///
/// The counterpart to the store's `Settlements`, for the one durable change
/// that has nowhere else to come from: nothing waits on a Set arriving, so
/// until the viewer wanted to hear about it there was nobody to tell.
#[derive(Debug, Clone)]
pub(crate) struct Creations(broadcast::Sender<()>);

impl Creations {
    pub(crate) fn new() -> Self {
        let (arrivals, _) = broadcast::channel(CREATION_BACKLOG);
        Self(arrivals)
    }

    /// Tell the open pages that a Set has arrived. A send error means none are
    /// open, which is the ordinary case — there is usually no browser pointed
    /// at this at all.
    pub(crate) fn announce(&self) {
        let _ = self.0.send(());
    }

    fn subscribe(&self) -> broadcast::Receiver<()> {
        self.0.subscribe()
    }
}

/// `GET /api/ui/nudges` — the stream an open page listens on.
pub(crate) async fn nudges(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    // Subscribed to before the response goes back, so a change landing while
    // the page is still opening the stream is one it hears rather than one that
    // slips past it.
    let arrived = BroadcastStream::new(state.creations.subscribe());
    let settled = BroadcastStream::new(state.settlements.subscribe());

    // Falling behind is folded in with everything else rather than handled: a
    // page cannot miss what a Nudge said, because a Nudge says nothing. Being
    // overtaken and being told are the same event here.
    let nudges = arrived
        .map(|_| ())
        .merge(settled.map(|_| ()))
        .map(|()| Ok(nudge()));

    Sse::new(nudges).keep_alive(KeepAlive::new().interval(KEEP_ALIVE))
}

/// One Nudge on the wire.
///
/// The event name is the whole of the message. The data field is there only
/// because a browser drops an event that has none, so it repeats the name
/// rather than saying anything a page is meant to read.
fn nudge() -> Event {
    Event::default().event("nudge").data("nudge")
}
