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

use askance_schema::{Answer, Liveness, Response};
use leptos::prelude::*;
use leptos_router::components::A;
use leptos_router::hooks::{use_navigate, use_params_map};
use serde::{Deserialize, Serialize};
use time::format_description::well_known::Rfc3339;
use time::{OffsetDateTime, UtcOffset};

/// One Question Set as the browser receives it.
///
/// Everything the agent wrote — the Preface, every Question's and Sub-question's
/// text, and every Option's — arrives as HTML rather than as its source, and so
/// does the Diff: the server has the markdown parser and the diff highlighter,
/// and this way the browser needs neither.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetView {
    pub id: i64,
    pub title: String,
    pub project: Option<String>,
    pub branch: Option<String>,
    pub preface_html: Option<String>,
    pub diff: Option<DiffView>,
    pub questions: Vec<QuestionView>,

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

/// One Question as the page draws it, with its Sub-questions nested one level
/// under it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuestionView {
    pub ask: AskView,
    pub subquestions: Vec<AskView>,

    /// The Question's own text as plain words, for the line the table of
    /// contents gives it.
    ///
    /// The nav cannot use `ask.text_html`: it is a line of text in a narrow
    /// column, and the markup in there would have to be taken back out to get
    /// the words — which means a parser on the browser's side of the wire, the
    /// one thing rendering on the server is for. So the words travel beside the
    /// HTML, rendered from the same markdown by the same pass.
    ///
    /// Sub-questions have none, because the nav does not list them.
    pub nav_text: String,
}

/// A Question or a Sub-question as the page draws it: the name it answers to,
/// its text already rendered, and the Options it offers.
///
/// One type for both, because the page asks them the same way and the schema's
/// distinction between them is spent by the time it gets here: a Sub-question's
/// name is its parent's label and its letter, resolved on the way out.
///
/// The form is built from the Options' numbers and their Recommendation flags,
/// and a Response answers by number.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AskView {
    /// `Q7` for a Question, `Q7a` for a Sub-question.
    pub name: String,

    /// The text as HTML, rendered and sanitized by the server on the way out.
    pub text_html: String,

    pub options: Vec<OptionView>,
}

/// One Option as the page draws it: the number a Response answers by, its text
/// already rendered, and whether the agent recommended it.
///
/// Its text is inline markup and nothing blockier, because an Option is one line
/// beside a radio and the whole row is the tap target: a paragraph or a list
/// emitted inside that label would split the row in two.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OptionView {
    pub n: u32,

    /// The text as inline HTML, rendered and sanitized by the server on the way
    /// out.
    pub text_html: String,

    pub recommended: bool,
}

/// The Set's Questions as the page needs them: named as a Response answers them,
/// with the agent's markdown rendered.
///
/// Server-only, because the rendering is — this is the seam that keeps the
/// markdown parser off the browser's side of the wire.
#[cfg(feature = "ssr")]
fn viewed(questions: Vec<askance_schema::Question>) -> Vec<QuestionView> {
    questions
        .into_iter()
        .map(|question| QuestionView {
            subquestions: question
                .subquestions
                .iter()
                .map(|subquestion| AskView {
                    name: subquestion.name(&question),
                    text_html: crate::markdown::to_html(&subquestion.text),
                    options: offered_as(&subquestion.options),
                })
                .collect(),
            nav_text: crate::markdown::to_plain(&question.text),
            ask: AskView {
                name: question.name().to_owned(),
                text_html: crate::markdown::to_html(&question.text),
                options: offered_as(&question.options),
            },
        })
        .collect()
}

/// One question's Options as the page draws them, in the order the agent offered
/// them. Rendered inline: a row beside a radio has room for markup and none for
/// a block.
#[cfg(feature = "ssr")]
fn offered_as(options: &[askance_schema::QuestionOption]) -> Vec<OptionView> {
    options
        .iter()
        .map(|option| OptionView {
            n: option.n,
            text_html: crate::markdown::to_inline_html(&option.text),
            recommended: option.recommended,
        })
        .collect()
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
        questions: viewed(stored.set.questions),
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

/// The draft being held under this key, if there is one.
///
/// Under `ssr` there is nowhere for one to be held, so this is `None` and the
/// server renders the Set as the agent sent it — which is what hydration then
/// has to find waiting for it. The effects that keep a draft only ever run in a
/// browser, so the server half never writes one either.
fn stored_draft(key: &str) -> Option<String> {
    crate::device::read(key)
}

/// Write the draft out, replacing whatever was under the key.
fn store_draft(key: &str, draft: &Draft) {
    let Ok(body) = serde_json::to_string(draft) else {
        return;
    };

    crate::device::write(key, &body);
}

/// Drop the draft under this key.
fn clear_draft(key: &str) {
    crate::device::forget(key);
}

/// One question as the page holds on to it: the name it answers to, whether it
/// offered Options, and the fields the human fills.
#[derive(Debug, Clone)]
struct Asked {
    label: String,
    multiple_choice: bool,
    fields: Fields,
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

/// The shape of the table of contents: the page's sections top to bottom, each
/// with its own parts under it.
///
/// Built from the Set the page was drawn from rather than from the page, so a
/// section the Set does not have is a section the nav does not list — and so
/// the nav is in the HTML the server writes, which means it is there to be read
/// before hydration and its links work as plain hash links until then.
///
/// Kept apart from the drawing of it because the scroll-spy watches the same
/// list: the nav and the spy have to agree about what the page is made of, or
/// the highlight ends up on a line that is not where the reader is.
fn outline(set: &SetView) -> Vec<Section> {
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
                anchor: anchor(&question.ask.name, index + 1),
                label: Some(question.ask.name.clone()),
                // The words rather than the rendered text: this is a line in a
                // column, and the markup the Question is drawn with has no
                // place in it. See [`QuestionView::nav_text`].
                text: question.nav_text.clone(),
                whole: format!("{} {}", question.ask.name, question.nav_text),
            })
            .collect(),
    });

    // Sub-questions are not listed: one scrolls into view with its parent, and
    // a nav that listed them would be the page again rather than a way around
    // it.
    sections
}

