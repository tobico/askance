/* @refresh reload */
import { render } from "solid-js/web";

import { App } from "./App";

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

render(() => <App />, app);
