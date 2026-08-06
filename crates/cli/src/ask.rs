//! `askance ask`: read a Question Set, check it, enrich it, send it, and wait.

use std::io::{Read, Write};
use std::path::Path;

use anyhow::{Context, Result, anyhow, bail};
use askance_schema::QuestionSet;

use crate::client::Client;
use crate::repo;

/// Put a Question Set to the human and block until it is answered.
///
/// The Set is refused here, before anything is sent, if it breaks the question
/// grammar — the agent gets the same violations the server would have given it,
/// only sooner and naming the Question at fault.
pub fn ask(file: Option<&Path>, server: &str) -> Result<()> {
    let yaml = read(file)?;

    let mut set = QuestionSet::from_yaml(&yaml)
        .map_err(|error| anyhow!("the Question Set is not well-formed YAML: {error}"))?;

    set.validate().map_err(|invalid| {
        let listed: Vec<String> = invalid
            .violations
            .iter()
            .map(|violation| format!("  {violation}"))
            .collect();
        anyhow!(
            "the Question Set breaks the question grammar:\n{}",
            listed.join("\n")
        )
    })?;

    // Whatever the agent put in these, the working directory is the authority
    // (ADR-0001), so they are overwritten rather than trusted.
    let cwd = std::env::current_dir().context("reading the working directory")?;
    let derived = repo::enrichment(&cwd);
    set.project = derived.project;
    set.branch = derived.branch;
    set.diff = derived.diff;

    // A wait that goes to plan is silent. A harness runs this in the background
    // and captures both streams into one file, so anything said here on the way
    // would arrive ahead of the Response the agent came for.
    let client = Client::new(server);
    let created = client.submit(&set)?;
    let response = client.wait(created.id)?;

    // The Response is the CLI's whole output: the agent parses stdout, so
    // nothing else has ever been written there.
    let mut yaml = response
        .to_yaml()
        .context("rendering the Response as YAML")?;
    if !yaml.ends_with('\n') {
        yaml.push('\n');
    }

    let mut stdout = std::io::stdout().lock();
    stdout
        .write_all(yaml.as_bytes())
        .and_then(|()| stdout.flush())
        .context("writing the Response to stdout")
}

/// The Set as the agent gave it: from a file, or from stdin when there is no
/// file argument.
fn read(file: Option<&Path>) -> Result<String> {
    let yaml = match file {
        Some(path) => std::fs::read_to_string(path)
            .with_context(|| format!("reading the Question Set from {}", path.display()))?,
        None => {
            let mut yaml = String::new();
            std::io::stdin()
                .read_to_string(&mut yaml)
                .context("reading the Question Set from stdin")?;
            yaml
        }
    };

    if yaml.trim().is_empty() {
        match file {
            Some(path) => bail!("{} is empty", path.display()),
            None => bail!("no Question Set arrived on stdin"),
        }
    }

    Ok(yaml)
}
