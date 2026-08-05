//! Installing the service worker, which is what makes Askance installable — and
//! what a push notification is delivered to.

/// Register the worker, from `/sw.js` so that its scope is the whole site.
///
/// A worker only controls the paths beneath the one it was served from, and one
/// served out of the bundle's own directory could never show a notification for
/// `/sets/12`, nor navigate the open app to it when the notification is tapped.
export function registerWorker(): void {
  // `navigator.serviceWorker` is absent in a browser that has none, and in any
  // browser outside a secure context — which is what `tailscale serve` is for. A
  // browser without it loses the install and nothing else, so this is worth no
  // more than a look and a shrug.
  const container = navigator.serviceWorker;
  if (!container) {
    return;
  }

  // The promise is left to settle on its own: nothing in the app waits on the
  // worker, and a browser that refuses the registration reports it to the
  // console in more detail than this could.
  void container.register("/sw.js").catch(() => {});
}
