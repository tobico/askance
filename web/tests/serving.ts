//! A stand-in for the server, shared by the component tests.
//!
//! Every payload a test serves through this comes out of `tests/fixtures/`,
//! which `cargo test` writes from the real endpoints — so what a component is
//! fed here is what the server actually said.

import { vi } from "vitest";

/// One answer per fetch in the order given. The last answer is repeated,
/// because a page polls for as long as it is open and a test should not have to
/// say how many times.
export function serving(...answers: Array<() => Promise<Response>>) {
  let asked = 0;
  // Typed as `fetch` is called rather than as a bare thunk, so a test can read
  // back what a page put on the wire and not just that it asked.
  const fetching = vi.fn((_path: RequestInfo | URL, _init?: RequestInit) =>
    answers[Math.min(asked++, answers.length - 1)]!(),
  );
  vi.stubGlobal("fetch", fetching);
  return fetching;
}

/// One answer, as the server would have written it.
export function json(body: unknown, status = 200): () => Promise<Response> {
  return () =>
    Promise.resolve(
      new Response(JSON.stringify(body), {
        status,
        headers: { "content-type": "application/json" },
      }),
    );
}
