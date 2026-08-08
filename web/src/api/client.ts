//! Talking to `/api/ui/` — the viewer's own half of the server.
//!
//! Every payload's type comes from [`./types`], which `cargo test` writes out
//! of the Rust the server fills them in from. Nothing here declares a shape of
//! its own: a hand-written interface is a second opinion about the wire, and
//! the whole point of generating them is that there is only ever one.

import type {
  ApiError,
  ArchiveEntry,
  Archived,
  PendingEntry,
  PushKey,
  Response as Decided,
  SetView,
  Submitted,
  Subscribed,
  Subscription,
  UpdateNotice,
} from "./types";

/// A refusal from the server, in the shape both halves refuse in.
///
/// Carries the server's own wording rather than a status code, because that
/// wording is what the page has to show the human — see the `error` field on
/// `ApiError`.
export class RefusedError extends Error {
  readonly status: number;
  readonly violations: NonNullable<ApiError["violations"]>;

  constructor(status: number, refusal: ApiError) {
    super(refusal.error);
    this.name = "RefusedError";
    this.status = status;
    this.violations = refusal.violations ?? [];
  }
}

/// The Sets still waiting on the human, newest first.
export function listPending(): Promise<PendingEntry[]> {
  return get<PendingEntry[]>("/api/ui/pending");
}

/// The Sets that have been settled, newest first — answered or closed
/// unanswered, which is the whole of the Archive.
export function listArchive(): Promise<ArchiveEntry[]> {
  return get<ArchiveEntry[]>("/api/ui/archive");
}

/// One Set, rendered, with where it stands.
///
/// The id is whatever the URL held, unparsed: one that is not a number cannot
/// name a Set, and the server answers for that the same way it answers for one
/// that names no Set — a 404, which the page reads as "there isn't one".
export function loadSet(id: string): Promise<SetView> {
  return get<SetView>(`/api/ui/sets/${encodeURIComponent(id)}`);
}

/// Answer a Set, which ends the wait the agent is holding on it.
///
/// The outcome is the answer's body rather than its status: every one of them —
/// taken, already answered, archived, refused by the grammar — is something the
/// page has to say in words, and only a server that could not answer at all
/// throws.
export function submitResponse(
  id: number,
  response: Decided,
): Promise<Submitted> {
  return post<Submitted>(`/api/ui/sets/${id}/response`, response);
}

/// Close a Set unanswered: the human declaring that nobody is ever going to
/// answer it. There is nothing to send but the Set's own id, which is in the
/// path.
export function archiveSet(id: number): Promise<Archived> {
  return post<Archived>(`/api/ui/sets/${id}/archive`);
}

/// Whether a newer Askance has been released than the one serving this page.
///
/// The server is the side that asks GitHub, once a day, and this hands over
/// whatever it last concluded — so the answer costs it nothing and the browser
/// never waits on GitHub being reachable.
export function updateNotice(): Promise<UpdateNotice> {
  return get<UpdateNotice>("/api/ui/update");
}

/// The public half of the server's VAPID keypair — what `PushManager.subscribe`
/// takes as its `applicationServerKey`, and the only way a browser names the
/// server it is subscribing to.
export async function pushKey(): Promise<string> {
  return (await get<PushKey>("/api/ui/push/key")).key;
}

/// Hand this device's subscription over, so a Set arriving can reach it.
export function subscribePush(
  subscription: Subscription,
): Promise<Subscribed> {
  return post<Subscribed>("/api/ui/push/subscribe", subscription);
}

/// Ask the server to forget this device, named by the endpoint that is the only
/// name a subscription has.
///
/// Answered with nothing at all — there is no outcome to read, because an
/// endpoint the server never stored leaves what was asked for holding either
/// way.
export async function unsubscribePush(endpoint: string): Promise<void> {
  await refused(await sent("/api/ui/push/unsubscribe", { endpoint }));
}

async function get<T>(path: string): Promise<T> {
  return taken(
    await fetch(path, {
      headers: { accept: "application/json" },
    }),
  );
}

async function post<T>(path: string, body?: unknown): Promise<T> {
  return taken(await sent(path, body));
}

function sent(path: string, body?: unknown): Promise<Response> {
  return fetch(path, {
    method: "POST",
    headers: {
      accept: "application/json",
      "content-type": "application/json",
    },
    // An empty object rather than no body at all: the routes that take one
    // want JSON, and the ones that take none are not troubled by it.
    body: JSON.stringify(body ?? {}),
  });
}

async function taken<T>(response: Response): Promise<T> {
  await refused(response);

  return (await response.json()) as T;
}

/// Throw if the server refused, in its own words. Split out from [`taken`] for
/// the one endpoint that answers with no body to read.
async function refused(response: Response): Promise<void> {
  if (!response.ok) {
    throw new RefusedError(response.status, await refusal(response));
  }
}

/// What a refusal said, or a stand-in when it did not say anything readable —
/// a proxy in front of the server can answer where the server would have.
async function refusal(response: Response): Promise<ApiError> {
  try {
    const body: unknown = await response.json();
    if (
      typeof body === "object" &&
      body !== null &&
      typeof (body as ApiError).error === "string"
    ) {
      return body as ApiError;
    }
  } catch {
    // Not JSON at all. Falls through to the status line below.
  }

  return { error: `the server answered ${response.status}` };
}
