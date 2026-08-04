//! What the device in front of you remembers: `localStorage`, and the settings
//! kept in it.
//!
//! Deliberately per device and never sent to the server — a phone and a laptop
//! answer the same Set from different places, and neither has any business
//! deciding how the other draws a Diff. Push notifications are per device for
//! the same reason, but the browser is the one that remembers those, so nothing
//! about them is kept here.
//!
//! Storage is a convenience the whole way down: a browser that refuses it costs
//! the human their drafts and their settings and nothing else, so nothing on
//! this path is worth a panic. Every read comes back `None` and every write is
//! dropped.

/// Where the wrap setting lives. Namespaced like the drafts beside it, so
/// everything this app leaves in a browser is legible as its own.
const WRAP: &str = "askance.diff-wrap";

/// Whether Diffs are drawn wrapped on this device.
///
/// Anything but the value [`set_wrapping`] writes reads as off, which is also
/// what an untouched browser and the server both answer: unwrapped is the
/// setting nobody has expressed an opinion about.
pub fn wrapping() -> bool {
    read(WRAP).as_deref() == Some("on")
}

/// Remember how this device wants Diffs drawn.
pub fn set_wrapping(on: bool) {
    if on {
        write(WRAP, "on");
    } else {
        // Removed rather than written as "off": the absence is already the
        // default, and this way turning it off leaves nothing behind.
        forget(WRAP);
    }
}

/// The browser's `localStorage`, or `None` when there is none to be had — a
/// browser that blocks it, or one that has none at all.
#[cfg(feature = "hydrate")]
fn storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok().flatten()
}

/// What is being held under this key, if anything.
#[cfg(feature = "hydrate")]
pub fn read(key: &str) -> Option<String> {
    storage()?.get_item(key).ok().flatten()
}

/// Write `body` out, replacing whatever was under the key.
#[cfg(feature = "hydrate")]
pub fn write(key: &str, body: &str) {
    let Some(storage) = storage() else {
        return;
    };

    // Full, or refused: what was being written is gone, and the page carries on
    // regardless.
    let _ = storage.set_item(key, body);
}

/// Drop whatever is under this key.
#[cfg(feature = "hydrate")]
pub fn forget(key: &str) {
    if let Some(storage) = storage() {
        let _ = storage.remove_item(key);
    }
}

// Under `ssr` there is no browser and so no storage: the server renders every
// page as though this device had never been here, which is what hydration then
// has to find waiting for it. These three stand in for storage the server half
// has no way to reach and no reason to.

#[cfg(not(feature = "hydrate"))]
pub fn read(_key: &str) -> Option<String> {
    None
}

#[cfg(not(feature = "hydrate"))]
pub fn write(_key: &str, _body: &str) {}

#[cfg(not(feature = "hydrate"))]
pub fn forget(_key: &str) {}
