/* @refresh reload */
import { render } from "solid-js/web";

import { App } from "./App";
import { registerWorker } from "./push/worker";

// The stylesheet is still the one the Leptos build processes, up at the repo's
// `style/`, because both viewers are standing until the cutover. Imported here
// rather than linked from the document so that vite hashes it into the bundle
// like everything else; when the Leptos half goes, the file moves in here and
// this line stops reaching outside `web/`.
import "../../style/main.css";

const app = document.getElementById("app");
if (!app) {
  throw new Error("index.html has no #app to mount into");
}

// Before the mount rather than after it: nothing on the page waits on the
// worker, and the notifications switch waits on the registration being in
// control, so the sooner it is asked for the sooner the switch can answer.
registerWorker();

render(() => <App />, app);
