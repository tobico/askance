//! The Archive: every Question Set that has been settled, newest first — the
//! ones that were answered, and the ones the human closed unanswered because
//! nobody was ever going to answer them.
//!
//! A permanent log. A Set lands here by being answered or by being archived
//! unanswered rather than by anyone filing it, and nothing here is ever deleted —
//! which is what the rows are worded for: no Liveness badge, because nothing is
//! waiting on a settled Set, and the date it was settled rather than how long ago
//! it was. A Set that was never answered says so, because reading it as a
//! decision would be reading a decision nobody made.

use leptos::prelude::*;
use leptos_router::components::A;

/// One row of the Archive as the browser receives it, and the wording of the
/// date on it — both `askance-render`'s, like the pending list's row.
pub use askance_render::{ArchiveEntry, settled_when};

/// The Sets that have been settled, newest first.
#[server]
pub async fn list_archive() -> Result<Vec<ArchiveEntry>, ServerFnError> {
    use askance_store::Settled;

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
            settled_at: settled_when(&set.settled_at),
            unanswered: set.settled == Settled::ArchivedUnanswered,
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
                        view! {
                            <p class="empty">"Nothing has been answered or archived yet."</p>
                        }
                            .into_any()
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

/// One line of the log: what was asked and where from, and the day it was
/// settled — with, when that is what happened, the fact that it was closed
/// without ever being answered.
///
/// Built like a pending row and styled as one — the lists are read the same way,
/// and the same Set may well have been looked at in both.
fn archive_row(entry: ArchiveEntry) -> impl IntoView {
    // In the same words the set view uses, and in the same place a decision's
    // date sits: what a row of this log has to say first is which of the two it
    // is, because only one of them is a decision.
    let (class, said) = if entry.unanswered {
        ("set-row archived-set unanswered", "archived unanswered ")
    } else {
        ("set-row archived-set", "answered ")
    };

    view! {
        <li class=class>
            <A href=format!("/sets/{}", entry.id)>
                <span class="title">{entry.title}</span>
                <span class="meta">
                    {entry.project.map(|project| view! { <span class="project">{project}</span> })}
                    {entry.branch.map(|branch| view! { <span class="branch">{branch}</span> })}
                    <span class="decided-at">{said} {entry.settled_at}</span>
                </span>
            </A>
        </li>
    }
}
