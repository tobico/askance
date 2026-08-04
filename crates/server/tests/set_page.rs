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

/// The same Set written the way agents write it: Questions carrying a bulleted
/// list with a code span in it, a fenced code block, and a GFM table on a
/// Sub-question — and Options carrying markup of their own, one of them with a
/// block an Option has no room for.
///
/// The labels and the Option numbers are untouched, so a Response resolving
/// [`full_grammar_set`] resolves this too.
fn marked_up_set() -> QuestionSet {
    let mut set = full_grammar_set();

    set.questions[0].text = "Where should the request counter live?\n\n\
         - in-process, per instance\n\
         - in `redis`, shared across instances\n"
        .to_owned();
    set.questions[0].options[0].text =
        "In-process, per instance — see `Counter::local`.".to_owned();
    set.questions[0].options[1].text = "In **Redis**, shared across instances.".to_owned();
    set.questions[1].text = "How should a throttled client be told to back off?\n\n\
         ```rust\n\
         fn allowance() -> u32 { 600 }\n\
         ```\n"
        .to_owned();
    set.questions[1].options[0].text = "A bare 429.\n\n\
         - no headers\n\
         - no body\n"
        .to_owned();
    set.questions[1].subquestions[0].text = "What should Retry-After say?\n\n\
         | header | seconds |\n\
         | --- | --- |\n\
         | Retry-After | 30 |\n"
        .to_owned();

    set
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
async fn a_questions_markdown_is_rendered_by_the_server() {
    let (_dir, pool) = fresh_pool().await;

    let html = set_page(&pool, &marked_up_set()).await;

    assert!(
        html.contains("<li>in-process, per instance</li>"),
        "expected the Question's list rendered to HTML:\n{html}"
    );
    assert!(
        html.contains("<code>redis</code>"),
        "expected the Question's code span rendered to HTML:\n{html}"
    );
    assert!(
        html.contains("<pre>") && html.contains("fn allowance()"),
        "expected the Question's fenced block rendered as one:\n{html}"
    );
    assert!(
        html.contains("<table>") && html.contains("<td>Retry-After</td>"),
        "expected the Sub-question's table rendered to HTML:\n{html}"
    );
    assert!(
        !html.contains("| --- |"),
        "nothing may reach the page as raw markup:\n{html}"
    );
    // A list or a table inside a `<p>` closes it where the browser says so
    // rather than where the markup does, which would leave the page the server
    // rendered and the page the browser hydrates disagreeing about its shape.
    assert!(
        !html.contains(r#"<p class="text">"#),
        "a question's text holds block markdown, so it cannot be a paragraph:\n{html}"
    );
}

#[tokio::test]
async fn a_questions_label_still_sits_at_the_head_of_its_rendered_text() {
    let (_dir, pool) = fresh_pool().await;

    let html = set_page(&pool, &marked_up_set()).await;

    // The label a Response answers by, then the text it labels — a Question and
    // a Sub-question alike, however blocky the markdown under it.
    let found = positions(
        &html,
        &[
            r#"class="label">Q1"#,
            "<li>in-process, per instance</li>",
            r#"class="label">Q2a"#,
            "<td>Retry-After</td>",
        ],
    );
    assert!(
        found.windows(2).all(|pair| pair[0] < pair[1]),
        "expected each label at the head of its own text, got offsets {found:?}:\n{html}"
    );
}

#[tokio::test]
async fn markdown_that_would_run_in_the_browser_does_not_reach_a_question() {
    let (_dir, pool) = fresh_pool().await;
    let mut set = full_grammar_set();
    set.questions[0].text = "Careful now.\n\n<script>alert('pwned')</script>\n\n\
         <img src=x onerror=\"alert('pwned')\">\n\n\
         [click me](javascript:alert('pwned'))\n"
        .to_owned();

    let html = set_page(&pool, &set).await;

    assert!(
        html.contains("Careful now."),
        "expected the Question's prose"
    );
    assert!(
        !html.contains("alert('pwned')"),
        "the Question's script should have been sanitised away:\n{html}"
    );
    assert!(
        !html.contains("onerror"),
        "the Question's event handler should have been sanitised away:\n{html}"
    );
    assert!(
        !html.contains("javascript:"),
        "the Question's script link should have been sanitised away:\n{html}"
    );
}

#[tokio::test]
async fn a_settled_sets_questions_are_rendered_the_way_the_form_rendered_them() {
    let (_dir, pool) = fresh_pool().await;

    let (answered, _) = answered_set_page(&pool, &marked_up_set(), &decided_every_way()).await;
    let archived = archived_set_page(&pool, &marked_up_set()).await;

    for html in [&answered, &archived] {
        assert!(
            html.contains("<li>in-process, per instance</li>")
                && html.contains("<code>redis</code>")
                && html.contains("<td>Retry-After</td>"),
            "a settled Set is read for what was asked, so its markdown is rendered too:\n{html}"
        );
    }
}

#[tokio::test]
async fn an_options_markdown_is_rendered_inline_by_the_server() {
    let (_dir, pool) = fresh_pool().await;

    let html = set_page(&pool, &marked_up_set()).await;

    let quoted = option_row(&html, "Counter::local");
    assert!(
        quoted.contains("<code>Counter::local</code>"),
        "expected the Option's code span rendered to HTML:\n{quoted}"
    );
    assert!(
        html.contains("<strong>Redis</strong>"),
        "expected the Option's emphasis rendered to HTML:\n{html}"
    );

    // The row is the tap target, and it is the label wrapping the radio that
    // makes it one: the rendered text has to sit inside that label beside the
    // radio it selects, still answering by number.
    assert!(
        quoted.contains("<label>")
            && quoted.contains(r#"name="Q1-option""#)
            && quoted.contains(r#"value="1""#),
        "expected the Option's radio and its text in the one label:\n{quoted}"
    );
    assert!(
        quoted.matches("<input").count() == 1,
        "expected exactly the one radio in the row:\n{quoted}"
    );
}

#[tokio::test]
async fn block_markdown_in_an_option_is_flattened_into_its_row() {
    let (_dir, pool) = fresh_pool().await;

    let html = set_page(&pool, &marked_up_set()).await;

    // An Option is one line beside a radio, so a list inside its label would
    // break the row apart — and the whole row is what the human taps.
    assert!(
        !html.contains("<li>no headers</li>"),
        "an Option's list may not be drawn as one:\n{html}"
    );

    let row = option_row(&html, "A bare 429.");
    assert!(
        row.contains("no headers") && row.contains("no body"),
        "flattened, not dropped: every word the agent wrote is still in the row:\n{row}"
    );
    assert!(
        row.matches("<input").count() == 1 && row.contains(r#"name="Q2-option""#),
        "expected the flattened Option still drawn as a single row:\n{row}"
    );
}

#[tokio::test]
async fn markdown_that_would_run_in_the_browser_does_not_reach_an_option() {
    let (_dir, pool) = fresh_pool().await;
    let mut set = full_grammar_set();
    set.questions[0].options[0].text = "Careful now. <script>alert('pwned')</script> \
         <img src=\"x\" onerror=\"alert('pwned')\"> \
         [click me](javascript:alert('pwned'))"
        .to_owned();

    let html = set_page(&pool, &set).await;

    assert!(html.contains("Careful now."), "expected the Option's words");
    assert!(
        html.contains("click me"),
        "expected the link's words, which are all that is left of it:\n{html}"
    );
    assert!(
        !html.contains("alert('pwned')"),
        "the Option's script should have been sanitised away:\n{html}"
    );
    assert!(
        !html.contains("onerror"),
        "the Option's event handler should have been sanitised away:\n{html}"
    );
    assert!(
        !html.contains("javascript:"),
        "the Option's script link should have been sanitised away:\n{html}"
    );
}

#[tokio::test]
async fn a_settled_sets_options_read_with_their_markup_and_their_marks() {
    let (_dir, pool) = fresh_pool().await;

    let (html, _) = answered_set_page(&pool, &marked_up_set(), &decided_every_way()).await;

    // Q1: Option 1 was chosen and Option 2 carries the ★. Read back, each still
    // has its number and its marks beside the text the agent wrote.
    let chosen = option_row(&html, "<code>Counter::local</code>");
    assert!(
        chosen.contains(r#"class="n">1"#) && chosen.contains("chosen"),
        "expected the chosen Option numbered and marked beside its markup:\n{chosen}"
    );

    let recommended = option_row(&html, "<strong>Redis</strong>");
    assert!(
        recommended.contains(r#"class="n">2"#) && recommended.contains("★"),
        "expected the Recommendation numbered and starred beside its markup:\n{recommended}"
    );
    assert!(
        !recommended.contains("chosen"),
        "the Recommendation was not taken:\n{recommended}"
    );
}

/// Every place the server renders the agent's markdown hangs off the one class,
/// so a heading, a table or a fenced block is drawn the same way wherever it was
/// written. Without this each place grows a copy of the rules, and they drift.
#[tokio::test]
async fn rendered_markdown_is_marked_for_one_set_of_styles_wherever_it_appears() {
    let (_dir, pool) = fresh_pool().await;

    let html = set_page(&pool, &marked_up_set()).await;

    for marked in [
        // The Preface's body rather than its section: the section is anchored
        // and headed for the table of contents, and the markdown is what sits
        // inside it.
        r#"<div class="preface-body markdown""#,
        r#"<div class="markdown""#,
        r#"<span class="option-text markdown""#,
    ] {
        assert!(
            html.contains(marked),
            "expected rendered markdown marked by `{marked}`:\n{html}"
        );
    }
}

#[tokio::test]
async fn the_recommendation_is_marked_but_nothing_is_preselected() {
    let (_dir, pool) = fresh_pool().await;

    let html = set_page(&pool, &full_grammar_set()).await;

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
        !html.contains(r#"class="preface"#),
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
    for absent in ["<input", "<textarea"] {
        assert!(
            !html.contains(absent),
            "a Set is answered once, so {absent} has no business on the page:\n{html}"
        );
    }

    // The nav's bar is a button, and the only one an answered Set has: it is a way
    // around the record rather than anything that acts on it. Counted rather than
    // excused, so a button that does act on the Set still fails this.
    assert_eq!(
        html.matches("<button").count(),
        1,
        "expected the nav's bar and nothing else to press:\n{html}"
    );
    assert!(
        html.contains(r#"class="contents-bar""#),
        "and that one button to be the nav's:\n{html}"
    );
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

/// The nav is a line of text in a narrow column, so a Question written as
/// markdown is named there by its words alone. The page draws that same Question
/// as blocks, and the two are rendered from the one source — this is the seam
/// where the table of contents and the rendered markdown meet, and neither
/// feature's own tests can see it.
#[tokio::test]
async fn the_contents_names_a_markdown_question_by_its_words_alone() {
    let (_dir, pool) = fresh_pool().await;

    let contents = table_of_contents(&set_page(&pool, &marked_up_set()).await);

    // Q1's text is a paragraph over a bulleted list; Q2's is a paragraph over a
    // fenced block. Flattened, both read on as one line.
    assert!(
        contents.contains(
            "Where should the request counter live? in-process, per instance \
             in redis, shared across instances"
        ),
        "the nav wants the words, with the list flattened into the line:\n{contents}"
    );
    assert!(
        contents.contains("How should a throttled client be told to back off? fn allowance()"),
        "a fenced block is words in the nav too:\n{contents}"
    );

    for markup in ["<ul>", "<li>", "<pre>", "<code>", "<p>"] {
        assert!(
            !contents.contains(markup),
            "the nav is text, so `{markup}` has no place in it:\n{contents}"
        );
    }
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

/// The bar's own words: what the narrow-viewport reader sees before they open
/// anything. Read as the rendered HTML of the bar's name rather than as text,
/// because a Question's name is its label and its words in two faces — and
/// because a reactive child arrives with hydration markers around it.
fn bar_says(contents: &str) -> String {
    let opens = r#"<span class="contents-bar-name"#;
    let at = contents
        .find(opens)
        .unwrap_or_else(|| panic!("expected a bar naming the section in the nav:\n{contents}"));
    let from = at + contents[at..].find('>').expect("the tag opens") + 1;
    let closes = contents[from..]
        .find("</span>")
        .unwrap_or_else(|| panic!("expected the bar's name to close:\n{contents}"));

    contents[from..from + closes].to_owned()
}

#[tokio::test]
async fn the_bar_names_the_line_the_nav_has_highlighted() {
    let (_dir, pool) = fresh_pool().await;
    let mut set = full_grammar_set();
    set.diff = Some(modified_and_untracked_diff());

    let contents = table_of_contents(&set_page(&pool, &set).await);

    assert!(
        bar_says(&contents).contains("Preface"),
        "the bar reads out the line the nav has lit, and on a page nobody has \
         scrolled that is the first of them:\n{contents}"
    );
    assert_eq!(
        highlighted(&contents),
        "preface",
        "which is the same line the sidebar marks — one scroll-spy answers for both",
    );
}

#[tokio::test]
async fn the_bar_names_whatever_section_the_set_starts_with() {
    let (_dir, pool) = fresh_pool().await;
    let mut set = full_grammar_set();
    set.preface = None;
    set.diff = Some(modified_and_untracked_diff());

    let contents = table_of_contents(&set_page(&pool, &set).await);
    let says = bar_says(&contents);

    assert!(
        says.contains("Diff") && !says.contains("Preface"),
        "with no Preface the page opens on the Diff, so that is what the bar \
         names:\n{contents}"
    );
}

#[tokio::test]
async fn the_bar_arrives_shut() {
    let (_dir, pool) = fresh_pool().await;
    let mut set = full_grammar_set();
    set.diff = Some(modified_and_untracked_diff());

    let contents = table_of_contents(&set_page(&pool, &set).await);

    assert!(
        contents.contains(r#"aria-expanded="false""#),
        "the list is down only once the reader asks for it:\n{contents}"
    );
    assert!(
        !contents.contains("contents-open"),
        "and nothing has opened it yet:\n{contents}"
    );
    assert!(
        contents.contains(r##"href="#q1""##),
        "the entries are in the page all the same — the same list the sidebar \
         draws, so opening the bar has nothing to fetch and a hash link works \
         before the wasm lands:\n{contents}"
    );
}

#[tokio::test]
async fn the_bar_and_the_sidebar_are_the_one_nav_on_every_standing() {
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
        // One nav holding one bar and one list, so which of the bar and the
        // sidebar the reader gets is the stylesheet's business at a width — and
        // there is no second copy to fall out of step with the first.
        assert_eq!(
            html.matches("<nav").count(),
            1,
            "expected exactly one nav on a {standing} Set:\n{html}"
        );
        assert_eq!(
            html.matches("contents-bar\"").count(),
            1,
            "expected exactly one bar on a {standing} Set:\n{html}"
        );
        assert_eq!(
            html.matches(r#"class="contents-sections""#).count(),
            1,
            "and exactly one list for the two of them to share on a {standing} \
             Set:\n{html}"
        );
    }
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
