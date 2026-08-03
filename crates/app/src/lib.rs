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

// The Preface's markdown is rendered before it leaves the server, so the parser
// belongs to the server half only.
#[cfg(feature = "ssr")]
mod markdown;
pub mod pending;
pub mod set_view;

/// The HTML document the server sends and the browser hydrates. Phone-first:
/// the viewport tag is the one thing a responsive layout cannot do without.
pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
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
                        path=(StaticSegment("sets"), ParamSegment("id"))
                        view=set_view::SetPage
                        ssr=SsrMode::Async
                    />
                </Routes>
            </main>
        </Router>
    }
}
