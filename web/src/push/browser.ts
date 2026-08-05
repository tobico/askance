//! Asking the browser about push, and telling it what to do about it.
//!
//! The push manager lives on the service worker's registration, and neither is
//! there in every browser: everything here begins by looking, and a browser with
//! nothing to look at leaves the device [`unavailable`](Standing) rather than
//! throwing.
//!
//! Nothing is remembered between visits — what the control says is read out of
//! the browser on every load, because an installed app reopened a week later
//! must not offer to enable what is already enabled, and the browser is the only
//! thing that knows.

import { pushKey, subscribePush, unsubscribePush } from "../api/client";
import type { Subscription } from "../api/types";
import type { Permission, Standing } from "./standing";
import { refusal, standing } from "./standing";

/// Where this device stands, read out of the browser rather than remembered.
///
/// Asks nothing of the human: this runs on every load, and a page that prompted
/// for permission just by being opened would be answered "no" once and never get
/// another chance.
export async function look(): Promise<Standing> {
  const manager = await pushManager();
  if (!manager) {
    return "unavailable";
  }

  return standing(permission(), (await current(manager)) !== null);
}

/// Ask to be told, and hand the resulting subscription to the server.
export async function enable(): Promise<Standing> {
  // The prompt goes up before anything at all is awaited: a phone gives the
  // permission prompt to a tap and to nothing else, and every await ahead of it
  // is a chance for the browser to decide the tap is over. The control only
  // offers this where push was found, so nothing is being asked for that could
  // not then be granted.
  //
  // Asked here rather than left to `subscribe` to trigger, so that a refusal is
  // reported as the dead end it is instead of as a subscribe that failed.
  switch (await requestPermission()) {
    case "denied":
      return "blocked";
    // Dismissed without an answer: nothing was decided, so nothing is said about
    // it and the offer stands.
    case "undecided":
      return "off";
    case "granted":
      break;
  }

  const manager = await pushManager();
  if (!manager) {
    return "unavailable";
  }

  // A subscription already here is used as it is. The browser hands back the
  // same one anyway, and the server may be the half that is missing it — a tap
  // that failed after subscribing, or a database restored from before it.
  const held = (await current(manager)) ?? (await subscribe(manager));

  const described = describe(held);
  if (!described) {
    throw new Error(
      "The browser gave a subscription with no endpoint or key in it, so " +
        "nothing could be sent to this device.",
    );
  }

  const taken = await handed(described);
  if (taken === "Incomplete") {
    throw new Error("The server refused this device's subscription as incomplete.");
  }

  return "on";
}

/// Ask not to be told any more, here.
///
/// The browser's subscription goes first and the server's row second: if the
/// second half fails, this device is off and says so, and the endpoint the
/// server kept is one no push can be delivered to. The other order would leave
/// the control saying "on" over a device nothing is ever sent to.
export async function disable(): Promise<Standing> {
  const manager = await pushManager();
  if (!manager) {
    return "unavailable";
  }

  const held = await current(manager);
  if (!held) {
    // Nothing to turn off — the browser dropped it, or another tab did.
    return standing(permission(), false);
  }
  const endpoint = held.endpoint;

  await held.unsubscribe();

  try {
    await unsubscribePush(endpoint);
  } catch (error) {
    throw new Error(`This device stopped, but the server still has it: ${said(error)}`);
  }

  return standing(permission(), false);
}

/// This page's push manager, or `null` when push is not to be had here.
async function pushManager(): Promise<PushManager | null> {
  // Service workers, push and notifications are all withheld outside a secure
  // context, so a page served over plain HTTP from anything but localhost has
  // none of them — which over the tailnet is what `tailscale serve`'s
  // certificate is for.
  if (!globalThis.isSecureContext) {
    return null;
  }

  // Asked of the globals rather than of the objects: reading `permission` off a
  // `Notification` that is not there throws rather than coming back undefined,
  // and a registration without a push manager would only fail later, on a tap.
  if (!("Notification" in globalThis) || !("PushManager" in globalThis)) {
    return null;
  }

  const container = navigator.serviceWorker;
  if (!container) {
    return null;
  }

  try {
    // `ready` rather than `getRegistration`: the worker is registered as the app
    // mounts, and this is what waits for it to be in control instead of racing
    // it. A registration that never succeeds leaves this pending, and the
    // control still saying it is looking — which is the truth.
    return (await container.ready).pushManager ?? null;
  } catch {
    return null;
  }
}

/// This device's subscription, if it has one. `null` covers both a browser that
/// has none and one that would not say.
async function current(manager: PushManager): Promise<PushSubscription | null> {
  try {
    return await manager.getSubscription();
  } catch {
    return null;
  }
}

/// Subscribe this device against the server's public key.
async function subscribe(manager: PushManager): Promise<PushSubscription> {
  let key: string;
  try {
    key = await pushKey();
  } catch (error) {
    throw new Error(`The server's push key could not be read: ${said(error)}`);
  }

  try {
    return await manager.subscribe({
      // Every push this server sends shows a notification, and Chrome refuses to
      // subscribe without the promise that it will.
      userVisibleOnly: true,
      // The base64url string rather than bytes: `applicationServerKey` takes
      // either, and the encoding the server hands out is already this one.
      applicationServerKey: key,
    });
  } catch (error) {
    throw new Error(refusal(said(error)));
  }
}

/// Hand a subscription to the server, saying what the server said if it would
/// not take it.
async function handed(subscription: Subscription) {
  try {
    return await subscribePush(subscription);
  } catch (error) {
    throw new Error(`The server did not take it: ${said(error)}`);
  }
}

/// A browser's subscription as the server takes one.
///
/// Through `toJSON` rather than `getKey`: its keys are base64url already, which
/// is the encoding the server stores and a push is encrypted with, where
/// `getKey` hands back buffers that would have to be encoded here.
function describe(subscription: PushSubscription): Subscription | null {
  const { endpoint, keys } = subscription.toJSON();

  if (!endpoint || !keys?.p256dh || !keys?.auth) {
    return null;
  }

  return { endpoint, p256dh: keys.p256dh, auth: keys.auth };
}

/// What this browser has already been told about notifications for this site.
function permission(): Permission {
  return read(Notification.permission);
}

/// Put the browser's own permission prompt to the human.
///
/// A prompt that could not be raised at all reads as undecided rather than as a
/// refusal: nothing was asked, so nothing was refused, and the offer to try
/// again is honest.
async function requestPermission(): Promise<Permission> {
  try {
    return read(await Notification.requestPermission());
  } catch {
    return "undecided";
  }
}

/// One of the browser's three words for it, under the name it goes by here.
function read(permission: string): Permission {
  switch (permission) {
    case "granted":
      return "granted";
    case "denied":
      return "denied";
    // `default`, and anything a later browser adds: nothing has been decided, so
    // the human is the one to decide it.
    default:
      return "undecided";
  }
}

/// What went wrong, in whosever words there were: a browser names the actual
/// obstacle — an insecure origin, a key the push service rejected — in more
/// detail than anything here could guess at, and so does the server.
export function said(error: unknown): string {
  if (error instanceof Error) {
    return error.message;
  }

  return typeof error === "string" ? error : "It refused, without saying why.";
}
