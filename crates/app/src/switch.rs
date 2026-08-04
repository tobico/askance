//! The one on/off control in the UI, wherever a device setting is offered.
//!
//! A checkbox under the paint rather than a button that redraws itself: the
//! browser gives a checkbox the keyboard, the label association and the
//! focus ring for nothing, and `role="switch"` is the one word that tells a
//! screen reader it is a state and not a choice in a set.
//!
//! It says on or off and nothing else. Anything a switch cannot say — that this
//! browser has no push to offer, that a tap failed — is said in words beside it
//! by whoever is showing it, because only they know what to say.

use leptos::prelude::*;

/// A labelled switch.
///
/// `on` and `disabled` are read reactively, so the caller holds the state and
/// this draws it; `flip` is handed what the switch would become, which is the
/// one thing the caller cannot work out for itself while a change is in flight.
#[component]
pub fn Switch(
    /// The words beside it. They are the control's name, so they are also what
    /// a screen reader reads.
    label: &'static str,
    /// Whether it reads as on.
    #[prop(into)]
    on: Signal<bool>,
    /// Whether it will take a flip. A disabled switch still says where it
    /// stands — that is the whole of what it is for while it is waiting.
    #[prop(into, optional)]
    disabled: Signal<bool>,
    /// What to do about a flip, given the state being asked for.
    flip: impl Fn(bool) + 'static,
) -> impl IntoView {
    view! {
        <label class="switch">
            <span class="switch-label">{label}</span>
            <input
                type="checkbox"
                role="switch"
                prop:checked=move || on.get()
                prop:disabled=move || disabled.get()
                // The box the browser has just ticked, rather than the opposite
                // of what the signal held: a disabled flip never gets here, and
                // reading the element is the account that cannot disagree with
                // what the human sees.
                on:change:target=move |ev| flip(ev.target().checked())
            />
        </label>
    }
}
