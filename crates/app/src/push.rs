//! Turning notifications on for the device in front of you: the switch that says
//! where this device stands, the server's public key it subscribes against, and
//! where the subscription the browser hands back is sent.
//!
//! Per device, like the drafts in `localStorage`: the phone being subscribed says
//! nothing about the laptop. Nothing here is remembered between visits either —
//! what the control says is read out of the browser on every load, because an
//! installed app reopened a week later must not offer to enable what is already
//! enabled, and the browser is the only thing that knows.

use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};

/// A subscription as `PushManager.subscribe` describes one, flattened to the
/// three things a push needs: where to send it, and the two keys it is
/// encrypted for.
///
/// Flattened rather than passed through as the browser's own JSON, because the
/// nesting it uses — `keys.p256dh`, `keys.auth` — is the browser's shape and not
/// something the server has any reason to learn.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Subscription {
    pub endpoint: String,
    pub p256dh: String,
    pub auth: String,
}

/// The public half of the server's VAPID keypair, base64url-encoded from the
/// uncompressed point — what `PushManager.subscribe` takes as its
/// `applicationServerKey`.
///
/// The private half stays on the server: this is only how a browser names the
/// server it is subscribing to.
///
/// The path is spelled out rather than left to the macro's default so it is
/// legible in a log beside `/api/v1/`, which the agents use.
#[server(prefix = "/api/ui", endpoint = "push-key")]
pub async fn push_public_key() -> Result<String, ServerFnError> {
    let pool: sqlx::SqlitePool = expect_context();

    let keys = askance_store::vapid_keys(&pool)
        .await
        .map_err(|err| ServerFnError::new(format!("{err:#}")))?;

    Ok(keys.public_key)
}

/// What became of a device asking to be notified.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Subscribed {
    /// This device will be told about a Set from now on. It is stored once
    /// however many times it subscribes.
    Stored,

    /// Refused: the browser handed over a subscription with no endpoint, or
    /// missing a key. Nothing could ever be sent to it, so nothing was stored.
    Incomplete,
}

/// Take a device's subscription, so a Set arriving can reach it.
#[server(prefix = "/api/ui", endpoint = "subscribe-push", input = server_fn::codec::Json)]
pub async fn subscribe_push(subscription: Subscription) -> Result<Subscribed, ServerFnError> {
    use askance_store::{PushSubscription, Subscribing};

    let pool: sqlx::SqlitePool = expect_context();

    let subscribing = askance_store::store_subscription(
        &pool,
        &PushSubscription {
            endpoint: subscription.endpoint,
            p256dh: subscription.p256dh,
            auth: subscription.auth,
        },
    )
    .await
    .map_err(|err| ServerFnError::new(format!("{err:#}")))?;

    Ok(match subscribing {
        Subscribing::Stored => Subscribed::Stored,
        Subscribing::Incomplete => Subscribed::Incomplete,
    })
}

/// Forget a device, because it asked not to be told any more.
///
/// Named by its endpoint, which is the only name a subscription has. An endpoint
/// the server never stored is not an error: a browser can drop its own
/// subscription without the server having heard of it, and afterwards what was
/// asked for holds either way — nothing is sent there.
#[server(prefix = "/api/ui", endpoint = "unsubscribe-push", input = server_fn::codec::Json)]
pub async fn unsubscribe_push(endpoint: String) -> Result<(), ServerFnError> {
    let pool: sqlx::SqlitePool = expect_context();

    askance_store::forget_subscription(&pool, &endpoint)
        .await
        .map_err(|err| ServerFnError::new(format!("{err:#}")))
}

/// Where notifications stand on the device the page is open on.
///
/// Every one of these is somewhere a browser can leave a device, and the control
/// says which: an offer to turn something on that is already on, or that this
/// browser will never allow, is worse than saying so.
///
/// Only a browser ever reaches any of them but the first — the server renders the
/// control unlooked-at — so under `ssr` these are states nothing here can put it
/// in, rather than states that never happen.
#[cfg_attr(not(feature = "hydrate"), allow(dead_code))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Standing {
    /// Not looked yet. What the server renders — only a browser knows — and what
    /// a browser whose service worker never takes control keeps saying.
    Unknown,

    /// Push is not to be had here at all: no service worker, no push manager, or
    /// a page outside a secure context. Nothing to offer, so nothing is offered.
    Unavailable,

    /// Permission was refused. A dead end: the browser will not ask again, so the
    /// way out is its own site settings and not a tap here.
    Blocked,

    /// Available, and this device has not asked to be told.
    Off,

    /// This device has asked to be told, and can be asked to stop.
    On,
}

