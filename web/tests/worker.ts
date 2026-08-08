//! Askance's own service worker, run somewhere a test can drive it.
//!
//! `assets/sw.js` is a static file rather than a module of this bundle — vite
//! copies it to the site root untouched, because that is the only place a worker
//! can be served from and still control `/sets/12` — so it is read here as text
//! and evaluated against a stand-in for the global scope a worker is given.
//! Everything it reaches for hangs off that object, so a worker driven here
//! disturbs nothing about the environment the rest of the tests run in.

import { vi } from "vitest";

/// The worker exactly as the browser is served it. Reading above `web/` is what
/// `server.fs.allow` is opened for under vitest — see `vite.config.ts`.
import SOURCE from "../../assets/sw.js?raw";

/// Where this Askance is, as the worker reads its own location.
export const ORIGIN = "https://askance.example";

/// A push body that is not the JSON the worker expects — a push service having
/// mangled it, or a sender that was never Askance.
export const UNREADABLE = Symbol("a push body that is not JSON");

/// An open window, as `clients.matchAll` hands one over.
export interface Pane {
  url: string;
  /// Every message the worker has posted to this window, newest last.
  posted: unknown[];
  postMessage(message: unknown): void;
}

/// One notification, as the worker asked for it to be shown.
export interface Shown {
  title: string;
  options: NotificationOptions;
}

/// The worker installed and running, with nothing open over it yet.
export function worker() {
  const listeners = new Map<string, (event: never) => void>();
  const shown: Shown[] = [];
  const panes: Pane[] = [];

  const scope = {
    addEventListener: (name: string, listener: (event: never) => void) =>
      listeners.set(name, listener),
    location: { origin: ORIGIN },
    skipWaiting: vi.fn(),
    registration: {
      showNotification: vi.fn((title: string, options: NotificationOptions) => {
        shown.push({ title, options });
        return Promise.resolve();
      }),
    },
    clients: {
      claim: vi.fn(() => Promise.resolve()),
      // Handed the live array rather than a copy, so a window opened after the
      // worker was loaded is still one the worker finds.
      matchAll: vi.fn(() => Promise.resolve(panes)),
      openWindow: vi.fn((url: string) => Promise.resolve(opens(url))),
    },
  };

  // The worker's `self`, passed in rather than stubbed as a global: a service
  // worker is handed its scope, and a test that made this the page's `self`
  // would be describing a browser that does not exist.
  new Function("self", SOURCE)(scope);

  /// A window this worker can reach, as though the human had left it open.
  function opens(url = `${ORIGIN}/`): Pane {
    const pane: Pane = {
      url,
      posted: [],
      postMessage: (message: unknown) => pane.posted.push(message),
    };
    panes.push(pane);
    return pane;
  }

  /// A push delivered, awaited to the end of everything the handler kept the
  /// worker alive for — a test asks what the worker did, not that it was called.
  ///
  /// `notice` is what the server sent: nothing at all where the push carried no
  /// body, or [`UNREADABLE`] where the body is not the JSON this expects.
  async function pushes(notice?: unknown): Promise<void> {
    const kept: Array<Promise<unknown>> = [];

    fire("push", {
      data:
        notice === undefined
          ? null
          : {
              json: () => {
                if (notice === UNREADABLE) {
                  throw new SyntaxError("not JSON");
                }
                return notice;
              },
            },
      waitUntil: (promise: Promise<unknown>) => kept.push(promise),
    });

    await Promise.all(kept);
  }

  function fire(name: string, event: unknown): void {
    const listener = listeners.get(name);
    if (!listener) {
      throw new Error(`the worker listens for no ${name}`);
    }
    listener(event as never);
  }

  return {
    /// Every notification the worker has shown, in the order it showed them.
    shown,
    /// Every message the worker has posted to any open window, gathered across
    /// all of them.
    get relayed(): unknown[] {
      return panes.flatMap((pane) => pane.posted);
    },
    opens,
    pushes,
  };
}