/// One anchored part of the page as the nav has it: the id the scroll-spy
/// watches, and the name to put on it.
///
/// The name travels with the id because two things read it out — the sidebar's
/// own line, and the bar, which says nothing but the name of the line the
/// highlight is on. Kept as one list rather than as a list of ids beside a list
/// of names, so the bar cannot name a different part of the page than the one
/// the spy is pointing at.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Watched {
    /// The id it jumps to, without the `#`.
    anchor: String,

    /// The name a Question answers to, kept apart from the words beside it. A
    /// section and a file have none.
    label: Option<String>,

    /// What the line reads as: a section's name, a path already cut to what will
    /// fit, or a Question's words.
    text: String,

    /// The class that sets those words — see [`face`].
    kind: &'static str,
}

/// The class that sets one line's words: a Question's label and prose in the
/// page's own face, a file's path in the Diff's, so that a path in the nav and
/// the same path over its fold read as the same name.
///
/// Taken from whether there is a label because that is what tells the two apart:
/// a file's path is the whole of what it is called.
fn face(label: Option<&str>) -> &'static str {
    if label.is_some() {
        "contents-question"
    } else {
        "contents-path"
    }
}

/// Every anchored part of the page, in page order — the ids the scroll-spy
/// watches, and what [`lit`]'s answer counts along.
///
/// Each section's own heading and then whatever is under it, which is the order
/// the page has them in. The two levels do not fight over the highlight because
/// [`lit`] takes the *last* part to have begun: a file always begins after the
/// Diff heading it is under, so the file wins for as long as the reader is in it,
/// and the heading only holds the highlight in the gap above the first file.
fn spied(sections: &[Section]) -> Vec<Watched> {
    sections
        .iter()
        .flat_map(|section| {
            let heading = Watched {
                anchor: section.anchor.to_owned(),
                label: None,
                text: section.name.to_owned(),
                kind: "contents-section",
            };

            std::iter::once(heading).chain(section.entries.iter().map(|entry| Watched {
                anchor: entry.anchor.clone(),
                label: entry.label.clone(),
                text: entry.text.clone(),
                kind: face(entry.label.as_deref()),
            }))
        })
        .collect()
}

/// Where this id sits among the parts the spy watches, if it is one of them.
fn spot(watched: &[Watched], anchor: &str) -> Option<usize> {
    watched.iter().position(|watched| watched.anchor == anchor)
}

/// What one line of the nav answers for, as places along the watched list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Stands {
    /// The part of the page it names. The line is *the* highlight while the
    /// reader is exactly here.
    at: usize,

    /// The last place under it: its own, unless it is a section with files or
    /// Questions beneath it, and then the last of those. While the reader is
    /// anywhere in between, the line says so quietly and leaves the highlight to
    /// whichever of its entries they are actually in — so a file lit up reads as
    /// being within the Diff without the two competing for the same mark.
    through: usize,
}

impl Stands {
    /// One part of the page with nothing under it: a file, or a Question.
    fn just(at: usize) -> Self {
        Self { at, through: at }
    }
}

/// How the highlight touches one line of the nav.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mark {
    /// The reader is at this very part of the page: this line is the highlight.
    At,

    /// They are somewhere inside it — a section drawn over the file or Question
    /// they are actually in. Said quietly, so that it never competes with the
    /// line that says where they are.
    Within,
}

impl Mark {
    /// The class that draws it.
    fn class(self) -> &'static str {
        match self {
            Mark::At => "contents-here",
            Mark::Within => "contents-within",
        }
    }
}

/// Which mark a line standing for these places carries while the reader is at
/// `here`, and none when they are elsewhere in the page.
///
/// The two are exclusive by construction rather than by being drawn carefully:
/// there is one mark per line and the loud one wins, so a section at its own
/// heading is never quietly marked as containing itself.
fn mark(stands: Option<Stands>, here: usize) -> Option<Mark> {
    let stands = stands?;

    if stands.at == here {
        Some(Mark::At)
    } else if (stands.at..=stands.through).contains(&here) {
        Some(Mark::Within)
    } else {
        None
    }
}

/// What the line naming this section answers for: its own heading, reaching down
/// through the last of whatever is under it.
fn stands(watched: &[Watched], section: &Section) -> Option<Stands> {
    let at = spot(watched, section.anchor)?;
    let through = section
        .entries
        .last()
        .and_then(|last| spot(watched, &last.anchor))
        .unwrap_or(at);

    Some(Stands { at, through })
}

/// Which of the spied parts of the page the highlight belongs on, given which of
/// them have started above the reading line.
///
/// The last one to have started. Their tops run down the page in this order, so
/// the last one to have passed the line is the one whose text is under it — and
/// asking it this way rather than as "the one across the line" means the answer
/// never depends on where the reader came from, only on where the page is now.
///
/// With none begun, the line is still above the first of them, which is the top
/// of the page: that counts as being in the first section, so the first line is
/// the lit one. It is also the answer before the spy has run at all, which is
/// what the server writes into the nav.
fn lit(started: &[bool]) -> usize {
    started.iter().rposition(|&started| started).unwrap_or(0)
}

