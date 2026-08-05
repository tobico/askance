//! What makes this an app the phone can install, and a push a notification it
//! can show: the service worker the SPA registers, and the tags in the document
//! that name the manifest and the icons.

import { afterEach, describe, expect, it, vi } from "vitest";

import { registerWorker } from "../src/push/worker";
// The document vite builds the bundle into, read as text: the tags asserted
// here are the ones no component renders.
import shell from "../index.html?raw";

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("the service worker", () => {
  it("is registered from the site root, so its scope is the whole site", () => {
    const register = vi.fn(() => Promise.resolve({}));
    vi.stubGlobal("navigator", { serviceWorker: { register } });

    registerWorker();

    // A worker only controls the paths beneath the one it was served from, and
    // one under the bundle's directory could never show a notification for
    // `/sets/12` — nor navigate to it when the notification is tapped.
    expect(register).toHaveBeenCalledWith("/sw.js");
  });

  it("shrugs where the browser has none", () => {
    // Absent in a browser without service workers, and in any browser outside a
    // secure context. Either way the install is lost and nothing else is.
    vi.stubGlobal("navigator", {});

    expect(() => registerWorker()).not.toThrow();
  });

  it("shrugs when the browser refuses the registration", () => {
    vi.stubGlobal("navigator", {
      serviceWorker: { register: () => Promise.reject(new Error("refused")) },
    });

    // Nothing on the page waits on the worker, and the browser reports a
    // refusal to the console in more detail than this could.
    expect(() => registerWorker()).not.toThrow();
  });
});

describe("the document", () => {
  it("names the manifest, so the phone can install it", () => {
    expect(shell).toContain('rel="manifest" href="/manifest.webmanifest"');
  });

  it("names an icon iOS will read, which is neither the manifest's nor an SVG", () => {
    expect(shell).toContain('rel="apple-touch-icon" href="/icons/apple-touch-icon.png"');
  });

  it("is coloured like the manifest says the app is", () => {
    expect(shell).toContain('name="theme-color" content="#7a5c3e"');
  });

  it("names everything it needs at the site root", () => {
    // The worker, the manifest and the icons are static files nothing here
    // renders — vite copies them out of the repo's `assets/` untouched, and the
    // root is where the worker's scope needs them. A path under the bundle's
    // directory would build and install and then never notify.
    for (const named of shell.matchAll(/(?:href|src)="([^"]*)"/g)) {
      expect(named[1]!.startsWith("/"), `${named[1]} is not at the root`).toBe(
        true,
      );
    }
  });
});
