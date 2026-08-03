//! The browser's entry point: take over the HTML the server already rendered.
//!
//! wasm-bindgen exports `hydrate` and the snippet Leptos puts in the document
//! head calls it once the module loads.

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    // Without this a panic in wasm is a bare "unreachable executed".
    console_error_panic_hook::set_once();

    register_service_worker();

    leptos::mount::hydrate_body(askance_app::App);
}

/// Install the service worker, which is what makes Askance installable — and,
/// from here on, what a push notification will be delivered to.
///
/// It is registered from `/sw.js` so that its scope is the whole site: a worker
/// only controls the paths beneath the one it was served from, and one under
/// `/pkg/` could never show a notification for `/sets/12`.
#[cfg(feature = "hydrate")]
fn register_service_worker() {
    let Some(window) = web_sys::window() else {
        return;
    };

    // `navigator.serviceWorker` is absent in a browser that has none, and in any
    // browser outside a secure context — which is what `tailscale serve` is for.
    // A browser without it loses the install and nothing else, so this is worth
    // no more than a look and a shrug.
    let container = window.navigator().service_worker();
    if container.is_undefined() {
        return;
    }

    // The promise is left to settle on its own: nothing on the page waits on the
    // worker, and a browser that refuses the registration reports it to the
    // console in more detail than this could.
    let _ = container.register("/sw.js");
}
