//! Wire types shared by the server, the CLI, and (later) the web UI: Question
//! Sets going in, Responses coming back, and the invariants of the question
//! grammar that both ends enforce.
//!
//! This crate is compiled to `wasm32-unknown-unknown` for the browser, so it
//! must stay free of server-only dependencies — no tokio, axum, or SQLite.
