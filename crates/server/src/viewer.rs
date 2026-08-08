//! The viewer, served out of the binary: the SolidJS SPA under `web/`, built by
//! vite and embedded here (ADR-0003).
//!
//! There is no site directory to point the server at and nothing to install
//! beside it — one file is the whole deployment, which is what the tailnet box
//! and the NixOS module both wanted. In a release build the files are compiled
//! in; in a debug build rust-embed reads them off disk on every request, so
//! `pnpm build` is visible to a running `cargo run -p askance-cli -- serve`
//! without a recompile.
//!
//! Two decisions live here. The first is the fallback: the viewer routes on the
//! client, so `/sets/12` names no file and has to be answered with the document
//! that carries the router — but a *file* that is missing stays a 404, because a
//! bundle answered with HTML dies as a syntax error at the top of the page
//! instead of reporting a stale URL. The second is how long an answer may be
//! reused, which is [`KEEP`] and [`ASK`] below.

use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE};
use axum::http::{StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use rust_embed::Embed;

/// The viewer as `pnpm build` leaves it. `allow_missing` so that a checkout which
/// has never built it still compiles: the server then serves no viewer and says
/// so, rather than refusing to build the agent API along with it.
#[derive(Embed)]
#[folder = "$CARGO_MANIFEST_DIR/../../web/dist"]
#[allow_missing = true]
pub(crate) struct Built;

/// The document every path the viewer routes on is answered with.
const DOCUMENT: &str = "index.html";

/// Where vite writes what it names by content — pinned in `web/vite.config.ts`
/// to the same word, because [`KEEP`] is granted on nothing but the hashing.
const HASHED: &str = "/assets/";

/// How long a file named by its content may be kept: a year, which is as long as
/// the specification lets anyone ask for, and unconditionally, since a hashed
/// name is never reused.
const KEEP: &str = "public, max-age=31536000, immutable";

/// What everything else gets: kept, but never used without asking the server
/// first. The document has to be revalidated because it *names* the hashed
/// bundles — a stale copy asks for a previous build's, which this build does not
/// have. The service worker, the manifest and the icons keep the names the phone
/// knows them by and so cannot be kept either.
const ASK: &str = "no-cache";

/// Answer everything the agents' API has not claimed.
///
/// Mounted as the fallback, so `/api/v1/` and `/api/ui/` keep their exact paths
/// and only what nothing there answers to arrives here.
pub(crate) async fn serve<V: Embed>(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');

    // Anything under the API's own namespaces reaching this far is a path neither
    // half answers to. An agent that mistyped an endpoint has to be told so: a
    // 200 of HTML would reach `askance ask` as a Response it could not parse.
    if path == "api" || path.starts_with("api/") {
        return StatusCode::NOT_FOUND.into_response();
    }

    if let Some(file) = V::get(path) {
        return served(file, path);
    }

    // A path whose last segment carries an extension was asking for a file, and
    // the file is not there. Everything else is a route the viewer draws.
    if names_a_file(path) {
        return StatusCode::NOT_FOUND.into_response();
    }

    match V::get(DOCUMENT) {
        Some(document) => served(document, DOCUMENT),
        // Only reachable in a checkout where the viewer was never built — see
        // `allow_missing` above. The agent API is up regardless, which is what
        // makes this worth answering rather than refusing to start over.
        None => (
            StatusCode::SERVICE_UNAVAILABLE,
            "the viewer was not built into this binary: run `pnpm build` in web/\n",
        )
            .into_response(),
    }
}

/// Whether `path` was asking for a file rather than naming a route. A dot in the
/// last segment is the whole test: every file the build writes has an extension,
/// and no route the viewer draws does.
fn names_a_file(path: &str) -> bool {
    path.rsplit('/')
        .next()
        .is_some_and(|last| last.contains('.'))
}

/// One embedded file, with what may be done with it.
fn served(file: rust_embed::EmbeddedFile, path: &str) -> Response {
    let policy = if format!("/{path}").starts_with(HASHED) {
        KEEP
    } else {
        ASK
    };

    (
        [
            (CONTENT_TYPE, file.metadata.mimetype()),
            (CACHE_CONTROL, policy),
        ],
        file.data,
    )
        .into_response()
}
