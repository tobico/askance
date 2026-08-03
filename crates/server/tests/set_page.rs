//! The set view as the human's browser gets it: the Preface rendered from
//! markdown by the server, then every Question in order, ready to answer.

use askance_schema::{Question, QuestionOption, QuestionSet, Subquestion};
use askance_server::{open_database, router_with_ui, store};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use leptos::prelude::LeptosOptions;
use sqlx::SqlitePool;
use tower::ServiceExt;

/// A pool over a fresh database, plus the directory keeping it alive.
async fn fresh_pool() -> (tempfile::TempDir, SqlitePool) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("askance.db")).await.unwrap();
    (dir, pool)
}

/// The UI's file locations, which these tests never reach: they ask for a
/// rendered page, not for anything under the site root.
fn options() -> LeptosOptions {
    LeptosOptions::builder()
        .output_name("askance")
        .site_root("target/site")
        .build()
}

fn option(n: u32, text: &str, recommended: bool) -> QuestionOption {
    QuestionOption {
        n,
        text: text.to_owned(),
        recommended,
    }
}

fn subquestion(letter: &str, text: &str, options: Vec<QuestionOption>) -> Subquestion {
    Subquestion {
        letter: letter.to_owned(),
        text: text.to_owned(),
        options,
        subquestions: Vec::new(),
    }
}

/// A Set exercising every feature of the question grammar at once: Options
/// with and without a Recommendation, a mixed node carrying both its own
/// Options and Sub-questions, and questions offering no Options at all.
fn full_grammar_set() -> QuestionSet {
    QuestionSet {
        title: "Rate limiting for the public API".to_owned(),
        preface: Some(
            "`POST /v1/messages` has no rate limit.\n\n\
             - one client sent 40k requests in a minute\n\
             - the queue was backed up for twenty\n"
                .to_owned(),
        ),
        questions: vec![
            Question {
                label: "Q1".to_owned(),
                text: "Where should the request counter live?".to_owned(),
                options: vec![
                    option(1, "In-process, per instance.", false),
                    option(2, "In Redis, shared across instances.", true),
                ],
                subquestions: Vec::new(),
            },
            Question {
                label: "Q2".to_owned(),
                text: "How should a throttled client be told to back off?".to_owned(),
                options: vec![
                    option(1, "A bare 429.", false),
                    option(2, "A 429 plus RateLimit headers.", false),
                ],
                subquestions: vec![
                    subquestion(
                        "a",
                        "What should Retry-After say?",
                        vec![
                            option(1, "The exact number of seconds.", false),
                            option(2, "A rounded number.", false),
                        ],
                    ),
                    subquestion("b", "Anything else about the headers?", Vec::new()),
                ],
            },
            Question {
                label: "Q3".to_owned(),
                text: "Anything I should know before starting?".to_owned(),
                options: Vec::new(),
                subquestions: Vec::new(),
            },
        ],
        project: Some("askance".to_owned()),
        branch: Some("answering-web-ui".to_owned()),
        diff: None,
    }
}

/// A Diff as the CLI captures one: a tracked file edited, and an untracked file
/// diffed against the empty file.
fn modified_and_untracked_diff() -> String {
    concat!(
        "diff --git a/src/limits.rs b/src/limits.rs\n",
        "index 4cb29ea..ddc897f 100644\n",
        "--- a/src/limits.rs\n",
        "+++ b/src/limits.rs\n",
        "@@ -1,4 +1,4 @@\n",
        " pub fn allowance() -> u32 {\n",
        "-    60\n",
        "+    600\n",
        " }\n",
        "diff --git a/notes.txt b/notes.txt\n",
        "new file mode 100644\n",
        "index 0000000..cdd6835\n",
        "--- /dev/null\n",
        "+++ b/notes.txt\n",
        "@@ -0,0 +1,2 @@\n",
        "+the queue backed up at 40k/min\n",
        "+a shared counter needs redis\n",
    )
    .to_owned()
}

async fn page(pool: &SqlitePool, path: &str) -> (StatusCode, String) {
    let response = router_with_ui(pool.clone(), options())
        .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
        .await
        .unwrap();

    let status = response.status();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    (status, String::from_utf8(body.to_vec()).unwrap())
}

/// The set view of a stored Set, which is the only page these tests want.
async fn set_page(pool: &SqlitePool, set: &QuestionSet) -> String {
    let stored = store::insert_set(pool, set).await.unwrap();
    let (status, html) = page(pool, &format!("/sets/{}", stored.id)).await;

    assert_eq!(status, StatusCode::OK);
    html
}

/// Where each of these markers sits in the page, in the order given, failing
/// by name when one is missing.
fn positions(html: &str, markers: &[&str]) -> Vec<usize> {
    markers
        .iter()
        .map(|marker| {
            html.find(marker)
                .unwrap_or_else(|| panic!("expected {marker} in the page:\n{html}"))
        })
        .collect()
}

