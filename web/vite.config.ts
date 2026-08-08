/// <reference types="vitest/config" />
import { defineConfig } from "vite";
import solid from "vite-plugin-solid";

/// Where the axum server listens by default — see `ASKANCE_LISTEN`.
const SERVER = "http://127.0.0.1:8422";

export default defineConfig(({ mode }) => ({
  plugins: [solid()],

  // The service worker, the manifest and the icons, served from the site root
  // and copied into the build untouched. They are the repo's `assets/`, which is
  // this and nothing else's now.
  //
  // The root is where they have to be: a service worker only controls the paths
  // beneath the one it was served from, and one under the bundle's directory
  // could never show a notification for `/sets/12`.
  publicDir: "../assets",

  build: {
    // Named rather than left to the default, because the server keeps everything
    // under this directory for a year and revalidates everything outside it —
    // see `HASHED` in `crates/server/src/viewer.rs`. What earns the year is that
    // vite names these files by their content; a file arriving here under a
    // stable name would be cached past the build that replaced it.
    assetsDir: "assets",
  },

  server: {
    // `pnpm dev` serves the viewer and nothing else; everything under `/api`
    // is the real server's, so the two run side by side and the browser sees
    // one origin. Development only — a build is one binary serving both, with
    // no proxy anywhere in it.
    proxy: {
      "/api": SERVER,
    },

    // Under vitest, let a test read a file above this directory — which
    // `relaying.test.ts` does, the service worker it drives being `assets/sw.js`
    // at the repo root rather than a module of this bundle. Said only for the
    // test run: what the dev server hands a browser stays inside `web/`.
    ...(mode === "test" ? { fs: { allow: [".."] } } : {}),
  },

  resolve: {
    // Under vitest, resolve solid-js the way a browser would. Left to itself
    // Node would take the server build, which renders to a string: the test
    // would then find nothing in the document and say so as if the component
    // were at fault. Said only for the test run, because a production build
    // must not ship the development build of solid-js.
    ...(mode === "test" ? { conditions: ["development", "browser"] } : {}),
  },

  test: {
    // Left off, vitest replaces every CSS import with an empty string — which
    // takes `?raw` with it, and `diagrams.test.ts` reads the stylesheet that way
    // to assert the two rules about a drawn diagram. There is nothing to query
    // those off: the SVG is mermaid's, and no component renders it.
    css: true,
    environment: "jsdom",
    setupFiles: ["./tests/setup.ts"],
  },
}));
