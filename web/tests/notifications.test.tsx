//! The switch that turns notifications on for the device in front of you.
//!
//! Driven through a stand-in browser (see `pushing`) rather than a mock of the
//! control's own functions: what this is really about is the four round trips
//! between a tap and a subscribed device — the permission prompt, the push
//! manager, the server's key and the server's row — and a test that stubbed
//! those out would be asserting its own arrangement.

import { render, screen, waitFor } from "@solidjs/testing-library";
import { afterEach, describe, expect, it, vi } from "vitest";

import { Notifications } from "../src/push/Notifications";
import { AUTH, ENDPOINT, KEY, P256DH, nothing, pushing, subscription } from "./pushing";
import { json, serving } from "./serving";

afterEach(() => {
  vi.unstubAllGlobals();
});

/// The switch itself, once the control has finished asking the browser where
/// this device stands.
async function control() {
  const { container } = render(() => <Notifications />);
  const flip = screen.getByRole("switch") as HTMLInputElement;

  // The look runs on mount and is three promises deep; until it lands the
  // switch is disabled, which is the one state every test here starts past.
  await waitFor(() => expect(flip.disabled).toBe(false));

  return { container, flip, state: () => container.querySelector(".state")!.textContent };
}

/// The same, for a device the control will never enable — where waiting for the
/// switch to come alive would be waiting forever.
async function stalled(says: string) {
  const { container } = render(() => <Notifications />);
  await waitFor(() => expect(container.querySelector(".state")!.textContent).toContain(says));
  return { container, flip: screen.getByRole("switch") as HTMLInputElement };
}

