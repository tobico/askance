//! The web UI, shared by both halves of the build: compiled natively into the
//! server for SSR, and to wasm by `askance-frontend` for hydration.
//!
//! Server functions live here because both halves need them — the server needs
//! the body, the browser needs the stub that calls it. That is why the store is
//! its own crate: this crate reaches down to it under `ssr`, and the server
//! binary reaches up to link this crate.

use leptos::prelude::*;
use leptos_meta::{MetaTags, Title, provide_meta_context};
use leptos_router::components::{Route, Router, Routes};
use leptos_router::{ParamSegment, SsrMode, StaticSegment};

pub mod archive;
pub mod device;
pub mod pending;
pub mod push;
pub mod set_view;
pub mod switch;

/// The HTML document the server sends and the browser hydrates. Phone-first:
/// the viewport tag is the one thing a responsive layout cannot do without.
///
/// The manifest and the icons are static files from the site root, not from
/// `/pkg/` — see the workspace's `assets-dir`.
///
/// Server-only: this is the document written *around* what the browser hydrates,
/// and the wasm half mounts `App` into the body it finds already there.
#[cfg(feature = "ssr")]
pub fn shell(options: LeptosOptions) -> impl IntoView {
    // Named here rather than by a `Stylesheet` inside `App`, because under
    // `hash-files` the name holds a hash that only the server can read — see
    // `stylesheet`. Nothing in the browser touches this link, and a plain one in
    // the head needs nothing to: `cargo leptos watch` swaps the stylesheet by
    // matching the href it just wrote, hashed or not, and the id is only the
    // conventional handle on it.
    let stylesheet = stylesheet(&options);

    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
                <link rel="manifest" href="/manifest.webmanifest" />
                <meta name="theme-color" content="#7a5c3e" />
                <link rel="icon" href="/icons/askance.svg" type="image/svg+xml" />
                // iOS reads neither the manifest's icons nor an SVG favicon, so
                // it gets its own link, to a PNG of the size it wants.
                <link rel="apple-touch-icon" href="/icons/apple-touch-icon.png" />
                <link rel="stylesheet" id="leptos" href=stylesheet />
                <AutoReload options=options.clone() />
                <HydrationScripts options />
                <MetaTags />
            </head>
            <body>
                <App />
            </body>
        </html>
    }
}

/// Where this build's stylesheet is, under whatever name the build gave it.
///
/// `HydrationScripts` reads the same `hash.txt` to name the wasm and its loader,
/// but keeps what it read to itself, so the stylesheet's line is read again here.
/// The file sits beside the binary rather than in the site root — that is where
/// Leptos looks for it, and the two have to look in the same place.
///
/// Read per render, as Leptos reads it for the wasm: it is three lines, and a
/// build cannot change under a server that is already running anyway.
///
/// Without `hash-files` there is no hash and no file to read, which is the
/// standing under a bare `cargo build` and in the tests.
#[cfg(feature = "ssr")]
fn stylesheet(options: &LeptosOptions) -> String {
    let pkg = &options.site_pkg_dir;
    let name = &options.output_name;

    match hashed(options, "css") {
        Some(hash) => format!("/{pkg}/{name}.{hash}.css"),
        None => format!("/{pkg}/{name}.css"),
    }
}

/// The hash this build gave one kind of bundle, from the `kind: hash` lines
/// cargo-leptos writes.
///
/// An unreadable hash file is worth no panic: the page then names the unhashed
/// stylesheet, which is a page that has lost its styling and not one that failed
/// to load. Leptos logs the same case for the wasm.
#[cfg(feature = "ssr")]
fn hashed(options: &LeptosOptions, kind: &str) -> Option<String> {
    if !options.hash_files {
        return None;
    }

    // `join` on an absolute `hash_file` takes it whole, which is how a packaged
    // server points at a hash file kept outside its `bin` directory.
    let path = std::env::current_exe()
        .ok()?
        .parent()?
        .join(options.hash_file.as_ref());

    let hashes = std::fs::read_to_string(&path)
        .inspect_err(|err| {
            leptos::logging::error!("could not read {}: {err}", path.display());
        })
        .ok()?;

    hashes.lines().find_map(|line| {
        let (file, hash) = line.split_once(':')?;
        (file.trim() == kind).then(|| hash.trim().to_owned())
    })
}

/// The application: everything under the API routes the agents use.
#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    view! {
        // The stylesheet is the shell's, not this component's: its name carries
        // this build's hash, and only the server can read which.
        <Title text="Askance" />

        <Router>
            <main>
                <Routes fallback=|| view! { <p class="empty">"No such page."</p> }>
                    // Rendered whole rather than streamed: the pending list is
                    // the entire page, so there is nothing to show first, and
                    // a local SQLite query is not worth a loading flash. The
                    // set view is the same story, and its Preface has to be
                    // rendered before any of the page can go out.
                    <Route
                        path=StaticSegment("")
                        view=pending::PendingList
                        ssr=SsrMode::Async
                    />
                    <Route
                        path=StaticSegment("archive")
                        view=archive::Archive
                        ssr=SsrMode::Async
                    />
                    <Route
                        path=(StaticSegment("sets"), ParamSegment("id"))
                        view=set_view::SetPage
                        ssr=SsrMode::Async
                    />
                </Routes>
            </main>
        </Router>
    }
}
