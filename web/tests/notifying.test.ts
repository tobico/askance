//! Where a device stands about notifications, and what has to be said about it.
//!
//! The judgement, kept apart from the browser it is made about: these assertions
//! belong wherever the decision is made rather than wherever the browser is
//! asked.

import { describe, expect, it } from "vitest";

import { flippable, refusal, said, standing } from "../src/push/standing";

describe("where a device stands", () => {
  it("is on, and can be turned off, once it has subscribed", () => {
    expect(standing("granted", true)).toBe("on");
    expect(flippable("on")).toBe(true);
  });

  it("is off, and can be turned on, until it has", () => {
    expect(standing("granted", false)).toBe("off");
    expect(standing("undecided", false)).toBe("off");
    expect(flippable("off")).toBe(true);
  });

  it("is a dead end wherever permission was refused, however it was left", () => {
    // Denied with a subscription still on it is a device that will be sent to
    // and show nothing. Saying "on" there would be found out by a missed Set;
    // saying "blocked" is found out by reading the control.
    expect(standing("denied", true)).toBe("blocked");
    expect(standing("denied", false)).toBe("blocked");
  });

  it("takes no flip anywhere a flip could not help", () => {
    // Including `unknown`, which is what the control draws before the browser
    // has answered: a switch that took a flip before the device's standing was
    // established would be acting on a state nobody had.
    for (const nothing of ["unknown", "unavailable", "blocked"] as const) {
      expect(flippable(nothing), `${nothing} took a flip`).toBe(false);
    }
  });
});

describe("what the control says in words", () => {
  it("says nothing about a device the switch already speaks for", () => {
    // The switch says on and off, so a line saying either would be the same
    // answer twice — and "still looking" would be a line that exists only to be
    // replaced a moment later.
    for (const silent of ["on", "off", "unknown"] as const) {
      expect(said(silent), `${silent} was written about`).toBeNull();
    }
  });

  it("says why, where the switch cannot", () => {
    for (const spoken of ["unavailable", "blocked"] as const) {
      expect(said(spoken), `${spoken} went unexplained`).not.toBeNull();
    }
  });
});

describe("a refused subscribe", () => {
  it("names the setting that allows it when the push service is the obstacle", () => {
    // Chromium's own words for it, which is all a de-Googled build says.
    const refused = refusal("Registration failed - push service error");

    // The browser's account survives: it is the half that names the obstacle,
    // and the hint is only the way out of it.
    expect(refused.startsWith("Registration failed - push service error")).toBe(
      true,
    );
    expect(refused).toContain("Use Google services for push messaging");
  });

  it("gets the same way out when the push service is missing altogether", () => {
    // The other wording Chromium has for a push service it cannot use.
    expect(refusal("Registration failed - push service not available")).toContain(
      "Use Google services for push messaging",
    );
  });

  it("is passed on as the browser put it when it is about anything else", () => {
    // Nothing to do with the push service, so the hint would be a wrong guess
    // at the obstacle rather than help with it.
    for (const unrelated of [
      "Registration failed - permission denied",
      "The provided applicationServerKey is not valid.",
      "The browser refused, without saying why.",
    ]) {
      expect(refusal(unrelated)).toBe(unrelated);
    }
  });
});