describe("the notifications switch", () => {
  it("reads where this device stands without asking the human anything", async () => {
    const { notification } = pushing({ permission: "default" });
    serving();

    const { flip, state } = await control();

    expect(flip.checked).toBe(false);
    // A page that prompted just by being opened would be answered "no" once and
    // never get another chance.
    expect(notification.requestPermission).not.toHaveBeenCalled();
    // The switch is the whole of the answer; a line restating it would be one
    // more thing to read on the page that is opened most.
    expect(state()).toBe("");
  });

  it("reads as on where the browser already holds a subscription", async () => {
    pushing({ permission: "granted", subscription: subscription() });
    serving();

    const { flip } = await control();

    expect(flip.checked).toBe(true);
  });

  it("subscribes the browser and hands what it gave to the server", async () => {
    const { manager } = pushing({ permission: "default", answers: "granted" });
    const fetching = serving(json({ key: KEY }), json("Stored"));

    const { flip } = await control();
    flip.click();

    await waitFor(() => expect(flip.checked).toBe(true));
    await waitFor(() => expect(flip.disabled).toBe(false));

    // Subscribed against the server's own key, with the promise that every push
    // shows something — Chrome refuses to subscribe without it.
    expect(manager.subscribe).toHaveBeenCalledWith({
      userVisibleOnly: true,
      applicationServerKey: KEY,
    });

    expect(fetching.mock.calls[0]![0]).toBe("/api/ui/push/key");

    const [where, sent] = fetching.mock.calls[1]!;
    expect(where).toBe("/api/ui/push/subscribe");
    expect(JSON.parse(String(sent?.body))).toEqual({
      // Flattened out of the browser's nesting, which is the browser's shape
      // and not something the server has any reason to learn.
      endpoint: ENDPOINT,
      p256dh: P256DH,
      auth: AUTH,
    });
  });

  it("drops the browser's subscription and then the server's row", async () => {
    const held = subscription();
    pushing({ permission: "granted", subscription: held });
    const fetching = serving(nothing());

    const { flip } = await control();
    expect(flip.checked).toBe(true);
    flip.click();

    await waitFor(() => expect(flip.checked).toBe(false));
    await waitFor(() => expect(flip.disabled).toBe(false));

    // The browser's first: if the server's half fails, this device is off and
    // says so, where the other order would leave the switch saying "on" over a
    // device nothing is ever sent to.
    expect(held.unsubscribe).toHaveBeenCalled();

    const [where, sent] = fetching.mock.calls[0]!;
    expect(where).toBe("/api/ui/push/unsubscribe");
    expect(JSON.parse(String(sent?.body))).toEqual({ endpoint: ENDPOINT });
  });

  it("offers nothing where there is no push to be had", async () => {
    // Nothing stubbed at all: jsdom has neither notifications nor a push
    // manager, which is the browser this is about.
    serving();

    const { flip } = await stalled("not available here");

    expect(flip.disabled).toBe(true);
    expect(flip.checked).toBe(false);
  });

  it("offers nothing over a connection the browser will not trust", async () => {
    pushing({ permission: "granted", secure: false });
    serving();

    const { flip } = await stalled("not available here");

    expect(flip.disabled).toBe(true);
  });

  it("says permission is a dead end rather than offering a flip that cannot help", async () => {
    pushing({ permission: "denied", subscription: subscription() });
    serving();

    const { container, flip } = await stalled("blocked for this device");

    // Denied over a subscription that is still there: the browser will show
    // nothing for it, so "on" would be a lie the human finds out about by
    // missing a Set.
    expect(flip.checked).toBe(false);
    expect(container.querySelector(".state")!.textContent).toContain(
      "allow them in its settings",
    );
  });

  it("leaves the offer standing when the prompt was dismissed", async () => {
    const { manager } = pushing({ permission: "default" });
    const fetching = serving();

    const { flip, state } = await control();
    flip.click();

    await waitFor(() => expect(flip.disabled).toBe(false));

    expect(flip.checked).toBe(false);
    // Nothing was decided, so nothing is said about it.
    expect(state()).toBe("");
    expect(manager.subscribe).not.toHaveBeenCalled();
    expect(fetching).not.toHaveBeenCalled();
  });

  it("becomes a dead end when the prompt was refused", async () => {
    pushing({ permission: "default", answers: "denied" });
    serving();

    const { flip, container } = await control();
    flip.click();

    await waitFor(() =>
      expect(container.querySelector(".state")!.textContent).toContain(
        "blocked for this device",
      ),
    );
    expect(flip.checked).toBe(false);
    expect(flip.disabled).toBe(true);
  });

  it("passes on what a browser said about a refused subscribe, with the way out", async () => {
    pushing({
      permission: "granted",
      refuses: "Registration failed - push service error",
    });
    serving(json({ key: KEY }));

    const { flip, state } = await control();
    flip.click();

    await waitFor(() => expect(state()).toContain("push service error"));
    // The one refusal neither another tap nor the site's own settings lead out
    // of, so the control names the setting that does.
    expect(state()).toContain("Use Google services for push messaging");

    // Back where the device actually is, rather than where the flip meant to
    // leave it — which is asked for once the control has looked again, because
    // that is the whole of the point.
    await waitFor(() => expect(flip.disabled).toBe(false));
    expect(flip.checked).toBe(false);
  });

  it("says so when the server would not take the subscription", async () => {
    pushing({ permission: "granted" });
    serving(json({ key: KEY }), json({ error: "the subscription could not be stored" }, 500));

    const { flip, state } = await control();
    flip.click();

    await waitFor(() => expect(state()).toContain("The server did not take it"));
    expect(state()).toContain("the subscription could not be stored");
  });

  it("hands over a subscription the browser already had rather than asking for a second", async () => {
    const { manager, held } = pushing({ permission: "granted" });
    const fetching = serving(json("Stored"));

    // Nothing subscribed when the control looked…
    const { flip } = await control();
    expect(flip.checked).toBe(false);

    // …and something by the time it was flipped: another tab, or a tap that
    // failed after the browser's half of it went through. Either way the server
    // may be the half that is missing it.
    held.subscription = subscription();

    flip.click();
    await waitFor(() => expect(flip.disabled).toBe(false));
    expect(flip.checked).toBe(true);

    // The browser would hand back the same subscription anyway, so asking it
    // for a second one is a round trip — and a key fetch — that buys nothing.
    expect(manager.subscribe).not.toHaveBeenCalled();
    expect(fetching.mock.calls[0]![0]).toBe("/api/ui/push/subscribe");
  });

  it("will not take a second flip while one is in flight", async () => {
    const { manager } = pushing({ permission: "granted" });
    // The key is held open, which is the middle of a subscribe: the switch has
    // moved, nothing has been decided, and a second tap must not start another.
    let deliver: () => void;
    const held = new Promise<void>((resolve) => {
      deliver = resolve;
    });

    serving(() => held.then(() => new Response(JSON.stringify({ key: KEY }))), json("Stored"));

    const { flip } = await control();
    flip.click();

    // The switch shows where the flip is going while it waits: subscribing is
    // four round trips, and a switch that snapped back for a second of it would
    // read as a flip that failed.
    await waitFor(() => expect(flip.checked).toBe(true));
    expect(flip.disabled).toBe(true);

    deliver!();
    await waitFor(() => expect(flip.disabled).toBe(false));
    expect(manager.subscribe).toHaveBeenCalledTimes(1);
  });

  it("says the server still has this device when only the browser's half came off", async () => {
    const held = subscription();
    pushing({ permission: "granted", subscription: held });
    serving(json({ error: "the subscription could not be forgotten" }, 500));

    const { flip, state } = await control();
    flip.click();

    await waitFor(() => expect(state()).toContain("the server still has it"));
    // The browser's half did come off, so this device is off and says so.
    expect(flip.checked).toBe(false);
  });
});
