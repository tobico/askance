//! The pending list: the Question Sets still waiting on the human, newest
//! first.

use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

/// One row of the pending list as the browser receives it.
///
/// The age arrives already worded rather than as a timestamp: the server has a
/// clock, and this way the browser needs neither one nor a date library to
/// draw the list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingEntry {
    pub id: i64,
    pub title: String,
    pub project: Option<String>,
    pub branch: Option<String>,
    pub age: String,
}

/// The Sets still waiting on the human, newest first.
#[server]
pub async fn list_pending() -> Result<Vec<PendingEntry>, ServerFnError> {
    let pool: sqlx::SqlitePool = expect_context();
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

    view! {
        <h1>"Pending"</h1>
        <Suspense fallback=|| view! { <p class="empty">"Loading…"</p> }>
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
                            <ul class="pending">{sets.into_iter().map(pending_row).collect_view()}</ul>
                        }
                            .into_any()
                    }
                }
            })}
        </Suspense>
    }
}

fn pending_row(entry: PendingEntry) -> impl IntoView {
    view! {
        <li class="pending-set">
            <span class="title">{entry.title}</span>
            <span class="meta">
                {entry.project.map(|project| view! { <span class="project">{project}</span> })}
                {entry.branch.map(|branch| view! { <span class="branch">{branch}</span> })}
                <span class="age">{entry.age}</span>
            </span>
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
