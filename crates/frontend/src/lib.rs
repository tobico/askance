//! The browser's entry point: take over the HTML the server already rendered.
//!
//! wasm-bindgen exports `hydrate` and the snippet Leptos puts in the document
//! head calls it once the module loads.

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    // Without this a panic in wasm is a bare "unreachable executed".
    console_error_panic_hook::set_once();

    leptos::mount::hydrate_body(askance_app::App);
}
