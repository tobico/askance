//! The set view: one Question Set laid out to be answered — its Preface, then
//! every Question and Sub-question in the order the agent asked them, and the
//! submit that ends the agent's wait.
//!
//! The Response this page builds is explicit rather than complete: every
//! question gets an entry, and one the human left alone becomes an Unanswered
//! marker rather than being left out. Leaving a question open is a thing the
//! human is allowed to do — it just has to be said out loud, and for a question
//! that offered Options the warning before submit is where they say it. A
//! free-text question goes back open without one: there was no offered choice
//! to overlook, so skipping it reads as deliberate on its own.
//!
//! A Set that has already been answered gets the same page read rather than
//! filled in: its own material above, and under it what was decided — the
//! Option chosen beside the one the agent recommended, whatever was written,
//! and the questions that went back open. A Set is answered once, so there is
//! nothing here to press. Which of the two the page draws is decided from the
//! Set as the server loads it, so an answered Set never flashes a form.
//!
//! A Set still waiting also says whether anyone is still on the other end, and
//! offers the one thing that is not answering it: archiving it unanswered, for
//! the Set whose agent is gone for good. That belongs here rather than on a list
//! row because it is irreversible, and the Questions and the badge are what the
//! decision is made with. A Set that was archived reads like an answered one —
//! permanently, and with nothing to press — except that there is no Response to
//! show, because there never was one.

use askance_schema::{Answer, Liveness, Question, QuestionOption, Response};
use leptos::prelude::*;
use leptos_router::components::A;
use leptos_router::hooks::{use_navigate, use_params_map};
use serde::{Deserialize, Serialize};
use time::format_description::well_known::Rfc3339;
use time::{OffsetDateTime, UtcOffset};

/// One Question Set as the browser receives it.
///
/// The Preface and the Diff arrive as HTML rather than as their sources: the
/// server has the markdown parser and the diff highlighter, and this way the
/// browser needs neither. The Questions arrive exactly as the agent sent them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetView {
    pub id: i64,
    pub title: String,
    pub project: Option<String>,
    pub branch: Option<String>,
    pub preface_html: Option<String>,
    pub diff: Option<DiffView>,
    pub questions: Vec<Question>,

    /// Where the Set stands. It decides whether this page is a form or a record,
    /// so it travels with the Set rather than being fetched once the page is
    /// already up.
    pub standing: Standing,
}

/// The Diff as the browser receives it: the HTML the server rendered, and the
/// path of each file in it, in Diff order — `paths[0]` is what `#diff-1` shows.
///
/// The two travel together rather than as two fields on the Set, because they
/// describe the same thing and the table of contents is built from both: the
/// nav names the folds from the paths and jumps by their positions, so a nav
/// out of step with the markup would jump to the wrong file. Reading the paths
/// back out of the rendered HTML instead would mean shipping a parser to do it
/// with, and would make the nav a description of the page rather than of the
/// Set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiffView {
    pub html: String,
    pub paths: Vec<String>,
}

/// How a Set stands: still waiting on the human, answered, or closed unanswered.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Standing {
    /// Waiting on the human, with what the server can say about the agent on the
    /// other end. Display only (ADR-0001): it is the human who decides what a
    /// disconnected agent means.
    Waiting(Liveness),

    /// Answered: what the human decided, and when.
    Answered(Answered),

    /// Archived unanswered by the human, at this time. No Response was ever sent
    /// and none ever will be.
    ArchivedUnanswered(String),
}

/// A Set's Response as the page needs it: the Answers, and when they were sent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Answered {
    pub submitted_at: String,
    pub response: Response,
}

/// The Set with this id, or `None` if there is no such Set.
#[server]
pub async fn load_set(id: i64) -> Result<Option<SetView>, ServerFnError> {
    use askance_store::Settlement;

    let pool: sqlx::SqlitePool = expect_context();
    let waits: askance_store::Waits = expect_context();

    let stored = askance_store::load_set(&pool, id)
        .await
        .map_err(|err| ServerFnError::new(format!("{err:#}")))?;

    let Some(stored) = stored else {
        return Ok(None);
    };

    // Read here rather than by the page once it is up: which view the human
    // gets turns on this, and asking afterwards is how a Set that was answered
    // days ago would draw a form for a moment first.
    let settlement = askance_store::settlement(&pool, id)
        .await
        .map_err(|err| ServerFnError::new(format!("{err:#}")))?;

    let standing = match settlement {
        Some(Settlement::Answered(answered)) => Standing::Answered(Answered {
            submitted_at: answered.submitted_at,
            response: answered.response,
        }),
        Some(Settlement::ArchivedUnanswered(archived)) => {
            Standing::ArchivedUnanswered(archived.archived_at)
        }
        // The same verdict the pending list's row carries, from the same
        // registry: this page is where it is acted on.
        None => {
            Standing::Waiting(waits.liveness(id, &stored.created_at, OffsetDateTime::now_utc()))
        }
    };

    Ok(Some(SetView {
        id: stored.id,
        title: stored.set.title,
        project: stored.set.project,
        branch: stored.set.branch,
        // An empty Preface is the same as none at all: no point drawing the
        // section for it.
        preface_html: stored
            .set
            .preface
            .as_deref()
            .map(str::trim)
            .filter(|preface| !preface.is_empty())
            .map(crate::markdown::to_html),
        // A Diff with no files in it is the same as none: the CLI attaches one
        // only when the tree is dirty, but an empty patch is not worth a
        // heading either.
        diff: stored.set.diff.as_deref().and_then(crate::diff::to_html),
        questions: stored.set.questions,
        standing,
    }))
}

/// What became of the human's Response, as the page needs to know it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Submitted {
    /// Stored as the Set's answer; whoever was waiting has been woken.
    Accepted,

    /// The Set was answered before this Response arrived — the first stands,
    /// and this one was discarded.
    AlreadyAnswered,

    /// There is no such Set, though there was one when the page loaded.
    NoSuchSet,

    /// The Set was archived unanswered before this Response arrived — from
    /// another device, or another tab. Archiving closes a Set for good, so it
    /// cannot also become an answered one.
    Archived,

    /// The Response does not resolve the Set. The page builds Responses that
    /// do, so this is a bug rather than something the human can fix — but it
    /// is carried back and shown rather than swallowed.
    Rejected(Vec<String>),
}

/// Answer a Set. Goes through the same path as the agent-facing endpoint, so a
/// submit from this page wakes a waiting agent exactly as `curl` would.
///
/// The path is spelled out rather than left to the macro's default so it is
/// legible in a log beside `/api/v1/`, which the agents use.
#[server(prefix = "/api/ui", endpoint = "submit-response", input = server_fn::codec::Json)]
pub async fn submit_response(id: i64, response: Response) -> Result<Submitted, ServerFnError> {
    use askance_store::Submission;

    let pool: sqlx::SqlitePool = expect_context();
    let settlements: askance_store::Settlements = expect_context();

    let submission = askance_store::submit_response(&pool, &settlements, id, &response)
        .await
        .map_err(|err| ServerFnError::new(format!("{err:#}")))?;

    Ok(match submission {
        Submission::Accepted(_) => Submitted::Accepted,
        Submission::AlreadyAnswered => Submitted::AlreadyAnswered,
        Submission::NoSuchSet => Submitted::NoSuchSet,
        Submission::Archived => Submitted::Archived,
        Submission::Invalid(invalid) => {
            Submitted::Rejected(invalid.violations.iter().map(ToString::to_string).collect())
        }
    })
}

/// What became of the human closing a Set unanswered.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Archived {
    /// Closed: the Set is off the pending list and in the Archive, and a CLI
    /// still holding a wait on it has been told.
    Closed,

    /// It was answered before this arrived, so it is in the Archive as a
    /// decision. Nothing was changed — a decision is not something to close.
    AlreadyAnswered,

    /// It had already been archived, from another device or another tab.
    AlreadyArchived,

    /// There is no such Set, though there was one when the page loaded.
    NoSuchSet,
}

/// Archive a Set unanswered: the human declaring that nobody is ever going to
/// answer it, so it stops being something that is waiting on them.
///
/// Only ever reached from a human's browser (ADR-0001) — the agent API has no
/// route for it, because a disconnected agent is not evidence: the CLI
/// reconnects through transient drops.
#[server(prefix = "/api/ui", endpoint = "archive-set", input = server_fn::codec::Json)]
pub async fn archive_set(id: i64) -> Result<Archived, ServerFnError> {
    use askance_store::Archiving;

    let pool: sqlx::SqlitePool = expect_context();
    let settlements: askance_store::Settlements = expect_context();

    let archiving = askance_store::archive_set(&pool, &settlements, id)
        .await
        .map_err(|err| ServerFnError::new(format!("{err:#}")))?;

    Ok(match archiving {
        Archiving::Archived(_) => Archived::Closed,
        Archiving::AlreadyAnswered => Archived::AlreadyAnswered,
        Archiving::AlreadyArchived => Archived::AlreadyArchived,
        Archiving::NoSuchSet => Archived::NoSuchSet,
    })
}

