//! Mounting the Set page the way it is really mounted, for the tests that read
//! what it drew.
//!
//! Shared because three files' worth of assertions are about one page: the
//! record it draws, the Diff on it, and the table of contents down its margin.
//! One mount between them means all three are asking about the page the app
//! really builds.

import { MemoryRouter, Route, createMemoryHistory } from "@solidjs/router";
import { cleanup, render, waitFor } from "@solidjs/testing-library";
import { QueryClient, QueryClientProvider } from "@tanstack/solid-query";
import { expect } from "vitest";

import type { SetView } from "../src/api/types";
import { SetPage } from "../src/set/SetPage";
import { json, serving } from "./serving";

/// Where a Set leads when it is done with: the pending list and the Archive.
/// Neither is this page's subject, so they are stand-ins — what a test asks is
/// which of them the page went to, which is `history.get()`.
const Elsewhere = () => <p class="elsewhere" />;

/// The page on its own route, so the id it fetches is the one the URL names,
/// and inside a router, because the way back out of a Set is a link — and,
/// once it is answered or archived, a navigation.
export function mount(id = "1") {
  // No retries: a test that asked for a refusal should see it at once, rather
  // than after the three attempts a real page is right to make.
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });

  const history = createMemoryHistory();
  history.set({ value: `/sets/${id}` });

  return {
    ...render(() => (
      <QueryClientProvider client={client}>
        <MemoryRouter history={history}>
          <Route path="/sets/:id" component={SetPage} />
          <Route path="/" component={Elsewhere} />
          <Route path="/archive" component={Elsewhere} />
        </MemoryRouter>
      </QueryClientProvider>
    )),
    history,
  };
}

/// The page once the Set it was asked for has arrived.
///
/// Whatever was drawn before it goes first, so that a test reading one Set
/// after another is reading one page at a time: two pages in the document at
/// once are two `#preface`s, and an id that names two elements names neither.
export async function reading(set: SetView): Promise<HTMLElement> {
  cleanup();
  serving(json(set));
  const { container } = mount();
  await waitFor(() => expect(container.querySelector("h1")).toBeTruthy());
  return container;
}

/// The same page, kept hold of for the tests that fill it in: the Set is served
/// first and whatever follows answers the submit or the archive it goes on to
/// make.
///
/// The fetch mock comes back with it, because what a sheet was filled in with is
/// read off the request it sent — and so does the history, because a Set that
/// has been settled leaves the page.
export async function answering(
  set: SetView,
  ...answers: Array<() => Promise<globalThis.Response>>
) {
  cleanup();
  const fetching = serving(json(set), ...answers);
  const { container, history } = mount(String(set.id));
  await waitFor(() => expect(container.querySelector("h1")).toBeTruthy());
  return { page: container, fetching, history };
}

/// The same Set, closing with a Postscript.
///
/// The markup is written here rather than taken from a fixture: the Set
/// `cargo test` writes those from is the one that closes with nothing, and what
/// its Postscript would be rendered into is asked of the server in
/// `ui_content.rs`. What is asked here is where the page puts it — so this is
/// the shape that renderer really emits, prose and a list with a code span in
/// it, and nothing about the rendering rides on it.
export const POSTSCRIPT =
  "<p>Worth taking up in the comment:</p>\n<ul>\n<li>whether <code>ops/export</code> gets an allowlist entry</li>\n</ul>\n";

export function withPostscript(set: SetView): SetView {
  return { ...set, postscript_html: POSTSCRIPT };
}

/// The body of the last request the page sent, as JSON — what it actually put
/// on the wire, rather than what it was asked to.
export function sent(fetching: ReturnType<typeof serving>): unknown {
  const last = fetching.mock.calls.at(-1);
  expect(last, "expected the page to have sent something").toBeTruthy();
  return JSON.parse(String(last![1]?.body));
}

/// Everything of `selector` in the page, as the text of each. Shared with the
/// lists' own mount, because reading a page's order back is the same act on
/// either.
export { texts } from "./listing";
