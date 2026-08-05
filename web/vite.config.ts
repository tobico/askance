/// <reference types="vitest/config" />
import { defineConfig } from "vite";
import solid from "vite-plugin-solid";

/// Where the axum server listens by default — see `ASKANCE_LISTEN`.
const SERVER = "http://127.0.0.1:8422";

export default defineConfig(({ mode }) => ({
  plugins: [solid()],

  server: {
    // `pnpm dev` serves the viewer and nothing else; everything under `/api`
    // is the real server's, so the two run side by side and the browser sees
    // one origin. Development only — a build is one binary serving both, with
    // no proxy anywhere in it.
    proxy: {
      "/api": SERVER,
    },
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
    environment: "jsdom",
    setupFiles: ["./tests/setup.ts"],
  },
}));
