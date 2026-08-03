# 01. Installable app shell

## What to build

Askance installs to a phone's home screen and opens standalone: a web manifest,
icons, and a service worker registered at root scope.

The manifest and the service worker are static files served from the site root,
not from `/pkg/` — a worker can only control the paths under the one it is served
from, and a worker under `/pkg/` could never show a notification for `/sets/12`.
Add an assets directory to the workspace's Leptos metadata so `cargo leptos`
copies them into the site root, where the UI's existing fallback file handler
serves them with the right content types.

The worker does no caching: it is here for push. Every page is server-rendered
against live SQLite, and a cached copy of a Set that has since been answered is
worse to the human than a failure to load. Registration happens from the browser
half of the UI only, and a browser without service workers loses nothing but the
install.

Icons come from one SVG source rasterized to the PNG sizes the manifest and iOS
need. Nothing in the dev shell can rasterize SVG today, so add a rasterizer to
the flake and make the icons reproducible from their source rather than committed
binaries of unknown provenance.

## Acceptance criteria

- [ ] `cargo leptos build` puts the manifest, the icons and the worker in the
      site root, and the running server serves the manifest as
      `application/manifest+json` and the worker from the root path
- [ ] The worker registers with root scope on page load, and a browser without
      service worker support renders and behaves exactly as it does now
- [ ] The worker passes fetches straight through — no cache is populated and no
      response is served from one
- [ ] The icons regenerate from their SVG source with a documented command, using
      only what the dev shell provides
- [ ] Opening the site over the `ts.net` HTTPS URL offers "install" / "Add to
      Home Screen", and the installed app opens standalone on the pending list
