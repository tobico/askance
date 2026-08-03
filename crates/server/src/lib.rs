//! The Askance server: the agents' HTTP API and the human's web UI, over one
//! SQLite store and out of one binary.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, Result};
use askance_app::{App, shell};
use axum::Router;
use axum::extract::{DefaultBodyLimit, FromRef};
use axum::routing::{get, post};
use leptos::prelude::{LeptosOptions, provide_context};
use leptos_axum::{LeptosRoutes, generate_route_list};
use sqlx::SqlitePool;
use tokio::sync::broadcast;

mod reply;
mod responses;
mod sets;

/// Persistence lives in its own crate so the UI's server functions can reach
/// it without depending on the binary that links them. It is re-exported here
/// because, from the API's side of things, it is still the server's store.
pub use askance_store as store;
pub use askance_store::open_database;

/// How large a submitted Question Set may be. Generous, because the CLI
/// attaches the whole uncommitted Diff to every Set.
const MAX_SET_BYTES: usize = 32 * 1024 * 1024;

/// The longest a client may ask to have a wait held open. There is no expiry
/// on the waiting itself — the client owns retry (ADR-0001), so it picks the
/// hold length and the server only bounds it.
const MAX_HOLD: Duration = Duration::from_secs(60);

/// How many submissions a held wait can fall behind before it gives up
/// following along and goes back to the store instead. One notification per
/// answered Set, for a single human answering them: this is generous.
const SUBMISSION_BACKLOG: usize = 64;

/// What the handlers share: the store, and word of Sets that have just been
/// answered so held waits need not poll it.
#[derive(Clone)]
pub(crate) struct AppState {
    pool: SqlitePool,
    submissions: broadcast::Sender<i64>,
}

impl FromRef<AppState> for SqlitePool {
    fn from_ref(state: &AppState) -> Self {
        state.pool.clone()
    }
}

/// How the server is pointed at its database and its socket. There is no
/// app-level auth: the tailnet is the perimeter, so the defaults keep the
/// server on the loopback interface until told otherwise.
#[derive(Debug, Clone, clap::Parser)]
#[command(name = "askance-server", version, about = "Askance server")]
pub struct Config {
    /// Path to the SQLite database. Created, with its parent directory, if
    /// it does not exist.
    #[arg(long, env = "ASKANCE_DATABASE", default_value = "askance.db")]
    pub database: PathBuf,

    /// Address and port to bind. Bind a tailnet address to reach the server
    /// from other devices.
    #[arg(long, env = "ASKANCE_LISTEN", default_value = "127.0.0.1:8422")]
    pub listen: SocketAddr,
}

/// The agent-facing routes. REST lives under `/api/v1/` to stay clear of
/// `/api/{fn_name}`, which Leptos server functions claim by default.
pub fn router(pool: SqlitePool) -> Router {
    let (submissions, _) = broadcast::channel(SUBMISSION_BACKLOG);

    Router::new()
        .route("/api/v1/health", get(health))
        .route(
            "/api/v1/sets",
            post(sets::create_set).layer(DefaultBodyLimit::max(MAX_SET_BYTES)),
        )
        .route(
            "/api/v1/sets/{id}/response",
            post(responses::submit_response).get(responses::wait_for_response),
        )
        .with_state(AppState { pool, submissions })
}

async fn health() -> &'static str {
    "ok"
}

/// Everything the one binary serves: the agent API above, plus the Leptos UI
/// on every other path.
///
/// The UI is merged in second and takes the fallback, so `/api/v1/` keeps its
/// exact paths and anything unclaimed — pages, server functions, the wasm and
/// CSS under `/pkg/` — reaches Leptos.
pub fn router_with_ui(pool: SqlitePool, leptos_options: LeptosOptions) -> Router {
    let routes = generate_route_list(App);

    // Server functions run outside any axum handler, so the pool reaches them
    // through the Leptos context rather than through router state.
    let context_pool = pool.clone();
    let shell_options = leptos_options.clone();

    let ui = Router::new()
        .leptos_routes_with_context(
            &leptos_options,
            routes,
            move || provide_context(context_pool.clone()),
            move || shell(shell_options.clone()),
        )
        .fallback(leptos_axum::file_and_error_handler(shell))
        .with_state(leptos_options);

    router(pool).merge(ui)
}

/// Open the database and serve until the process is stopped.
pub async fn run(config: Config) -> Result<()> {
    let pool = open_database(&config.database).await?;

    let leptos_options = leptos_options();

    let listener = tokio::net::TcpListener::bind(config.listen)
        .await
        .with_context(|| format!("binding {}", config.listen))?;

    tracing::info!(
        listen = %config.listen,
        database = %config.database.display(),
        "askance is listening",
    );

    axum::serve(listener, router_with_ui(pool, leptos_options))
        .await
        .context("serving Askance")
}

/// Where the built UI's files are and what they are called.
///
/// Under `cargo leptos` this comes from the environment, which also carries
/// what live reload needs. Built plainly the environment is empty, so fall back
/// to the same site root and name the workspace's Leptos metadata configures —
/// a `cargo run -p askance-server` then serves whatever `cargo leptos build`
/// last produced, instead of refusing to start.
pub fn leptos_options() -> LeptosOptions {
    if std::env::var_os("LEPTOS_OUTPUT_NAME").is_some()
        && let Ok(conf) = leptos::config::get_configuration(None)
    {
        return conf.leptos_options;
    }

    LeptosOptions::builder()
        .output_name("askance")
        .site_root("target/site")
        .build()
}
