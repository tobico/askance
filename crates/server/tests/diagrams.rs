//! The two files a page with a Diagram on it names: the vendored mermaid bundle
//! and the script of ours that drives it.
//!
//! Neither can be exercised without a browser, so what is checked here is what a
//! browser would go looking for — that both are served from the site root under
//! the names the page uses, that the committed bundle is the version
//! `tools/update-mermaid.sh` pins, and that the terms the carve-out was granted
//! on (ADR-0002) are still written into the script that renders: mermaid's strict
//! security level, and no pass of mermaid's own replacing a diagram that will not
//! draw with a graphic saying so.
//!
//! Which pages name them at all is `set_page.rs`'s business.

use std::fs;
use std::path::{Path, PathBuf};

use askance_server::{open_database, router_with_ui};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use leptos::prelude::LeptosOptions;
use tower::ServiceExt;

/// The workspace root, from the crate this test is compiled into.
fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

/// The directory `cargo leptos` copies into the site root, which is therefore
/// what the site root looks like as far as these files are concerned.
fn assets() -> PathBuf {
    workspace_root().join("assets")
}

fn options() -> LeptosOptions {
    LeptosOptions::builder()
        .output_name("askance")
        .site_root(assets().to_str().unwrap().to_owned())
        .build()
}

/// Ask the running server for a path, as a browser reading the page's head would.
async fn get(path: &str) -> axum::http::Response<Body> {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("askance.db")).await.unwrap();

    router_with_ui(pool, options())
        .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
        .await
        .unwrap()
}

/// A file served from the site root, as JavaScript and not empty.
async fn served(path: &str) -> Vec<u8> {
    let response = get(path).await;

    assert_eq!(response.status(), StatusCode::OK, "asking for {path}");

    let served_as = response
        .headers()
        .get(axum::http::header::CONTENT_TYPE)
        .expect("a served file should carry a content type")
        .to_str()
        .unwrap()
        .to_owned();
    assert!(
        served_as.contains("javascript"),
        "{path} is served as {served_as}",
    );

    let body = response.into_body().collect().await.unwrap().to_bytes();
    assert!(!body.is_empty(), "{path} came back empty");

    body.to_vec()
}

/// The version `tools/update-mermaid.sh` pins, which is what the committed
/// bundle has to be.
fn pinned_version() -> String {
    let script = fs::read_to_string(workspace_root().join("tools/update-mermaid.sh")).unwrap();

    script
        .lines()
        .find_map(|line| line.trim().strip_prefix("VERSION="))
        .expect("the update script should pin a version to fetch")
        .trim()
        .to_owned()
}

#[tokio::test]
async fn the_bundle_and_the_script_are_served_from_the_root() {
    // The paths the set view names — see `diagram_renderer`. They are in the
    // assets directory rather than under `/pkg/` because nothing in the Leptos
    // build knows about them.
    served("/mermaid.min.js").await;
    served("/diagrams.js").await;
}

#[test]
fn the_committed_bundle_is_the_version_the_update_script_pins() {
    let version = pinned_version();
    let bundle = fs::read_to_string(assets().join("mermaid.min.js")).unwrap();

    assert!(
        bundle.contains(&format!("version:\"{version}\"")),
        "assets/mermaid.min.js is not mermaid {version}; run tools/update-mermaid.sh",
    );
}

#[test]
fn the_renderer_draws_at_mermaid_s_strict_security_level() {
    let script = fs::read_to_string(assets().join("diagrams.js")).unwrap();

    // Every diagram on a page was written by an agent, so the source is
    // untrusted: strict is what has mermaid sanitize the labels it draws and
    // refuse the click handlers a diagram can ask for.
    assert!(
        script.contains(r#"securityLevel: "strict""#),
        "the renderer should initialize mermaid at its strict security level",
    );

    // Mermaid's own pass over the page would replace an unparseable diagram with
    // a graphic saying so. The source block is the error state, so the deciding
    // has to stay with the script that leaves it alone.
    assert!(
        script.contains("startOnLoad: false"),
        "the renderer should turn mermaid's own load-time pass off",
    );
}

#[test]
fn the_renderer_lets_mermaid_draw_nothing_for_a_diagram_that_will_not_draw() {
    let script = fs::read_to_string(assets().join("diagrams.js")).unwrap();

    // Asked for a diagram it cannot parse, mermaid draws a bomb and the words
    // "Syntax error in text" — and it draws them into the document before it
    // reports the failure, so the page ends up carrying the graphic whatever the
    // caller then does about the source block. This is the option that has it
    // report the failure and draw nothing, which is what leaves the fallback
    // silent.
    assert!(
        script.contains("suppressErrorRendering: true"),
        "the renderer should stop mermaid drawing its own error graphic",
    );
}
