//! Mounting a list of Sets the way it is really mounted, for the tests that read
//! what it drew.
//!
//! Shared by the two lists, because they are one page twice over: the same card,
//! the same ways out of it, and the same thing to say when there is nothing in
//! them. One mount between them means both are asking about the page the app
//! really builds.

import { MemoryRouter, Route, createMemoryHistory } from "@solidjs/router";
import { render } from "@solidjs/testing-library";
import { QueryClient, QueryClientProvider } from "@tanstack/solid-query";
import type { JSX } from "solid-js";

/// The list on a route, inside a router: everything either list leads to — a
/// Set's own page, and the other list — is a router link rather than a fresh
/// document, and there has to be a router around it to resolve one.
export function mount(component: () => JSX.Element) {
  // No retries: a test that asked for a refusal should see it at once, rather
  // than after the three attempts a real page is right to make.
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });

  const history = createMemoryHistory();

  return {
    ...render(() => (
      <QueryClientProvider client={client}>
        <MemoryRouter history={history}>
          <Route path="*" component={component} />
        </MemoryRouter>
      </QueryClientProvider>
    )),
    history,
  };
}

/// Everything of `selector` in the page, as the text of each — the order they
/// come out in is the order the page has them in.
export function texts(container: ParentNode, selector: string): string[] {
  return [...container.querySelectorAll(selector)].map(
    (found) => found.textContent ?? "",
  );
}
