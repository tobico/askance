//! The set view: one Question Set laid out to be answered — its Preface, then
//! every Question and Sub-question in the order the agent asked them.
//!
//! This renders the ask. Nothing here holds answer state or submits anything
//! yet; the fields are named after the questions they belong to, which is the
//! handle a Response will need.

use askance_schema::{Question, QuestionOption};
use leptos::prelude::*;
use leptos_router::components::A;
use leptos_router::hooks::use_params_map;
use serde::{Deserialize, Serialize};

/// One Question Set as the browser receives it.
///
/// The Preface arrives as HTML rather than as its markdown source: the server
/// already has a markdown parser, and this way the browser needs none. The
/// Questions arrive exactly as the agent sent them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetView {
    pub id: i64,
    pub title: String,
    pub project: Option<String>,
    pub branch: Option<String>,
    pub preface_html: Option<String>,
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
        questions: stored.set.questions,
    }))
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

/// The whole ask, top to bottom.
fn ask_sheet(set: SetView) -> impl IntoView {
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

    view! {
        <A href="/" attr:class="back">"← Pending"</A>
        <h1>{set.title}</h1>
        {provenance}
        {set.preface_html.map(|html| view! { <section class="preface" inner_html=html></section> })}
        <ol class="questions">{set.questions.into_iter().map(question).collect_view()}</ol>
        <section class="set-comment">
            <label for="set-comment">"Anything about the Set as a whole"</label>
            <textarea id="set-comment" name="set-comment" rows="3"></textarea>
        </section>
    }
}

/// One Question, with its Sub-questions nested one level under it.
fn question(question: Question) -> impl IntoView {
    let subquestions: Vec<_> = question
        .subquestions
        .iter()
        .map(|subquestion| {
            let ask = ask(
                subquestion.name(&question),
                subquestion.text.clone(),
                subquestion.options.clone(),
            );
            view! { <li class="subquestion">{ask}</li> }
        })
        .collect();

    let nested = if subquestions.is_empty() {
        None
    } else {
        Some(view! { <ol class="subquestions">{subquestions}</ol> })
    };

    view! {
        <li class="question">
            {ask(question.name().to_owned(), question.text, question.options)}
            {nested}
        </li>
    }
}

/// A Question or a Sub-question — both are asked the same way: the name it
/// answers to, its text, its Options as a radio group, then a free-text field.
///
/// `name` is what a Response answers by (`Q7`, `Q7a`), so it names the fields
/// too.
fn ask(name: String, text: String, options: Vec<QuestionOption>) -> impl IntoView {
    let group = format!("{name}-option");
    let field = format!("{name}-free-text");
    let has_options = !options.is_empty();

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
            .map(|option| offered(group.clone(), option))
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
            <textarea id=field.clone() name=field rows="2"></textarea>
        </div>
    }
}

/// One Option on offer: a radio labelled by its number and text.
///
/// The Recommendation is marked and never selected — nothing is selected on
/// load, so an unread Recommendation cannot be submitted by accident.
fn offered(group: String, option: QuestionOption) -> impl IntoView {
    let id = format!("{group}-{}", option.n);
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
                <input type="radio" id=id name=group value=option.n.to_string() />
                <span class="n">{option.n}</span>
                <span class="option-text">{option.text}</span>
                {star}
            </label>
        </li>
    }
}
