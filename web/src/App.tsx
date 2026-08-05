//! The application: everything under the API routes the agents use.

import { Route, Router } from "@solidjs/router";
import { QueryClient, QueryClientProvider } from "@tanstack/solid-query";
import type { JSX } from "solid-js";

import { PendingList } from "./pending/PendingList";

/// One client for the whole app, made once rather than per render: it is where
/// the cache lives, and a page that rebuilt it would have no cache at all.
///
/// Refetching on focus is off because the pages that want to be current say so
/// themselves, on an interval — coming back to a tab is not new information
/// about a Set, and every extra fetch is one the phone pays for.
const queries = new QueryClient({
  defaultOptions: { queries: { refetchOnWindowFocus: false } },
});

export function App(): JSX.Element {
  return (
    <QueryClientProvider client={queries}>
      <Router root={Shell}>
        <Route path="/" component={PendingList} />
        <Route path="*" component={NoSuchPage} />
      </Router>
    </QueryClientProvider>
  );
}

/// What every page sits in. The column the stylesheet sets its width on.
function Shell(props: { children?: JSX.Element }): JSX.Element {
  return <main>{props.children}</main>;
}

function NoSuchPage(): JSX.Element {
  return <p class="empty">No such page.</p>;
}