#[component]
pub fn SetPage() -> impl IntoView {
    let params = use_params_map();
    let id = move || {
        params
            .read()
            .get("id")
            .and_then(|id| id.parse::<i64>().ok())
    };

    // An id that is not a number cannot name a Set, so it gets the same answer
    // as one that names no Set: there isn't one.
    let set = Resource::new(id, |id| async move {
        match id {
            Some(id) => load_set(id).await,
            None => Ok(None),
        }
    });

    view! {
        <Suspense fallback=|| view! { <p class="empty">"Loading…"</p> }>
            {move || Suspend::new(async move {
                match set.await {
                    Err(err) => {
                        view! { <p class="error">"Could not read the Set: " {err.to_string()}</p> }
                            .into_any()
                    }
                    Ok(None) => view! { <p class="empty">"No such Set."</p> }.into_any(),
                    Ok(Some(set)) => sheet(set).into_any(),
                }
            })}
        </Suspense>
    }
}

/// The live fields of one question: the Option the human picked, if any, and
/// whatever they wrote.
#[derive(Debug, Clone, Copy)]
struct Fields {
    selected: RwSignal<Option<u32>>,
    free_text: RwSignal<String>,
}

impl Fields {
    fn new() -> Self {
        Self {
            selected: RwSignal::new(None),
            free_text: RwSignal::new(String::new()),
        }
    }

    /// What is in them right now. Read untracked: this is called from the
    /// submit handler, which wants a snapshot and not a subscription.
    fn filled(&self, label: &str) -> Filled {
        Filled {
            label: label.to_owned(),
            selected: self.selected.get_untracked(),
            free_text: self.free_text.get_untracked(),
        }
    }

    /// The same, subscribed rather than sampled: the draft is written from an
    /// effect, which has to run again on every tap and every keystroke.
    fn watched(&self, label: &str) -> Filled {
        Filled {
            label: label.to_owned(),
            selected: self.selected.get(),
            free_text: self.free_text.get(),
        }
    }
}

/// One question's fields as the human left them, away from the signals holding
/// them — the shape [`drafted`] turns into a Response, and the shape a draft is
/// stored as.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Filled {
    /// The name the question answers to: `Q7` for a Question, `Q7a` for a
    /// Sub-question.
    label: String,
    selected: Option<u32>,
    free_text: String,
}

impl Filled {
    /// Whether the human has put anything in: an Option, words, or both.
    /// Whitespace is not an answer here any more than it is at submit.
    fn answered(&self) -> bool {
        self.selected.is_some() || !self.free_text.trim().is_empty()
    }
}

/// How one question stands when accept-all is pressed: what its fields hold,
/// and the Option the agent recommended if it recommended one.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Considered {
    filled: Filled,
    recommended: Option<u32>,
}

/// Which questions accept-all fills, and with which Option: the position of
/// every question that is both unanswered and carrying a Recommendation, paired
/// with the Recommendation's number.
///
/// It fills nothing else. A question the human already answered keeps their
/// answer, and one with no ★ stays open rather than being handed an arbitrary
/// Option — which also makes a second press a no-op, since the first press
/// answers exactly the questions it names.
fn accepting(standing: &[Considered]) -> Vec<(usize, u32)> {
    standing
        .iter()
        .enumerate()
        .filter(|(_, question)| !question.filled.answered())
        .filter_map(|(index, question)| Some((index, question.recommended?)))
        .collect()
}

/// The Response a page full of fields adds up to.
///
/// Every question gets an entry, in the order it was asked: an Answer when the
/// human put something in, the Unanswered marker when they did not. Whitespace
/// is not an answer, and neither is a blank comment.
fn drafted(filled: &[Filled], comment: &str) -> Response {
    let answers = filled
        .iter()
        .map(|field| {
            let free_text = field.free_text.trim();

            Answer {
                label: field.label.clone(),
                selected: field.selected,
                free_text: (!free_text.is_empty()).then(|| free_text.to_owned()),
                // Exclusive with an Answer, so it goes on exactly the entries
                // that carry nothing.
                unanswered: !field.answered(),
            }
        })
        .collect();

    let comment = comment.trim();

    Response {
        answers,
        comment: (!comment.is_empty()).then(|| comment.to_owned()),
    }
}

/// A half-finished answer sheet, as it sits in `localStorage` between visits:
/// every question's fields in the order the Set asked them, plus the set-level
/// comment.
///
/// The same per-question shape submit snapshots, so a draft serializes as it
/// stands rather than through a parallel one.
///
/// Deliberately per device and never sent to the server: a phone and a laptop
/// keep their own drafts of the same Set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Draft {
    filled: Vec<Filled>,
    comment: String,
}

impl Draft {
    /// Whether there is nothing in here to come back to. An untouched sheet is
    /// not worth a stored draft, and whitespace is no more an answer here than
    /// it is at submit.
    fn empty(&self) -> bool {
        self.comment.trim().is_empty() && !self.filled.iter().any(Filled::answered)
    }
}

/// Where one Set's draft lives. Keyed by the Set's id, so two Sets being
/// answered in turn keep independent drafts.
fn draft_key(id: i64) -> String {
    format!("askance.draft.{id}")
}

/// The draft in `body`, if it is one this Set can be filled in from.
///
/// A body that will not parse, or whose questions are not this Set's asked in
/// this order, is discarded whole rather than applied in part: the Set as the
/// agent sent it wins over a draft that no longer describes it.
fn restorable(body: &str, labels: &[&str]) -> Option<Draft> {
    let draft: Draft = serde_json::from_str(body).ok()?;

    let drafted: Vec<&str> = draft
        .filled
        .iter()
        .map(|field| field.label.as_str())
        .collect();

    (drafted == labels).then_some(draft)
}

/// The browser's `localStorage`, or `None` when there is none to be had — a
/// browser that blocks it, or one that has none at all.
///
/// Storage is a convenience the whole way down: `None` costs the human their
/// drafts and nothing else, so nothing on this path is worth a panic.
#[cfg(feature = "hydrate")]
fn local_storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok().flatten()
}

/// The draft being held under this key, if there is one.
#[cfg(feature = "hydrate")]
fn stored_draft(key: &str) -> Option<String> {
    local_storage()?.get_item(key).ok().flatten()
}

/// Write the draft out, replacing whatever was under the key.
#[cfg(feature = "hydrate")]
fn store_draft(key: &str, draft: &Draft) {
    let Some(storage) = local_storage() else {
        return;
    };
    let Ok(body) = serde_json::to_string(draft) else {
        return;
    };

    // Full, or refused: the draft is gone, and the page carries on regardless.
    let _ = storage.set_item(key, &body);
}

/// Drop the draft under this key.
#[cfg(feature = "hydrate")]
fn clear_draft(key: &str) {
    if let Some(storage) = local_storage() {
        let _ = storage.remove_item(key);
    }
}

// Under `ssr` there is no browser and so no draft: the server renders the Set as
// the agent sent it, which is what hydration then has to find waiting for it.
// The effects that keep a draft only ever run in a browser, so these three stand
// in for storage the server half has no way to reach and no reason to.
#[cfg(not(feature = "hydrate"))]
fn stored_draft(_key: &str) -> Option<String> {
    None
}

#[cfg(not(feature = "hydrate"))]
fn store_draft(_key: &str, _draft: &Draft) {}

#[cfg(not(feature = "hydrate"))]
fn clear_draft(_key: &str) {}

/// One question as the page holds on to it: the name it answers to, whether it
/// offered Options, the Option the agent recommended, and the fields the human
/// fills.
#[derive(Debug, Clone)]
struct Asked {
    label: String,
    multiple_choice: bool,
    recommended: Option<u32>,
    fields: Fields,
}

/// The Option the agent recommended among these, if it recommended one. At most
/// one Option per question may be the Recommendation, so the first ★ is it.
fn recommendation(options: &[QuestionOption]) -> Option<u32> {
    options
        .iter()
        .find(|option| option.recommended)
        .map(|option| option.n)
}

