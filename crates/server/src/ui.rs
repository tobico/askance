//! The viewer's own JSON namespace: everything the human's browser asks of the
//! server, and nothing an agent ever calls.
//!
//! Kept under `/api/ui/` rather than mixed into `/api/v1/` because the two are
//! different promises. The agent contract is versioned and public to whatever is
//! installed out there, so it changes only by adding; this namespace is private
//! to the viewer that ships in the same binary, and the two of them may be
//! rearranged together whenever it suits.
//!
//! What crosses it is JSON, where the agents' half speaks YAML: the agents' side
//! is read and written by humans in a terminal, and this side is read by a
//! browser. Everything the agent wrote arrives already rendered to sanitized HTML
//! — see [`askance_render`] — so the viewer needs no markdown parser, no diff
//! highlighter and no sanitizer of its own.
//!
//! The two mutations answer 200 with a named outcome rather than a status code,
//! including for the Set that has gone: every one of those outcomes is something
//! the viewer has to say to the human in words, and a Set answered from another
//! device is not an error to be retried. What does get a status code is a Set
//! that cannot be read at all — a 404, because that is a page the viewer draws
//! differently.

use askance_render::{
    ArchiveEntry, Archived, PendingEntry, PushKey, SetView, Standing, Submitted, Subscribed,
    Subscription, Unsubscribe,
};
use askance_schema::{ApiError, Response};
use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response as HttpResponse};
use axum::routing::{get, post};
use time::OffsetDateTime;

use crate::{AppState, store};

/// The viewer's routes, over the state the agent API is already holding: a
/// submit from the browser has to reach an agent waiting on the REST endpoint,
/// so both halves settle Sets through the same channel and read Liveness out of
/// the same registry of held waits.
pub(crate) fn routes() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/api/ui/pending", get(pending))
        .route("/api/ui/archive", get(archive))
        .route("/api/ui/sets/{id}", get(set))
        .route("/api/ui/sets/{id}/response", post(submit_response))
        .route("/api/ui/sets/{id}/archive", post(archive_set))
        .route("/api/ui/push/key", get(push_key))
        .route("/api/ui/push/subscribe", post(subscribe))
        .route("/api/ui/push/unsubscribe", post(unsubscribe))
}

/// `GET /api/ui/pending` — the Sets still waiting on the human, newest first.
async fn pending(State(state): State<AppState>) -> HttpResponse {
    let now = OffsetDateTime::now_utc();

    let pending = match store::pending_sets(&state.pool).await {
        Ok(pending) => pending,
        Err(error) => {
            tracing::error!(error = ?error, "reading the pending Sets failed");
            return unavailable("the pending Sets could not be read");
        }
    };

    let rows: Vec<PendingEntry> = pending
        .into_iter()
        .map(|set| PendingEntry {
            id: set.id,
            title: set.title,
            project: set.project,
            branch: set.branch,
            // All three already decided here rather than sent as timestamps:
            // this is the side with the clock and with the registry of held
            // waits.
            age: askance_render::relative_age(&set.created_at, now),
            created_stamp: askance_render::utc_stamp(&set.created_at),
            liveness: state.waits.liveness(set.id, &set.created_at, now),
        })
        .collect();

    Json(rows).into_response()
}

/// `GET /api/ui/archive` — the Sets that have been settled, newest first.
async fn archive(State(state): State<AppState>) -> HttpResponse {
    let now = OffsetDateTime::now_utc();

    let archived = match store::archived_sets(&state.pool).await {
        Ok(archived) => archived,
        Err(error) => {
            tracing::error!(error = ?error, "reading the Archive failed");
            return unavailable("the Archive could not be read");
        }
    };

    let rows: Vec<ArchiveEntry> = archived
        .into_iter()
        .map(|set| ArchiveEntry {
            id: set.id,
            title: set.title,
            project: set.project,
            branch: set.branch,
            settled_at: askance_render::settled_age(&set.settled_at, now),
            settled_stamp: askance_render::utc_stamp(&set.settled_at),
            unanswered: set.settled == store::Settled::ArchivedUnanswered,
        })
        .collect();

    Json(rows).into_response()
}

/// `GET /api/ui/sets/{id}` — one Set, rendered, with where it stands.
///
/// Where it stands travels with it rather than being asked for afterwards:
/// it decides whether the viewer draws a form or a record, and a Set answered
/// days ago must not flash a form first.
async fn set(State(state): State<AppState>, Path(id): Path<String>) -> HttpResponse {
    // An id that is not a number cannot name a Set, so it gets the same answer
    // as one that names no Set: there isn't one. The viewer's own routes are
    // this permissive too — the id comes out of a URL the human may have typed.
    let Ok(id) = id.parse::<i64>() else {
        return no_such_set(&id);
    };

    let stored = match store::load_set(&state.pool, id).await {
        Ok(Some(stored)) => stored,
        Ok(None) => return no_such_set(&id.to_string()),
        Err(error) => {
            tracing::error!(error = ?error, set_id = id, "loading a Question Set failed");
            return unavailable("the Question Set could not be read");
        }
    };

    let settlement = match store::settlement(&state.pool, id).await {
        Ok(settlement) => settlement,
        Err(error) => {
            tracing::error!(error = ?error, set_id = id, "reading how a Set stands failed");
            return unavailable("the Question Set could not be read");
        }
    };

    let standing = match settlement {
        Some(store::Settlement::Answered(answered)) => {
            Standing::Answered(askance_render::Answered {
                submitted_at: answered.submitted_at,
                response: answered.response,
            })
        }
        Some(store::Settlement::ArchivedUnanswered(archived)) => {
            Standing::ArchivedUnanswered(archived.archived_at)
        }
        // The same verdict the pending list's row carries, from the same
        // registry: this is the page it is acted on.
        None => Standing::Waiting(state.waits.liveness(
            id,
            &stored.created_at,
            OffsetDateTime::now_utc(),
        )),
    };

    // Everything the agent wrote, rendered — which is the whole of what is left
    // to do, and none of it this crate's.
    let view: SetView = askance_render::set_view(stored.id, stored.set, standing);

    Json(view).into_response()
}

