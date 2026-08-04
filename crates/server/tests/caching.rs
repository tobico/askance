//! How long the browser may reuse what it was served.
//!
//! There is one thing to get right here, and it is worth a file of its own: the
//! wasm and the HTML that hydrates it have to come from the same build. A
//! browser running the last build's wasm against this build's markup panics out
//! of hydration part-way and leaves a page that is there but does nothing —
//! which, on a page whose whole purpose is answering a Set, means answers that
//! cannot be given.
//!
//! Neither half of the answer is visible in the served bytes, so both are
//! asserted on the headers: the bundles are named by content and may be kept
//! forever, and everything that *names* them is checked with the server every
//! time.

use std::fs;
use std::path::Path;

use askance_server::{open_database, router_with_ui};
use axum::body::Body;
use axum::http::{Request, StatusCode, header::CACHE_CONTROL};
use leptos::prelude::LeptosOptions;
use tower::ServiceExt;

/// A site root shaped like the one `cargo leptos build` leaves behind: bundles
/// under `pkg/` beside the assets copied into the root. The names under `pkg/`
/// are the hashed ones, since that is what `hash-files` writes and what makes
/// keeping them safe.
fn site(root: &Path) {
    fs::create_dir_all(root.join("pkg")).unwrap();
    fs::write(
        root.join("pkg/askance.Ai7Hs0mEtEsThIsIsAhAsH.wasm"),
        "\0asm",
    )
    .unwrap();
    fs::write(
        root.join("pkg/askance.Ai7Hs0mEtEsThIsIsAhAsH.css"),
        "main {}",
    )
    .unwrap();
    fs::write(root.join("sw.js"), "// worker").unwrap();
}

/// Renders take turns — see the note in `set_page.rs`: two server-side renders
/// at once can deadlock inside leptos's reactive graph (leptos-rs/leptos#4673),
/// and building the router walks the routes.
static ONE_RENDER_AT_A_TIME: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// Ask the running server for a path, as a browser would, and say what it may do
/// with the answer.
async fn policy(path: &str) -> (StatusCode, String) {
    let _turn = ONE_RENDER_AT_A_TIME.lock().await;

    let dir = tempfile::tempdir().unwrap();
    site(dir.path());

    let options = LeptosOptions::builder()
        .output_name("askance")
        .site_root(dir.path().to_str().unwrap().to_owned())
        .build();

    let pool = open_database(&dir.path().join("askance.db")).await.unwrap();

    let response = router_with_ui(pool, options)
        .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
        .await
        .unwrap();

    let said = response
        .headers()
        .get(CACHE_CONTROL)
        .unwrap_or_else(|| panic!("{path} was served without a Cache-Control"))
        .to_str()
        .unwrap()
        .to_owned();

    (response.status(), said)
}

#[tokio::test]
async fn a_hashed_bundle_may_be_kept_for_good() {
    for bundle in [
        "/pkg/askance.Ai7Hs0mEtEsThIsIsAhAsH.wasm",
        "/pkg/askance.Ai7Hs0mEtEsThIsIsAhAsH.css",
    ] {
        let (status, said) = policy(bundle).await;

        assert_eq!(status, StatusCode::OK);
        assert!(
            said.contains("immutable") && said.contains("max-age=31536000"),
            "{bundle} is named by its content, so it should be keepable for good: \
             got `{said}`",
        );
    }
}

#[tokio::test]
async fn a_page_is_never_reused_without_asking() {
    // The page names the hashed bundles. A browser that reuses a stale one asks
    // for the previous build's wasm and hydrates this build's markup with it,
    // which is the failure this whole file is about.
    let (status, said) = policy("/").await;

    assert_eq!(status, StatusCode::OK);
    assert!(
        said.contains("no-cache"),
        "a page names the bundles it hydrates with, so it has to be revalidated: \
         got `{said}`",
    );
}

#[tokio::test]
async fn the_service_worker_is_never_reused_without_asking() {
    // Its name is fixed, so a kept copy is a copy that can never be replaced —
    // and this is the thing that would be holding back a fix to push handling.
    let (status, said) = policy("/sw.js").await;

    assert_eq!(status, StatusCode::OK);
    assert!(
        said.contains("no-cache"),
        "the service worker has a fixed name and so cannot be kept: got `{said}`",
    );
}

#[test]
fn the_build_names_the_bundles_by_content() {
    let manifest = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("Cargo.toml"),
    )
    .unwrap();

    // Keeping a bundle for good is only safe because its name changes when its
    // content does. Without this, every one of those year-long answers above is
    // a promise about a name that the next build reuses.
    assert!(
        manifest.contains("hash-files = true"),
        "the workspace's Leptos metadata should turn on `hash-files`, or the \
         bundles are served under stable names and cannot be kept",
    );
}
