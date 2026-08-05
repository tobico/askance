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

/// The page on its own route, so the id it fetches is the one the URL names,
/// and inside a router, because the way back out of a Set is a link.
export function mount(id = "1") {
  // No retries: a test that asked for a refusal should see it at once, rather
  // than after the three attempts a real page is right to make.
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });

  const history = createMemoryHistory();
  history.set({ value: `/sets/${id}` });

  return render(() => (
    <QueryClientProvider client={client}>
      <MemoryRouter history={history}>
        <Route path="/sets/:id" component={SetPage} />
      </MemoryRouter>
    </QueryClientProvider>
  ));
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

/// Everything of `selector` in the page, as the text of each — the order they
/// come out in is the order the page has them in.
export function texts(container: ParentNode, selector: string): string[] {
  return [...container.querySelectorAll(selector)].map(
    (found) => found.textContent ?? "",
  );
}
