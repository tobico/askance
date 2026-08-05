//! Turning notifications on for the device in front of you: the switch that says
//! where this device stands, and the one flip that changes it.
//!
//! Per device, like the drafts in `localStorage`: the phone being subscribed
//! says nothing about the laptop. What the switch shows is read out of the
//! browser on every load rather than remembered — see [`look`].

import type { JSX } from "solid-js";
import { createSignal, onMount } from "solid-js";

import { Switch } from "../Switch";
import { disable, enable, look, said as saidAbout } from "./browser";
import type { Standing } from "./standing";
import { flippable, said } from "./standing";

/// The switch on the pending list: where this device stands, and the one flip
/// that changes it.
export function Notifications(): JSX.Element {
  const [standing, setStanding] = createSignal<Standing>("unknown");
  // Why a flip did not do what it offered to, in the browser's own words where
  // it had any. Cleared by the next flip: it describes that attempt and no
  // other.
  const [trouble, setTrouble] = createSignal<string | null>(null);
  // What a flip in flight is asking for, and `null` when none is. The
  // destination rather than a bare "busy", because that is what the switch shows
  // while it waits — subscribing is four round trips through the browser and a
  // push service, and a switch that snapped back to where it started for a
  // second of it would read as a flip that failed.
  const [wanted, setWanted] = createSignal<boolean | null>(null);

  // The browser's alone, which is why the control draws `unknown` first and this
  // is what replaces it.
  onMount(() => {
    void look().then(setStanding);
  });

  // Where the switch sits: what is being asked for while something is being
  // asked for, and otherwise where the device actually is. Every standing but
  // `on` reads as off — an unknown device is not a device that is on, and
  // neither is a blocked one.
  const on = () => wanted() ?? standing() === "on";

  // Nothing a flip could do, or nothing until the one in flight is done.
  const waiting = () => wanted() !== null || !flippable(standing());

  const flip = (on: boolean) => {
    if (wanted() !== null) {
      return;
    }
    setWanted(on);
    setTrouble(null);

    void (async () => {
      try {
        setStanding(await (on ? enable() : disable()));
      } catch (error) {
        setTrouble(saidAbout(error));
        // Where the device actually ended up, rather than where the flip meant
        // to leave it: a subscribe that failed halfway is exactly when the
        // switch must not be guessed at.
        setStanding(await look());
      } finally {
        setWanted(null);
      }
    })();
  };

  // The switch says on or off; the line under it says the things a switch cannot
  // — that this browser has none to offer, that permission is a dead end, that a
  // flip failed. It is always in the document, so that a screen reader has a live
  // region to announce into rather than a paragraph appearing beside it; the
  // stylesheet takes an empty one out of the layout.
  return (
    <section class="notifications">
      <Switch
        label="Push notifications"
        on={on()}
        disabled={waiting()}
        flip={flip}
      />
      <p class="state" aria-live="polite">
        {trouble() ?? said(standing())}
      </p>
    </section>
  );
}