/// What every line of the nav shares: where the reader is, and the two things
/// about how they got there that a line has to be able to change.
///
/// Passed about as one because all three travel together — a press on a line
/// moves the highlight, pins it, and puts the bar's list away, and the alternative
/// is threading three arguments through every one of these functions.
#[derive(Debug, Clone, Copy)]
struct Nav {
    /// Where the highlight is, as a place in the watched list.
    here: RwSignal<usize>,

    /// Whether the highlight is being held where a jump put it. Not reactive:
    /// nothing is drawn from it — it only decides whether the spy is the one
    /// saying where the reader is.
    pinned: StoredValue<bool>,

    /// Whether the bar's list is down. Nothing on a wide viewport reads it: the
    /// sidebar's list is always there, so the bar is the only thing this opens.
    open: RwSignal<bool>,
}

/// The table of contents, drawn from the Set and following the reader down it.
///
/// One nav for both of the shapes it takes: the sidebar in the wide margin, and
/// the bar with the list under it below that width. The same entries, the same
/// scroll-spy, and the same list of links in the HTML either way — which of the
/// two the reader gets is the stylesheet's answer to how wide their window is,
/// and there is no second copy to fall out of step with the first.
///
/// `use<>`: the nav is built from the Set here and keeps nothing of it, so it
/// outlives the borrow.
fn contents(set: &SetView) -> impl IntoView + use<> {
    let sections = outline(set);
    let watched = spied(&sections);

    let nav = Nav {
        // It starts where a page nobody has scrolled puts it, so the nav the
        // server writes is already right and the spy has nothing to correct when
        // it arrives.
        here: RwSignal::new(lit(&[])),
        pinned: StoredValue::new(false),
        // Shut: the bar names where the reader is, and the list is what they ask
        // for on top of that.
        open: RwSignal::new(false),
    };

    let sections: Vec<_> = sections
        .into_iter()
        .map(|section| {
            let stands = stands(&watched, &section);

            let entries = (!section.entries.is_empty()).then(|| {
                let entries: Vec<_> = section
                    .entries
                    .into_iter()
                    .map(|line| {
                        let stands = spot(&watched, &line.anchor).map(Stands::just);
                        entry(line, nav, stands)
                    })
                    .collect();
                view! { <ol class="contents-entries">{entries}</ol> }
            });

            view! {
                <li class="contents-section">
                    {link(section.anchor.to_owned(), None, section.name.to_owned(), None, nav, stands)}
                    {entries}
                </li>
            }
        })
        .collect();

    let bar = bar(watched.clone(), nav);

    // Both only ever the browser's doing: on the server the highlight stays where
    // a page nobody has scrolled puts it, and a list nothing can open needs
    // nothing to close it.
    follow(watched, nav);
    dismiss(nav);

    view! {
        <nav
            // Reactive whole, like the lines below: hydration re-applies a fixed
            // `class` and would drop whatever it found there.
            class=move || {
                if nav.open.get() { "contents contents-open" } else { "contents" }
            }
            aria-label="On this page"
        >
            {bar}
            // What a tap away from the open list lands on. It is here rather than
            // as a listener watching for presses elsewhere so that the tap hits
            // *this* and nothing else: a reader taking the list back is not also
            // choosing an Option or folding a file, and on a page whose whole
            // purpose is answering carefully, a stray tap that answers something
            // is not a small thing. Hidden from assistive tech, which has Escape
            // and the button's own state instead.
            {move || {
                nav.open
                    .get()
                    .then(|| {
                        view! {
                            <div
                                class="contents-backdrop"
                                aria-hidden="true"
                                on:click=move |_| nav.open.set(false)
                            ></div>
                        }
                    })
            }}
            <ol class="contents-sections" id="contents-list">
                {sections}
            </ol>
        </nav>
    }
}

/// The bar: on a narrow viewport, the whole of the nav until it is tapped.
///
/// It says one thing — the name of the line the sidebar would have lit — so the
/// reader knows where they are without a margin to put a list in, and tapping it
/// brings the list itself down. Not drawn at all where the sidebar is, which is
/// the stylesheet's doing: there is only ever one of the two on screen.
///
/// A button rather than a `details`, because the list it opens is the same `ol`
/// the sidebar shows and a closed `details` would have to hide that from the wide
/// reader too. The cost is that the bar does nothing until the wasm lands; the
/// entries are hash links, so the list itself works from the moment it is open.
fn bar(watched: Vec<Watched>, nav: Nav) -> impl IntoView {
    let name = move || {
        // A place the watched list does not have cannot happen — `here` only
        // ever holds one of them — but the bar is not worth a panic.
        let named = watched.get(nav.here.get())?;

        Some(view! {
            <span class=format!("contents-bar-name {}", named.kind)>
                {named
                    .label
                    .clone()
                    .map(|label| view! { <span class="contents-label">{label}</span> })}
                {named.text.clone()}
            </span>
        })
    };

    view! {
        <button
            type="button"
            class="contents-bar"
            // The bar's own words are its name — "Preface, button" — and the nav
            // around it says what a list of them is for.
            aria-expanded=move || if nav.open.get() { "true" } else { "false" }
            aria-controls="contents-list"
            on:click=move |_| nav.open.update(|open| *open = !*open)
        >
            {name}
            // Which way the list will go, and no part of what the bar is called.
            <span class="contents-bar-mark" aria-hidden="true">
                "▾"
            </span>
        </button>
    }
}

/// One nested line of the nav.
fn entry(entry: Entry, nav: Nav, stands: Option<Stands>) -> impl IntoView {
    // Prefixed like every other class here: `question` on its own is the page's
    // own Question card, and a nav line is not one.
    let kind = face(entry.label.as_deref());

    view! {
        <li class=format!("contents-entry {kind}")>
            {link(entry.anchor, entry.label, entry.text, Some(entry.whole), nav, stands)}
        </li>
    }
}

