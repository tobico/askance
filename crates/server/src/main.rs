use anyhow::Result;
use askance_server::Config;
use clap::Parser;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| "askance_server=info".into()),
        )
        .init();

    askance_server::run(Config::parse()).await
}
