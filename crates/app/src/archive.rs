//! The Archive: every Question Set that has been answered, newest decision
//! first.
//!
//! A permanent decision log. A Set lands here by being answered rather than by
//! anyone filing it, and nothing here is ever deleted — which is what the rows
//! are worded for: no Liveness badge, because nothing is waiting on an answered
//! Set, and the date the decision was made rather than how long ago it was.

use leptos::prelude::*;
use leptos_router::components::A;
use serde::{Deserialize, Serialize};

/// One row of the Archive as the browser receives it.
///
/// The date arrives already worded, for the reason the pending list's ages do:
/// the server has the clock and the date library, and the browser is given
/// neither.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchiveEntry {
    pub id: i64,
    pub title: String,
    pub project: Option<String>,
    pub branch: Option<String>,
    pub answered_at: String,
}

/// The Sets that have been answered, newest decision first.
#[server]
pub async fn list_archive() -> Result<Vec<ArchiveEntry>, ServerFnError> {
    let pool: sqlx::SqlitePool = expect_context();

    let archived = askance_store::archived_sets(&pool)
        .await
        .map_err(|err| ServerFnError::new(format!("{err:#}")))?;

    Ok(archived
        .into_iter()
        .map(|set| ArchiveEntry {
            id: set.id,
            title: set.title,
            project: set.project,
            branch: set.branch,
            answered_at: crate::set_view::submitted_when(&set.answered_at),
        })
        .collect())
}

#[component]
pub fn Archive() -> impl IntoView {
    // Read once, unlike the pending list's ten-second refetch: nothing here is
    // waiting on the human, and a decision that has already been made does not
    // go stale while the page is open.
    let archive = Resource::new(|| (), |()| list_archive());

    view! {
        <A href="/" attr:class="back">"← Pending"</A>
        <h1>"Archive"</h1>
        <Suspense fallback=|| view! { <p class="empty">"Loading…"</p> }>
            {move || Suspend::new(async move {
                match archive.await {
                    Err(err) => {
                        view! {
                            <p class="error">"Could not read the Archive: " {err.to_string()}</p>
                        }
                            .into_any()
                    }
                    Ok(sets) if sets.is_empty() => {
                        view! { <p class="empty">"Nothing has been answered yet."</p> }.into_any()
                    }
                    Ok(sets) => {
                        view! {
                            <ul class="set-list">
                                {sets.into_iter().map(archive_row).collect_view()}
                            </ul>
                        }
                            .into_any()
                    }
                }
            })}
        </Suspense>
    }
}

/// One decision in the log: what was asked and where from, and the day it was
/// answered.
///
/// Built like a pending row and styled as one — the two lists are read the same
/// way, and the same Set may well have been looked at in both.
fn archive_row(entry: ArchiveEntry) -> impl IntoView {
    view! {
        <li class="set-row archived-set">
            <A href=format!("/sets/{}", entry.id)>
                <span class="title">{entry.title}</span>
                <span class="meta">
                    {entry.project.map(|project| view! { <span class="project">{project}</span> })}
                    {entry.branch.map(|branch| view! { <span class="branch">{branch}</span> })}
                    <span class="decided-at">"answered " {entry.answered_at}</span>
                </span>
            </A>
        </li>
    }
}
