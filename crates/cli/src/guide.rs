//! The Guide: what an agent needs in order to ask well, carried by the binary
//! that does the asking.
//!
//! The text is a markdown file in this repo, embedded at compile time rather
//! than assembled at run time — so what an agent reads is exactly what was
//! reviewed, and the binary alone is the whole documentation.

use std::io::Write;

use anyhow::{Context, Result, anyhow};

/// The core Guide: everything any ask needs.
const CORE: &str = include_str!("../guide/core.md");

/// The Topics, each under the name `askance guide <topic>` takes. Order is the
/// order they are listed back in, so it stays the order the core Guide names
/// them in.
const TOPICS: &[(&str, &str)] = &[("gates", include_str!("../guide/gates.md"))];

/// Print the core Guide, or one of its Topics, on stdout.
///
/// A Topic that does not exist is an error rather than a fallback to the core:
/// the agent asked for required reading, and quietly handing it something else
/// would have it write the very thing the Topic exists to get right.
pub fn guide(topic: Option<&str>) -> Result<()> {
    let text = match topic {
        None => CORE,
        Some(name) => *TOPICS
            .iter()
            .find_map(|(topic, text)| (*topic == name).then_some(text))
            .ok_or_else(|| {
                anyhow!(
                    "no Guide topic named {name:?}. The Topics are: {}",
                    topics()
                )
            })?,
    };

    let mut stdout = std::io::stdout().lock();
    stdout
        .write_all(text.as_bytes())
        .and_then(|()| stdout.flush())
        .context("writing the Guide to stdout")
}

/// The Topic names, for telling an agent what it could have asked for.
fn topics() -> String {
    TOPICS
        .iter()
        .map(|(topic, _)| *topic)
        .collect::<Vec<_>>()
        .join(", ")
}
