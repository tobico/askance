//! The Response endpoints: the human's reply in, and the waiting agent's
//! long-poll out.

use std::time::Duration;

use askance_schema::{ApiError, Response};
use askance_store::{Settlement, Submission};
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response as HttpResponse};
use serde::Deserialize;
use tokio::sync::broadcast;
use tokio::sync::broadcast::error::RecvError;
use tokio::time::{Instant, timeout};

use crate::reply::yaml;
use crate::{AppState, MAX_HOLD, store};

/// `POST /api/v1/sets/{id}/response` — take the human's reply, check it
/// resolves the Set, store it, and wake whoever is waiting.
///
/// Malformed YAML is a 400; a Response that leaves a question unaccounted for
/// is a 422 naming it. A Set is answered once: a second Response is a 409 and
/// the first one stands. A Set the human archived unanswered is closed for good,
/// so a Response to one is a 410.
pub(crate) async fn submit_response(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    body: String,
) -> HttpResponse {
    let response = match Response::from_yaml(&body) {
        Ok(response) => response,
        Err(error) => {
            return yaml(
                StatusCode::BAD_REQUEST,
                &ApiError::new(format!("the Response is not well-formed: {error}")),
            );
        }
    };

    match store::submit_response(&state.pool, &state.settlements, id, &response).await {
        Ok(Submission::Accepted(accepted)) => yaml(StatusCode::CREATED, &accepted),
        Ok(Submission::NoSuchSet) => not_found(id),
        Ok(Submission::Archived) => gone(id),
        Ok(Submission::Invalid(invalid)) => yaml(
            StatusCode::UNPROCESSABLE_ENTITY,
            &ApiError::with_violations(
                "the Response does not resolve the Question Set",
                invalid.violations,
            ),
        ),
        Ok(Submission::AlreadyAnswered) => yaml(
            StatusCode::CONFLICT,
            &ApiError::new(format!("Question Set {id} has already been answered")),
        ),
        Err(error) => {
            tracing::error!(error = ?error, set_id = id, "taking a Response failed");
            unavailable("the Response could not be taken")
        }
    }
}

/// How long a wait is held open when the client does not say.
const DEFAULT_HOLD: Duration = Duration::from_secs(30);

/// What the client asks for when it opens a wait.
#[derive(Debug, Deserialize)]
pub(crate) struct Wait {
    /// How many seconds the client is willing to have the request held open,
    /// clamped to [`MAX_HOLD`]. `0` makes it a plain poll.
    hold: Option<u64>,
}

/// `GET /api/v1/sets/{id}/response` — hand the Response to the waiting agent.
///
/// Answers straight away if the Set has been answered already; otherwise holds
/// the connection until it is, or until the hold window closes and the reply
/// is a bare 204 meaning "nothing yet, come back". There is no expiry on the
/// waiting itself: the client owns retry, so it simply opens another wait.
///
/// A Set the human archived unanswered ends the wait with a 410 instead: nothing
/// is ever coming, and the client is told so rather than left polling a Set
/// nobody will answer.
pub(crate) async fn wait_for_response(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Query(wait): Query<Wait>,
) -> HttpResponse {
    let hold = wait
        .hold
        .map_or(DEFAULT_HOLD, Duration::from_secs)
        .min(MAX_HOLD);

    // Subscribe before the first read, so a Set settled between the two wakes
    // this wait instead of slipping past it.
    let mut settlements = state.settlements.subscribe();

    match store::set_exists(&state.pool, id).await {
        Ok(true) => {}
        Ok(false) => return not_found(id),
        Err(error) => {
            tracing::error!(error = ?error, set_id = id, "looking for a Question Set failed");
            return unavailable("the Question Set could not be read");
        }
    }

    // Taken once the Set is known to exist, so a 404 leaves no trace in the
    // registry, and held for exactly as long as this future lives: a client that
    // vanishes mid-hold has its future dropped rather than returned, and the
    // guard is what both endings have in common. Display only — nothing below
    // consults it, and no Set is withdrawn for want of one.
    let _held = state.waits.hold(id);

    let deadline = Instant::now() + hold;

    loop {
        match store::settlement(&state.pool, id).await {
            Ok(Some(Settlement::Answered(stored))) => {
                return yaml(StatusCode::OK, &stored.response);
            }
            Ok(Some(Settlement::ArchivedUnanswered(_))) => return gone(id),
            Ok(None) => {}
            Err(error) => {
                tracing::error!(error = ?error, set_id = id, "loading a Response failed");
                return unavailable("the Response could not be read");
            }
        }

        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return StatusCode::NO_CONTENT.into_response();
        }

        if !matches!(
            timeout(remaining, settled(&mut settlements, id)).await,
            Ok(true)
        ) {
            return StatusCode::NO_CONTENT.into_response();
        }
    }
}

/// Wait until this Set is settled, one way or the other. `false` when the
/// channel is gone, which only happens as the server itself does.
async fn settled(settlements: &mut broadcast::Receiver<i64>, id: i64) -> bool {
    loop {
        match settlements.recv().await {
            Ok(settled) if settled == id => return true,
            Ok(_) => continue,
            // Overtaken by a burst of settlements: ours may have been among
            // them, so go back and look at the store.
            Err(RecvError::Lagged(_)) => return true,
            Err(RecvError::Closed) => return false,
        }
    }
}

fn not_found(id: i64) -> HttpResponse {
    yaml(
        StatusCode::NOT_FOUND,
        &ApiError::new(format!("there is no Question Set {id}")),
    )
}

/// The Set was archived unanswered: it is closed, and no amount of retrying will
/// change that. The CLI treats this as fatal.
fn gone(id: i64) -> HttpResponse {
    yaml(
        StatusCode::GONE,
        &ApiError::new(format!(
            "Question Set {id} was archived unanswered, so it is closed"
        )),
    )
}

fn unavailable(message: &str) -> HttpResponse {
    yaml(StatusCode::INTERNAL_SERVER_ERROR, &ApiError::new(message))
}
