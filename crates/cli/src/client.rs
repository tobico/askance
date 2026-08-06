//! The CLI's end of the API: submit a Set, then hold a reconnecting long-poll
//! until the Response lands.
//!
//! There is no expiry on the waiting (ADR-0001). The client owns retry, so
//! "nothing yet", a dropped connection, a refused connection and a server
//! restart are all the same thing here: go back and open another wait. Only
//! delivery, a refusal the server will keep repeating — including the Set having
//! been archived unanswered — or a kill ends it.

use std::time::Duration;

use anyhow::{Context, Result, anyhow, bail};
use askance_schema::{ApiError, QuestionSet, Response, SetCreated};

/// How long the server is asked to hold each wait open. Its own ceiling is a
/// minute; well under that leaves room for a reply to make it back before any
/// intermediary decides the connection is idle.
const HOLD: Duration = Duration::from_secs(30);

/// How long a single request may take before the client gives up on it and
/// opens another. Comfortably longer than [`HOLD`], so an ordinary "nothing
/// yet" is never mistaken for a dead connection.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);

/// How long to wait before the first retry, and the ceiling the wait doubles
/// up to. Short: the human may be answering right now.
const FIRST_BACKOFF: Duration = Duration::from_secs(1);
const MAX_BACKOFF: Duration = Duration::from_secs(10);

pub struct Client {
    agent: ureq::Agent,
    base: String,
}

impl Client {
    /// A client pointed at an Askance server, e.g. `http://127.0.0.1:8422`.
    pub fn new(base: &str) -> Self {
        let config = ureq::Agent::config_builder()
            .timeout_global(Some(REQUEST_TIMEOUT))
            // Statuses are answers, not errors: a 204 means "nothing yet" and
            // a 422 carries the violations worth printing.
            .http_status_as_error(false)
            .build();

        Client {
            agent: config.into(),
            base: base.trim_end_matches('/').to_owned(),
        }
    }

    /// Submit a Set and take back the id the wait is held on.
    ///
    /// This one does not retry. A Set is not idempotent — a retry could leave
    /// the human with the same questions twice — and an agent that cannot reach
    /// the server is better off being told so now than blocking forever on a
    /// Set that was never accepted.
    pub fn submit(&self, set: &QuestionSet) -> Result<SetCreated> {
        let body = set
            .to_yaml()
            .context("rendering the Question Set as YAML")?;

        let mut reply = self
            .agent
            .post(format!("{}/api/v1/sets", self.base))
            .header("Content-Type", "application/yaml")
            .send(&body)
            .with_context(|| format!("submitting the Question Set to {}", self.base))?;

        let status = reply.status().as_u16();
        let text = reply
            .body_mut()
            .read_to_string()
            .context("reading the server's reply")?;

        if status != 201 {
            bail!("the server refused the Question Set: {}", refusal(&text));
        }

        serde_saphyr::from_str(&text)
            .with_context(|| format!("the server's reply was not a stored Set: {text}"))
    }

    /// Block until Set `id` has been answered, reconnecting for as long as it
    /// takes. Retries are reported on stderr, so stdout carries the Response
    /// and nothing else — and as a YAML comment, so that a harness merging the
    /// two streams into one file still hands its agent something that parses.
    pub fn wait(&self, id: i64) -> Result<Response> {
        let mut backoff = FIRST_BACKOFF;

        loop {
            match self.poll(id) {
                Ok(Some(response)) => return Ok(response),
                // Nothing yet: the hold window simply closed. Straight back in.
                Ok(None) => backoff = FIRST_BACKOFF,
                Err(Interrupted::Fatal(error)) => return Err(error),
                Err(Interrupted::Transient(error)) => {
                    eprintln!(
                        "# askance: {error:#}; retrying in {}s",
                        backoff.as_secs().max(1)
                    );
                    std::thread::sleep(backoff);
                    backoff = (backoff * 2).min(MAX_BACKOFF);
                }
            }
        }
    }

    /// One wait: the Response, `None` for "nothing yet", or a reason to retry.
    fn poll(&self, id: i64) -> Result<Option<Response>, Interrupted> {
        let hold = HOLD.as_secs();
        let url = format!("{}/api/v1/sets/{id}/response?hold={hold}", self.base);

        let mut reply =
            self.agent.get(&url).call().map_err(|error| {
                Interrupted::Transient(anyhow!("the wait was cut short: {error}"))
            })?;

        match reply.status().as_u16() {
            200 => {
                let text = reply.body_mut().read_to_string().map_err(|error| {
                    Interrupted::Transient(anyhow!("reading the Response: {error}"))
                })?;
                Response::from_yaml(&text)
                    .map(Some)
                    // A reply that is not a Response will not become one on the
                    // next attempt.
                    .map_err(|error| {
                        Interrupted::Fatal(anyhow!(
                            "the server sent something that is not a Response: {error}\n{text}"
                        ))
                    })
            }
            204 => Ok(None),
            404 => Err(Interrupted::Fatal(anyhow!(
                "the server has no Question Set {id} — it may be running against \
                 a different database"
            ))),
            // The human closed the Set without answering it, which is a thing
            // only they can do and is not undone. Retrying would be waiting on a
            // Set nobody is ever going to answer.
            410 => Err(Interrupted::Fatal(anyhow!(
                "Question Set {id} was archived unanswered — the human closed it \
                 without answering, so no Response is coming"
            ))),
            status => {
                let text = reply.body_mut().read_to_string().unwrap_or_default();
                Err(Interrupted::Transient(anyhow!(
                    "the server answered {status}: {}",
                    refusal(&text)
                )))
            }
        }
    }
}

/// Why a wait ended early: because it is worth trying again, or because it
/// never will be.
enum Interrupted {
    Transient(anyhow::Error),
    Fatal(anyhow::Error),
}

/// What the server said when it refused, unpacked from its YAML envelope if it
/// sent one. Violations are listed one per line, each naming its Question.
fn refusal(body: &str) -> String {
    let Ok(error) = serde_saphyr::from_str::<ApiError>(body) else {
        return body.trim().to_owned();
    };

    let mut refusal = error.error;
    for violation in &error.violations {
        refusal.push_str("\n  ");
        refusal.push_str(&violation.to_string());
    }
    refusal
}