/// What a browser's answer about notification permission comes to. The three
/// states `Notification.permission` has, under names that say what they mean
/// here.
///
/// A browser's answer, so the `ssr` build never asks for one — see [`Standing`].
#[cfg_attr(not(feature = "hydrate"), allow(dead_code))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Permission {
    /// Never asked, or asked and dismissed without an answer.
    Undecided,
    Granted,
    Denied,
}

/// Where a device with push to be had stands, from what the browser says about
/// it.
///
/// Denied beats a subscription that is still there, rather than the other way
/// about: a browser told not to show notifications will not show them for a
/// subscription it kept, so calling that "on" would be a lie the human only
/// finds out about by missing a Set.
///
/// Kept out of the browser module so that the one piece of judgement in the
/// control can be tested on the host, where there is no browser to ask.
#[cfg_attr(not(feature = "hydrate"), allow(dead_code))]
fn standing(permission: Permission, subscribed: bool) -> Standing {
    match (permission, subscribed) {
        (Permission::Denied, _) => Standing::Blocked,
        (_, true) => Standing::On,
        (_, false) => Standing::Off,
    }
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
///
/// Kept out of the browser module, like [`standing`], so the judgement in it can
/// be tested on the host where there is no browser to refuse.
#[cfg_attr(not(feature = "hydrate"), allow(dead_code))]
fn refusal(said: String) -> String {
    if !said.to_lowercase().contains("push service") {
        return said;
    }

    format!(
        "{said} — a browser that de-Googles, Brave among them, refuses this \
         until \"Use Google services for push messaging\" is turned on in its \
         privacy settings and it has been restarted."
    )
}

/// The switch on the pending list: where this device stands, and the one flip
/// that changes it.
#[component]
pub fn Notifications() -> impl IntoView {
    let standing = RwSignal::new(Standing::Unknown);
    // Why a flip did not do what it offered to, in the browser's own words where
    // it had any. Cleared by the next flip: it describes that attempt and no
    // other.
    let trouble = RwSignal::new(None::<String>);
    // What a flip in flight is asking for, and `None` when none is. The
    // destination rather than a bare "busy", because that is what the switch
    // shows while it waits — subscribing is four round trips through the browser
    // and a push service, and a switch that snapped back to where it started for
    // a second of it would read as a flip that failed.
    let wanted = RwSignal::new(None::<bool>);

    // The browser's alone: an effect never runs during SSR, which is why the
    // server renders `Unknown` and this is what replaces it.
    Effect::new(move |_| {
        spawn_local(async move { standing.set(look().await) });
    });

    // Where the switch sits: what is being asked for while something is being
    // asked for, and otherwise where the device actually is. Every standing but
    // `On` reads as off — an unknown device is not a device that is on, and
    // neither is a blocked one.
    let on = Signal::derive(move || match wanted.get() {
        Some(wanted) => wanted,
        None => standing.get() == Standing::On,
    });

    // Nothing a flip could do, or nothing until the one in flight is done.
    let waiting = Signal::derive(move || wanted.get().is_some() || !flippable(standing.get()));

    let flip = move |on: bool| {
        if wanted.get().is_some() {
            return;
        }
        wanted.set(Some(on));
        trouble.set(None);

        spawn_local(async move {
            match if on { enable().await } else { disable().await } {
                Ok(now) => standing.set(now),
                Err(said) => {
                    trouble.set(Some(said));
                    // Where the device actually ended up, rather than where the
                    // flip meant to leave it: a subscribe that failed halfway is
                    // exactly when the switch must not be guessed at.
                    standing.set(look().await);
                }
            }
            wanted.set(None);
        });
    };

    // The switch says on or off; the line under it says the things a switch
    // cannot — that this browser has none to offer, that permission is a dead
    // end, that a flip failed. It is always in the document, so that a screen
    // reader has a live region to announce into rather than a paragraph
    // appearing beside it; the stylesheet takes an empty one out of the layout.
    view! {
        <section class="notifications">
            <crate::switch::Switch label="Push notifications" on=on disabled=waiting flip=flip />
            <p class="state" aria-live="polite">
                {move || trouble.get().or_else(|| said(standing.get()).map(str::to_owned))}
            </p>
        </section>
    }
}

/// What has to be said in words, because the switch cannot say it — and `None`
/// wherever the switch already does.
///
/// A device that is on or off is not written about at all: the switch is the
/// whole of the answer, and a sentence restating it would be one more thing to
/// read on the page that is opened most.
fn said(standing: Standing) -> Option<&'static str> {
    match standing {
        // Both halves of it, because the page cannot tell which: a browser
        // without push, or one withholding it because the connection is not
        // secure. Over the tailnet the second is what `tailscale serve` is for.
        Standing::Unavailable => Some(
            "Notifications are not available here — this browser has no push \
             support, or the page is not being served over https.",
        ),
        Standing::Blocked => Some(
            "Notifications are blocked for this device. This browser will not ask \
             again, so allow them in its settings for this site.",
        ),
        // Still looking says nothing rather than saying so: the switch is
        // disabled for the moment it takes, and a sentence that appears only to
        // be replaced is worse than a switch that waits.
        Standing::Unknown | Standing::Off | Standing::On => None,
    }
}

