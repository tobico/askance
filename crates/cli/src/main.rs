//! `askance` — the command coding agents run to put a Question Set to the
//! human and block until it is answered. See the crate docs for the shape of
//! it; this is only the exit code.

use std::process::ExitCode;

use askance_cli::Cli;
use clap::Parser;

fn main() -> ExitCode {
    match Cli::parse().run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("askance: {error:#}");
            ExitCode::FAILURE
        }
    }
}