#[tokio::test]
async fn every_question_and_subquestion_renders_in_order() {
    let (_dir, pool) = fresh_pool().await;

    let html = set_page(&pool, &full_grammar_set()).await;

    let found = positions(
        &html,
        &[
            "Q1-free-text",
            "Q2-free-text",
            "Q2a-free-text",
            "Q2b-free-text",
            "Q3-free-text",
        ],
    );
    assert!(
        found.windows(2).all(|pair| pair[0] < pair[1]),
        "expected the questions in Set order, got offsets {found:?}:\n{html}"
    );
}

#[tokio::test]
async fn every_option_of_every_question_is_offered() {
    let (_dir, pool) = fresh_pool().await;

    let html = set_page(&pool, &full_grammar_set()).await;

    for text in [
        "In-process, per instance.",
        "In Redis, shared across instances.",
        "A bare 429.",
        "A 429 plus RateLimit headers.",
        "The exact number of seconds.",
        "A rounded number.",
    ] {
        assert!(html.contains(text), "expected Option {text:?}:\n{html}");
    }

    // Selecting by number is what a Response does, so the radios carry the
    // Option numbers and group by the question they belong to.
    for group in ["Q1-option", "Q2-option", "Q2a-option"] {
        assert!(
            html.contains(&format!(r#"name="{group}""#)),
            "expected a radio group for {group}:\n{html}"
        );
    }
}

#[tokio::test]
async fn a_question_with_no_options_offers_none() {
    let (_dir, pool) = fresh_pool().await;

    let html = set_page(&pool, &full_grammar_set()).await;

    // Q2b and Q3 offer nothing to select, so they get no radio group at all —
    // just their text and a free-text field.
    for group in ["Q2b-option", "Q3-option"] {
        assert!(
            !html.contains(&format!(r#"name="{group}""#)),
            "{group} offers no Options, so it should have no radio group:\n{html}"
        );
    }
}

#[tokio::test]
async fn the_preface_is_rendered_from_markdown_by_the_server() {
    let (_dir, pool) = fresh_pool().await;

    let html = set_page(&pool, &full_grammar_set()).await;

    assert!(
        html.contains("<code>POST /v1/messages</code>"),
        "expected the Preface's markdown rendered to HTML:\n{html}"
    );
    assert!(
        html.contains("<li>one client sent 40k requests in a minute</li>"),
        "expected the Preface's list rendered to HTML:\n{html}"
    );
}

#[tokio::test]
async fn markdown_that_would_run_in_the_browser_does_not_reach_the_page() {
    let (_dir, pool) = fresh_pool().await;
    let mut set = full_grammar_set();
    set.preface = Some(
        "Careful now.\n\n<script>alert('pwned')</script>\n\n\
         <img src=x onerror=\"alert('pwned')\">\n\n\
         [click me](javascript:alert('pwned'))\n"
            .to_owned(),
    );

    let html = set_page(&pool, &set).await;

    assert!(
        html.contains("Careful now."),
        "expected the Preface's prose"
    );
    assert!(
        !html.contains("alert('pwned')"),
        "the Preface's script should have been sanitised away:\n{html}"
    );
    assert!(
        !html.contains("onerror"),
        "the Preface's event handler should have been sanitised away:\n{html}"
    );
    assert!(
        !html.contains("javascript:"),
        "the Preface's script link should have been sanitised away:\n{html}"
    );
}

#[tokio::test]
async fn the_recommendation_is_marked_but_nothing_is_preselected() {
    let (_dir, pool) = fresh_pool().await;

    let html = set_page(&pool, &full_grammar_set()).await;

    // The marks on the Options, not every ★ on the page: the accept-all button
    // spells its own name with one.
    assert_eq!(
        html.matches(r#"class="star">★"#).count(),
        1,
        "expected exactly the one recommended Option marked:\n{html}"
    );
    assert!(
        html.contains("option recommended"),
        "expected the recommended Option to be marked for styling too:\n{html}"
    );
    assert!(
        !html.contains("checked"),
        "nothing may be selected on load, or an unread Recommendation \
         could be submitted by accident:\n{html}"
    );
}

#[tokio::test]
async fn a_set_carrying_a_recommendation_offers_to_accept_them_all() {
    let (_dir, pool) = fresh_pool().await;

    let html = set_page(&pool, &full_grammar_set()).await;

    assert!(
        html.contains(r#"class="accept-all""#),
        "expected the accept-all button on a Set with a Recommendation:\n{html}"
    );
}

#[tokio::test]
async fn a_recommendation_on_a_subquestion_counts_the_same_as_one_on_a_question() {
    let (_dir, pool) = fresh_pool().await;
    let mut set = full_grammar_set();
    set.questions[0].options[1].recommended = false;
    set.questions[1].subquestions[0].options[0].recommended = true;

    let html = set_page(&pool, &set).await;

    assert!(
        html.contains(r#"class="accept-all""#),
        "Sub-questions carry Options, so a ★ on one is a ★ on the Set:\n{html}"
    );
}

#[tokio::test]
async fn a_set_with_no_recommendation_anywhere_offers_no_accept_all() {
    let (_dir, pool) = fresh_pool().await;
    let mut set = full_grammar_set();
    for question in &mut set.questions {
        let subquestions = question.subquestions.iter_mut().map(|sub| &mut sub.options);
        for options in std::iter::once(&mut question.options).chain(subquestions) {
            for option in options {
                option.recommended = false;
            }
        }
    }

    let html = set_page(&pool, &set).await;

    assert!(
        !html.contains("accept-all"),
        "with nothing to accept the button is absent, not disabled:\n{html}"
    );
}

#[tokio::test]
async fn every_question_has_a_free_text_field_and_the_set_has_one_comment_box() {
    let (_dir, pool) = fresh_pool().await;

    let html = set_page(&pool, &full_grammar_set()).await;

    // Five questions — Q1, Q2, Q2a, Q2b, Q3 — plus the set-level comment.
    assert_eq!(
        html.matches("<textarea").count(),
        6,
        "expected a free-text field per question and one comment box:\n{html}"
    );
    assert!(
        html.contains(r#"name="set-comment""#),
        "expected the set-level comment box:\n{html}"
    );
}

#[tokio::test]
async fn the_set_view_shows_where_the_ask_came_from() {
    let (_dir, pool) = fresh_pool().await;

    let html = set_page(&pool, &full_grammar_set()).await;

    assert!(html.contains("Rate limiting for the public API"));
    assert!(html.contains("askance"), "expected the project");
    assert!(html.contains("answering-web-ui"), "expected the branch");
}

#[tokio::test]
async fn an_ask_from_outside_a_repo_shows_no_provenance_at_all() {
    let (_dir, pool) = fresh_pool().await;
    let mut set = full_grammar_set();
    set.project = None;
    set.branch = None;

    let html = set_page(&pool, &set).await;

    assert!(html.contains("Rate limiting for the public API"));
    assert!(
        !html.contains(r#"<p class="meta">"#),
        "with nothing to say, the provenance line should be absent:\n{html}"
    );
}

#[tokio::test]
async fn a_set_with_no_preface_shows_no_preface_section() {
    let (_dir, pool) = fresh_pool().await;
    let mut set = full_grammar_set();
    set.preface = Some("   \n".to_owned());

    let html = set_page(&pool, &set).await;

    assert!(
        !html.contains(r#"<section class="preface">"#),
        "an empty Preface is the same as none:\n{html}"
    );
}

#[tokio::test]
async fn the_attached_diff_is_rendered_per_file() {
    let (_dir, pool) = fresh_pool().await;
    let mut set = full_grammar_set();
    set.diff = Some(modified_and_untracked_diff());

    let html = set_page(&pool, &set).await;

    assert!(
        html.contains(r#"<section class="diff">"#),
        "expected the diff section:\n{html}"
    );
    for path in ["src/limits.rs", "notes.txt"] {
        assert!(
            html.contains(path),
            "expected a section for {path}:\n{html}"
        );
    }
    assert_eq!(
        html.matches(r#"class="diff-file""#).count(),
        2,
        "expected one section per file, whatever git knew of it:\n{html}"
    );

    // The colouring comes from the server: the browser gets no diff parser.
    assert!(html.contains("diff-line add"), "{html}");
    assert!(html.contains("diff-line del"), "{html}");
    assert!(
        html.contains(r#"<span class="tok-"#),
        "expected the Rust file highlighted server-side:\n{html}"
    );
}

#[tokio::test]
async fn a_set_with_no_diff_shows_no_diff_section() {
    let (_dir, pool) = fresh_pool().await;
    let set = full_grammar_set();
    assert!(set.diff.is_none(), "this Set is the one without a Diff");

    let html = set_page(&pool, &set).await;

    assert!(
        !html.contains(r#"<section class="diff">"#),
        "with no Diff attached there is no section to draw:\n{html}"
    );
}

#[tokio::test]
async fn a_set_that_does_not_exist_says_so() {
    let (_dir, pool) = fresh_pool().await;

    let (status, html) = page(&pool, "/sets/404").await;

    assert_eq!(status, StatusCode::OK);
    assert!(
        html.contains("No such Set"),
        "expected the missing-Set state:\n{html}"
    );
}