/// Whether the switch will take a flip here.
///
/// Everywhere else is somewhere a flip could not help: still looking, never
/// going to work, or a dead end only the browser's own settings lead out of.
fn flippable(standing: Standing) -> bool {
    matches!(standing, Standing::Off | Standing::On)
}

// What follows is the browser half: the push manager lives on the service
// worker's registration, and neither exists on the server. The effects that
// reach for them only ever run in a browser, so the `ssr` build gets stubs — the
// control renders there as what it renders before anything is known.
#[cfg(feature = "hydrate")]
mod browser {
    use super::{Permission, Standing, Subscribed, Subscription, refusal, standing};
    use wasm_bindgen::{JsCast, JsValue};
    use wasm_bindgen_futures::JsFuture;

    /// Where this device stands, read out of the browser rather than remembered.
    ///
    /// Asks nothing of the human: this runs on every load, and a page that
    /// prompted for permission just by being opened would be answered "no" once
    /// and never get another chance.
    pub(super) async fn look() -> Standing {
        let Some(manager) = push_manager().await else {
            return Standing::Unavailable;
        };

        standing(permission(), current(&manager).await.is_some())
    }

    /// Ask to be told, and hand the resulting subscription to the server.
    pub(super) async fn enable() -> Result<Standing, String> {
        // The prompt goes up before anything at all is awaited: a phone gives
        // the permission prompt to a tap and to nothing else, and every await
        // ahead of it is a chance for the browser to decide the tap is over. The
        // control only offers this where push was found, so nothing is being
        // asked for that could not then be granted.
        //
        // Asked here rather than left to `subscribe` to trigger, so that a
        // refusal is reported as the dead end it is instead of as a subscribe
        // that failed.
        match request_permission().await {
            Permission::Denied => return Ok(Standing::Blocked),
            // Dismissed without an answer: nothing was decided, so nothing is
            // said about it and the offer stands.
            Permission::Undecided => return Ok(Standing::Off),
            Permission::Granted => {}
        }

        let Some(manager) = push_manager().await else {
            return Ok(Standing::Unavailable);
        };

        // A subscription already here is used as it is. The browser hands back
        // the same one anyway, and the server may be the half that is missing it
        // — a tap that failed after subscribing, or a database restored from
        // before it.
        let subscription = match current(&manager).await {
            Some(subscription) => subscription,
            None => subscribe(&manager).await?,
        };

        let described = described(&subscription).ok_or_else(|| {
            "The browser gave a subscription with no endpoint or key in it, so \
             nothing could be sent to this device."
                .to_owned()
        })?;

        match super::subscribe_push(described).await {
            Ok(Subscribed::Stored) => Ok(Standing::On),
            Ok(Subscribed::Incomplete) => Err("The server refused this device's \
                                               subscription as incomplete."
                .to_owned()),
            Err(err) => Err(format!("The server did not take it: {err}")),
        }
    }

    /// Ask not to be told any more, here.
    ///
    /// The browser's subscription goes first and the server's row second: if the
    /// second half fails, this device is off and says so, and the endpoint the
    /// server kept is one no push can be delivered to. The other order would
    /// leave the control saying "on" over a device nothing is ever sent to.
    pub(super) async fn disable() -> Result<Standing, String> {
        let Some(manager) = push_manager().await else {
            return Ok(Standing::Unavailable);
        };

        let Some(subscription) = current(&manager).await else {
            // Nothing to turn off — the browser dropped it, or another tab did.
            return Ok(standing(permission(), false));
        };
        let endpoint = subscription.endpoint();

        let dropped = subscription.unsubscribe().map_err(js_said)?;
        JsFuture::from(dropped).await.map_err(js_said)?;

        super::unsubscribe_push(endpoint)
            .await
            .map_err(|err| format!("This device stopped, but the server still has it: {err}"))?;

        Ok(standing(permission(), false))
    }

