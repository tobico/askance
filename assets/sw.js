// Askance's service worker, served from the site root so its scope is the whole
// site: a worker can only control the paths under the one it was served from,
// and a worker under /pkg/ could never show a notification for /sets/12.
//
// It does no caching. Every page is rendered against live SQLite, and a cached
// copy of a Set that has since been answered is worse to the human than a
// failure to load. This listener never answers a request, which leaves the
// browser to fetch it exactly as it would with no worker at all; it is here
// because installability checks still look for a fetch handler.
self.addEventListener("fetch", () => {});

// Take over as soon as a new worker is available, rather than waiting for every
// tab to close. There is no cached state for a version skew to corrupt, and the
// stale worker would be the thing holding back a fix to push handling.
self.addEventListener("install", () => self.skipWaiting());
self.addEventListener("activate", (event) => event.waitUntil(self.clients.claim()));

// A Question Set has arrived. Every push the server sends is one, and every one
// of them shows a notification: the subscription was made with
// `userVisibleOnly`, and a push that showed nothing would cost the subscription
// itself.
self.addEventListener("push", (event) => {
  const notice = read(event.data);

  // The title is the Set's own, so the notification says which decision is
  // waiting rather than that some decision is. The project goes in the body,
  // because it is what tells two Sets apart at a glance.
  event.waitUntil(
    self.registration.showNotification(notice.title || "A Question Set is waiting", {
      body: notice.project ? `Askance · ${notice.project}` : "Askance",
      icon: "/icons/icon-192.png",
      // One notification per Set, tagged by its id: a push service that
      // delivers the same push twice then replaces the notification instead of
      // stacking a second one over it.
      tag: notice.id ? `askance-set-${notice.id}` : "askance",
      data: { url: notice.id ? `/sets/${notice.id}` : "/" },
    }),
  );
});

// Tapped. The Set it was about is what opens, in the Askance that is already
// there if there is one.
self.addEventListener("notificationclick", (event) => {
  event.notification.close();
  event.waitUntil(open((event.notification.data || {}).url || "/"));
});

// What the server sent, or an empty notice if it sent something unreadable —
// a notification saying a Set is waiting is still worth more than none.
function read(data) {
  if (!data) return {};

  try {
    return data.json() || {};
  } catch (_) {
    return {};
  }
}

async function open(url) {
  const target = new URL(url, self.location.origin).href;

  // includeUncontrolled: an Askance opened before this worker took control is
  // still the window the human means, and opening a second one over it is the
  // thing to avoid.
  const windows = await self.clients.matchAll({ type: "window", includeUncontrolled: true });

  for (const client of windows) {
    if (new URL(client.url).origin !== self.location.origin) continue;

    // Navigated before it is focused, so the human is not shown whichever page
    // was open for as long as the navigation takes. A window this worker does
    // not control refuses to be navigated, and is focused where it stands
    // rather than being abandoned for a new window.
    const opened = client.url === target ? client : await client.navigate(target).catch(() => client);

    return opened.focus();
  }

  return self.clients.openWindow(target);
}