/// `POST /api/ui/sets/{id}/response` — answer a Set.
///
/// Goes through the same store call the agent-facing endpoint does, so a submit
/// from the browser wakes a waiting agent exactly as `curl` would.
async fn submit_response(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(response): Json<Response>,
) -> HttpResponse {
    let Ok(id) = id.parse::<i64>() else {
        return Json(Submitted::NoSuchSet).into_response();
    };

    let submission =
        match store::submit_response(&state.pool, &state.settlements, id, &response).await {
            Ok(submission) => submission,
            Err(error) => {
                tracing::error!(error = ?error, set_id = id, "taking a Response failed");
                return unavailable("the Response could not be taken");
            }
        };

    Json(match submission {
        store::Submission::Accepted(_) => Submitted::Accepted,
        store::Submission::AlreadyAnswered => Submitted::AlreadyAnswered,
        store::Submission::NoSuchSet => Submitted::NoSuchSet,
        store::Submission::Archived => Submitted::Archived,
        store::Submission::Invalid(invalid) => {
            Submitted::Rejected(invalid.violations.iter().map(ToString::to_string).collect())
        }
    })
    .into_response()
}

/// `POST /api/ui/sets/{id}/archive` — close a Set unanswered.
///
/// The human declaring that nobody is ever going to answer it, so it stops being
/// something that is waiting on them. Only ever reached from a browser
/// (ADR-0001) — the agent API has no route for it, because a disconnected agent
/// is not evidence: the CLI reconnects through transient drops.
async fn archive_set(State(state): State<AppState>, Path(id): Path<String>) -> HttpResponse {
    let Ok(id) = id.parse::<i64>() else {
        return Json(Archived::NoSuchSet).into_response();
    };

    let archiving = match store::archive_set(&state.pool, &state.settlements, id).await {
        Ok(archiving) => archiving,
        Err(error) => {
            tracing::error!(error = ?error, set_id = id, "archiving a Set failed");
            return unavailable("the Question Set could not be archived");
        }
    };

    Json(match archiving {
        store::Archiving::Archived(_) => Archived::Closed,
        store::Archiving::AlreadyAnswered => Archived::AlreadyAnswered,
        store::Archiving::AlreadyArchived => Archived::AlreadyArchived,
        store::Archiving::NoSuchSet => Archived::NoSuchSet,
    })
    .into_response()
}

/// `GET /api/ui/push/key` — the public half of the server's VAPID keypair.
async fn push_key(State(state): State<AppState>) -> HttpResponse {
    match store::vapid_keys(&state.pool).await {
        Ok(keys) => Json(PushKey {
            key: keys.public_key,
        })
        .into_response(),
        Err(error) => {
            tracing::error!(error = ?error, "reading the VAPID keypair failed");
            unavailable("the push key could not be read")
        }
    }
}

/// `POST /api/ui/push/subscribe` — take a device's subscription, so a Set
/// arriving can reach it.
async fn subscribe(
    State(state): State<AppState>,
    Json(subscription): Json<Subscription>,
) -> HttpResponse {
    let subscribing = store::store_subscription(
        &state.pool,
        &store::PushSubscription {
            endpoint: subscription.endpoint,
            p256dh: subscription.p256dh,
            auth: subscription.auth,
        },
    )
    .await;

    match subscribing {
        Ok(store::Subscribing::Stored) => Json(Subscribed::Stored).into_response(),
        Ok(store::Subscribing::Incomplete) => Json(Subscribed::Incomplete).into_response(),
        Err(error) => {
            tracing::error!(error = ?error, "storing a push subscription failed");
            unavailable("the subscription could not be stored")
        }
    }
}

/// `POST /api/ui/push/unsubscribe` — forget a device, because it asked not to be
/// told any more.
///
/// An endpoint the server never stored is not an error: a browser can drop its
/// own subscription without the server having heard of it, and afterwards what
/// was asked for holds either way — nothing is sent there.
async fn unsubscribe(
    State(state): State<AppState>,
    Json(unsubscribe): Json<Unsubscribe>,
) -> HttpResponse {
    match store::forget_subscription(&state.pool, &unsubscribe.endpoint).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => {
            tracing::error!(error = ?error, "forgetting a push subscription failed");
            unavailable("the subscription could not be forgotten")
        }
    }
}

/// There is no such Set to read. Worded with whatever was asked for, which for a
/// typed URL is not a number at all.
fn no_such_set(id: &str) -> HttpResponse {
    refused(
        StatusCode::NOT_FOUND,
        ApiError::new(format!("there is no Question Set {id}")),
    )
}

fn unavailable(message: &str) -> HttpResponse {
    refused(StatusCode::INTERNAL_SERVER_ERROR, ApiError::new(message))
}

/// A refusal, in the same shape the agent API refuses in: the viewer's fetches
/// then have one thing to read whichever half of the server answered.
fn refused(status: StatusCode, error: ApiError) -> HttpResponse {
    (status, Json(error)).into_response()
}
