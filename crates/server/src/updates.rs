//! Learning that a newer Askance has been released.
//!
//! One unauthenticated request to GitHub's latest-release API, at startup and
//! once a day after — server-side rather than from the browser, because a
//! tailnet of devices each asking GitHub would have the whole household phoning
//! home, and because the viewer must not need GitHub to be reachable to draw a
//! page.
//!
//! The verdict is held in memory and nothing is written to the store: it is
//! cheap to work out again, and a server that has just started should ask rather
//! than believe what it concluded a week ago. A poll that fails leaves the last
//! verdict standing and costs nothing — the next cycle asks again.
//!
//! Nothing is ever installed on the human's behalf, and the whole thing can be
//! turned off: with no address to ask, no task is started and no request is ever
//! made.

use std::sync::{Arc, RwLock};
use std::time::Duration;

use anyhow::{Context, Result};
use askance_render::UpdateNotice;
use serde::Deserialize;

/// Where the latest release is asked about.
///
/// `releases/latest` rather than the tag list, because it excludes
/// pre-releases: a rehearsal tag is marked as one and is not something to tell
/// the human to update to. Before the first real release the endpoint answers
/// `404`, which is no news rather than a failure.
pub(crate) const LATEST_RELEASE: &str =
    "https://api.github.com/repos/tobico/askance/releases/latest";

/// How often to ask. A release is not an urgent thing to hear about, and the
/// startup poll covers the server that was restarted just after one.
const DAILY: Duration = Duration::from_secs(24 * 60 * 60);

/// How long to wait on GitHub before giving up. Whatever the answer would have
/// been, it will be asked for again tomorrow.
const REACHABLE_WITHIN: Duration = Duration::from_secs(10);

/// Who is asking. GitHub refuses a caller that will not name itself, and this
/// is the only place Askance has to introduce itself to anyone.
const ASKING: &str = concat!("askance/", env!("CARGO_PKG_VERSION"));

/// The release this binary was built as, which is what a tag is compared
/// against.
const RUNNING: &str = env!("CARGO_PKG_VERSION");

/// What the server has concluded about updating, as the handler reads it.
///
/// Cheap to clone and shared with whatever task is doing the asking: the
/// verdict is one small value replaced whole, so a reader never waits on more
/// than the swap.
#[derive(Clone)]
pub(crate) struct Updates(Arc<RwLock<UpdateNotice>>);

impl Updates {
    /// Nothing learned yet — what a server answers between starting and its
    /// first poll coming back, and what one with the check turned off answers
    /// for as long as it runs. It says there is nothing to update to, which is
    /// the honest thing to say when you do not know.
    pub(crate) fn nothing_learned() -> Self {
        Self(Arc::new(RwLock::new(UpdateNotice::Current)))
    }

    /// The verdict as it stands.
    pub(crate) fn notice(&self) -> UpdateNotice {
        self.0
            .read()
            .expect("the verdict lock is never poisoned")
            .clone()
    }

    fn learned(&self, notice: UpdateNotice) {
        *self.0.write().expect("the verdict lock is never poisoned") = notice;
    }
}

/// Start asking, and hand back the verdict the handler reads.
///
/// `releases` is where to ask — [`LATEST_RELEASE`] in the running server, and a
/// server the test stood up itself under test. `None` is the check turned off:
/// no task, and so no request ever made.
pub(crate) fn watching(releases: Option<&str>) -> Updates {
    let updates = Updates::nothing_learned();

    let Some(releases) = releases else {
        return updates;
    };
    let releases = releases.to_owned();

    let client = match reqwest::Client::builder()
        .timeout(REACHABLE_WITHIN)
        .user_agent(ASKING)
        .build()
    {
        Ok(client) => client,
        // Nothing left to do but say nothing: this is the whole of the check,
        // and it is not something the human asked for by starting the server.
        Err(error) => {
            tracing::warn!(error = ?error, "the update check could not be started");
            return updates;
        }
    };

    let asking = updates.clone();
    tokio::spawn(async move {
        loop {
            match ask(&client, &releases).await {
                Ok(notice) => asking.learned(notice),
                // Left standing rather than cleared. GitHub being out of reach
                // is no reason to stop naming an update already learned of, and
                // it costs nothing: tomorrow's cycle asks again.
                Err(error) => {
                    tracing::debug!(error = ?error, "asking about newer releases failed");
                }
            }

            tokio::time::sleep(DAILY).await;
        }
    });

    updates
}

/// What GitHub answers with, of which one field is any of our business.
#[derive(Deserialize)]
struct Release {
    tag_name: String,
}

/// Ask once, and say what the answer means for this server.
async fn ask(client: &reqwest::Client, releases: &str) -> Result<UpdateNotice> {
    let response = client
        .get(releases)
        .send()
        .await
        .context("reaching GitHub")?;

    // There is no latest release until there is one, and GitHub says so with a
    // 404. Nothing to update to, rather than something to report.
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(UpdateNotice::Current);
    }

    let release: Release = response
        .error_for_status()
        .context("asking GitHub for the latest release")?
        .json()
        .await
        .context("reading what GitHub answered")?;

    Ok(verdict(&release.tag_name))
}

/// What one release tag means for the version this binary was built as.
///
/// A tag that will not parse is no news, same as no release at all: the
/// alternative is telling the human to update to something we cannot name.
fn verdict(tag: &str) -> UpdateNotice {
    let running = semver::Version::parse(RUNNING)
        .expect("the version cargo built this with is a semver version");

    // Releases are tagged `v0.1.0`; the version inside one is not.
    let Ok(released) = semver::Version::parse(tag.trim_start_matches('v')) else {
        tracing::debug!(
            tag,
            "the latest release is not tagged with a version we can read"
        );
        return UpdateNotice::Current;
    };

    if released > running {
        UpdateNotice::Available {
            version: released.to_string(),
        }
    } else {
        UpdateNotice::Current
    }
}

#[cfg(test)]
mod tests {
    use super::{RUNNING, UpdateNotice, verdict};

    #[test]
    fn a_higher_tag_is_an_update_named_without_its_v() {
        assert_eq!(
            verdict("v99.0.0"),
            UpdateNotice::Available {
                version: "99.0.0".to_owned(),
            },
        );
    }

    #[test]
    fn the_running_version_is_nothing_to_update_to() {
        assert_eq!(verdict(&format!("v{RUNNING}")), UpdateNotice::Current);
    }

    #[test]
    fn a_tag_that_will_not_parse_is_no_news() {
        assert_eq!(verdict("the-first-one"), UpdateNotice::Current);
    }
}
