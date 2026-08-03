//! How the API answers: YAML, in both directions, like the Sets themselves.

use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde::Serialize;

pub(crate) fn yaml<T: Serialize>(status: StatusCode, body: &T) -> Response {
    match serde_saphyr::to_string(body) {
        Ok(text) => (status, [(header::CONTENT_TYPE, "application/yaml")], text).into_response(),
        Err(error) => {
            tracing::error!(error = ?error, "serialising a reply failed");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "the reply could not be serialised\n",
            )
                .into_response()
        }
    }
}