/// The jump itself: an anchor to the id, which works as a plain hash link with
/// no script at all, and which script — once there is any — takes over so the
/// jump can unfold what it lands on and leave the history alone.
///
/// It is also where the highlight shows, from what this line [`Stands`] for: the
/// mark when the reader is at it, and a quieter one on a section while they are
/// somewhere inside it.
fn link(
    anchor: String,
    label: Option<String>,
    text: String,
    whole: Option<String>,
    nav: Nav,
    stands: Option<Stands>,
) -> impl IntoView {
    let target = anchor.clone();

    let mark = move || mark(stands, nav.here.get());

    view! {
        <a
            // The whole class in one reactive attribute rather than a fixed one
            // with reactive flags beside it: hydration re-applies a fixed `class`
            // and would wipe the mark the server wrote, leaving the nav blank
            // until the reader scrolled.
            class=move || match mark() {
                Some(mark) => format!("contents-link {}", mark.class()),
                None => "contents-link".to_owned(),
            }
            // The nav is a list of places in this page and the highlight says
            // which one the reader is at, which is what `location` means. Only
            // the one line carries it — the section around it is not where they
            // are, it is what they are in.
            aria-current=move || (mark() == Some(Mark::At)).then_some("location")
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
                // The highlight goes where the reader asked to be, and is held
                // there until they scroll for themselves: the page cannot always
                // bring a section to the top — the last Question has only the
                // submit below it — and the spy would otherwise answer for
                // wherever the scroll ran out instead of for what was pressed.
                if let Some(stands) = stands {
                    nav.here.set(stands.at);
                    nav.pinned.set_value(true);
                }
                // The list has done what it was opened for. On a wide viewport
                // there was no list to put away — the sidebar stays whatever this
                // says.
                nav.open.set(false);
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

/// Where the reading line sits, written as the margins the browser is to put
/// around the window before it decides what is in view.
///
/// The bottom one lifts the line to a tenth of the way down the window: what is
/// under it is what the reader has in front of them, and a section is "started"
/// once its top has passed it. The top one reaches far above the window so that
/// a section long scrolled past still counts as started — [`lit`] wants the last
/// section to have started, and one that stopped counting from up there was not
/// going to be the last. Far enough for any page this UI draws; a section that
/// did fall out from above cannot be the answer either.
#[cfg(feature = "hydrate")]
const READING_LINE: &str = "100000px 0px -90% 0px";

/// The reader taking the scroll back off a jump: the ways of moving a page that
/// are the human's own rather than something the page did to itself.
///
/// A wheel, a finger or a key, and `pointerdown` for the scrollbar being taken
/// hold of. A press on the nav itself is a `pointerdown` too, which is harmless:
/// the click that follows it pins the highlight again straight after.
#[cfg(feature = "hydrate")]
const BY_HAND: [&str; 3] = ["wheel", "pointerdown", "keydown"];

/// Follow the reader down the page: keep `here` on the last of `anchors` to have
/// started above the reading line, which is the one whose text they have in
/// front of them.
///
/// An IntersectionObserver rather than a scroll handler: the browser is the one
/// that knows where everything is, it works this out off the main thread, and it
/// speaks up only when the answer changes. How the page got there does not come
/// into it either, so a reader who asked for no motion gets the highlight an
/// instant jump earns just as a smooth one does — and nothing here touches the
/// URL, because where the reader has scrolled to is not a place they navigated.
///
/// A jump holds the highlight where it landed until the reader scrolls by hand,
/// and letting go of it puts the highlight straight back on wherever the page
/// actually is — which is why what has started is kept out here, where both the
/// observer and the release can read it.
///
/// The observer is disconnected when the page goes and its callback dropped after
/// that, in that order: an observer still watching a callback that had been freed
/// is a panic waiting for the next scroll.
#[cfg(feature = "hydrate")]
fn follow(anchors: Vec<Watched>, nav: Nav) {
    use std::cell::RefCell;
    use std::rc::Rc;
    use wasm_bindgen::JsCast;
    use wasm_bindgen::closure::Closure;

    // What the watching is made of once it has begun. Held together so that the
    // disconnect and the removals happen while the callbacks the observer and the
    // window hold are still alive, and so that all of it is let go at once.
    type Watching = (
        web_sys::IntersectionObserver,
        Closure<dyn FnMut(js_sys::Array)>,
        Closure<dyn FnMut()>,
    );

    // In an effect because it needs the page to be there to watch: on the server
    // this never runs, and in the browser it runs once the nav is being drawn.
    Effect::new(move |_| {
        // What the frame below leaves behind, and nothing until it has run — or
        // ever, if the page goes first.
        let watching = StoredValue::new_local(None::<Watching>);

        let anchors = anchors.clone();

        // A frame later rather than now, because "the nav is being drawn" is not
        // yet "the page is in the document": a Set the reader walked to from
        // another page arrives through a Suspense, which builds the whole sheet
        // away from the document and only then puts it in — and this effect runs
        // in between. Asking the document for the sections at that moment finds
        // none of them, which is a spy that watches nothing and a highlight that
        // never leaves the first line until the page is loaded again. By the next
        // frame the browser has the page, however the reader came to it.
        let begin = move || {
            let Some(window) = web_sys::window() else {
                return;
            };
            let Some(document) = window.document() else {
                return;
            };

            // Which of them have started, by their place in `anchors`. Kept because
            // the browser reports only what has changed since it last spoke.
            let started = Rc::new(RefCell::new(vec![false; anchors.len()]));

            let watched = anchors.clone();
            let crossed = Rc::clone(&started);
            let crossings =
                Closure::<dyn FnMut(js_sys::Array)>::new(move |crossings: js_sys::Array| {
                    let mut started = crossed.borrow_mut();

                    for crossing in crossings.iter() {
                        let crossing: web_sys::IntersectionObserverEntry =
                            crossing.unchecked_into();
                        if let Some(at) = spot(&watched, &crossing.target().id()) {
                            started[at] = crossing.is_intersecting();
                        }
                    }

                    // Recorded either way, so that letting go of a pin has the truth to
                    // hand rather than having to wait for the next crossing.
                    if !nav.pinned.get_value() {
                        nav.here.set(lit(&started));
                    }
                });

            let watch = web_sys::IntersectionObserverInit::new();
            watch.set_root_margin(READING_LINE);

            let Ok(observer) = web_sys::IntersectionObserver::new_with_options(
                crossings.as_ref().unchecked_ref(),
                &watch,
            ) else {
                return;
            };

            // A section the page does not have is skipped rather than fatal: the nav
            // is drawn from the Set and so is this list, but a highlight is not worth
            // dropping the rest of the page over.
            for part in &anchors {
                if let Some(section) = document.get_element_by_id(&part.anchor) {
                    observer.observe(&section);
                }
            }

            // The reader taking over: the pin goes, and the highlight catches up to
            // where the page is in the same breath, since nothing may cross the
            // reading line for a while yet.
            let by_hand = Closure::<dyn FnMut()>::new(move || {
                if nav.pinned.get_value() {
                    nav.pinned.set_value(false);
                    nav.here.set(lit(&started.borrow()));
                }
            });

            for moved in BY_HAND {
                let _ = window
                    .add_event_listener_with_callback(moved, by_hand.as_ref().unchecked_ref());
            }

            // Left where the cleanup below can find it. A page that went while the
            // frame was pending never gets here: the frame is canceled first.
            let _ = watching.try_set_value(Some((observer, crossings, by_hand)));
        };

        let Ok(frame) = request_animation_frame_with_handle(begin) else {
            return;
        };

        on_cleanup(move || {
            // Canceled whether or not it has run: a frame that already fired is
            // no longer there to cancel, and one that has not must not begin
            // watching for a page that has gone.
            frame.cancel();

            let _ = watching.try_with_value(|watching| {
                let Some((observer, _, by_hand)) = watching else {
                    return;
                };

                observer.disconnect();

                if let Some(window) = web_sys::window() {
                    for moved in BY_HAND {
                        let _ = window.remove_event_listener_with_callback(
                            moved,
                            by_hand.as_ref().unchecked_ref(),
                        );
                    }
                }
            });
        });
    });
}

// No reader to follow on the server, and nothing to follow them with: the
// highlight stays where a page nobody has scrolled puts it, which is the first
// line of the nav.
#[cfg(not(feature = "hydrate"))]
fn follow(_anchors: Vec<Watched>, _nav: Nav) {}

/// Put the bar's list away on Escape.
///
/// The other way out of it — tapping the page — is the backdrop's doing rather
/// than a listener's, so that the tap taking the list back cannot also press
/// something on the page underneath. This is the way out that needs no aim: a
/// list drawn over the page has to be dismissible from the keyboard, and there is
/// nothing to tab to that would do it.
#[cfg(feature = "hydrate")]
fn dismiss(nav: Nav) {
    use wasm_bindgen::JsCast;
    use wasm_bindgen::closure::Closure;

    Effect::new(move |_| {
        let Some(document) = web_sys::window().and_then(|window| window.document()) else {
            return;
        };

        let escape =
            Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(move |ev: web_sys::KeyboardEvent| {
                if ev.key() == "Escape" {
                    nav.open.set(false);
                }
            });

        let _ =
            document.add_event_listener_with_callback("keydown", escape.as_ref().unchecked_ref());

        // Removed while the closure is still alive, and let go when the page is: a
        // listener holding a freed callback is a panic waiting for the next press.
        let listening = StoredValue::new_local(escape);
        on_cleanup(move || {
            let _ = listening.try_with_value(|escape| {
                if let Some(document) = web_sys::window().and_then(|window| window.document()) {
                    let _ = document.remove_event_listener_with_callback(
                        "keydown",
                        escape.as_ref().unchecked_ref(),
                    );
                }
            });
        });
    });
}

// Nothing on the server can open the list, so nothing there has to close it.
#[cfg(not(feature = "hydrate"))]
fn dismiss(_nav: Nav) {}

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
        //
        // The body is marked as rendered markdown, so the agent's headings,
        // tables and code get the same rules there as they get inside a
        // Question — the section around it is all that is the Preface's own.
        {set
            .preface_html
            .map(|html| {
                view! {
                    <section class="preface" id="preface">
                        <h2 class="section-heading">"Preface"</h2>
                        <div class="preface-body markdown" inner_html=html></div>
                    </section>
                }
            })}
        // Between the Preface and the Questions: the Preface says what the
        // agent is asking about, and the Diff is the evidence for it.
        {set.diff.map(diff_section)}
        {body}
    }
}

