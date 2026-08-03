//! The pending list: the Question Sets still waiting on the human, newest
//! first.

use std::time::Duration;

use askance_schema::Liveness;
use leptos::prelude::*;
use leptos_router::components::A;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

/// How often the open page refetches the list.
///
/// The badge has to keep up with an agent that has died, and while the page is
/// asking it also keeps the ages honest and brings a newly arrived Set into view
/// — which is what stands in for push notifications until there are any.
const REFRESH: Duration = Duration::from_secs(10);

/// One row of the pending list as the browser receives it.
///
/// The age and the Liveness arrive already decided rather than as timestamps:
/// the server has the clock and the registry of held waits, and this way the
/// browser needs neither one nor a date library to draw the list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingEntry {
    pub id: i64,
    pub title: String,
    pub project: Option<String>,
    pub branch: Option<String>,
    pub age: String,
    pub liveness: Liveness,
}

/// The Sets still waiting on the human, newest first.
#[server]
pub async fn list_pending() -> Result<Vec<PendingEntry>, ServerFnError> {
    let pool: sqlx::SqlitePool = expect_context();
    let waits: askance_store::Waits = expect_context();
    let now = OffsetDateTime::now_utc();

    let pending = askance_store::pending_sets(&pool)
        .await
        .map_err(|err| ServerFnError::new(format!("{err:#}")))?;

    Ok(pending
        .into_iter()
        .map(|set| PendingEntry {
            id: set.id,
            title: set.title,
            project: set.project,
            branch: set.branch,
            age: relative_age(&set.created_at, now),
            liveness: waits.liveness(set.id, &set.created_at, now),
        })
        .collect())
}

/// How long ago `created_at` was, in the roughest unit that still says
/// something: a pending list is scanned, not read.
///
/// A timestamp that will not parse is not worth failing the page over — the
/// list is still useful without an age — so it comes back empty.
pub fn relative_age(created_at: &str, now: OffsetDateTime) -> String {
    let Ok(then) = OffsetDateTime::parse(created_at, &Rfc3339) else {
        return String::new();
    };

    let seconds = (now - then).whole_seconds();
    if seconds < 60 {
        return "just now".to_owned();
    }

    let minutes = seconds / 60;
    if minutes < 60 {
        return format!("{minutes}m ago");
    }

    let hours = minutes / 60;
    if hours < 24 {
        return format!("{hours}h ago");
    }

    format!("{}d ago", hours / 24)
}

#[component]
pub fn PendingList() -> impl IntoView {
    let pending = Resource::new(|| (), |()| list_pending());

    // The browser's alone: an effect never runs during SSR, so the server draws
    // the list once and only an open page goes back for it. The interval is
    // cleared as the list is unmounted, so walking into a Set does not leave one
    // refetching behind it.
    Effect::new(move |_| {
        // No interval to be had is the page as it was before there was one: a
        // list that is current as of its load, which is not worth an error.
        if let Ok(refresh) = set_interval_with_handle(move || pending.refetch(), REFRESH) {
            on_cleanup(move || refresh.clear());
        }
    });

    view! {
        // The way through to what was already decided, in the slot the set
        // view's "← Pending" sits in: every page here starts with where else
        // there is to go, so neither list needs a typed URL to reach the other.
        <A href="/archive" attr:class="to-archive">"Archive →"</A>
        <h1>"Pending"</h1>
        // Above the list rather than buried in a settings page: this is the one
        // page that is open often enough to be where the human notices that the
        // phone is not being told, and there is nowhere else to put it.
        <crate::push::Notifications />
        // A Transition rather than a Suspense: the fallback belongs to the first
        // load, and a refetch every ten seconds must not blink the list away.
        <Transition fallback=|| view! { <p class="empty">"Loading…"</p> }>
            {move || Suspend::new(async move {
                match pending.await {
                    Err(err) => {
                        view! {
                            <p class="error">"Could not read the pending Sets: " {err.to_string()}</p>
                        }
                            .into_any()
                    }
                    Ok(sets) if sets.is_empty() => {
                        view! { <p class="empty">"Nothing is waiting on you."</p> }.into_any()
                    }
                    Ok(sets) => {
                        view! {
                            <ul class="set-list">
                                {sets.into_iter().map(pending_row).collect_view()}
                            </ul>
                        }
                            .into_any()
                    }
                }
            })}
        </Transition>
    }
}

/// What the badge says, and the word that colours it.
///
/// Worded about the agent rather than about the connection: what the human wants
/// to know before answering is whether anyone is still on the other end.
///
/// Shared with the set view, which badges the Set it is about to be archived
/// from: the same state has to read as the same words wherever it is met.
pub(crate) fn badge(liveness: Liveness) -> (&'static str, &'static str) {
    match liveness {
        Liveness::Waiting => ("waiting", "agent waiting"),
        Liveness::Disconnected => ("disconnected", "agent disconnected"),
    }
}

fn pending_row(entry: PendingEntry) -> impl IntoView {
    let (state, liveness) = badge(entry.liveness);

    // The whole row is the link: on a phone the tap target should be the card,
    // not the title inside it.
    view! {
        <li class="set-row pending-set">
            <A href=format!("/sets/{}", entry.id)>
                <span class="title">{entry.title}</span>
                <span class="meta">
                    {entry.project.map(|project| view! { <span class="project">{project}</span> })}
                    {entry.branch.map(|branch| view! { <span class="branch">{branch}</span> })}
                    <span class=format!("liveness {state}")>{liveness}</span>
                    <span class="age">{entry.age}</span>
                </span>
            </A>
        </li>
    }
}

#[cfg(test)]
mod tests {
    use super::relative_age;
    use time::OffsetDateTime;
    use time::format_description::well_known::Rfc3339;

    fn at(stamp: &str) -> OffsetDateTime {
        OffsetDateTime::parse(stamp, &Rfc3339).unwrap()
    }

    #[test]
    fn ages_are_worded_in_the_roughest_useful_unit() {
        let now = at("2026-08-03T12:00:00.000Z");

        assert_eq!(relative_age("2026-08-03T11:59:31.000Z", now), "just now");
        assert_eq!(relative_age("2026-08-03T11:52:00.000Z", now), "8m ago");
        assert_eq!(relative_age("2026-08-03T09:00:00.000Z", now), "3h ago");
        assert_eq!(relative_age("2026-07-31T12:00:00.000Z", now), "3d ago");
    }

    #[test]
    fn an_unparseable_stamp_costs_only_its_age() {
        let now = at("2026-08-03T12:00:00.000Z");

        assert_eq!(relative_age("not a timestamp", now), "");
    }
}
