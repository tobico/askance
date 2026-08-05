//! Where a device stands about notifications, and the judgement that decides it.
//!
//! Kept apart from the browser half beside it so that the one piece of reasoning
//! in the control can be tested without a browser to ask: what a permission and
//! a subscription come to together, what a switch cannot say for itself, and
//! what a refusal needs adding to it.

/// Where notifications stand on the device the page is open on.
///
/// Every one of these is somewhere a browser can leave a device, and the control
/// says which: an offer to turn something on that is already on, or that this
/// browser will never allow, is worse than saying so.
export type Standing =
  /// Not looked yet. What the control draws before the browser has answered.
  | "unknown"
  /// Push is not to be had here at all: no service worker, no push manager, or
  /// a page outside a secure context. Nothing to offer, so nothing is offered.
  | "unavailable"
  /// Permission was refused. A dead end: the browser will not ask again, so the
  /// way out is its own site settings and not a tap here.
  | "blocked"
  /// Available, and this device has not asked to be told.
  | "off"
  /// This device has asked to be told, and can be asked to stop.
  | "on";

/// What a browser's answer about notification permission comes to. The three
/// states `Notification.permission` has, under names that say what they mean
/// here.
export type Permission =
  /// Never asked, or asked and dismissed without an answer.
  | "undecided"
  | "granted"
  | "denied";

/// Where a device with push to be had stands, from what the browser says about
/// it.
///
/// Denied beats a subscription that is still there, rather than the other way
/// about: a browser told not to show notifications will not show them for a
/// subscription it kept, so calling that "on" would be a lie the human only
/// finds out about by missing a Set.
export function standing(permission: Permission, subscribed: boolean): Standing {
  if (permission === "denied") {
    return "blocked";
  }

  return subscribed ? "on" : "off";
}

/// What has to be said in words, because the switch cannot say it — and `null`
/// wherever the switch already does.
///
/// A device that is on or off is not written about at all: the switch is the
/// whole of the answer, and a sentence restating it would be one more thing to
/// read on the page that is opened most.
export function said(standing: Standing): string | null {
  switch (standing) {
    // Both halves of it, because the page cannot tell which: a browser without
    // push, or one withholding it because the connection is not secure. Over the
    // tailnet the second is what `tailscale serve` is for.
    case "unavailable":
      return (
        "Notifications are not available here — this browser has no push " +
        "support, or the page is not being served over https."
      );
    case "blocked":
      return (
        "Notifications are blocked for this device. This browser will not ask " +
        "again, so allow them in its settings for this site."
      );
    // Still looking says nothing rather than saying so: the switch is disabled
    // for the moment it takes, and a sentence that appears only to be replaced
    // is worse than a switch that waits.
    case "unknown":
    case "off":
    case "on":
      return null;
  }
}

/// Whether the switch will take a flip here.
///
/// Everywhere else is somewhere a flip could not help: still looking, never
/// going to work, or a dead end only the browser's own settings lead out of.
export function flippable(standing: Standing): boolean {
  return standing === "off" || standing === "on";
}

/// The browser's account of a refused subscribe, with a way out appended where
/// its wording names the push service.
///
/// A Chromium browser has no push transport but Google's, and the ones that
/// de-Google — Brave chiefly — ship with it switched off. A subscribe there is
/// refused inside the browser, before anything is asked of this server, and all
/// the browser says about it is "push service error". Left at that it reads as
/// the server's fault, and it is the one refusal here that neither another tap
/// nor the site's own permission settings lead out of.
///
/// Matched on what the browser said rather than on which browser said it: a
/// build that reports this cannot subscribe whatever it calls itself, and the
/// user agent is no help anyway — Brave answers that it is Chrome. Wording is
/// the browser's to change, so a version that rephrases this costs the way out
/// and not the error.
export function refusal(said: string): string {
  if (!said.toLowerCase().includes("push service")) {
    return said;
  }

  return (
    `${said} — a browser that de-Googles, Brave among them, refuses this ` +
    'until "Use Google services for push messaging" is turned on in its ' +
    "privacy settings and it has been restarted."
  );
}