/// The questions the warning before submit names: those the Response leaves
/// open among the ones that offered Options. A free-text question left blank
/// still goes back marked Unanswered — it just draws no warning, because there
/// was no offered choice to overlook.
fn unanswered(response: &Response, multiple_choice: &[String]) -> Vec<String> {
    response
        .answers
        .iter()
        .filter(|answer| answer.unanswered && multiple_choice.contains(&answer.label))
        .map(|answer| answer.label.clone())
        .collect()
}

/// The heading naming the Questions, and the anchor they are reached by.
///
/// The one section every Set has, so unlike the Preface and the Diff it is drawn
/// unconditionally — and drawn the same way whether the Set is being answered or
/// read back. The id sits on the heading rather than on the list, so a jump
/// lands on the name of the thing rather than just above its first row.
fn questions_heading() -> impl IntoView {
    view! { <h2 class="section-heading" id="questions">"Questions"</h2> }
}

/// The id a Question is reached by: its label, lowercased — `Q3` becomes `q3`,
/// which is also what a human writing the link by hand would type.
///
/// A label is the agent's own string, and an id cannot hold everything a string
/// can, so anything an id will not take becomes a hyphen; a label made of
/// nothing else falls back to the Question's position. Labels are distinct
/// across a Set and in practice they are `Q1`, `Q2`, …, so the fallback is for
/// the pathological Set rather than the ordinary one.
///
/// Sub-questions get none: one scrolls into view with its parent.
fn anchor(label: &str, position: usize) -> String {
    let id: String = label
        .trim()
        .to_lowercase()
        .chars()
        .map(|ch| match ch {
            'a'..='z' | '0'..='9' | '-' | '_' => ch,
            _ => '-',
        })
        .collect();

    let id = id.trim_matches('-');
    if id.is_empty() {
        format!("q{position}")
    } else {
        id.to_owned()
    }
}

/// How much of a file's path the table of contents has room for, in characters.
///
/// A count rather than a measurement: the paths are monospaced, so characters
/// are what the column is made of — and a cut made here can land on a directory
/// boundary, where one made by the column would land wherever the pixels ran
/// out. The stylesheet still clips what overruns, for the narrow end of the
/// range the nav is drawn across; this is what keeps the ordinary path legible.
const PATH_ROOM: usize = 24;

/// A file's path as the table of contents shows it: whole when it fits, and
/// otherwise cut from the *left* — `crates/app/src/set_view.rs` becomes
/// `…/app/src/set_view.rs`.
///
/// From the left because the end of a path is the part being looked for: a
/// column of `crates/app/src/…` names nothing. The cut lands on a directory
/// boundary so what is left is still a path, and only cuts into the filename
/// itself when the filename alone is longer than the line — at which point
/// something has to give, and the extension is worth more than the stem.
fn shortened(path: &str) -> String {
    if path.chars().count() <= PATH_ROOM {
        return path.to_owned();
    }

    // What is left once the leading `…/` has taken its two.
    let room = PATH_ROOM - 2;

    // The first boundary whose tail fits is the one that keeps the most of the
    // path.
    let boundary = path
        .char_indices()
        .filter(|(_, ch)| *ch == '/')
        .map(|(at, ch)| at + ch.len_utf8())
        .find(|&start| path[start..].chars().count() <= room);

    match boundary {
        Some(start) => format!("…/{}", &path[start..]),
        None => {
            let cut = path
                .char_indices()
                .nth(path.chars().count() - (room + 1))
                .map_or(0, |(at, _)| at);
            format!("…{}", &path[cut..])
        }
    }
}

/// One line of the table of contents under a section heading: a file of the
/// Diff, or a Question.
struct Entry {
    /// The id it jumps to, without the `#`.
    anchor: String,

    /// The name a Question answers to, kept apart from the words beside it so
    /// it can be styled as the fixed thing it is. A file has none — its path is
    /// the whole of what it is called.
    label: Option<String>,

    /// What the line reads as, already cut down to what will fit if it is a
    /// path. A Question's words are left whole and cut by the column.
    text: String,

    /// The whole of it, for the browser's own tooltip: the nav is narrow, and
    /// this is where the truncated line can be read out in full.
    whole: String,
}

/// One section of the page in the table of contents: the heading it jumps to,
/// and whatever the section is made of.
struct Section {
    anchor: &'static str,
    name: &'static str,
    entries: Vec<Entry>,
}

/// The table of contents: the page's sections top to bottom, each with its own
/// parts nested under it.
///
/// Built from the Set the page was drawn from rather than from the page, so a
/// section the Set does not have is a section the nav does not list — and so
/// the nav is in the HTML the server writes, which means it is there to be read
/// before hydration and its links work as plain hash links until then.
///
/// `use<>`: the nav is built from the Set here and keeps nothing of it, so it
/// outlives the borrow.
fn contents(set: &SetView) -> impl IntoView + use<> {
    let mut sections: Vec<Section> = Vec::new();

    if set.preface_html.is_some() {
        sections.push(Section {
            anchor: "preface",
            name: "Preface",
            entries: Vec::new(),
        });
    }

    if let Some(diff) = &set.diff {
        sections.push(Section {
            anchor: "diff",
            name: "Diff",
            entries: diff
                .paths
                .iter()
                .enumerate()
                .map(|(index, path)| Entry {
                    // The renderer counts the folds from one, and these are the
                    // same folds in the same order.
                    anchor: format!("diff-{}", index + 1),
                    label: None,
                    text: shortened(path),
                    whole: path.clone(),
                })
                .collect(),
        });
    }

    // Unconditional, like the heading it points at: every Set has Questions.
    sections.push(Section {
        anchor: "questions",
        name: "Questions",
        entries: set
            .questions
            .iter()
            .enumerate()
            .map(|(index, question)| Entry {
                anchor: anchor(question.name(), index + 1),
                label: Some(question.name().to_owned()),
                text: question.text.clone(),
                whole: format!("{} {}", question.name(), question.text),
            })
            .collect(),
    });

    // Sub-questions are not listed: one scrolls into view with its parent, and
    // a nav that listed them would be the page again rather than a way around
    // it.
    let sections: Vec<_> = sections
        .into_iter()
        .map(|section| {
            let entries = (!section.entries.is_empty()).then(|| {
                let entries: Vec<_> = section.entries.into_iter().map(entry).collect();
                view! { <ol class="contents-entries">{entries}</ol> }
            });

            view! {
                <li class="contents-section">
                    {link(section.anchor.to_owned(), None, section.name.to_owned(), None)}
                    {entries}
                </li>
            }
        })
        .collect();

    view! {
        <nav class="contents" aria-label="On this page">
            <ol class="contents-sections">{sections}</ol>
        </nav>
    }
}

/// One nested line of the nav.
fn entry(entry: Entry) -> impl IntoView {
    // A file's path is set in the same face the Diff sets it in, so the two
    // read as the same name. Prefixed like every other class here: `question`
    // on its own is the page's own Question card, and a nav line is not one.
    let kind = if entry.label.is_some() {
        "contents-question"
    } else {
        "contents-path"
    };

    view! {
        <li class=format!("contents-entry {kind}")>
            {link(entry.anchor, entry.label, entry.text, Some(entry.whole))}
        </li>
    }
}

/// The jump itself: an anchor to the id, which works as a plain hash link with
/// no script at all, and which script — once there is any — takes over so the
/// jump can unfold what it lands on and leave the history alone.
fn link(
    anchor: String,
    label: Option<String>,
    text: String,
    whole: Option<String>,
) -> impl IntoView {
    let target = anchor.clone();

    view! {
        <a
            class="contents-link"
            href=format!("#{anchor}")
            title=whole
            on:click=move |ev: leptos::ev::MouseEvent| {
                // Only the plain click: a modified one is the reader asking
                // their browser for a tab or a window, which is the browser's
                // business and not ours.
                if ev.ctrl_key() || ev.meta_key() || ev.shift_key() || ev.alt_key() {
                    return;
                }
                ev.prevent_default();
                jump_to(&target);
            }
        >
            {label.map(|label| view! { <span class="contents-label">{label}</span> })}
            {text}
        </a>
    }
}

/// Take the reader to the section, file or Question this id names.
///
/// A folded file is unfolded before the jump: landing on a closed fold is
/// landing on nothing. The scroll is the browser's own, so how it moves is the
/// stylesheet's business — which is where `prefers-reduced-motion` is honoured.
///
/// The URL is set with `replaceState` rather than by letting the link navigate:
/// the reader gets a hash they can copy or reload into, but moving around a page
/// is not somewhere to come *back* to, and twenty jumps should not bury the
/// list this Set was opened from twenty presses of Back away.
#[cfg(feature = "hydrate")]
fn jump_to(anchor: &str) {
    use wasm_bindgen::JsCast;

    let Some(window) = web_sys::window() else {
        return;
    };
    let Some(target) = window
        .document()
        .and_then(|document| document.get_element_by_id(anchor))
    else {
        return;
    };

    if let Some(fold) = target.dyn_ref::<web_sys::HtmlDetailsElement>() {
        fold.set_open(true);
    }

    target.scroll_into_view();

    if let Ok(history) = window.history() {
        let _ = history.replace_state_with_url(
            &wasm_bindgen::JsValue::NULL,
            "",
            Some(&format!("#{anchor}")),
        );
    }
}

