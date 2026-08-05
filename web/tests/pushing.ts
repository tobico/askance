//! A stand-in for the browser's half of push: the permission, the service
//! worker's registration, and the push manager hanging off it.
//!
//! None of the three exists in jsdom, which is the same absence a browser
//! without push presents — so a test that wants push to be had says so by
//! calling [`pushing`], and a test that wants it missing says nothing at all.

import { vi } from "vitest";

/// The endpoint and keys a fake subscription hands over. Shaped like a real
/// one — an https URL and two base64url strings — because the server's own
/// validation is what they are eventually put to.
export const ENDPOINT = "https://push.example/f/aG9sZA";
export const P256DH = "BJxKZ3Z0aGVyZS1pcy1uby1zdWNoLWtleS1oZXJlLWF0LWFsbA";
export const AUTH = "c2VjcmV0LWF1dGgtc2VjcmV0";

/// The VAPID public key the fake server hands out.
export const KEY = "BFakeVapidPublicKeyForTheTestsOnly";

/// A subscription as a browser describes one: the endpoint it is named by, the
/// nested JSON the keys arrive in, and the promise that drops it.
export function subscription(endpoint: string = ENDPOINT) {
  return {
    endpoint,
    toJSON: () => ({ endpoint, keys: { p256dh: P256DH, auth: AUTH } }),
    unsubscribe: vi.fn(() => Promise.resolve(true)),
  };
}

export type Subscription = ReturnType<typeof subscription>;

/// A browser with push to be had, in whatever state this test wants it.
export function pushing(
  device: {
    /// What the browser has already been told about notifications here.
    permission?: NotificationPermission;
    /// What the human answers the prompt with, when one goes up. Defaults to
    /// leaving `permission` as it stands, which is a prompt dismissed.
    answers?: NotificationPermission;
    /// The subscription this browser already holds, if any.
    subscription?: Subscription | null;
    /// Why a subscribe is refused, when it is.
    refuses?: string;
    /// Whether the page is in a secure context. Push is withheld outside one.
    secure?: boolean;
  } = {},
) {
  const held = { subscription: device.subscription ?? null };

  const notification = {
    permission: device.permission ?? "default",
    requestPermission: vi.fn(() => {
      notification.permission = device.answers ?? notification.permission;
      return Promise.resolve(notification.permission);
    }),
  };

  const manager = {
    getSubscription: vi.fn(() => Promise.resolve(held.subscription)),
    subscribe: vi.fn((_options: PushSubscriptionOptionsInit) => {
      if (device.refuses !== undefined) {
        return Promise.reject(new Error(device.refuses));
      }

      held.subscription = subscription();
      return Promise.resolve(held.subscription);
    }),
  };

  vi.stubGlobal("isSecureContext", device.secure ?? true);
  vi.stubGlobal("Notification", notification);
  // Only ever looked for by name — the control feature-tests the global rather
  // than reaching for a registration's manager and finding it missing later.
  vi.stubGlobal("PushManager", class {});
  vi.stubGlobal("navigator", {
    serviceWorker: {
      ready: Promise.resolve({ pushManager: manager }),
      register: vi.fn(() => Promise.resolve({})),
    },
  });

  return { notification, manager, held };
}

/// An answer with no body, as the server ends an unsubscribe.
export function nothing(): () => Promise<Response> {
  return () => Promise.resolve(new Response(null, { status: 204 }));
}
