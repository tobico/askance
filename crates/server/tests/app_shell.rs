//! The installable shell: the web manifest, the icons and the service worker.
//!
//! What matters about these files is where they are served from. A service
//! worker only controls the paths beneath the one it was served from, so a
//! worker under `/pkg/` could never show a notification for `/sets/12`; the
//! manifest has to be reachable from the document that links it. `cargo leptos`
//! copies the assets directory into the site root, so these tests point the
//! server's site root at that directory and ask for the paths a phone asks for.

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

/// Renders take turns — see the note in `set_page.rs`: two server-side renders
/// at once can deadlock inside leptos's reactive graph (leptos-rs/leptos#4673),
/// and building the router walks the routes.
static ONE_RENDER_AT_A_TIME: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// Ask the running server for a path, as a browser would.
async fn get(path: &str) -> axum::http::Response<Body> {
    let _turn = ONE_RENDER_AT_A_TIME.lock().await;

    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("askance.db")).await.unwrap();

    router_with_ui(pool, options())
        .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
        .await
        .unwrap()
}

fn content_type(response: &axum::http::Response<Body>) -> String {
    response
        .headers()
        .get(axum::http::header::CONTENT_TYPE)
        .expect("a served file should carry a content type")
        .to_str()
        .unwrap()
        .to_owned()
}

#[test]
fn the_build_copies_the_assets_directory_into_the_site_root() {
    let manifest = fs::read_to_string(workspace_root().join("Cargo.toml")).unwrap();

    assert!(
        manifest.contains(r#"assets-dir = "assets""#),
        "the workspace's Leptos metadata should name the assets directory these \
         tests read, or `cargo leptos build` will leave it out of the site root",
    );
}

#[tokio::test]
async fn the_manifest_is_served_from_the_root_as_a_manifest() {
    let response = get("/manifest.webmanifest").await;

    assert_eq!(response.status(), StatusCode::OK);
    assert!(
        content_type(&response).starts_with("application/manifest+json"),
        "served as {}",
        content_type(&response),
    );
}

#[tokio::test]
async fn the_service_worker_is_served_from_the_root_path() {
    let response = get("/sw.js").await;

    assert_eq!(response.status(), StatusCode::OK);
    assert!(
        content_type(&response).contains("javascript"),
        "served as {}",
        content_type(&response),
    );

    let body = response.into_body().collect().await.unwrap().to_bytes();
    assert!(!body.is_empty());
}

#[tokio::test]
async fn every_page_links_the_manifest_and_the_icons() {
    let response = get("/").await;

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let page = String::from_utf8(body.to_vec()).unwrap();

    for link in [
        r#"rel="manifest" href="/manifest.webmanifest""#,
        r#"rel="apple-touch-icon" href="/icons/apple-touch-icon.png""#,
    ] {
        assert!(page.contains(link), "the document head should carry {link}");
    }
}

#[test]
fn the_manifest_asks_to_be_installed_with_icons_that_exist() {
    let manifest: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(assets().join("manifest.webmanifest")).unwrap())
            .expect("the manifest should be JSON");

    assert_eq!(manifest["display"], "standalone");
    assert_eq!(manifest["start_url"], "/");
    assert_eq!(manifest["scope"], "/");
    assert!(manifest["name"].is_string());

    let icons = manifest["icons"].as_array().expect("icons");
    assert!(!icons.is_empty(), "an installable manifest needs an icon");

    // Android's launcher crops to a circle, so at least one icon has to be
    // declared safe to mask.
    assert!(
        icons.iter().any(|icon| {
            icon["purpose"]
                .as_str()
                .is_some_and(|purpose| purpose.split_whitespace().any(|p| p == "maskable"))
        }),
        "one of the icons should be maskable",
    );

    for icon in icons {
        let src = icon["src"].as_str().expect("an icon needs a src");
        let path = src.strip_prefix('/').expect("icon srcs should be absolute");
        assert!(
            assets().join(path).exists(),
            "the manifest names {src}, which is not in the assets directory",
        );
    }
}

#[test]
fn the_service_worker_populates_no_cache_and_serves_nothing_from_one() {
    let worker = fs::read_to_string(assets().join("sw.js")).unwrap();

    // Every page is rendered against live SQLite. A cached copy of a Set that
    // has since been answered is worse to the human than a failure to load, so
    // the worker is here for push and nothing else.
    for forbidden in ["caches", "respondWith", "CacheStorage"] {
        assert!(
            !worker.contains(forbidden),
            "the service worker mentions `{forbidden}`; it should pass fetches \
             straight through",
        );
    }
}
