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
