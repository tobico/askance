//! Wire types shared by the server, the CLI, and the web UI: Question Sets
//! going in, Responses coming back, and the invariants of the question grammar
//! that both ends enforce. Also what the server tells the human's browser about
//! a Set that the agents never see, such as its Liveness — the store computes
//! it and the UI draws it, and this is the crate the two have in common.
//!
//! This crate is compiled to `wasm32-unknown-unknown` for the browser, so it
//! must stay free of server-only dependencies — no tokio, axum, or SQLite.

mod api;
mod liveness;
mod response;
mod set;
mod validate;

pub use api::{ApiError, ResponseAccepted, SetCreated};
pub use liveness::Liveness;
pub use response::{Answer, Response};
pub use set::{Question, QuestionOption, QuestionSet, Subquestion};
pub use validate::{ValidationError, Violation};
