//! `askance` — the command coding agents run to put a Question Set to the
//! human and block until it is answered.
//!
//! The CLI is the agent-facing compatibility surface (ADR-0001): agents run it
//! as a background shell command, so the wait outlives any harness tool
//! timeout, and nothing but a shell is needed to integrate. It also derives
//! `project`, `branch` and the Diff from the working directory, so those are
//! never at the mercy of what an agent claims.

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};

mod ask;
mod client;
pub mod repo;

/// Where the server lives when nothing says otherwise. The tailnet is the
/// perimeter, so the default stays on the loopback interface.
const DEFAULT_SERVER: &str = "http://127.0.0.1:8422";

#[derive(Debug, Parser)]
#[command(
    name = "askance",
    version,
    about = "Put a Question Set to the human and wait for the answer"
)]
pub struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Submit a Question Set and block until the human answers it.
    ///
    /// Prints the Response as YAML on stdout and exits 0. Nothing else is ever
    /// written to stdout, so the agent can parse it as it stands.
    Ask {
        /// The Question Set, as YAML. Read from stdin when absent.
        file: Option<PathBuf>,

        /// Base URL of the Askance server.
        #[arg(long, env = "ASKANCE_SERVER", default_value = DEFAULT_SERVER)]
        server: String,
    },
}

impl Cli {
    pub fn run(self) -> Result<()> {
        match self.command {
            Command::Ask { file, server } => ask::ask(file.as_deref(), &server),
        }
    }
}
