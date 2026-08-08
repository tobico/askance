//! A stand-in for the server, shared by the component tests.
//!
//! Every payload a test serves through this comes out of `tests/fixtures/`,
//! which `cargo test` writes from the real endpoints — so what a component is
//! fed here is what the server actually said.

import { vi } from "vitest";

/// One answer per fetch in the order given. The last answer is repeated,
/// because a page polls for as long as it is open and a test should not have to
/// say how many times.
///
/// An answer handed to [`whenever`] is held out of that order and belongs to the
/// one path it names: a page with two things to fetch has no fixed order between
/// them, and a test about one of them should not have to say when the other went
/// out.
export function serving(...answers: Array<Answer>) {
  const asked: Array<() => Promise<Response>> = [];
  const paths = new Map<string, () => Promise<Response>>();
  for (const answer of answers) {
    if (typeof answer === "function") {
      asked.push(answer);
    } else {
      paths.set(answer.path, answer.answer);
    }
  }

  let taken = 0;
  // Typed as `fetch` is called rather than as a bare thunk, so a test can read
  // back what a page put on the wire and not just that it asked.
  const fetching = vi.fn((path: RequestInfo | URL, _init?: RequestInit) => {
    const held = paths.get(String(path));
    return held ? held() : asked[Math.min(taken++, asked.length - 1)]!();
  });
  vi.stubGlobal("fetch", fetching);
  return fetching;
}

/// What a test hands [`serving`]: an answer in the sequence, or one belonging to
/// a path.
type Answer = (() => Promise<Response>) | { path: string; answer: () => Promise<Response> };

/// One answer for one path, however often and whenever it is asked for. For the
/// endpoint a page fetches alongside the one a test is about.
export function whenever(
  path: string,
  answer: () => Promise<Response>,
): Answer {
  return { path, answer };
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
