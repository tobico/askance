//! The Askance server: an HTTP API over a SQLite store.

use std::net::SocketAddr;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use axum::Router;
use axum::routing::get;
use sqlx::SqlitePool;
use sqlx::sqlite::SqliteConnectOptions;

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

/// Open the SQLite database at `path`, creating the file if it is absent.
pub async fn open_database(path: &Path) -> Result<SqlitePool> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating database directory {}", parent.display()))?;
    }

    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true);

    SqlitePool::connect_with(options)
        .await
        .with_context(|| format!("opening database {}", path.display()))
}

/// The application's routes. REST lives under `/api/v1/` to stay clear of
/// `/api/{fn_name}`, which Leptos server functions claim by default.
pub fn router(pool: SqlitePool) -> Router {
    Router::new()
        .route("/api/v1/health", get(health))
        .with_state(pool)
}

async fn health() -> &'static str {
    "ok"
}

/// Open the database and serve until the process is stopped.
pub async fn run(config: Config) -> Result<()> {
    let pool = open_database(&config.database).await?;

    let listener = tokio::net::TcpListener::bind(config.listen)
        .await
        .with_context(|| format!("binding {}", config.listen))?;

    tracing::info!(
        listen = %config.listen,
        database = %config.database.display(),
        "askance is listening",
    );

    axum::serve(listener, router(pool))
        .await
        .context("serving the API")
}
