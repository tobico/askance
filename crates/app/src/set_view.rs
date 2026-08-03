//! The set view: one Question Set laid out to be answered — its Preface, then
//! every Question and Sub-question in the order the agent asked them, and the
//! submit that ends the agent's wait.
//!
//! The Response this page builds is explicit rather than complete: every
//! question gets an entry, and one the human left alone becomes an Unanswered
//! marker rather than being left out. Leaving a question open is a thing the
//! human is allowed to do — it just has to be said out loud, and the warning
//! before submit is where they say it.

use askance_schema::{Answer, Question, QuestionOption, Response};
use leptos::prelude::*;
use leptos_router::components::A;
use leptos_router::hooks::{use_navigate, use_params_map};
use serde::{Deserialize, Serialize};

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
    pub diff_html: Option<String>,
    pub questions: Vec<Question>,
}

/// The Set with this id, or `None` if there is no such Set.
#[server]
pub async fn load_set(id: i64) -> Result<Option<SetView>, ServerFnError> {
    let pool: sqlx::SqlitePool = expect_context();

    let stored = askance_store::load_set(&pool, id)
        .await
        .map_err(|err| ServerFnError::new(format!("{err:#}")))?;

    Ok(stored.map(|stored| SetView {
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
        diff_html: stored.set.diff.as_deref().and_then(crate::diff::to_html),
        questions: stored.set.questions,
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
    let submissions: askance_store::Submissions = expect_context();

    let submission = askance_store::submit_response(&pool, &submissions, id, &response)
        .await
        .map_err(|err| ServerFnError::new(format!("{err:#}")))?;

    Ok(match submission {
        Submission::Accepted(_) => Submitted::Accepted,
        Submission::AlreadyAnswered => Submitted::AlreadyAnswered,
        Submission::NoSuchSet => Submitted::NoSuchSet,
        Submission::Invalid(invalid) => {
            Submitted::Rejected(invalid.violations.iter().map(ToString::to_string).collect())
        }
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
                    Ok(Some(set)) => ask_sheet(set).into_any(),
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
}

/// One question's fields as the human left them, away from the signals holding
/// them — the shape [`drafted`] turns into a Response.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Filled {
    /// The name the question answers to: `Q7` for a Question, `Q7a` for a
    /// Sub-question.
    label: String,
    selected: Option<u32>,
    free_text: String,
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
                unanswered: field.selected.is_none() && free_text.is_empty(),
            }
        })
        .collect();

    let comment = comment.trim();

    Response {
        answers,
        comment: (!comment.is_empty()).then(|| comment.to_owned()),
    }
}

/// The questions a Response leaves open, by name — what the warning before
/// submit lists.
fn unanswered(response: &Response) -> Vec<String> {
    response
        .answers
        .iter()
        .filter(|answer| answer.unanswered)
        .map(|answer| answer.label.clone())
        .collect()
}

/// The whole ask, top to bottom, with the submit that ends the agent's wait.
fn ask_sheet(set: SetView) -> impl IntoView {
    let id = set.id;

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

    // The fields are gathered as the questions are drawn, so the list holds one
    // entry per question in the order the Set asked them — which is the order a
    // Response has to account for them in. A question that went missing here
    // would make the Response incomplete, and the server refuses those by name
    // rather than letting one through.
    let mut fields: Vec<(String, Fields)> = Vec::new();
    let asked: Vec<_> = set
        .questions
        .into_iter()
        .map(|asked| question(asked, &mut fields))
        .collect();

    let fields = StoredValue::new(fields);
    let comment = RwSignal::new(String::new());

    let draft = move || {
        let filled: Vec<Filled> = fields
            .read_value()
            .iter()
            .map(|(label, fields)| fields.filled(label))
            .collect();
        drafted(&filled, &comment.get_untracked())
    };

    let submit = Action::new(move |(id, response): &(i64, Response)| {
        let (id, response) = (*id, response.clone());
        async move { submit_response(id, response).await }
    });

    // `Some(names)` puts the warning between the human and the send. It never
    // holds an empty list: with nothing left open there is nothing to warn
    // about, and the Response goes straight out.
    let confirming = RwSignal::new(None::<Vec<String>>);

    let start = move |_| {
        let response = draft();
        let open = unanswered(&response);

        if open.is_empty() {
            submit.dispatch((id, response));
        } else {
            confirming.set(Some(open));
        }
    };

    // Drafted again rather than kept from the warning: nothing can have changed
    // while the dialog was up, and this way there is no stale copy to send.
    let send_anyway = move |_| {
        confirming.set(None);
        submit.dispatch((id, draft()));
    };

    let navigate = use_navigate();
    Effect::new(move |_| {
        if let Some(Ok(Submitted::Accepted)) = submit.value().get() {
            // Back to the pending list, where the Set's absence is the
            // confirmation that the agent has its answer.
            navigate("/", Default::default());
        }
    });

    view! {
        <A href="/" attr:class="back">"← Pending"</A>
        <h1>{set.title}</h1>
        {provenance}
        {set.preface_html.map(|html| view! { <section class="preface" inner_html=html></section> })}
        // Between the Preface and the Questions: the Preface says what the
        // agent is asking about, and the Diff is the evidence for it.
        {set
            .diff_html
            .map(|html| {
                view! {
                    <section class="diff">
                        <h2>"Diff"</h2>
                        <div class="diff-files" inner_html=html></div>
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
        // The warning that stands between the human and a submit leaving
        // questions open: every one of them by name, and the choice to go back.
        // It warns and never blocks — leaving the whole Set open with only a
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
/// `use<>`: the view is built here and outlives the borrow of `fields`, which
/// it does not hold on to.
fn question(question: Question, fields: &mut Vec<(String, Fields)>) -> impl IntoView + use<> {
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

    view! { <li class="question">{own} {nested}</li> }
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
    collected: &mut Vec<(String, Fields)>,
) -> impl IntoView + use<> {
    let group = format!("{name}-option");
    let field = format!("{name}-free-text");
    let has_options = !options.is_empty();

    let live = Fields::new();
    collected.push((name.clone(), live));

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

#[cfg(test)]
mod tests {
    use super::{Filled, drafted, unanswered};

    fn filled(label: &str, selected: Option<u32>, free_text: &str) -> Filled {
        Filled {
            label: label.to_owned(),
            selected,
            free_text: free_text.to_owned(),
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
    fn the_warning_names_every_question_being_left_open() {
        let response = drafted(
            &[
                filled("Q1", Some(1), ""),
                filled("Q2", None, ""),
                filled("Q2a", None, "no"),
                filled("Q2b", None, "   "),
            ],
            "",
        );

        assert_eq!(unanswered(&response), ["Q2", "Q2b"]);
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
}
