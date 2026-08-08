//! Learning that a newer Askance has been released, and saying so on the
//! viewer's namespace.
//!
//! Against a GitHub stood up in-process rather than against a mock: what is
//! worth proving is that a request goes out at all, that the tag it comes back
//! with is read the way a human would read it, and that the ways GitHub can fail
//! to answer cost a notice rather than the server. A latest-release API only
//! ever tells us a tag or a status code, so a fake that hands over both is the
//! whole of the contract.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use askance_render::UpdateNotice;
use askance_server::{Config, open_database, router_checking_updates};
use axum::Router;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{Request, StatusCode};
use axum::routing::get;
use axum::{Json, response::IntoResponse};
use clap::Parser;
use http_body_util::BodyExt;
use tower::ServiceExt;

/// The release this test binary was built as, which is the version the server
/// under test is comparing tags against.
const RUNNING: &str = env!("CARGO_PKG_VERSION");

/// A tag no Askance will ever carry, so a test that expects an update is never
/// one release away from expecting nothing.
const FAR_AHEAD: &str = "v99.0.0";

/// How long a test will wait for the startup poll to come back: generous,
/// because it is only ever paid when the assertion is about to fail.
const PATIENCE: Duration = Duration::from_secs(5);

/// How long to keep watching after the poll has been answered, to catch a
/// request that should not have been made at all.
const SETTLING: Duration = Duration::from_millis(300);

/// Every request one of these fake GitHubs has taken.
type Asked = Arc<Mutex<Vec<String>>>;

/// A GitHub on a loopback port, and what it has been asked for.
///
/// The path says what it should answer, which is how one of them stands in for
/// every case: [`GitHub::releasing`] is a repository whose latest release is
/// tagged so, and [`GitHub::nothing_released`] is one that has never released
/// anything — which is what `releases/latest` 404s for.
struct GitHub {
    address: String,
    asked: Asked,
}

impl GitHub {
    /// Where to ask about a repository whose latest release is tagged `tag`.
    fn releasing(&self, tag: &str) -> String {
        format!("{}/releases/{tag}", self.address)
    }

    /// Where to ask about one with no release to be latest.
    fn nothing_released(&self) -> String {
        format!("{}/nothing", self.address)
    }

    fn times_asked(&self) -> usize {
        self.asked.lock().unwrap().len()
    }

    /// Wait until it has been asked at all. Nothing sends that request but the
    /// server starting, so this is what says the poll happens at startup rather
    /// than a day into the run.
    async fn is_asked(&self) {
        let deadline = tokio::time::Instant::now() + PATIENCE;

        while self.times_asked() == 0 {
            assert!(
                tokio::time::Instant::now() < deadline,
                "the server never asked about the latest release",
            );

            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    }
}

async fn github() -> GitHub {
    let asked: Asked = Arc::new(Mutex::new(Vec::new()));

    let app = Router::new()
        .route("/releases/{tag}", get(latest_release))
        .route("/nothing", get(no_release))
        .with_state(asked.clone());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

    GitHub {
        address: format!("http://{address}"),
        asked,
    }
}

/// The one field of a release that is any of the server's business, in the shape
/// GitHub sends it.
async fn latest_release(State(asked): State<Asked>, Path(tag): Path<String>) -> impl IntoResponse {
    asked.lock().unwrap().push(tag.clone());

    Json(serde_json::json!({ "tag_name": tag }))
}

/// How GitHub answers for a repository with no release yet.
async fn no_release(State(asked): State<Asked>) -> impl IntoResponse {
    asked.lock().unwrap().push("nothing".to_owned());

    (
        StatusCode::NOT_FOUND,
        Json(serde_json::json!({ "message": "Not Found" })),
    )
}

/// An address nothing is listening on: bound to claim a free port, then dropped.
async fn nowhere() -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    drop(listener);
    format!("http://{address}")
}

/// A server over a fresh database, checking for updates wherever it is pointed —
/// or not checking at all, where it is pointed nowhere.
async fn fresh_app(releases: Option<&str>) -> (tempfile::TempDir, Router) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("askance.db")).await.unwrap();

    (dir, router_checking_updates(pool, releases))
}

