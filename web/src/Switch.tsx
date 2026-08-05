//! The one on/off control in the UI, wherever a device setting is offered.
//!
//! A checkbox under the paint rather than a button that redraws itself: the
//! browser gives a checkbox the keyboard, the label association and the focus
//! ring for nothing, and `role="switch"` is the one word that tells a screen
//! reader it is a state and not a choice in a set.
//!
//! It says on or off and nothing else. Anything a switch cannot say — that this
//! browser has no push to offer, that a tap failed — is said in words beside it
//! by whoever is showing it, because only they know what to say.

import type { JSX } from "solid-js";

/// A labelled switch.
///
/// `on` and `disabled` are read from the props, so the caller holds the state
/// and this draws it; `flip` is handed what the switch would become, which is
/// the one thing the caller cannot work out for itself while a change is in
/// flight.
export function Switch(props: {
  /// The words beside it. They are the control's name, so they are also what a
  /// screen reader reads.
  label: string;
  /// Whether it reads as on.
  on: boolean;
  /// Whether it will take a flip. A disabled switch still says where it stands
  /// — that is the whole of what it is for while it is waiting.
  disabled?: boolean;
  /// What to do about a flip, given the state being asked for.
  flip: (on: boolean) => void;
}): JSX.Element {
  return (
    <label class="switch">
      <span class="switch-label">{props.label}</span>
      <input
        type="checkbox"
        role="switch"
        // Solid sets `checked` as a property rather than an attribute, which is
        // what a tick the human keeps changing wants: the attribute says only
        // what the box started as, and after one tap the two are different
        // things.
        checked={props.on}
        disabled={props.disabled}
        // The box the browser has just ticked, rather than the opposite of what
        // the caller held: a disabled flip never gets here, and reading the
        // element is the account that cannot disagree with what the human sees.
        onChange={(ev) => props.flip(ev.currentTarget.checked)}
      />
    </label>
  );
}