// The server draws the nav but never presses it: the handler is what hydration
// brings, and until then the link is a hash link the browser follows by itself.
#[cfg(not(feature = "hydrate"))]
fn jump_to(_anchor: &str) {}

/// One Set, top to bottom: how it stands and its own material — what the agent
/// asked about and the evidence for it — and then either the sheet to answer it
/// on or the record of what became of it.
///
/// The material above is the same however it stands: a settled Set is read for
/// what was decided *and* for what the decision was about.
fn sheet(set: SetView) -> impl IntoView {
    // Built before the Set is taken apart below, since it is a description of
    // the whole of it.
    let contents = contents(&set);

    // A Set sent from outside a repo has neither, and an empty line of
    // provenance is worse than none.
    let provenance = (set.project.is_some() || set.branch.is_some()).then(|| {
        view! {
            <p class="meta">
                {set.project.map(|project| view! { <span class="project">{project}</span> })}
                {set.branch.map(|branch| view! { <span class="branch">{branch}</span> })}
            </p>
        }
    });

    // Beside the provenance rather than down with the Answers: on a settled Set,
    // when it was settled is part of knowing what one is reading — and for one
    // that was closed unanswered it is most of what there is to know.
    let when = match &set.standing {
        Standing::Waiting(_) => None,
        Standing::Answered(answered) => {
            let when = submitted_when(&answered.submitted_at);
            Some(view! { <p class="answered-at">"Answered " {when}</p> }.into_any())
        }
        Standing::ArchivedUnanswered(archived_at) => {
            let when = submitted_when(archived_at);
            // A class of its own: the line sits where an answered Set's date
            // sits and is styled with it, but nothing here was answered.
            Some(
                view! {
                    <p class="archived-at">"Archived unanswered " {when}</p>
                }
                .into_any(),
            )
        }
    };

    // Whether anyone is still on the other end, and the one thing to do about it
    // if nobody is. Above the Preface because both are about the ask rather than
    // about answering it — and because archiving is decided with the badge and
    // the Questions in view, not from a list row.
    let waiting = match &set.standing {
        Standing::Waiting(liveness) => Some(*liveness),
        _ => None,
    };
    let standing = waiting.map(|liveness| pending_standing(set.id, liveness));

    // Back to the list this Set is on: a settled one is off the pending list for
    // good and lives in the Archive, so that is where reading it leads back to.
    let (back, out) = if waiting.is_some() {
        ("/", "← Pending")
    } else {
        ("/archive", "← Archive")
    };

    let body = match set.standing {
        Standing::Waiting(_) => answerable(set.id, set.questions).into_any(),
        Standing::Answered(answered) => settled(&set.questions, Some(answered.response)).into_any(),
        Standing::ArchivedUnanswered(_) => orphaned(&set.questions).into_any(),
    };

    view! {
        <A href=back attr:class="back">{out}</A>
        <h1>{set.title}</h1>
        // After the title and before the rest: the page says what it is, then
        // what is in it. It is taken out of the flow and put in the margin by
        // the stylesheet, so where it sits here is a reading order rather than
        // a position.
        {contents}
        {provenance}
        {when}
        {standing}
        // Named and anchored like the Diff below it: the heading is what a jump
        // from the table of contents lands on, and the id is what it jumps to.
        // Both are in the page the server writes, so a hash deep-link works
        // before any script has run.
        {set
            .preface_html
            .map(|html| {
                view! {
                    <section class="preface" id="preface">
                        <h2 class="section-heading">"Preface"</h2>
                        <div class="preface-body" inner_html=html></div>
                    </section>
                }
            })}
        // Between the Preface and the Questions: the Preface says what the
        // agent is asking about, and the Diff is the evidence for it.
        {set
            .diff
            .map(|diff| {
                view! {
                    <section class="diff" id="diff">
                        <h2 class="section-heading">"Diff"</h2>
                        // The per-file anchors — `diff-1`, `diff-2`, … — are
                        // stamped by the renderer, since this arrives already
                        // rendered.
                        <div class="diff-files" inner_html=diff.html></div>
                    </section>
                }
            })}
        {body}
    }
}

/// What the human is asked before a Set is closed unanswered.
///
/// Named rather than written into the dialog so that the one thing it must not
/// stop saying — that this cannot be taken back — can be held to. Archiving is
/// the only irreversible act in the whole UI.
const ARCHIVE_WARNING: &str = "It leaves the pending list for good and goes into the Archive \
     with no Response. An agent still waiting on it is told the Set was archived. This cannot be \
     undone.";

/// Where a still-waiting Set stands: whether an agent is listening, and the
/// offer to close it if none ever will be again.
///
/// The two sit together because one is why the other exists. Archiving is
/// confirmed first: it is the only thing on this page that cannot be taken back,
/// and it is a thumb's width from the questions.
fn pending_standing(id: i64, liveness: Liveness) -> impl IntoView {
    let (state, said) = crate::pending::badge(liveness);

    let archive = Action::new(move |id: &i64| {
        let id = *id;
        async move { archive_set(id).await }
    });

    // `true` while the human is being asked to confirm. Nothing is archived
    // until they answer it.
    let confirming = RwSignal::new(false);

    let close = move |_| {
        confirming.set(false);
        archive.dispatch(id);
    };

    let navigate = use_navigate();
    Effect::new(move |_| {
        let Some(Ok(Archived::Closed)) = archive.value().get() else {
            return;
        };

        // A Set that can never take a Response has no use for a half-filled
        // sheet, and the page is leaving anyway.
        clear_draft(&draft_key(id));

        // To the Archive rather than to the pending list: this Set was not
        // discarded, it was filed, and seeing it filed unanswered is the
        // confirmation that nothing was lost.
        navigate("/archive", Default::default());
    });

    view! {
        <section class="standing">
            <span class=format!("liveness {state}")>{said}</span>
            <button
                type="button"
                class="archive"
                on:click=move |_| confirming.set(true)
                prop:disabled=move || archive.pending().get()
            >
                {move || if archive.pending().get() { "Archiving…" } else { "Archive unanswered" }}
            </button>
            {move || unarchived(archive.value().get())}
        </section>
        // The one irreversible thing on the page, so it is asked about in as
        // many words — including that it cannot be undone, which is what tells
        // this dialog apart from the one before a submit.
        {move || {
            confirming
                .get()
                .then(|| {
                    view! {
                        <div class="confirm-backdrop">
                            <div
                                class="confirm"
                                role="dialog"
                                aria-modal="true"
                                aria-labelledby="archive-title"
                            >
                                <p id="archive-title">"Archive this Set unanswered?"</p>
                                <p class="note">{ARCHIVE_WARNING}</p>
                                <div class="confirm-actions">
                                    <button
                                        type="button"
                                        class="secondary"
                                        on:click=move |_| confirming.set(false)
                                    >
                                        "Keep it pending"
                                    </button>
                                    <button type="button" on:click=close>
                                        "Archive unanswered"
                                    </button>
                                </div>
                            </div>
                        </div>
                    }
                })
        }}
    }
}

/// Why the Set was not archived, when it was not. A Set that was says nothing
/// here — the page is already on its way to the Archive.
fn unarchived(outcome: Option<Result<Archived, ServerFnError>>) -> Option<AnyView> {
    let said = match outcome? {
        Ok(Archived::Closed) => return None,
        Ok(Archived::AlreadyAnswered) => {
            "This Set was answered while this page was open, so it was not \
             archived: it is in the Archive as a decision."
                .to_owned()
        }
        Ok(Archived::AlreadyArchived) => "This Set has already been archived.".to_owned(),
        Ok(Archived::NoSuchSet) => "This Set is no longer here.".to_owned(),
        Err(err) => format!("The Set was not archived: {err}"),
    };

    Some(view! { <p class="error">{said}</p> }.into_any())
}

