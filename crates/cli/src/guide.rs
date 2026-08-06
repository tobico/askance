//! The Guide: what an agent needs in order to ask well, carried by the binary
//! that does the asking.
//!
//! The text is a markdown file in this repo, embedded at compile time rather
//! than assembled at run time — so what an agent reads is exactly what was
//! reviewed, and the binary alone is the whole documentation.

use std::io::Write;

use anyhow::{Context, Result};

/// The core Guide: everything any ask needs.
const CORE: &str = include_str!("../guide/core.md");

/// Print the core Guide on stdout.
pub fn guide() -> Result<()> {
    let mut stdout = std::io::stdout().lock();
    stdout
        .write_all(CORE.as_bytes())
        .and_then(|()| stdout.flush())
        .context("writing the Guide to stdout")
}