    /// This page's push manager, or `None` when push is not to be had here.
    async fn push_manager() -> Option<web_sys::PushManager> {
        let window = web_sys::window()?;

        // Service workers, push and notifications are all withheld outside a
        // secure context, so a page served over plain HTTP from anything but
        // localhost has none of them — which over the tailnet is what
        // `tailscale serve`'s certificate is for.
        if !window.is_secure_context() {
            return None;
        }

        // Asked of the globals rather than of the objects: reading `permission`
        // off a `Notification` that is not there throws rather than coming back
        // undefined, and a registration without a push manager would only fail
        // later, on a tap.
        if !has_global(&window, "Notification") || !has_global(&window, "PushManager") {
            return None;
        }

        let container = window.navigator().service_worker();
        if container.is_undefined() {
            return None;
        }

        // `ready` rather than `getRegistration`: the worker is registered as the
        // page hydrates, and this is what waits for it to be in control instead
        // of racing it. A registration that never succeeds leaves this pending,
        // and the control still saying it is looking — which is the truth.
        let registration = JsFuture::from(container.ready().ok()?)
            .await
            .ok()?
            .dyn_into::<web_sys::ServiceWorkerRegistration>()
            .ok()?;

        registration.push_manager().ok()
    }

    /// This device's subscription, if it has one. `None` covers both a browser
    /// that has none and one that would not say.
    async fn current(manager: &web_sys::PushManager) -> Option<web_sys::PushSubscription> {
        let subscription = JsFuture::from(manager.get_subscription().ok()?)
            .await
            .ok()?;

        // Null when there is none, which `dyn_into` refuses along with anything
        // else unexpected.
        subscription.dyn_into().ok()
    }

    /// Subscribe this device against the server's public key.
    async fn subscribe(
        manager: &web_sys::PushManager,
    ) -> Result<web_sys::PushSubscription, String> {
        let key = super::push_public_key()
            .await
            .map_err(|err| format!("The server's push key could not be read: {err}"))?;

        let options = web_sys::PushSubscriptionOptionsInit::new();
        // Every push this server sends shows a notification, and Chrome refuses
        // to subscribe without the promise that it will.
        options.set_user_visible_only(true);
        // The base64url string rather than bytes: `applicationServerKey` takes
        // either, and the encoding the server hands out is already this one.
        options.set_application_server_key(&JsValue::from_str(&key));

        // Both halves through `refusal`: a browser with the push service off
        // refuses this asynchronously, but a throw from the call itself is the
        // same obstacle and deserves the same way out.
        let subscribing = manager
            .subscribe_with_options(&options)
            .map_err(|err| refusal(js_said(err)))?;

        JsFuture::from(subscribing)
            .await
            .map_err(|err| refusal(js_said(err)))?
            .dyn_into()
            .map_err(|_| "The browser subscribed to something other than push.".to_owned())
    }

    /// A browser's subscription as the server takes one.
    ///
    /// Through `toJSON` rather than `getKey`: its keys are base64url already,
    /// which is the encoding the server stores and a push is encrypted with,
    /// where `getKey` hands back buffers that would have to be encoded here.
    fn described(subscription: &web_sys::PushSubscription) -> Option<Subscription> {
        let json = subscription.to_json().ok()?;
        let keys = json.get_keys()?;

        Some(Subscription {
            endpoint: json.get_endpoint()?,
            p256dh: keys.get_p256dh()?,
            auth: keys.get_auth()?,
        })
    }

    /// What this browser has already been told about notifications for this site.
    fn permission() -> Permission {
        match web_sys::Notification::permission() {
            web_sys::NotificationPermission::Granted => Permission::Granted,
            web_sys::NotificationPermission::Denied => Permission::Denied,
            // `default`, and anything a later browser adds: nothing has been
            // decided, so the human is the one to decide it.
            _ => Permission::Undecided,
        }
    }

    /// Put the browser's own permission prompt to the human.
    ///
    /// A prompt that could not be raised at all reads as undecided rather than as
    /// a refusal: nothing was asked, so nothing was refused, and the offer to try
    /// again is honest.
    async fn request_permission() -> Permission {
        let Ok(asking) = web_sys::Notification::request_permission() else {
            return Permission::Undecided;
        };
        let Ok(answer) = JsFuture::from(asking).await else {
            return Permission::Undecided;
        };

        match answer.as_string().as_deref() {
            Some("granted") => Permission::Granted,
            Some("denied") => Permission::Denied,
            _ => Permission::Undecided,
        }
    }