/// The questions as a sheet to fill in, with the submit that ends the agent's
/// wait.
fn answerable(id: i64, questions: Vec<Question>) -> impl IntoView {
    // The fields are gathered as the questions are drawn, so the list holds one
    // entry per question in the order the Set asked them — which is the order a
    // Response has to account for them in. A question that went missing here
    // would make the Response incomplete, and the server refuses those by name
    // rather than letting one through.
    let mut fields: Vec<Asked> = Vec::new();
    let asked: Vec<_> = questions
        .into_iter()
        .enumerate()
        .map(|(index, asked)| question(index + 1, asked, &mut fields))
        .collect();

    // Whether accept-all has anything it could ever do here. Read off the
    // questions as they were drawn rather than by walking the Set again, and
    // fixed for the life of the page: the Set does not change under it.
    let recommended_anywhere = fields.iter().any(|asked| asked.recommended.is_some());

    let fields = StoredValue::new(fields);
    let comment = RwSignal::new(String::new());

    let response = move || {
        let filled: Vec<Filled> = fields
            .read_value()
            .iter()
            .map(|asked| asked.fields.filled(&asked.label))
            .collect();
        drafted(&filled, &comment.get_untracked())
    };

    // Whether this Set is done with: it has an answer, or it can never take
    // one. Nothing more is drafted after that.
    let settled = RwSignal::new(false);

    // The draft, in the two effects that keep it. Both are the browser's alone
    // — an effect never runs during SSR — so the server-rendered page is the
    // Set as the agent sent it whether or not a draft is waiting, and hydration
    // finds exactly what the server drew before either of these touches it.
    //
    // Restoring comes first, both here and in the order effects run.
    Effect::new(move |_| {
        let key = draft_key(id);
        let Some(body) = stored_draft(&key) else {
            return;
        };

        let asked = fields.read_value();
        let labels: Vec<&str> = asked.iter().map(|asked| asked.label.as_str()).collect();

        let Some(draft) = restorable(&body, &labels) else {
            // Stale, so it is no more use on the next visit than on this one.
            clear_draft(&key);
            return;
        };

        for (asked, filled) in asked.iter().zip(draft.filled) {
            asked.fields.selected.set(filled.selected);
            asked.fields.free_text.set(filled.free_text);
        }
        comment.set(draft.comment);
    });

    // Then every change from there on, written out as it happens.
    Effect::new(move |saved: Option<()>| {
        let draft = Draft {
            filled: fields
                .read_value()
                .iter()
                .map(|asked| asked.fields.watched(&asked.label))
                .collect(),
            comment: comment.get(),
        };
        let done = settled.get();

        // The first run is the page as it was drawn — an empty sheet, or the
        // draft just restored into it — and neither is news. Skipping it is
        // also what stops a stored draft being written over by an empty sheet
        // if the restore above has not had its turn yet.
        if saved.is_none() || done {
            return;
        }

        let key = draft_key(id);
        if draft.empty() {
            // Emptied again: a draft of nothing would only ever restore as
            // nothing.
            clear_draft(&key);
        } else {
            store_draft(&key, &draft);
        }
    });

    // Pressing it writes the same field the human's own tap would, so every
    // Answer it fills can still be changed, and submit is still a separate act.
    let accept_all = move |_| {
        let standing: Vec<Considered> = fields
            .read_value()
            .iter()
            .map(|asked| Considered {
                filled: asked.fields.filled(&asked.label),
                recommended: asked.recommended,
            })
            .collect();

        for (index, n) in accepting(&standing) {
            fields.read_value()[index].fields.selected.set(Some(n));
        }
    };

    let submit = Action::new(move |(id, response): &(i64, Response)| {
        let (id, response) = (*id, response.clone());
        async move { submit_response(id, response).await }
    });

    // `Some(names)` puts the warning between the human and the send. It never
    // holds an empty list: with no offered choice left open there is nothing to
    // warn about, and the Response goes straight out.
    let confirming = RwSignal::new(None::<Vec<String>>);

    let start = move |_| {
        let sending = response();
        let choices: Vec<String> = fields
            .read_value()
            .iter()
            .filter(|asked| asked.multiple_choice)
            .map(|asked| asked.label.clone())
            .collect();
        let open = unanswered(&sending, &choices);

        if open.is_empty() {
            submit.dispatch((id, sending));
        } else {
            confirming.set(Some(open));
        }
    };

    // Drafted again rather than kept from the warning: nothing can have changed
    // while the dialog was up, and this way there is no stale copy to send.
    let send_anyway = move |_| {
        confirming.set(None);
        submit.dispatch((id, response()));
    };

    // This Set will take no Response from here — so the draft goes, and the
    // effect above stops writing another. Keeping one would only resurface it,
    // stale and unsendable, on some later visit.
    let settle = move || {
        settled.set(true);
        clear_draft(&draft_key(id));
    };

    let navigate = use_navigate();
    Effect::new(move |_| {
        let Some(Ok(outcome)) = submit.value().get() else {
            return;
        };

        match outcome {
            Submitted::Accepted => {
                settle();
                // Back to the pending list, where the Set's absence is the
                // confirmation that the agent has its answer.
                navigate("/", Default::default());
            }
            Submitted::AlreadyAnswered | Submitted::NoSuchSet | Submitted::Archived => settle(),
            // The page builds Responses that resolve the Set, so this is a bug
            // here rather than anything the human did — and their draft stands,
            // because it is the only copy of what they wrote.
            Submitted::Rejected(_) => {}
        }
    });

    view! {
        // Above the accept-all rather than below it: the offer to accept every
        // Recommendation is part of the Questions, so a jump to them arrives at
        // it rather than past it.
        {questions_heading()}
        // Above the questions rather than beside the submit: it changes what is
        // drawn below it, and the human scrolls down through the result on the
        // way to sending it.
        {recommended_anywhere
            .then(|| {
                view! {
                    <section class="accept-all">
                        <button type="button" on:click=accept_all>
                            "Accept all ★ Recommendations"
                        </button>
                        <p class="note">
                            "Fills the questions you have not answered yet. You can still change any of them."
                        </p>
                    </section>
                }
            })}
        <ol class="questions">{asked}</ol>
        <section class="set-comment">
            <label for="set-comment">"Anything about the Set as a whole"</label>
            <textarea
                id="set-comment"
                name="set-comment"
                rows="3"
                prop:value=move || comment.get()
                on:input:target=move |ev| comment.set(ev.target().value())
            ></textarea>
        </section>
        <section class="submit">
            <button
                type="button"
                on:click=start
                prop:disabled=move || submit.pending().get()
            >
                {move || if submit.pending().get() { "Sending…" } else { "Submit" }}
            </button>
            {move || refusal(submit.value().get())}
        </section>
        // The warning that stands between the human and a submit skipping
        // offered choices: every multiple-choice question left open, by name,
        // and the choice to go back. A skipped free-text question passes
        // without a word — nothing was offered, so nothing was overlooked. It
        // warns and never blocks — leaving the whole Set open with only a
        // comment is a counter-question, not a mistake, and it comes through
        // here like any other.
        {move || {
            confirming
                .get()
                .map(|open| {
                    view! {
                        <div class="confirm-backdrop">
                            <div
                                class="confirm"
                                role="dialog"
                                aria-modal="true"
                                aria-labelledby="confirm-title"
                            >
                                <p id="confirm-title">"Going back unanswered:"</p>
                                <ul class="unanswered">
                                    {open
                                        .into_iter()
                                        .map(|name| view! { <li>{name}</li> })
                                        .collect_view()}
                                </ul>
                                <p class="note">
                                    "The agent will be told these are still open."
                                </p>
                                <div class="confirm-actions">
                                    <button
                                        type="button"
                                        class="secondary"
                                        on:click=move |_| confirming.set(None)
                                    >
                                        "Keep answering"
                                    </button>
                                    <button type="button" on:click=send_anyway>
                                        "Send anyway"
                                    </button>
                                </div>
                            </div>
                        </div>
                    }
                })
        }}
    }
}

/// Why the Response did not land, when it did not. A Response that was taken
/// says nothing here — the page is already on its way back to the list.
fn refusal(outcome: Option<Result<Submitted, ServerFnError>>) -> Option<AnyView> {
    let said = match outcome? {
        Ok(Submitted::Accepted) => return None,
        Ok(Submitted::AlreadyAnswered) => {
            "This Set had already been answered. The first Response stands, so \
             yours was not stored."
                .to_owned()
        }
        Ok(Submitted::NoSuchSet) => "This Set is no longer here.".to_owned(),
        Ok(Submitted::Archived) => {
            "This Set was archived unanswered, which closed it for good, so your \
             Response was not stored."
                .to_owned()
        }
        Ok(Submitted::Rejected(violations)) => {
            format!(
                "This Response does not resolve the Set: {}",
                violations.join("; ")
            )
        }
        Err(err) => format!("The Response did not get through: {err}"),
    };

    Some(view! { <p class="error">{said}</p> }.into_any())
}

