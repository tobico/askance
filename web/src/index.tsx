/* @refresh reload */
import { render } from "solid-js/web";

import { App } from "./App";
import { registerWorker } from "./push/worker";

// Imported here rather than linked from the document so that vite hashes it into
// the bundle like everything else, and it is therefore cached the same way — see
// the server's `viewer` module for what that buys.
import "./main.css";

const app = document.getElementById("app");
if (!app) {
  throw new Error("index.html has no #app to mount into");
}

// Before the mount rather than after it: nothing on the page waits on the
// worker, and the notifications switch waits on the registration being in
// control, so the sooner it is asked for the sooner the switch can answer.
registerWorker();

render(() => <App />, app);
