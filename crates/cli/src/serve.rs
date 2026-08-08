//! `askance serve` — the server, run out of the binary the agent already has.
//!
//! One binary carries both halves (ADR-0004), so this is where the two parts
//! the server needs and no other verb does are assembled: the async runtime,
//! and somewhere for the server's `tracing` to go.

use anyhow::{Context, Result};
use askance_server::Config;
use tracing_subscriber::EnvFilter;

/// What the server logs when `RUST_LOG` says nothing: its own startup line and
/// whatever else it has to report, and nothing from the crates beneath it.
const DEFAULT_FILTER: &str = "askance_server=info";

/// Serve until the process is stopped.
///
/// The runtime is built here rather than around `main`: `ask` and `guide` are
/// blocking calls, and an agent running one should not be paying for threads
/// nothing is going to use.
pub fn serve(config: Config) -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| DEFAULT_FILTER.into()),
        )
        .init();

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("starting the async runtime")?
        .block_on(askance_server::run(config))
}
