//! The set view as the human's browser gets it: the Preface rendered from
//! markdown by the server, then every Question in order, ready to answer — or,
//! once the Set has been answered, the record of what was decided instead of a
//! form.

use askance_schema::{Answer, Question, QuestionOption, QuestionSet, Response, Subquestion};
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

fn answer(label: &str, selected: Option<u32>, free_text: Option<&str>) -> Answer {
    Answer {
        label: label.to_owned(),
        selected,
        free_text: free_text.map(str::to_owned),
        unanswered: false,
    }
}

fn unanswered(label: &str) -> Answer {
    Answer {
        label: label.to_owned(),
        selected: None,
        free_text: None,
        unanswered: true,
    }
}

/// A Response resolving [`full_grammar_set`] every way a question can be
/// resolved: an Option chosen over the agent's Recommendation, an Option with
/// words beside it, words alone where there were no Options, and two questions
/// handed back open.
fn decided_every_way() -> Response {
    Response {
        answers: vec![
            // Q1's ★ is Option 2, and this picks 1: what was suggested and what
            // was decided have to be legible as different things.
            answer("Q1", Some(1), None),
            answer("Q2", Some(2), Some("and document them in the changelog")),
            unanswered("Q2a"),
            answer("Q2b", None, Some("keep them short")),
            unanswered("Q3"),
        ],
        comment: Some("Do the in-process one first; we can move it later.".to_owned()),
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

/// Renders take turns, however many threads the test harness runs on.
///
/// Two server-side renders at once can deadlock inside leptos's reactive graph —
/// a lock-ordering inversion between an effect, an async derived value and a
/// Suspense context (leptos-rs/leptos#4673, closed as not planned) — which wedged
/// this file about two runs in three, at 0% CPU and with nothing printed. Every
/// test here is "ask for a page, look at it", so queueing costs them nothing.
static ONE_RENDER_AT_A_TIME: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

async fn page(pool: &SqlitePool, path: &str) -> (StatusCode, String) {
    let _turn = ONE_RENDER_AT_A_TIME.lock().await;

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

/// The set view of a Set the human closed unanswered: the third standing the
/// page is drawn in, and the one with no Response behind it.
async fn archived_set_page(pool: &SqlitePool, set: &QuestionSet) -> String {
    let stored = store::insert_set(pool, set).await.unwrap();
    let archiving = store::archive_set(pool, &store::Settlements::new(1), stored.id)
        .await
        .unwrap();
    assert!(
        matches!(archiving, store::Archiving::Archived(_)),
        "a freshly stored Set archives unanswered: {archiving:?}"
    );

    let (status, html) = page(pool, &format!("/sets/{}", stored.id)).await;

    assert_eq!(status, StatusCode::OK);
    html
}

/// The set view of a Set that has already been answered, and the time its
/// Response landed.
///
/// The Response goes through validation on the way in, so a test cannot assert
/// on a page drawn from Answers the server would never have stored.
async fn answered_set_page(
    pool: &SqlitePool,
    set: &QuestionSet,
    response: &Response,
) -> (String, String) {
    response
        .validate(set)
        .expect("the Response a test answers with has to resolve its Set");

    let stored = store::insert_set(pool, set).await.unwrap();
    let accepted = store::insert_response(pool, stored.id, response)
        .await
        .unwrap()
        .expect("a freshly stored Set has no Response yet");

    let (status, html) = page(pool, &format!("/sets/{}", stored.id)).await;

    assert_eq!(status, StatusCode::OK);
    (html, accepted.submitted_at)
}

/// The `<li>` whose text contains `needle` — one Option's row, so a test can ask
/// what that Option was marked with.
fn option_row(html: &str, needle: &str) -> String {
    let at = html
        .find(needle)
        .unwrap_or_else(|| panic!("expected Option {needle:?} in the page:\n{html}"));
    let opens = html[..at]
        .rfind("<li")
        .unwrap_or_else(|| panic!("expected Option {needle:?} inside a row:\n{html}"));
    let closes = at + html[at..].find("</li>").unwrap_or(0);

    html[opens..closes].to_owned()
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
        !html.contains(r#"class="preface""#),
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
        html.contains(r#"class="diff""#),
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
        !html.contains(r#"class="diff""#),
        "with no Diff attached there is no section to draw:\n{html}"
    );
}

#[tokio::test]
async fn an_answered_set_offers_nothing_to_press() {
    let (_dir, pool) = fresh_pool().await;

    let (html, _) = answered_set_page(&pool, &full_grammar_set(), &decided_every_way()).await;

    // The Set is drawn, and what is drawn is the record: everything below is an
    // assertion about a page that is really there.
    assert!(
        html.contains(r#"class="questions decided""#),
        "expected the answered Set's questions:\n{html}"
    );
    for absent in ["<input", "<textarea", "<button", "accept-all"] {
        assert!(
            !html.contains(absent),
            "a Set is answered once, so {absent} has no business on the page:\n{html}"
        );
    }
}

#[tokio::test]
async fn an_answered_set_shows_what_was_chosen_apart_from_what_was_recommended() {
    let (_dir, pool) = fresh_pool().await;

    let (html, _) = answered_set_page(&pool, &full_grammar_set(), &decided_every_way()).await;

    // Q1: Option 1 was chosen, and it is Option 2 that carries the ★.
    let chosen = option_row(&html, "In-process, per instance.");
    assert!(
        chosen.contains("chosen"),
        "expected the human's Option marked as chosen:\n{chosen}"
    );
    assert!(
        !chosen.contains("★"),
        "the agent recommended the other one:\n{chosen}"
    );

    let recommended = option_row(&html, "In Redis, shared across instances.");
    assert!(
        recommended.contains("★"),
        "expected the Recommendation still marked as one:\n{recommended}"
    );
    assert!(
        !recommended.contains("chosen"),
        "the Recommendation was not taken, and the page must not read as if it was:\n{recommended}"
    );

    // Every Option is kept, chosen or not: what was turned down is half of what
    // the decision was.
    for text in ["A bare 429.", "The exact number of seconds."] {
        assert!(
            html.contains(text),
            "expected the Option that was not taken:\n{html}"
        );
    }
}

#[tokio::test]
async fn an_answered_set_shows_what_was_written() {
    let (_dir, pool) = fresh_pool().await;

    let (html, _) = answered_set_page(&pool, &full_grammar_set(), &decided_every_way()).await;

    assert!(
        html.contains("and document them in the changelog"),
        "expected the words written beside a chosen Option:\n{html}"
    );
    assert!(
        html.contains("keep them short"),
        "expected the words that were the whole Answer on a question with no Options:\n{html}"
    );
}

#[tokio::test]
async fn a_question_that_went_back_open_says_it_went_back_unanswered() {
    let (_dir, pool) = fresh_pool().await;

    let (html, _) = answered_set_page(&pool, &full_grammar_set(), &decided_every_way()).await;

    // Q2a and Q3 went back open, and both are still drawn: an Unanswered
    // question is part of what the agent was told, not an omission.
    assert!(
        html.contains("What should Retry-After say?"),
        "expected the Sub-question that went back open:\n{html}"
    );
    assert!(
        html.contains("Anything I should know before starting?"),
        "expected the Question that went back open:\n{html}"
    );
    assert_eq!(
        html.matches(r#"class="unanswered""#).count(),
        2,
        "expected exactly the two questions that went back open marked:\n{html}"
    );
}

#[tokio::test]
async fn an_answered_set_says_what_was_said_about_it_and_when() {
    let (_dir, pool) = fresh_pool().await;

    let (html, submitted_at) =
        answered_set_page(&pool, &full_grammar_set(), &decided_every_way()).await;

    assert!(
        html.contains("Do the in-process one first; we can move it later."),
        "expected the set-level comment:\n{html}"
    );
    assert!(
        html.contains(r#"class="answered-at""#),
        "expected the page to say when the Response landed:\n{html}"
    );
    // The stamp SQLite wrote, as the page dates it: the day it was answered,
    // read in the server's own offset.
    assert!(
        html.contains(&submitted_at[..10]) && html.contains(" UTC"),
        "expected {submitted_at} dated on the page:\n{html}"
    );
}

#[tokio::test]
async fn a_set_answered_with_only_a_comment_reads_as_a_counter_question() {
    let (_dir, pool) = fresh_pool().await;
    let set = full_grammar_set();
    let counter_question = Response {
        answers: ["Q1", "Q2", "Q2a", "Q2b", "Q3"].map(unanswered).to_vec(),
        comment: Some("Neither, really — why not cache it upstream?".to_owned()),
    };

    let (html, _) = answered_set_page(&pool, &set, &counter_question).await;

    assert!(
        html.contains(r#"class="counter-question""#),
        "a Response that resolved nothing is still a Response, and has to read as one:\n{html}"
    );
    assert!(
        html.contains("Neither, really — why not cache it upstream?"),
        "expected the comment that is the whole Response:\n{html}"
    );
    assert_eq!(
        html.matches(r#"class="unanswered""#).count(),
        5,
        "every question went back open, and every one of them says so:\n{html}"
    );
}

#[tokio::test]
async fn a_set_nobody_has_answered_yet_is_still_a_form() {
    let (_dir, pool) = fresh_pool().await;

    let html = set_page(&pool, &full_grammar_set()).await;

    assert!(
        html.contains(r#"name="set-comment""#),
        "expected the answerable form:\n{html}"
    );
    for absent in ["answered-at", "counter-question", "chosen"] {
        assert!(
            !html.contains(absent),
            "nothing has been decided here, so {absent} does not belong:\n{html}"
        );
    }
}

#[tokio::test]
async fn the_way_back_out_of_a_set_is_the_list_it_is_on() {
    let (_dir, pool) = fresh_pool().await;

    let waiting = set_page(&pool, &full_grammar_set()).await;
    assert!(
        waiting.contains(r#"href="/""#) && waiting.contains("← Pending"),
        "a Set still waiting is on the pending list:\n{waiting}"
    );

    let (answered, _) = answered_set_page(&pool, &full_grammar_set(), &decided_every_way()).await;
    assert!(
        answered.contains(r#"href="/archive""#) && answered.contains("← Archive"),
        "an answered Set is off the pending list and in the Archive:\n{answered}"
    );
}

#[tokio::test]
async fn every_section_file_and_question_is_addressable_in_the_rendered_page() {
    let (_dir, pool) = fresh_pool().await;
    let mut set = full_grammar_set();
    set.diff = Some(modified_and_untracked_diff());

    let html = set_page(&pool, &set).await;

    // The ids are in the page the server writes, so a hash deep-link lands
    // before any script has run.
    for id in ["preface", "diff", "diff-1", "diff-2", "q1", "q2", "q3"] {
        assert!(
            html.contains(&format!(r#"id="{id}""#)),
            "expected #{id} anchored server-side:\n{html}"
        );
    }
    assert!(
        !html.contains(r#"id="q2a""#),
        "a Sub-question scrolls with its parent and needs no anchor of its own:\n{html}"
    );
}

#[tokio::test]
async fn each_question_anchor_sits_on_the_question_it_names() {
    let (_dir, pool) = fresh_pool().await;

    let html = set_page(&pool, &full_grammar_set()).await;

    let at = html.find(r#"id="q3""#).unwrap();
    assert!(
        html[at..].contains("Anything I should know before starting?"),
        "expected #q3 to open the Question labelled Q3:\n{html}"
    );
}

#[tokio::test]
async fn the_preface_and_the_questions_are_named_by_headings_on_every_standing() {
    let (_dir, pool) = fresh_pool().await;
    let mut set = full_grammar_set();
    set.diff = Some(modified_and_untracked_diff());

    let waiting = set_page(&pool, &set).await;
    let (answered, _) = answered_set_page(&pool, &set, &decided_every_way()).await;
    let archived = archived_set_page(&pool, &set).await;

    for (standing, html) in [
        ("waiting", &waiting),
        ("answered", &answered),
        ("archived unanswered", &archived),
    ] {
        // Named so a jump from the table of contents lands somewhere the reader
        // can see they have arrived at, and quiet enough not to shout over the
        // title — the same heading the Diff already had.
        for heading in ["Preface", "Questions", "Diff"] {
            assert!(
                html.contains(&format!(r#"class="section-heading">{heading}</h2>"#)),
                "expected the {heading} heading on a {standing} Set:\n{html}"
            );
        }
        for id in ["preface", "diff", "diff-1", "questions", "q1"] {
            assert!(
                html.contains(&format!(r#"id="{id}""#)),
                "expected #{id} on a {standing} Set:\n{html}"
            );
        }
    }
}

#[tokio::test]
async fn a_section_the_set_does_not_have_gets_no_heading_and_no_anchor() {
    let (_dir, pool) = fresh_pool().await;
    let mut set = full_grammar_set();
    set.preface = Some("   \n".to_owned());
    assert!(set.diff.is_none(), "and no Diff either");

    let html = set_page(&pool, &set).await;

    for absent in [r#"id="preface""#, r#"id="diff""#, r#"id="diff-1""#] {
        assert!(
            !html.contains(absent),
            "there is no section for {absent} to anchor:\n{html}"
        );
    }
    for heading in ["Preface", "Diff"] {
        assert!(
            !html.contains(&format!(r#"class="section-heading">{heading}</h2>"#)),
            "with no {heading} there is no heading to draw:\n{html}"
        );
    }
    assert!(
        html.contains(r#"id="questions""#),
        "the Questions are the one section every Set has:\n{html}"
    );
}

/// The table of contents as the server writes it — which is what a reader has
/// before any script has run, and what hydration then takes over.
fn table_of_contents(html: &str) -> String {
    let at = html
        .find("<nav")
        .unwrap_or_else(|| panic!("expected a table of contents in the page:\n{html}"));
    let closes = html[at..]
        .find("</nav>")
        .unwrap_or_else(|| panic!("expected the nav to close:\n{html}"));

    html[at..at + closes].to_owned()
}

#[tokio::test]
async fn the_table_of_contents_mirrors_the_page_top_to_bottom() {
    let (_dir, pool) = fresh_pool().await;
    let mut set = full_grammar_set();
    set.diff = Some(modified_and_untracked_diff());

    let contents = table_of_contents(&set_page(&pool, &set).await);

    let jumps = positions(
        &contents,
        &[
            r##"href="#preface""##,
            r##"href="#diff""##,
            r##"href="#diff-1""##,
            r##"href="#diff-2""##,
            r##"href="#questions""##,
            r##"href="#q1""##,
            r##"href="#q2""##,
            r##"href="#q3""##,
        ],
    );
    let mut ordered = jumps.clone();
    ordered.sort_unstable();
    assert_eq!(
        jumps, ordered,
        "the nav lists the sections in the order the page has them:\n{contents}"
    );

    assert!(
        !contents.contains(r##"href="#q2a""##),
        "a Sub-question scrolls into view with its parent, so it is not listed \
         separately:\n{contents}"
    );
    assert!(
        contents.contains("Where should the request counter live?"),
        "a Question is listed by its label and its own words:\n{contents}"
    );
}

#[tokio::test]
async fn the_contents_names_the_diff_files_in_diff_order() {
    let (_dir, pool) = fresh_pool().await;
    let mut set = full_grammar_set();
    set.diff = Some(modified_and_untracked_diff());

    let contents = table_of_contents(&set_page(&pool, &set).await);

    // The paths travel with the Set rather than being read back out of the
    // rendered Diff, and the nth of them has to be what the nth fold shows —
    // `#diff-1` is the file the Diff names first.
    let first = positions(&contents, &[r##"href="#diff-1""##])[0];
    let second = positions(&contents, &[r##"href="#diff-2""##])[0];

    assert!(
        contents[first..second].contains("src/limits.rs"),
        "expected the Diff's first file under #diff-1:\n{contents}"
    );
    assert!(
        contents[second..].contains("notes.txt"),
        "expected the Diff's second file under #diff-2:\n{contents}"
    );
}

#[tokio::test]
async fn the_contents_lists_only_the_sections_the_set_has() {
    let (_dir, pool) = fresh_pool().await;
    let mut set = full_grammar_set();
    set.preface = Some("   \n".to_owned());
    assert!(set.diff.is_none(), "and no Diff either");

    let contents = table_of_contents(&set_page(&pool, &set).await);

    for absent in [r##"href="#preface""##, r##"href="#diff""##] {
        assert!(
            !contents.contains(absent),
            "there is no such section to jump to: {absent}\n{contents}"
        );
    }
    assert!(
        contents.contains(r##"href="#questions""##) && contents.contains(r##"href="#q1""##),
        "the Questions are the one section every Set has:\n{contents}"
    );
}

#[tokio::test]
async fn every_standing_gets_a_table_of_contents() {
    let (_dir, pool) = fresh_pool().await;
    let mut set = full_grammar_set();
    set.diff = Some(modified_and_untracked_diff());

    let waiting = set_page(&pool, &set).await;
    let (answered, _) = answered_set_page(&pool, &set, &decided_every_way()).await;
    let archived = archived_set_page(&pool, &set).await;

    for (standing, html) in [
        ("waiting", &waiting),
        ("answered", &answered),
        ("archived unanswered", &archived),
    ] {
        let contents = table_of_contents(html);

        for jump in [
            r##"href="#preface""##,
            r##"href="#diff-1""##,
            r##"href="#questions""##,
            r##"href="#q1""##,
        ] {
            assert!(
                contents.contains(jump),
                "expected {jump} on a {standing} Set — a Set is read for what it \
                 asked about however it stands:\n{contents}"
            );
        }
    }
}

/// Where the nav says the reader is: the jump the highlighted line points at.
///
/// Read off the rendered nav rather than from a class list, because what the
/// highlight is worth is which part of the page it names.
fn highlighted(contents: &str) -> String {
    let lit = contents
        .find("contents-here")
        .unwrap_or_else(|| panic!("expected a highlighted line in the nav:\n{contents}"));

    // Back to the start of the line's own tag, since the class is written after
    // the href it carries.
    let opens = contents[..lit]
        .rfind("<a ")
        .unwrap_or_else(|| panic!("expected the highlight on a link:\n{contents}"));

    let jump = r##"href="#"##;
    let at = contents[opens..lit]
        .find(jump)
        .unwrap_or_else(|| panic!("expected the highlighted line to jump somewhere:\n{contents}"));

    let from = opens + at + jump.len();
    let closes = contents[from..].find('"').expect("an attribute closes");

    contents[from..from + closes].to_owned()
}

#[tokio::test]
async fn the_first_section_is_highlighted_before_any_script_has_run() {
    let (_dir, pool) = fresh_pool().await;
    let mut set = full_grammar_set();
    set.diff = Some(modified_and_untracked_diff());

    let contents = table_of_contents(&set_page(&pool, &set).await);

    assert_eq!(
        highlighted(&contents),
        "preface",
        "a page nobody has scrolled reads as being at the top of it, so the nav \
         the server writes is already right and the scroll-spy has nothing to \
         correct when the wasm arrives",
    );
    assert_eq!(
        contents.matches("contents-here").count(),
        1,
        "exactly one line is ever the highlight:\n{contents}"
    );
    assert_eq!(
        contents.matches("contents-within").count(),
        0,
        "and the quiet mark is on the section the highlight is inside, so at the \
         top of the page there is none:\n{contents}"
    );
}

#[tokio::test]
async fn the_highlight_starts_on_whatever_section_the_set_starts_with() {
    let (_dir, pool) = fresh_pool().await;
    let mut set = full_grammar_set();
    set.preface = None;
    set.diff = Some(modified_and_untracked_diff());

    let contents = table_of_contents(&set_page(&pool, &set).await);

    assert_eq!(
        highlighted(&contents),
        "diff",
        "with no Preface the page opens on the Diff, and the first line of the \
         nav is the Diff's",
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
