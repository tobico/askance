//! The viewer as the browser gets it: the SPA's own files out of the binary, on
//! every path the agents' API has not claimed.
//!
//! Two things are worth proving here, and neither is visible in the bytes. The
//! first is the fallback: the viewer routes on the client, so `/sets/12` is a
//! path only the browser knows about and the server has to answer it with the
//! document rather than a 404 — while a *file* that is not there stays a 404,
//! because a missing bundle answered with HTML is a syntax error at the top of
//! the page instead of an honest miss. The second is how long each answer may be
//! reused: the bundles are named by content and may be kept for a year, and the
//! document that names them has to be revalidated every time, or a browser
//! reuses a page pointing at a build's bundles that are no longer there.
//!
//! The site served here is `tests/site`, embedded the same way the built one is —
//! so none of this waits on `pnpm build`, and the fixture can name a bundle whose
//! hash a test is allowed to know.

use std::fs;
use std::path::{Path, PathBuf};

use askance_server::{Embed, open_database, router_with_viewer};
use axum::body::Body;
use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE};
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

/// A site shaped like the one vite builds: a document naming its bundles under
/// `assets/` by content, beside the files copied verbatim to the root.
#[derive(Embed)]
#[folder = "$CARGO_MANIFEST_DIR/tests/site"]
struct Site;

/// The workspace root, from the crate this test is compiled into.
fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

/// What vite copies into the site root verbatim — the manifest, the icons and
/// the service worker.
fn assets() -> PathBuf {
    workspace_root().join("assets")
}

/// Ask the server for a path, as a browser would.
async fn get(path: &str) -> axum::http::Response<Body> {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("askance.db")).await.unwrap();

    router_with_viewer::<Site>(pool)
        .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
        .await
        .unwrap()
}

fn header(response: &axum::http::Response<Body>, name: axum::http::HeaderName) -> String {
    response
        .headers()
        .get(&name)
        .unwrap_or_else(|| panic!("a served file should carry a {name}"))
        .to_str()
        .unwrap()
        .to_owned()
}

async fn text(response: axum::http::Response<Body>) -> String {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    String::from_utf8(bytes.to_vec()).unwrap()
}

