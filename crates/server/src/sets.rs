//! The Question Set endpoint: an agent's YAML comes in, an id goes back.

use askance_schema::{ApiError, QuestionSet};
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Response;
use sqlx::SqlitePool;

use crate::reply::yaml;
use crate::store;

/// `POST /api/v1/sets` — parse, validate, store, and answer with the id the
/// waiting agent will poll on.
///
/// Malformed YAML is a 400; a well-formed Set that breaks the question grammar
/// is a 422 listing every violation, each naming the Question it belongs to.
pub(crate) async fn create_set(State(pool): State<SqlitePool>, body: String) -> Response {
    let set = match QuestionSet::from_yaml(&body) {
        Ok(set) => set,
        Err(error) => {
            return yaml(
                StatusCode::BAD_REQUEST,
                &ApiError::new(format!("the Question Set is not well-formed: {error}")),
            );
        }
    };

    if let Err(invalid) = set.validate() {
        return yaml(
            StatusCode::UNPROCESSABLE_ENTITY,
            &ApiError::with_violations(
                "the Question Set breaks the question grammar",
                invalid.violations,
            ),
        );
    }

    match store::insert_set(&pool, &set).await {
        Ok(created) => {
            // Behind the answer, never in front of it: the agent hears that its
            // Set is stored the moment it is, and a push service that cannot be
            // reached costs a notification rather than the Set.
            crate::push::announce(&pool, created.id, &set);

            yaml(StatusCode::CREATED, &created)
        }
        Err(error) => {
            tracing::error!(error = ?error, "storing a Question Set failed");
            yaml(
                StatusCode::INTERNAL_SERVER_ERROR,
                &ApiError::new("the Question Set could not be stored"),
            )
        }
    }
}