/// The attached Diff, and the one setting that governs how it is read.
///
/// The wrap switch sits beside the heading rather than in a settings page
/// somewhere, because this is the only place its answer is visible — and it
/// governs every Diff, not this one, which is why it is remembered on the device
/// instead of per Set.
///
/// Wrapping is a class and nothing more: the Diff arrives as HTML the server
/// already rendered, so there is nothing here to render again and the stylesheet
/// is the whole of the change. The server has no way to know what this device
/// last chose, so the first paint is always unwrapped and settles a frame later
/// — a reflow, and one the alternative costs a script in the document head to
/// avoid.
fn diff_section(diff: DiffView) -> impl IntoView {
    let wrapped = RwSignal::new(false);

    // The browser's alone, like the drafts: an effect never runs during SSR, so
    // the server draws the Diff unwrapped and only an open page asks the device
    // what it wanted.
    Effect::new(move |_| wrapped.set(crate::device::wrapping()));

    view! {
        <section class=move || if wrapped.get() { "diff wrapped" } else { "diff" } id="diff">
            <div class="section-head">
                <h2 class="section-heading">"Diff"</h2>
                <crate::switch::Switch
                    label="Word wrap"
                    on=wrapped
                    flip=move |on| {
                        wrapped.set(on);
                        crate::device::set_wrapping(on);
                    }
                />
            </div>
            // The per-file anchors — `diff-1`, `diff-2`, … — are stamped by the
            // renderer, since this arrives already rendered.
            <div class="diff-files" inner_html=diff.html></div>
        </section>
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
fn answerable(id: i64, questions: Vec<QuestionView>) -> impl IntoView {
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
        {questions_heading()}
        <ol class="questions">{asked}</ol>
        <section class="set-comment">
            <div class="grow" data-value=move || comment.get()>
                <textarea
                    id="set-comment"
                    name="set-comment"
                    rows="1"
                    placeholder="Other comments"
                    aria-label="Other comments"
                    prop:value=move || comment.get()
                    on:input:target=move |ev| comment.set(ev.target().value())
                ></textarea>
            </div>
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
fn question(
    position: usize,
    question: QuestionView,
    fields: &mut Vec<Asked>,
) -> impl IntoView + use<> {
    let id = anchor(&question.ask.name, position);

    let own = ask(question.ask, fields);

    let subquestions: Vec<_> = question
        .subquestions
        .into_iter()
        .map(|subquestion| {
            let ask = ask(subquestion, fields);
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

/// What one question asks: the name it answers to, then its text as the server
/// rendered it.
///
/// A `div` rather than the `p` this used to be — the agent's markdown can be as
/// blocky as a list, a table or a fenced block, and none of those may live inside
/// a paragraph. The class the stylesheet and the tests know it by is unchanged,
/// and the rendered markdown is boxed inside it so the label can stay a child of
/// its own rather than being swallowed by `inner_html`.
///
/// Shared by the form and the record: a settled Set is read for what was asked,
/// so it is asked in the same words and the same markup.
fn asked_text(name: String, text_html: String) -> impl IntoView {
    view! {
        <div class="text">
            <span class="label">{name}</span>
            <div class="markdown" inner_html=text_html></div>
        </div>
    }
}

/// A Question or a Sub-question — both are asked the same way: the name it
/// answers to, its text, its Options as a radio group, then a free-text field.
///
/// The name is what a Response answers by (`Q7`, `Q7a`), so it names the fields
/// too.
fn ask(asked: AskView, collected: &mut Vec<Asked>) -> impl IntoView + use<> {
    let AskView {
        name,
        text_html,
        options,
    } = asked;

    let group = format!("{name}-option");
    let field = format!("{name}-free-text");
    let has_options = !options.is_empty();

    let live = Fields::new();
    collected.push(Asked {
        label: name.clone(),
        multiple_choice: has_options,
        fields: live,
    });

    // With no Options the free text *is* the answer; with them it is whatever the
    // human has to say, which may stand instead of an Option or beside one. Hence
    // the neutral word: "Or in your own words" read as a choice between the two,
    // which was never what it meant.
    //
    // Named for the question it belongs to, because five fields prompted alike is
    // five fields nothing tells apart — which a screen reader has no way around at
    // all, and which is worth a reminder of where you are even when you can see
    // the whole page.
    let prompt = format!(
        "{name} — {}",
        if has_options {
            "Your thoughts"
        } else {
            "Your answer"
        }
    );

    let radios = has_options.then(|| {
        let offers: Vec<_> = options
            .into_iter()
            .map(|option| offered(group.clone(), option, live))
            .collect();
        view! { <ul class="options">{offers}</ul> }
    });

    view! {
        <div class="ask">
            {asked_text(name, text_html)}
            {radios}
            // The prompt is the placeholder rather than a label above the field:
            // one line of small print per question, times five questions, was
            // more of the page spent saying what a text box is for than reading
            // the Questions. It is the `aria-label` as well, in the same words,
            // because a placeholder is not a label — it is a hint the browser is
            // free to leave unspoken, and a field with nothing else naming it
            // would reach a screen reader unnamed.
            //
            // The wrapper carries the text a second time, where the stylesheet
            // uses it to give the field its height — see `.grow`. It is the
            // signal's own value, so a restored draft arrives at the right height
            // rather than one line tall with the rest of it hidden.
            <div class="grow" data-value=move || live.free_text.get()>
                <textarea
                    id=field.clone()
                    name=field
                    rows="1"
                    placeholder=prompt.clone()
                    aria-label=prompt
                    prop:value=move || live.free_text.get()
                    on:input:target=move |ev| live.free_text.set(ev.target().value())
                ></textarea>
            </div>
        </div>
    }
}

/// What a click on Option `n` leaves the Question holding, given what it held
/// already: nothing, when the click landed on the Option that was already
/// selected, and otherwise the Option clicked.
///
/// Clearing a Question is the one thing a radio group cannot do on its own — the
/// browser has no gesture for un-picking one — and changing your mind about
/// approving a Recommendation you have thought better of is exactly the case that
/// wants it. A second click undoes the first, which is the only gesture there was
/// left to give it.
fn clicked(selected: Option<u32>, n: u32) -> Option<u32> {
    (selected != Some(n)).then_some(n)
}

/// One Option on offer: a radio labelled by its number and text.
///
/// The Recommendation is marked and never selected — nothing is selected on
/// load, so an unread Recommendation cannot be submitted by accident. Clicking
/// the selected Option clears it, which puts the Question back to unanswered and
/// so back into the warning before submit.
fn offered(group: String, option: OptionView, live: Fields) -> impl IntoView {
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
    //
    // The text is filled in wholesale, and it is inline markup all the way down
    // — anything blockier inside the label would end the row it is the tap
    // target for, so the rendering flattened it on the way here. It is marked as
    // rendered markdown all the same: what did survive, a code span above all,
    // is drawn as it is everywhere else.
    view! {
        <li class=class>
            <label>
                <input
                    type="radio"
                    id=id
                    name=group
                    value=n.to_string()
                    prop:checked=move || live.selected.get() == Some(n)
                    // Both, because they answer different gestures. An arrow key
                    // moves the selection and fires a change without ever firing
                    // a click; a click on the Option already selected is the
                    // other way round — the browser fires no change, because as
                    // far as it is concerned nothing changed. Space is a click
                    // here too, which is what gives the keyboard the clearing.
                    //
                    // The click runs before the change, so it still sees what the
                    // Question held before this gesture — which is the whole of
                    // how a second click on the same Option is told from a first.
                    on:change=move |_| live.selected.set(Some(n))
                    on:click=move |_| {
                        live.selected.set(clicked(live.selected.get_untracked(), n));
                    }
                />
                <span class="n">{n}</span>
                <span class="option-text markdown" inner_html=option.text_html></span>
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
fn orphaned(questions: &[QuestionView]) -> impl IntoView + use<> {
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
fn settled(questions: &[QuestionView], response: Option<Response>) -> impl IntoView + use<> {
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
    question: &QuestionView,
    response: Option<&Response>,
) -> impl IntoView + use<> {
    let id = anchor(&question.ask.name, position);

    let own = resolved(&question.ask, response);

    let subquestions: Vec<_> = question
        .subquestions
        .iter()
        .map(|subquestion| {
            let resolved = resolved(subquestion, response);
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
fn resolved(asked: &AskView, response: Option<&Response>) -> impl IntoView + use<> {
    let AskView {
        name,
        text_html,
        options,
    } = asked;

    let answer = response.and_then(|response| answer_to(response, name));
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

    // The form's own wording, minus the name of the Question it prefixes there: a
    // field in a column of five needs telling apart from the other four, and this
    // sits inside the one Question it belongs to with nothing to be confused with.
    let prompt = if options.is_empty() {
        "Your answer"
    } else {
        "Your thoughts"
    };

    let offers: Vec<_> = options
        .iter()
        .map(|option| decided_option(option, selected))
        .collect();
    let shown = (!offers.is_empty()).then(|| view! { <ul class="options">{offers}</ul> });

    view! {
        <div class="ask decided">
            {asked_text(name.clone(), text_html.clone())}
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
/// the agent recommended it, and marked apart from that if this is the one the
/// human chose.
///
/// The two marks are deliberately different things to read: the ★ is what was
/// suggested, and the outline is what was decided, which on any given question may
/// well not be the same Option.
///
/// "chosen" is still written, and the stylesheet takes it out of the layout rather
/// than out of the page — the outline says which one to a reader looking at it and
/// nothing at all to one who is not, and an archive that cannot say what was
/// decided is not much of an archive. See `.ask.decided .chose`.
fn decided_option(option: &OptionView, selected: Option<u32>) -> impl IntoView + use<> {
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
            <span class="option-text markdown" inner_html=option.text_html.clone()></span>
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
    use askance_schema::{Answer, Liveness, Response};

    use super::{
        ARCHIVE_WARNING, AskView, DiffView, Draft, Filled, Mark, QuestionView, SetView, Standing,
        Stands, Watched, anchor, answer_to, clicked, draft_key, drafted, lit, mark,
        nothing_answered, outline, restorable, shortened, spied, stands, submitted_when,
        unanswered,
    };

    /// A Question as the page receives it, with the nav's plain words and the
    /// rendered text saying the same thing — which is what the server's own
    /// rendering makes true.
    fn asked(label: &str, text: &str) -> QuestionView {
        QuestionView {
            ask: AskView {
                name: label.to_owned(),
                text_html: format!("<p>{text}</p>"),
                options: Vec::new(),
            },
            subquestions: Vec::new(),
            nav_text: text.to_owned(),
        }
    }

    /// A Set with every section a page can have: a Preface, a Diff of two files,
    /// and two Questions.
    fn every_section() -> SetView {
        SetView {
            id: 1,
            title: "Rate limiting for the public API".to_owned(),
            project: None,
            branch: None,
            preface_html: Some("<p>no rate limit</p>".to_owned()),
            diff: Some(DiffView {
                html: String::new(),
                paths: vec!["src/limits.rs".to_owned(), "notes.txt".to_owned()],
            }),
            questions: vec![
                asked("Q1", "Where should the request counter live?"),
                asked("Q2", "How should a throttled client be told?"),
            ],
            standing: Standing::Waiting(Liveness::Waiting),
        }
    }

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
    fn clicking_the_selected_option_clears_the_question() {
        assert_eq!(clicked(Some(2), 2), None);
    }

    #[test]
    fn clicking_any_other_option_selects_it() {
        assert_eq!(clicked(None, 2), Some(2), "the first click on a question");
        assert_eq!(clicked(Some(1), 2), Some(2), "changing which one");
    }

    #[test]
    fn a_cleared_question_is_open_again_and_warned_about() {
        // Clearing is not a third state: it puts the Question back exactly where
        // it was before anything was picked, warning and all.
        let response = drafted(&[filled("Q1", clicked(Some(1), 1), "")], "");

        assert!(response.answers[0].unanswered);
        assert_eq!(
            unanswered(&response, &["Q1".to_owned()]),
            ["Q1"],
            "an Option cleared is an Option not chosen, and the submit says so",
        );
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

    /// The ids of the watched parts of the page, in the order they are watched
    /// in.
    fn anchors(watched: &[Watched]) -> Vec<&str> {
        watched
            .iter()
            .map(|watched| watched.anchor.as_str())
            .collect()
    }

    #[test]
    fn the_spy_watches_every_anchored_part_of_the_page_in_page_order() {
        assert_eq!(
            anchors(&spied(&outline(&every_section()))),
            [
                "preface",
                "diff",
                "diff-1",
                "diff-2",
                "questions",
                "q1",
                "q2"
            ],
            "each section's own heading, then whatever is under it — which is \
             the order the page has them in, and what the highlight moves along",
        );
    }

    #[test]
    fn a_section_the_set_does_not_have_is_not_watched() {
        let mut set = every_section();
        set.preface_html = None;
        set.diff = None;

        assert_eq!(
            anchors(&spied(&outline(&set))),
            ["questions", "q1", "q2"],
            "the Questions are the one section every Set has",
        );
    }

    #[test]
    fn every_watched_part_of_the_page_carries_the_name_the_nav_gives_it() {
        let watched = spied(&outline(&every_section()));

        let said: Vec<_> = watched
            .iter()
            .map(|watched| (watched.label.as_deref(), watched.text.as_str()))
            .collect();

        assert_eq!(
            said,
            [
                (None, "Preface"),
                (None, "Diff"),
                (None, "src/limits.rs"),
                (None, "notes.txt"),
                (None, "Questions"),
                (Some("Q1"), "Where should the request counter live?"),
                (Some("Q2"), "How should a throttled client be told?"),
            ],
            "the bar reads a line out by the same name the sidebar shows it \
             under, because the two are one list — a section by its name, a file \
             by its path, and a Question by its label and its words",
        );
    }

    #[test]
    fn a_watched_line_is_set_in_the_face_its_kind_of_name_wants() {
        let watched = spied(&outline(&every_section()));

        assert_eq!(
            watched
                .iter()
                .map(|watched| watched.kind)
                .collect::<Vec<_>>(),
            [
                "contents-section",
                "contents-section",
                "contents-path",
                "contents-path",
                "contents-section",
                "contents-question",
                "contents-question",
            ],
            "so a path in the bar is set as the Diff sets it, and the two read \
             as the same name",
        );
    }

    #[test]
    fn a_path_the_bar_reads_out_is_cut_as_the_nav_cuts_it() {
        let mut set = every_section();
        set.diff = Some(DiffView {
            html: String::new(),
            paths: vec!["crates/app/src/set_view.rs".to_owned()],
        });

        let watched = spied(&outline(&set));

        assert_eq!(
            watched[2].text, "…/app/src/set_view.rs",
            "the bar shows the same one line the sidebar does, cut the same way",
        );
    }

    #[test]
    fn the_highlight_is_the_last_part_of_the_page_to_have_begun() {
        assert_eq!(
            lit(&[true, true, true, false, false]),
            2,
            "the third is the one being read: the two before it are above the \
             reader, and the two after have not begun",
        );
    }

    #[test]
    fn the_top_of_the_page_counts_as_being_in_the_first_section() {
        assert_eq!(
            lit(&[false, false, false]),
            0,
            "nothing has begun above the reading line, so the reader is at the \
             top — which reads as the first section rather than as nowhere",
        );
        assert_eq!(
            lit(&[]),
            0,
            "and the same before the spy has said anything at all, which is the \
             page the server writes",
        );
    }

    #[test]
    fn a_section_reaches_down_through_whatever_is_under_it() {
        let sections = outline(&every_section());
        let watched = spied(&sections);

        let places: Vec<_> = sections
            .iter()
            .map(|section| stands(&watched, section))
            .collect();

        assert_eq!(
            places,
            [
                Some(Stands { at: 0, through: 0 }),
                Some(Stands { at: 1, through: 3 }),
                Some(Stands { at: 4, through: 6 }),
            ],
            "the Preface has nothing under it, while the Diff reaches through \
             both its files and the Questions through both Questions — which is \
             how a lit file marks the Diff it is in without taking the \
             highlight from it",
        );
    }

    #[test]
    fn the_line_the_reader_is_at_is_marked_and_the_section_around_it_quietly() {
        // The Diff of a Set whose Preface is watched first: its own heading, then
        // its two files.
        let diff = Stands { at: 1, through: 3 };

        assert_eq!(
            mark(Some(diff), 1),
            Some(Mark::At),
            "at the Diff's own heading, above its first file, the Diff is where \
             the reader is",
        );
        assert_eq!(
            mark(Some(diff), 2),
            Some(Mark::Within),
            "and once they are in one of its files, the file is the highlight \
             and the Diff only says they are in it",
        );
        assert_eq!(mark(Some(diff), 3), Some(Mark::Within));
        assert_eq!(
            mark(Some(diff), 4),
            None,
            "past the last file the Diff is behind them and unmarked",
        );
        assert_eq!(mark(Some(diff), 0), None, "as it is before they reach it");

        let file = Stands::just(2);
        assert_eq!(mark(Some(file), 2), Some(Mark::At));
        assert_eq!(
            mark(Some(file), 3),
            None,
            "a file has nothing under it, so it is the highlight or it is nothing",
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
