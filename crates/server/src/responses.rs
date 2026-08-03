//! The Response endpoints: the human's reply in, and the waiting agent's
//! long-poll out.

use std::time::Duration;

use askance_schema::{ApiError, Response};
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
/// the first one stands.
pub(crate) async fn submit_response(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    body: String,
) -> HttpResponse {
    let set = match store::load_set(&state.pool, id).await {
        Ok(Some(stored)) => stored.set,
        Ok(None) => return not_found(id),
        Err(error) => {
            tracing::error!(error = ?error, set_id = id, "loading a Question Set failed");
            return unavailable("the Question Set could not be read");
        }
    };

    let response = match Response::from_yaml(&body) {
        Ok(response) => response,
        Err(error) => {
            return yaml(
                StatusCode::BAD_REQUEST,
                &ApiError::new(format!("the Response is not well-formed: {error}")),
            );
        }
    };

    if let Err(invalid) = response.validate(&set) {
        return yaml(
            StatusCode::UNPROCESSABLE_ENTITY,
            &ApiError::with_violations(
                "the Response does not resolve the Question Set",
                invalid.violations,
            ),
        );
    }

    match store::insert_response(&state.pool, id, &response).await {
        Ok(Some(accepted)) => {
            // Wake every wait held on this Set. An error means nobody is
            // listening, which is the ordinary case — the agent may well be
            // between polls.
            let _ = state.submissions.send(id);
            yaml(StatusCode::CREATED, &accepted)
        }
        Ok(None) => yaml(
            StatusCode::CONFLICT,
            &ApiError::new(format!("Question Set {id} has already been answered")),
        ),
        Err(error) => {
            tracing::error!(error = ?error, set_id = id, "storing a Response failed");
            unavailable("the Response could not be stored")
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
pub(crate) async fn wait_for_response(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Query(wait): Query<Wait>,
) -> HttpResponse {
    let hold = wait
        .hold
        .map_or(DEFAULT_HOLD, Duration::from_secs)
        .min(MAX_HOLD);

    // Subscribe before the first read, so a Response submitted between the two
    // wakes this wait instead of slipping past it.
    let mut submissions = state.submissions.subscribe();

    match store::set_exists(&state.pool, id).await {
        Ok(true) => {}
        Ok(false) => return not_found(id),
        Err(error) => {
            tracing::error!(error = ?error, set_id = id, "looking for a Question Set failed");
            return unavailable("the Question Set could not be read");
        }
    }

    let deadline = Instant::now() + hold;

    loop {
        match store::load_response(&state.pool, id).await {
            Ok(Some(stored)) => return yaml(StatusCode::OK, &stored.response),
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
            timeout(remaining, answered(&mut submissions, id)).await,
            Ok(true)
        ) {
            return StatusCode::NO_CONTENT.into_response();
        }
    }
}

/// Wait until this Set is answered. `false` when the channel is gone, which
/// only happens as the server itself does.
async fn answered(submissions: &mut broadcast::Receiver<i64>, id: i64) -> bool {
    loop {
        match submissions.recv().await {
            Ok(answered) if answered == id => return true,
            Ok(_) => continue,
            // Overtaken by a burst of submissions: ours may have been among
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

fn unavailable(message: &str) -> HttpResponse {
    yaml(StatusCode::INTERNAL_SERVER_ERROR, &ApiError::new(message))
}