/// What the viewer would be told, asked for the way the viewer asks.
async fn notice(app: &Router) -> UpdateNotice {
    let http = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/ui/update")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let status = http.status();
    let body = http.into_body().collect().await.unwrap().to_bytes();
    let body = String::from_utf8(body.to_vec()).unwrap();

    assert_eq!(status, StatusCode::OK, "GET /api/ui/update failed: {body}");

    serde_json::from_str(&body).unwrap_or_else(|err| panic!("reading {body:?}: {err}"))
}

/// The same, once the poll it depends on has been answered and anything behind
/// that answer has had time to land.
async fn settled_notice(github: &GitHub, app: &Router) -> UpdateNotice {
    github.is_asked().await;
    settle().await;

    notice(app).await
}

/// Give anything still on its way time to arrive, for an assertion about a
/// request that should never be sent.
async fn settle() {
    tokio::time::sleep(SETTLING).await;
}

/// Whether the server is still answering at all, for the cases where the update
/// check has just failed.
async fn healthy(app: &Router) -> bool {
    let http = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    http.status() == StatusCode::OK
}

#[tokio::test]
async fn a_newer_release_is_named_to_the_viewer() {
    let github = github().await;
    let (_dir, app) = fresh_app(Some(&github.releasing(FAR_AHEAD))).await;

    assert_eq!(
        settled_notice(&github, &app).await,
        UpdateNotice::Available {
            version: "99.0.0".to_owned(),
        },
        "the banner has to be able to name the version that is waiting",
    );
}

#[tokio::test]
async fn the_release_this_is_and_an_older_one_are_nothing_to_update_to() {
    for tag in [format!("v{RUNNING}"), "v0.0.1".to_owned()] {
        let github = github().await;
        let (_dir, app) = fresh_app(Some(&github.releasing(&tag))).await;

        assert_eq!(
            settled_notice(&github, &app).await,
            UpdateNotice::Current,
            "the latest release being {tag} is nothing to tell the human about",
        );
    }
}

#[tokio::test]
async fn a_latest_release_that_cannot_be_read_is_no_news() {
    // No release at all, which is what `releases/latest` 404s for — and a tag
    // that is not a version, which is as much use as no tag. One GitHub each,
    // so each case is genuinely waited on rather than riding the last one's
    // request.
    for release in ["nothing", "the-first-one"] {
        let github = github().await;
        let releases = match release {
            "nothing" => github.nothing_released(),
            tag => github.releasing(tag),
        };

        let (_dir, app) = fresh_app(Some(&releases)).await;

        assert_eq!(settled_notice(&github, &app).await, UpdateNotice::Current);
        assert!(healthy(&app).await, "and the server is still serving");
    }
}

#[tokio::test]
async fn a_github_that_cannot_be_reached_is_no_news_either() {
    let unreachable = nowhere().await;
    let (_dir, app) = fresh_app(Some(&unreachable)).await;

    settle().await;

    assert_eq!(notice(&app).await, UpdateNotice::Current);
    assert!(
        healthy(&app).await,
        "a GitHub that is not there costs a notice, not the server",
    );
}

#[tokio::test]
async fn the_poll_happens_at_startup_rather_than_a_day_later() {
    let github = github().await;
    let (_dir, _app) = fresh_app(Some(&github.releasing(FAR_AHEAD))).await;

    // Nothing but starting has happened, and the daily cycle is a day away.
    github.is_asked().await;
}

#[tokio::test]
async fn a_server_told_not_to_check_asks_nobody() {
    let github = github().await;
    let releases = github.releasing(FAR_AHEAD);

    // The control: pointed at this GitHub, a server asks it and comes back with
    // the verdict — so the count below moving is a real thing to watch.
    let (_checking_dir, checking) = fresh_app(Some(&releases)).await;
    assert!(matches!(
        settled_notice(&github, &checking).await,
        UpdateNotice::Available { .. },
    ));
    assert_eq!(github.times_asked(), 1);

    // Turned off, there is nowhere to ask — with a GitHub standing right there.
    let off = Config::parse_from(["askance serve", "--no-update-check"]);
    assert_eq!(off.releases(), None, "turned off leaves nowhere to ask");

    let (_quiet_dir, quiet) = fresh_app(off.releases()).await;
    settle().await;

    assert_eq!(
        github.times_asked(),
        1,
        "a server told not to check makes no request at all",
    );
    assert_eq!(
        notice(&quiet).await,
        UpdateNotice::Current,
        "and the endpoint still answers, saying there is nothing to update to",
    );
}
