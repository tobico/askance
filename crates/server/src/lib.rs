//! The Askance server: the agents' HTTP API and the human's web UI, over one
//! SQLite store and out of one binary.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, Result};
use askance_app::{App, shell};
use askance_store::{Settlements, Waits};
use axum::Router;
use axum::extract::{DefaultBodyLimit, FromRef};
use axum::routing::{get, post};
use leptos::prelude::{LeptosOptions, provide_context};
use leptos_axum::{LeptosRoutes, generate_route_list};
use sqlx::SqlitePool;

mod push;
mod reply;
mod responses;
mod sets;
mod ui;

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

/// How many settlements a held wait can fall behind before it gives up
/// following along and goes back to the store instead. One notification per
/// Set settled, for a single human settling them: this is generous.
const SETTLEMENT_BACKLOG: usize = 64;

/// What the handlers share: the store, word of Sets that have just been settled
/// so held waits need not poll it, and which Sets a wait is being held on.
#[derive(Clone)]
pub(crate) struct AppState {
    pool: SqlitePool,
    settlements: Settlements,
    waits: Waits,
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

/// Everything the server answers in a serialised format: the agents' contract
/// under `/api/v1/`, and the viewer's own namespace under `/api/ui/`.
///
/// Both live under `/api/` and neither uses `/api/{fn_name}`, which Leptos
/// server functions claim by default.
pub fn router(pool: SqlitePool) -> Router {
    api(pool, Settlements::new(SETTLEMENT_BACKLOG), Waits::new())
}

/// The same, over an already-made channel and registry, so the pages the Leptos
/// half still renders share the ones the waits are held on and recorded in.
fn api(pool: SqlitePool, settlements: Settlements, waits: Waits) -> Router {
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
        // The viewer's half. It shares this state rather than holding its own:
        // a submit or an archiving from the browser has to reach an agent
        // waiting on the endpoint above, and both halves have to agree about
        // which Sets a wait is being held on.
        .merge(ui::routes())
        .with_state(AppState {
            pool,
            settlements,
            waits,
        })
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
    let settlements = Settlements::new(SETTLEMENT_BACKLOG);
    let waits = Waits::new();

    // Server functions run outside any axum handler, so what they need reaches
    // them through the Leptos context rather than through router state. The
    // channel and the registry are the same ones the API's waits are held on: a
    // submit or an archiving from the browser has to reach an agent waiting on
    // the REST endpoint, and the pages have to see the waits it is holding.
    let context_pool = pool.clone();
    let context_settlements = settlements.clone();
    let context_waits = waits.clone();
    let shell_options = leptos_options.clone();

    // What the bundles are under, as a path prefix to match. Shared rather than
    // cloned per request: it is the same string for the life of the server.
    let pkg_dir: std::sync::Arc<str> = format!("/{}/", leptos_options.site_pkg_dir).into();

    let ui = Router::new()
        .leptos_routes_with_context(
            &leptos_options,
            routes,
            move || {
                provide_context(context_pool.clone());
                provide_context(context_settlements.clone());
                provide_context(context_waits.clone());
            },
            move || shell(shell_options.clone()),
        )
        .fallback(leptos_axum::file_and_error_handler(shell))
        .layer(axum::middleware::from_fn(move |request, next| {
            let pkg_dir = pkg_dir.clone();
            async move { cached(&pkg_dir, request, next).await }
        }))
        .with_state(leptos_options);

    api(pool, settlements, waits).merge(ui)
}

/// How long a bundle under `site-pkg-dir` may be kept: a year, which is as long
/// as the specification lets anyone ask for, and unconditionally, since a hashed
/// name is never reused. `hash-files` is what earns this — see the workspace's
/// Leptos metadata.
const KEEP: &str = "public, max-age=31536000, immutable";

/// What everything else gets: kept, but never used without asking the server
/// first. A page has to be revalidated because it *names* the hashed bundles —
/// serving a stale one hands the browser the previous build's wasm and the
/// hydration panic that comes with it. The service worker and the manifest have
/// stable names for their own reasons and so cannot be kept either.
const ASK: &str = "no-cache";

/// Say how long each response may be reused for.
///
/// Nothing here set a `Cache-Control` before this, which left it to each
/// browser's guess — and a browser guessing that a bundle under a stable name is
/// still fresh is a browser that runs the last build's wasm against this build's
/// HTML. Both halves of that are answered: the bundles are named by content and
/// kept forever, and everything that names them is revalidated every time.
///
/// A handler that has said its own answer keeps it.
async fn cached(
    pkg_dir: &str,
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    use axum::http::header::CACHE_CONTROL;

    let hashed = request.uri().path().starts_with(pkg_dir);

    let mut response = next.run(request).await;

    let headers = response.headers_mut();
    if !headers.contains_key(CACHE_CONTROL) {
        let policy = if hashed { KEEP } else { ASK };
        headers.insert(CACHE_CONTROL, policy.try_into().expect("a valid header"));
    }

    response
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
        // Matching the workspace's `hash-files`, since what this is falling back
        // to is exactly what `cargo leptos build` left behind: without it the
        // page would name a `pkg/askance.wasm` that no build writes any more.
        // The file itself is found beside the binary, which is where cargo-leptos
        // puts it.
        .hash_files(true)
        .build()
}
