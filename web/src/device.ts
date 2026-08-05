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
//! this path is worth a thrown error. Every read comes back `null` and every
//! write is dropped.

/// Where the wrap setting lives. Namespaced like the drafts beside it, so
/// everything this app leaves in a browser is legible as its own.
const WRAP = "askance.diff-wrap";

/// Whether Diffs are drawn wrapped on this device.
///
/// Anything but the value [`setWrapping`] writes reads as off, which is also
/// what an untouched browser answers: unwrapped is the setting nobody has
/// expressed an opinion about.
export function wrapping(): boolean {
  return read(WRAP) === "on";
}

/// Remember how this device wants Diffs drawn.
export function setWrapping(on: boolean): void {
  if (on) {
    write(WRAP, "on");
  } else {
    // Removed rather than written as "off": the absence is already the default,
    // and this way turning it off leaves nothing behind.
    forget(WRAP);
  }
}

/// What is being held under this key, if anything — and `null` when there is no
/// storage to be had, which is a browser that blocks it or one in a context
/// that has none.
export function read(key: string): string | null {
  try {
    return localStorage.getItem(key);
  } catch {
    return null;
  }
}

/// Write `body` out, replacing whatever was under the key.
export function write(key: string, body: string): void {
  try {
    localStorage.setItem(key, body);
  } catch {
    // Full, or refused: what was being written is gone, and the page carries on
    // regardless.
  }
}

/// Drop whatever is under this key.
export function forget(key: string): void {
  try {
    localStorage.removeItem(key);
  } catch {
    // As above: there was nothing to lose but the setting.
  }
}
