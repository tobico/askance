//! The web UI, shared by both halves of the build: compiled natively into the
//! server for SSR, and to wasm by `askance-frontend` for hydration.
//!
//! Server functions live here because both halves need them — the server needs
//! the body, the browser needs the stub that calls it. That is why the store is
//! its own crate: this crate reaches down to it under `ssr`, and the server
//! binary reaches up to link this crate.

use leptos::prelude::*;
use leptos_meta::{MetaTags, Stylesheet, Title, provide_meta_context};
use leptos_router::components::{Route, Router, Routes};
use leptos_router::{ParamSegment, SsrMode, StaticSegment};

pub mod archive;
pub mod pending;
pub mod push;
pub mod set_view;

// The agent's markdown and the Diff are rendered before they leave the server,
// so neither parser belongs to the browser half.
#[cfg(feature = "ssr")]
mod diff;
#[cfg(feature = "ssr")]
mod markdown;

/// The HTML document the server sends and the browser hydrates. Phone-first:
/// the viewport tag is the one thing a responsive layout cannot do without.
///
/// The manifest and the icons are static files from the site root, not from
/// `/pkg/` — see the workspace's `assets-dir`.
pub fn shell(options: LeptosOptions) -> impl IntoView {
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

/// The application: everything under the API routes the agents use.
#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    view! {
        <Stylesheet id="leptos" href="/pkg/askance.css" />
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