/// One Question, with its Sub-questions nested one level under it.
///
/// Each ask puts its fields on `fields` as it is drawn, so they come out in the
/// order the Set asked them.
///
/// `position` is the Question's place in the Set, counting from one — the name
/// it falls back to when its label makes no id.
///
/// `use<>`: the view is built here and outlives the borrow of `fields`, which
/// it does not hold on to.
fn question(position: usize, question: Question, fields: &mut Vec<Asked>) -> impl IntoView + use<> {
    let id = anchor(question.name(), position);

    let own = ask(
        question.name().to_owned(),
        question.text.clone(),
        question.options.clone(),
        fields,
    );

    let subquestions: Vec<_> = question
        .subquestions
        .iter()
        .map(|subquestion| {
            let ask = ask(
                subquestion.name(&question),
                subquestion.text.clone(),
                subquestion.options.clone(),
                fields,
            );
            view! { <li class="subquestion">{ask}</li> }
        })
        .collect();

    let nested = if subquestions.is_empty() {
        None
    } else {
        Some(view! { <ol class="subquestions">{subquestions}</ol> })
    };

    view! { <li class="question" id=id>{own} {nested}</li> }
}

/// A Question or a Sub-question — both are asked the same way: the name it
/// answers to, its text, its Options as a radio group, then a free-text field.
///
/// `name` is what a Response answers by (`Q7`, `Q7a`), so it names the fields
/// too.
fn ask(
    name: String,
    text: String,
    options: Vec<QuestionOption>,
    collected: &mut Vec<Asked>,
) -> impl IntoView + use<> {
    let group = format!("{name}-option");
    let field = format!("{name}-free-text");
    let has_options = !options.is_empty();

    let live = Fields::new();
    collected.push(Asked {
        label: name.clone(),
        multiple_choice: has_options,
        recommended: recommendation(&options),
        fields: live,
    });

    // With no Options the free text *is* the answer; with them it is the note
    // in the margin beside one.
    let prompt = if has_options {
        "Or in your own words"
    } else {
        "Your answer"
    };

    let radios = has_options.then(|| {
        let offers: Vec<_> = options
            .into_iter()
            .map(|option| offered(group.clone(), option, live))
            .collect();
        view! { <ul class="options">{offers}</ul> }
    });

    view! {
        <div class="ask">
            <p class="text">
                <span class="label">{name}</span>
                {text}
            </p>
            {radios}
            <label class="free-text" for=field.clone()>
                {prompt}
            </label>
            <textarea
                id=field.clone()
                name=field
                rows="2"
                prop:value=move || live.free_text.get()
                on:input:target=move |ev| live.free_text.set(ev.target().value())
            ></textarea>
        </div>
    }
}

/// One Option on offer: a radio labelled by its number and text.
///
/// The Recommendation is marked and never selected — nothing is selected on
/// load, so an unread Recommendation cannot be submitted by accident.
fn offered(group: String, option: QuestionOption, live: Fields) -> impl IntoView {
    let id = format!("{group}-{}", option.n);
    let n = option.n;
    let class = if option.recommended {
        "option recommended"
    } else {
        "option"
    };
    let star = option
        .recommended
        .then(|| view! { <span class="star" title="the agent's Recommendation">"★"</span> });

    // The label wraps the radio: the whole row becomes the tap target, and the
    // two are associated without a `for` to keep in step with the id.
    view! {
        <li class=class>
            <label>
                <input
                    type="radio"
                    id=id
                    name=group
                    value=n.to_string()
                    prop:checked=move || live.selected.get() == Some(n)
                    on:change=move |_| live.selected.set(Some(n))
                />
                <span class="n">{n}</span>
                <span class="option-text">{option.text}</span>
                {star}
            </label>
        </li>
    }
}

/// When a Set was settled, as the page says it — the Response landing, or the
/// human closing it unanswered. Shared with the Archive, which dates its rows by
/// the same reasoning and has to word them the same way.
///
/// Absolute rather than the pending list's "3h ago": that list is scanned for
/// what to do next, while a settled Set is an entry in a permanent log,
/// where relative to now stops meaning anything by the following week. UTC
/// because the server's clock is the only one in play — the browser is given no
/// date library, for the same reason it is given no markdown parser.
///
/// A stamp that will not parse is shown as stored: the page is still worth
/// drawing, and the raw stamp still says when.
pub(crate) fn submitted_when(submitted_at: &str) -> String {
    let Ok(when) = OffsetDateTime::parse(submitted_at, &Rfc3339) else {
        return submitted_at.trim().to_owned();
    };

    let when = when.to_offset(UtcOffset::UTC);
    format!(
        "{}-{:02}-{:02} {:02}:{:02} UTC",
        when.year(),
        u8::from(when.month()),
        when.day(),
        when.hour(),
        when.minute(),
    )
}

/// What to say at the head of a Response that resolved nothing — and `None`
/// when it resolved something, which is the ordinary case.
///
/// Answering a Set by leaving every question open is allowed: with the set-level
/// comment it is a counter-question, the human's "not these questions", and it
/// is as much a Response as any other. It has to read as one rather than as a
/// page whose Answers failed to arrive, which is what a column of Unanswered
/// with no word about why would look like.
fn nothing_answered(response: &Response) -> Option<&'static str> {
    if response.answers.iter().any(Answer::is_answer) {
        return None;
    }

    let commented = response
        .comment
        .as_deref()
        .is_some_and(|comment| !comment.trim().is_empty());

    Some(if commented {
        "Nothing here was answered. The comment below is the whole Response — a \
         counter-question — and every question went back to the agent still open."
    } else {
        "Nothing here was answered, and nothing was said about the Set either: \
         every question went back to the agent still open."
    })
}

/// The Response's entry for this question, if it has one.
///
/// A stored Response was validated against its Set, so every question has
/// exactly one entry. The lookup is still fallible because the page draws the
/// Set rather than the Response: a question with nothing to show reads as
/// Unanswered, which is true of one, rather than as a gap in the page.
fn answer_to<'a>(response: &'a Response, name: &str) -> Option<&'a Answer> {
    response
        .answers
        .iter()
        .find(|answer| answer.label.trim() == name)
}

/// A Set that was archived unanswered: the questions as they were asked, and the
/// fact that nobody ever answered them.
///
/// Kept readable forever rather than shown as a decision that was made: the
/// Archive is permanent, and what is permanent here is the ask.
fn orphaned(questions: &[Question]) -> impl IntoView + use<> {
    view! {
        <p class="counter-question">
            "This Set was archived unanswered: nobody answered these questions, and no Response \
             was ever sent. The agent was told the Set had been archived."
        </p>
        {settled(questions, None)}
    }
}

/// What became of a Set, question by question: every question as it was asked,
/// each with its Answer, and the set-level comment under them.
///
/// With no Response there is nothing to have decided — the Set was archived
/// unanswered — so the questions read as they were asked and nothing is marked.
fn settled(questions: &[Question], response: Option<Response>) -> impl IntoView + use<> {
    let outcomes: Vec<_> = questions
        .iter()
        .enumerate()
        .map(|(index, question)| settled_question(index + 1, question, response.as_ref()))
        .collect();

    let said = response.as_ref().and_then(nothing_answered);

    // Shown only when there is one, exactly as the submit only ever sends one
    // that has something in it.
    let comment = response
        .as_ref()
        .and_then(|response| response.comment.as_deref())
        .map(str::trim)
        .filter(|comment| !comment.is_empty())
        .map(str::to_owned);

    view! {
        // Above the heading: what a Response resolved — or did not — is said at
        // the head of the page, about the Set as a whole, not under the
        // Questions.
        {said.map(|said| view! { <p class="counter-question">{said}</p> })}
        {questions_heading()}
        <ol class="questions decided">{outcomes}</ol>
        {comment
            .map(|comment| {
                view! {
                    <section class="set-comment decided">
                        <h2>"On the Set as a whole"</h2>
                        <p class="comment">{comment}</p>
                    </section>
                }
            })}
    }
}

/// One settled Question, with its Sub-questions nested one level under it — the
/// read counterpart of [`question`], and laid out the same way, because it is the
/// same Set being looked at.
fn settled_question(
    position: usize,
    question: &Question,
    response: Option<&Response>,
) -> impl IntoView + use<> {
    let id = anchor(question.name(), position);

    let own = resolved(
        question.name().to_owned(),
        question.text.clone(),
        &question.options,
        response,
    );

    let subquestions: Vec<_> = question
        .subquestions
        .iter()
        .map(|subquestion| {
            let resolved = resolved(
                subquestion.name(question),
                subquestion.text.clone(),
                &subquestion.options,
                response,
            );
            view! { <li class="subquestion">{resolved}</li> }
        })
        .collect();

    let nested =
        (!subquestions.is_empty()).then(|| view! { <ol class="subquestions">{subquestions}</ol> });

    view! { <li class="question" id=id>{own} {nested}</li> }
}