#[tokio::test]
async fn the_document_is_served_from_the_root() {
    let response = get("/").await;

    assert_eq!(response.status(), StatusCode::OK);
    assert!(
        header(&response, CONTENT_TYPE).starts_with("text/html"),
        "served as {}",
        header(&response, CONTENT_TYPE),
    );
    assert!(text(response).await.contains(r#"<div id="app">"#));
}

#[tokio::test]
async fn every_path_the_viewer_routes_on_is_answered_with_the_document() {
    // None of these exist as files. The router that knows them is the one in the
    // browser, and it cannot run until the document has been served to it.
    for path in ["/archive", "/sets/12", "/sets/12/anything/deeper"] {
        let response = get(path).await;

        assert_eq!(response.status(), StatusCode::OK, "GET {path}");
        assert!(
            text(response).await.contains(r#"<div id="app">"#),
            "{path} should have been answered with the document",
        );
    }
}

#[tokio::test]
async fn a_file_that_is_not_there_is_a_miss_and_not_the_document() {
    // A bundle from a build that has gone is the case this is about: answered
    // with the document, it arrives as HTML where the browser expected a module,
    // and the page dies on a syntax error rather than reporting a stale URL.
    for missing in [
        "/assets/index-GoNeNoW.js",
        "/assets/index-GoNeNoW.css",
        "/icons/no-such-icon.png",
        "/robots.txt",
    ] {
        let response = get(missing).await;

        assert_eq!(response.status(), StatusCode::NOT_FOUND, "GET {missing}");
    }
}

#[tokio::test]
async fn the_agents_namespace_is_never_answered_with_the_document() {
    // An agent that mistypes an endpoint has to be told so. A 200 of HTML would
    // reach `askance ask` as a Response it could not parse.
    for path in ["/api/v1/no-such-endpoint", "/api/ui/no-such-endpoint"] {
        let response = get(path).await;

        assert_eq!(response.status(), StatusCode::NOT_FOUND, "GET {path}");
    }
}

#[tokio::test]
async fn a_bundle_named_by_its_content_may_be_kept_for_good() {
    for bundle in ["/assets/index-TeStHaSh.js", "/assets/index-TeStHaSh.css"] {
        let response = get(bundle).await;

        assert_eq!(response.status(), StatusCode::OK);
        let said = header(&response, CACHE_CONTROL);
        assert!(
            said.contains("immutable") && said.contains("max-age=31536000"),
            "{bundle} is named by its content, so it should be keepable for good: \
             got `{said}`",
        );
    }
}

#[tokio::test]
async fn the_document_is_never_reused_without_asking() {
    // It names the hashed bundles. A browser reusing a stale copy asks for a
    // previous build's bundles, which this build does not have — and the fallback
    // above deliberately will not paper over that with HTML.
    for path in ["/", "/sets/12"] {
        let said = header(&get(path).await, CACHE_CONTROL);

        assert!(
            said.contains("no-cache"),
            "{path} names the bundles it loads, so it has to be revalidated: \
             got `{said}`",
        );
    }
}

#[tokio::test]
async fn what_has_a_fixed_name_is_never_reused_without_asking() {
    // The service worker, the manifest and the icons keep the names the document
    // and the phone know them by, so a kept copy is one that can never be
    // replaced — and for the worker that is a copy holding back a fix to push
    // handling.
    for path in ["/sw.js", "/manifest.webmanifest", "/icons/icon-192.png"] {
        let response = get(path).await;

        assert_eq!(response.status(), StatusCode::OK, "GET {path}");
        let said = header(&response, CACHE_CONTROL);
        assert!(
            said.contains("no-cache"),
            "{path} has a fixed name and so cannot be kept: got `{said}`",
        );
    }
}

#[tokio::test]
async fn the_manifest_and_the_worker_are_served_from_the_root_as_themselves() {
    // A service worker only controls the paths beneath the one it was served
    // from, so one under the bundles' directory could never show a notification
    // for `/sets/12`; and a manifest served as plain text is a manifest the
    // phone will not install from.
    let manifest = get("/manifest.webmanifest").await;
    assert_eq!(manifest.status(), StatusCode::OK);
    assert!(
        header(&manifest, CONTENT_TYPE).starts_with("application/manifest+json"),
        "served as {}",
        header(&manifest, CONTENT_TYPE),
    );

    let worker = get("/sw.js").await;
    assert_eq!(worker.status(), StatusCode::OK);
    assert!(
        header(&worker, CONTENT_TYPE).contains("javascript"),
        "served as {}",
        header(&worker, CONTENT_TYPE),
    );
    assert!(!text(worker).await.is_empty());
}

#[test]
fn the_build_names_the_bundles_by_content_under_the_directory_that_is_kept() {
    let config = fs::read_to_string(workspace_root().join("web/vite.config.ts")).unwrap();

    // Keeping a bundle for a year is only safe because its name changes when its
    // content does, and only the files vite hashes are named that way. The
    // directory it writes them to is therefore half of the policy above, and is
    // pinned on both sides rather than left to a default that could move.
    assert!(
        config.contains(r#"assetsDir: "assets""#),
        "the viewer's build should pin the directory the server keeps for good, \
         or a file under a stable name lands there and is cached forever",
    );
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

    // Every list and every Set is read from live SQLite. A cached copy of one
    // that has since been answered is worse to the human than a failure to load,
    // so the worker is here for push and nothing else.
    for forbidden in ["caches", "respondWith", "CacheStorage"] {
        assert!(
            !worker.contains(forbidden),
            "the service worker mentions `{forbidden}`; it should pass fetches \
             straight through",
        );
    }
}