    /// Whether the browser has this global at all — the feature test for an API
    /// that is simply missing, rather than present and refusing.
    fn has_global(window: &web_sys::Window, name: &str) -> bool {
        js_sys::Reflect::has(window, &JsValue::from_str(name)).unwrap_or(false)
    }

    /// What went wrong, in the browser's words where it had any: they name the
    /// actual obstacle — an insecure origin, a key the push service rejected — in
    /// more detail than anything here could guess at.
    fn js_said(err: JsValue) -> String {
        err.dyn_ref::<js_sys::Error>()
            .map(|error| String::from(error.message()))
            .or_else(|| err.as_string())
            .unwrap_or_else(|| "The browser refused, without saying why.".to_owned())
    }
}

#[cfg(feature = "hydrate")]
use browser::{disable, enable, look};

// The server renders the control as unlooked-at, and never runs the effect that
// would look: these stand in for a browser the `ssr` build has no way to reach.
#[cfg(not(feature = "hydrate"))]
async fn look() -> Standing {
    Standing::Unknown
}

#[cfg(not(feature = "hydrate"))]
async fn enable() -> Result<Standing, String> {
    Ok(Standing::Unknown)
}

#[cfg(not(feature = "hydrate"))]
async fn disable() -> Result<Standing, String> {
    Ok(Standing::Unknown)
}

#[cfg(test)]
mod tests {
    use super::{Permission, Standing, flippable, refusal, said, standing};

    #[test]
    fn a_subscribed_device_is_on_and_can_be_turned_off() {
        assert_eq!(standing(Permission::Granted, true), Standing::On);
        assert!(flippable(Standing::On));
    }

    #[test]
    fn an_unsubscribed_device_is_off_and_can_be_turned_on() {
        assert_eq!(standing(Permission::Granted, false), Standing::Off);
        assert_eq!(standing(Permission::Undecided, false), Standing::Off);
        assert!(flippable(Standing::Off));
    }

    #[test]
    fn a_device_the_switch_speaks_for_is_not_also_written_about() {
        // The switch says on and off, so a line saying either would be the same
        // answer twice — and "still looking" would be a line that exists only to
        // be replaced a moment later.
        for silent in [Standing::On, Standing::Off, Standing::Unknown] {
            assert_eq!(said(silent), None, "{silent:?} was written about");
        }
    }

    #[test]
    fn a_device_the_switch_cannot_speak_for_says_why_in_words() {
        for spoken in [Standing::Unavailable, Standing::Blocked] {
            assert!(said(spoken).is_some(), "{spoken:?} went unexplained");
        }
    }

    #[test]
    fn a_refused_permission_is_a_dead_end_however_it_was_left() {
        // Denied with a subscription still on it is a device that will be sent
        // to and show nothing. Saying "on" there would be found out by a missed
        // Set; saying "blocked" is found out by reading the control.
        assert_eq!(standing(Permission::Denied, true), Standing::Blocked);
        assert_eq!(standing(Permission::Denied, false), Standing::Blocked);
    }

    #[test]
    fn nothing_a_flip_could_do_leaves_the_switch_unflippable() {
        // Including `Unknown`, which is what the server renders: a switch that
        // took a flip before the browser had said where the device stands would
        // be acting on a state nobody had established.
        for nothing in [Standing::Unknown, Standing::Unavailable, Standing::Blocked] {
            assert!(!flippable(nothing), "{nothing:?} took a flip");
        }
    }

    #[test]
    fn a_push_service_refused_names_the_setting_that_allows_it() {
        // Chromium's own words for it, which is all a de-Googled build says.
        let said = refusal("Registration failed - push service error".to_owned());

        // The browser's account survives: it is the half that names the
        // obstacle, and the hint is only the way out of it.
        assert!(said.starts_with("Registration failed - push service error"));
        assert!(said.contains("Use Google services for push messaging"));
    }

    #[test]
    fn a_push_service_missing_altogether_gets_the_same_way_out() {
        // The other wording Chromium has for a push service it cannot use.
        let said = refusal("Registration failed - push service not available".to_owned());

        assert!(said.contains("Use Google services for push messaging"));
    }

    #[test]
    fn any_other_refusal_is_passed_on_as_the_browser_put_it() {
        // Nothing to do with the push service, so the hint would be a wrong
        // guess at the obstacle rather than help with it.
        for unrelated in [
            "Registration failed - permission denied",
            "The provided applicationServerKey is not valid.",
            "The browser refused, without saying why.",
        ] {
            assert_eq!(refusal(unrelated.to_owned()), unrelated);
        }
    }
}