/// A Question or a Sub-question as it was resolved: its text, every Option it
/// offered with the human's pick marked among them, whatever they wrote, and —
/// when they left it open — the fact that it went back Unanswered.
///
/// Every Option is kept, not just the chosen one: what was turned down is half
/// of what a decision was.
///
/// Given the whole Response rather than this question's entry, because the
/// absence of a Response is itself something to draw: with none at all the Set
/// was archived unanswered, and there was nobody to tell that these questions
/// were still open.
fn resolved(
    name: String,
    text: String,
    options: &[QuestionOption],
    response: Option<&Response>,
) -> impl IntoView + use<> {
    let answer = response.and_then(|response| answer_to(response, &name));
    let selected = answer.and_then(|answer| answer.selected);
    let said = answer
        .and_then(|answer| answer.free_text.as_deref())
        .map(str::trim)
        .filter(|said| !said.is_empty())
        .map(str::to_owned);

    // No Option and no words is the Unanswered marker, whether or not the flag
    // is set: either way nothing was answered here. Only a Response can leave a
    // question open, though — an archived Set says so once, at the head of the
    // page, rather than claiming the agent was told anything.
    let open = response.is_some() && selected.is_none() && said.is_none();

    // The form's own wording, minus its "Or": there with Options the words were
    // the note in the margin beside one, and read back they are still. Addressed
    // to the human who answered, because there is only ever the one of them.
    let prompt = if options.is_empty() {
        "Your answer"
    } else {
        "In your own words"
    };

    let offers: Vec<_> = options
        .iter()
        .map(|option| decided_option(option, selected))
        .collect();
    let shown = (!offers.is_empty()).then(|| view! { <ul class="options">{offers}</ul> });

    view! {
        <div class="ask decided">
            <p class="text">
                <span class="label">{name}</span>
                {text}
            </p>
            {shown}
            {said
                .map(|said| {
                    view! {
                        <p class="answer-text">
                            <span class="prompt">{prompt}</span>
                            {said}
                        </p>
                    }
                })}
            {open
                .then(|| {
                    view! {
                        <p class="unanswered">
                            "Unanswered — the agent was told this one is still open."
                        </p>
                    }
                })}
        </div>
    }
}

/// One Option after the fact: numbered and worded as it was offered, marked if
/// the agent recommended it, and marked separately — in a word, not only in the
/// outline — if this is the one the human chose.
///
/// The two marks are deliberately different things to read: the ★ is what was
/// suggested, and "chosen" is what was decided, which on any given question may
/// well not be the same Option.
fn decided_option(option: &QuestionOption, selected: Option<u32>) -> impl IntoView + use<> {
    let chosen = selected == Some(option.n);
    let class = match (chosen, option.recommended) {
        (true, true) => "option chosen recommended",
        (true, false) => "option chosen",
        (false, true) => "option recommended",
        (false, false) => "option",
    };

    view! {
        <li class=class>
            <span class="n">{option.n}</span>
            <span class="option-text">{option.text.clone()}</span>
            {option
                .recommended
                .then(|| {
                    view! {
                        <span class="star" title="the agent's Recommendation">"★"</span>
                    }
                })}
            {chosen.then(|| view! { <span class="chose">"chosen"</span> })}
        </li>
    }
}

#[cfg(test)]
mod tests {
    use askance_schema::{Answer, Response};

    use super::{
        ARCHIVE_WARNING, Considered, Draft, Filled, accepting, anchor, answer_to, draft_key,
        drafted, nothing_answered, restorable, shortened, submitted_when, unanswered,
    };

    fn filled(label: &str, selected: Option<u32>, free_text: &str) -> Filled {
        Filled {
            label: label.to_owned(),
            selected,
            free_text: free_text.to_owned(),
        }
    }

    /// A sheet part-way filled in, as the human might leave it.
    fn part_way() -> Draft {
        Draft {
            filled: vec![
                filled("Q1", Some(2), ""),
                filled("Q2", None, "only for writes"),
                filled("Q2a", None, ""),
            ],
            comment: "back in an hour".to_owned(),
        }
    }

    fn standing(
        label: &str,
        selected: Option<u32>,
        free_text: &str,
        recommended: Option<u32>,
    ) -> Considered {
        Considered {
            filled: filled(label, selected, free_text),
            recommended,
        }
    }

    #[test]
    fn a_question_the_human_left_alone_goes_back_marked_unanswered() {
        let response = drafted(&[filled("Q1", None, "")], "");

        let answer = &response.answers[0];
        assert!(answer.unanswered);
        assert_eq!(answer.selected, None);
        assert_eq!(answer.free_text, None);
    }

    #[test]
    fn whitespace_is_not_an_answer() {
        let response = drafted(&[filled("Q1", None, "  \n ")], "  ");

        assert!(response.answers[0].unanswered);
        assert_eq!(response.comment, None, "a blank comment is no comment");
    }

    #[test]
    fn an_option_or_words_or_both_make_an_answer() {
        let response = drafted(
            &[
                filled("Q1", Some(2), ""),
                filled("Q2", None, "the second one, but only for writes"),
                filled("Q3", Some(1), " with a caveat "),
            ],
            "",
        );

        assert_eq!(response.answers[0].selected, Some(2));
        assert_eq!(
            response.answers[1].free_text.as_deref(),
            Some("the second one, but only for writes")
        );
        assert_eq!(response.answers[2].selected, Some(1));
        assert_eq!(
            response.answers[2].free_text.as_deref(),
            Some("with a caveat"),
            "free text is trimmed before it goes out"
        );

        assert!(
            response.answers.iter().all(|answer| !answer.unanswered),
            "an entry is an Answer or the marker, never both",
        );
    }

    #[test]
    fn every_question_gets_an_entry_in_the_order_it_was_asked() {
        let response = drafted(
            &[
                filled("Q1", Some(1), ""),
                filled("Q2", None, ""),
                filled("Q2a", None, "no"),
                filled("Q2b", None, ""),
            ],
            "",
        );

        let labels: Vec<&str> = response
            .answers
            .iter()
            .map(|answer| answer.label.as_str())
            .collect();
        assert_eq!(labels, ["Q1", "Q2", "Q2a", "Q2b"]);
    }

    #[test]
    fn the_warning_names_every_multiple_choice_question_being_left_open() {
        let response = drafted(
            &[
                filled("Q1", Some(1), ""),
                filled("Q2", None, ""),
                filled("Q2a", None, "no"),
                filled("Q2b", None, "   "),
            ],
            "",
        );
        let choices: Vec<String> = ["Q1", "Q2", "Q2a", "Q2b"].map(String::from).into();

        assert_eq!(unanswered(&response, &choices), ["Q2", "Q2b"]);
    }

    #[test]
    fn a_free_text_question_left_open_draws_no_warning() {
        let response = drafted(
            &[
                filled("Q1", None, ""),
                filled("Q2", None, ""),
                filled("Q3", None, ""),
            ],
            "",
        );
        let choices = vec!["Q2".to_owned()];

        assert_eq!(
            unanswered(&response, &choices),
            ["Q2"],
            "Q1 and Q3 offered no Options, so skipping them is not warned about",
        );
    }

    #[test]
    fn accept_all_fills_the_questions_nobody_has_touched() {
        let questions = [
            standing("Q1", None, "", Some(2)),
            standing("Q2", Some(1), "", Some(2)),
            standing("Q3", None, "the second one, but only for writes", Some(1)),
            standing("Q4", None, "   ", Some(3)),
        ];

        assert_eq!(
            accepting(&questions),
            [(0, 2), (3, 3)],
            "an Option or words is an answer and stands; whitespace is neither",
        );
    }

    #[test]
    fn a_question_with_no_recommendation_is_left_unanswered() {
        let questions = [
            standing("Q1", None, "", None),
            standing("Q2", None, "", Some(1)),
        ];

        assert_eq!(
            accepting(&questions),
            [(1, 1)],
            "with no ★ there is nothing to accept, and no Option to pick instead",
        );
    }

    #[test]
    fn pressing_accept_all_twice_does_nothing_the_second_time() {
        let mut questions = vec![
            standing("Q1", None, "", Some(2)),
            standing("Q2", Some(1), "", Some(2)),
            standing("Q3", None, "", None),
        ];

        let first = accepting(&questions);
        assert_eq!(first, [(0, 2)]);
        for (index, n) in first {
            questions[index].filled.selected = Some(n);
        }

        assert!(
            accepting(&questions).is_empty(),
            "the first press answered everything it names, so a second finds nothing",
        );
    }

    #[test]
    fn a_draft_comes_back_exactly_as_it_was_left() {
        let draft = part_way();
        let body = serde_json::to_string(&draft).unwrap();

        assert_eq!(
            restorable(&body, &["Q1", "Q2", "Q2a"]),
            Some(draft),
            "every Option, every word and the comment survive the round trip",
        );
    }

    #[test]
    fn a_draft_restores_only_what_the_human_put_there() {
        let draft = Draft {
            filled: vec![filled("Q1", None, ""), filled("Q2", Some(1), "")],
            comment: String::new(),
        };
        let body = serde_json::to_string(&draft).unwrap();

        let restored = restorable(&body, &["Q1", "Q2"]).unwrap();
        assert_eq!(
            restored.filled[0].selected, None,
            "a question the human left alone comes back untouched, not answered for them",
        );
        assert_eq!(restored.filled[1].selected, Some(1));
    }

    #[test]
    fn a_draft_whose_questions_are_not_this_sets_is_discarded() {
        let body = serde_json::to_string(&part_way()).unwrap();

        assert_eq!(
            restorable(&body, &["Q1", "Q2", "Q2b"]),
            None,
            "a Sub-question that was renamed makes the whole draft stale",
        );
        assert_eq!(
            restorable(&body, &["Q1", "Q2"]),
            None,
            "and so does a Set that has since lost a question",
        );
        assert_eq!(
            restorable(&body, &["Q2", "Q1", "Q2a"]),
            None,
            "the order is the order the Set asked them in, not a set of names",
        );
    }

    #[test]
    fn a_draft_that_will_not_parse_is_discarded() {
        assert_eq!(restorable(r#"{"filled": ["#, &["Q1"]), None);
        assert_eq!(
            restorable(r#"{"answers": [], "comment": null}"#, &["Q1"]),
            None,
            "a body from some other shape of draft is no more usable than a truncated one",
        );
    }

    #[test]
    fn an_empty_sheet_is_not_a_draft_worth_keeping() {
        let nothing = Draft {
            filled: vec![filled("Q1", None, ""), filled("Q2", None, "  \n")],
            comment: "   ".to_owned(),
        };
        assert!(nothing.empty(), "whitespace is not an answer here either");

        assert!(!part_way().empty());
        assert!(
            !Draft {
                filled: vec![filled("Q1", None, "")],
                comment: "why not cache it upstream?".to_owned(),
            }
            .empty(),
            "a comment on its own is a draft: it is a whole counter-question",
        );
    }

    #[test]
    fn a_question_is_reached_by_its_label_lowercased() {
        assert_eq!(anchor("Q3", 3), "q3");
        assert_eq!(
            anchor(" Q12 ", 12),
            "q12",
            "a padded label is still that one"
        );
    }

    #[test]
    fn a_label_an_id_cannot_hold_is_made_into_one() {
        assert_eq!(
            anchor("Q 7.a", 7),
            "q-7-a",
            "a label is the agent's own string, and an id takes less than one does",
        );
        assert_eq!(
            anchor("...", 4),
            "q4",
            "a label that makes no id at all falls back to the Question's place in the Set",
        );
    }

    #[test]
    fn a_path_the_contents_has_room_for_is_shown_whole() {
        assert_eq!(
            shortened("crates/app/src/diff.rs"),
            "crates/app/src/diff.rs"
        );
    }

    #[test]
    fn a_path_too_long_for_the_contents_keeps_its_filename() {
        assert_eq!(
            shortened("crates/app/src/set_view.rs"),
            "…/app/src/set_view.rs",
            "the end of a path is what is being looked for, so the cut takes \
             from the front — and lands on a directory boundary, so what is \
             left is still a path",
        );
        assert_eq!(
            shortened("crates/server/tests/deeply/nested/set_page.rs"),
            "…/nested/set_page.rs",
            "as much of the path as the line has room for, not just the file",
        );
    }

    #[test]
    fn a_filename_longer_than_the_line_is_cut_into() {
        assert_eq!(
            shortened("assets/a-very-long-name-for-one-file.webmanifest"),
            "…or-one-file.webmanifest",
            "no boundary leaves a tail that fits, so the extension is kept \
             over the start of the stem",
        );
    }

    #[test]
    fn each_set_keeps_its_own_draft() {
        assert_ne!(draft_key(7), draft_key(8));
    }

    #[test]
    fn the_archive_confirmation_says_it_cannot_be_undone() {
        assert!(
            ARCHIVE_WARNING.contains("cannot be undone"),
            "the one irreversible act in the UI has to be asked about as one: {ARCHIVE_WARNING}",
        );
        assert!(
            ARCHIVE_WARNING.contains("Archive"),
            "and it has to say where the Set is going, because it is not being deleted: \
             {ARCHIVE_WARNING}",
        );
    }

    #[test]
    fn a_comment_and_nothing_else_is_a_whole_counter_question() {
        let response = drafted(
            &[filled("Q1", None, ""), filled("Q2", None, "")],
            "Neither, really — why not cache it upstream?",
        );

        assert!(
            response.answers.iter().all(|answer| answer.unanswered),
            "nothing was answered, and every question still has to say so",
        );
        assert_eq!(
            response.comment.as_deref(),
            Some("Neither, really — why not cache it upstream?")
        );
    }

    /// A Response as the store hands one back, built the way the submit builds
    /// them: an entry per question, answered or marked.
    fn answered(entries: &[(&str, Option<u32>, &str)], comment: Option<&str>) -> Response {
        drafted(
            &entries
                .iter()
                .map(|(label, selected, free_text)| filled(label, *selected, free_text))
                .collect::<Vec<_>>(),
            comment.unwrap_or_default(),
        )
    }

    #[test]
    fn the_time_a_response_landed_is_shown_absolutely() {
        assert_eq!(
            submitted_when("2026-08-03T12:04:07.412Z"),
            "2026-08-03 12:04 UTC",
            "a decision in a permanent log is dated, not aged",
        );
        assert_eq!(
            submitted_when("2026-08-03T22:04:07+10:00"),
            "2026-08-03 12:04 UTC",
            "whatever offset it was written with, it is read in the server's",
        );
    }

    #[test]
    fn an_unreadable_stamp_is_shown_as_stored() {
        assert_eq!(
            submitted_when(" not a timestamp "),
            "not a timestamp",
            "the page is worth drawing without a pretty date",
        );
    }

    #[test]
    fn a_response_that_resolved_nothing_says_so_at_the_head_of_the_page() {
        let counter = answered(
            &[("Q1", None, ""), ("Q2", None, "")],
            Some("Neither, really — why not cache it upstream?"),
        );

        let said = nothing_answered(&counter).expect("a Response with no Answers has to say so");
        assert!(
            said.contains("counter-question"),
            "with a comment and no Answers it is a counter-question, not an empty page: {said}",
        );

        let silent = answered(&[("Q1", None, ""), ("Q2", None, "  ")], None);
        let said = nothing_answered(&silent).expect("nothing answered and nothing said, either");
        assert!(
            !said.contains("comment below"),
            "there is no comment below to point at: {said}",
        );
    }

    #[test]
    fn a_response_that_answered_anything_needs_no_such_note() {
        let one = answered(&[("Q1", None, ""), ("Q2", Some(1), "")], None);
        assert_eq!(nothing_answered(&one), None, "an Option is an Answer");

        let words = answered(&[("Q1", None, "only for writes"), ("Q2", None, "")], None);
        assert_eq!(nothing_answered(&words), None, "and so are words");
    }

    #[test]
    fn each_question_is_shown_the_entry_that_names_it() {
        let response = answered(
            &[("Q1", Some(2), ""), ("Q2", None, ""), ("Q2a", None, "no")],
            None,
        );

        assert_eq!(answer_to(&response, "Q1").unwrap().selected, Some(2));
        assert!(answer_to(&response, "Q2").unwrap().unanswered);
        assert_eq!(
            answer_to(&response, "Q2a").unwrap().free_text.as_deref(),
            Some("no"),
            "a Sub-question answers to its own name, not its parent's",
        );
    }

    #[test]
    fn a_question_the_response_never_mentions_reads_as_unanswered() {
        let response = Response {
            answers: vec![Answer {
                label: "Q1".to_owned(),
                selected: Some(1),
                free_text: None,
                unanswered: false,
            }],
            comment: None,
        };

        assert_eq!(
            answer_to(&response, "Q2"),
            None,
            "the page draws the Set, so a question with no entry is still drawn",
        );
    }
}
