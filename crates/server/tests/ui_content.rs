//! What the viewer is handed of a Set's own material: the agent's markdown
//! rendered and sanitized, the Diff parsed and highlighted, and the facts about
//! the Set that decide how the page is drawn.
//!
//! These are the content assertions the server-rendered page tests made about
//! the HTML they produced, asked here of the JSON `/api/ui/sets/{id}` answers
//! with instead. That is where they belong: rendering agent-supplied content is
//! the server's job whatever draws the page, and the drawing is the viewer's own
//! tests' subject.
//!
//! The same tests leave the golden fixtures behind — see
//! [`the_viewers_own_tests_are_fed_from_here`]. The viewer's component tests read
//! those files rather than a hand-written mock, so a change to the wire shape
//! that nobody carried across shows up as a failing fixture rather than as a
//! viewer that draws the wrong thing.

use askance_render::{Answered, SetView, Standing};
use askance_schema::{
    Answer, Liveness, Question, QuestionOption, QuestionSet, Response, Subquestion,
};
use askance_server::{open_database, router, store};
use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use sqlx::SqlitePool;
use tower::ServiceExt;

/// A router over a fresh database, plus the pool and the directory keeping it
/// alive.
async fn fresh_app() -> (tempfile::TempDir, SqlitePool, Router) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("askance.db")).await.unwrap();
    (dir, pool.clone(), router(pool))
}

fn option(n: u32, text: &str, recommended: bool) -> QuestionOption {
    QuestionOption {
        n,
        text: text.to_owned(),
        recommended,
        cells: Vec::new(),
    }
}

fn subquestion(letter: &str, text: &str, options: Vec<QuestionOption>) -> Subquestion {
    Subquestion {
        letter: letter.to_owned(),
        text: text.to_owned(),
        columns: Vec::new(),
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
                columns: Vec::new(),
                options: vec![
                    option(1, "In-process, per instance.", false),
                    option(2, "In Redis, shared across instances.", true),
                ],
                subquestions: Vec::new(),
            },
            Question {
                label: "Q2".to_owned(),
                text: "How should a throttled client be told to back off?".to_owned(),
                columns: Vec::new(),
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
                columns: Vec::new(),
                options: Vec::new(),
                subquestions: Vec::new(),
            },
        ],
        postscript: None,
        project: Some("askance".to_owned()),
        branch: Some("solid-viewer".to_owned()),
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

/// The same Set with its Options declared as Answer Tables: axes on `Q1` and on
/// the Sub-question `Q2a`, a row on each of their Options, and markup in the
/// headers and the cells alike.
///
/// The labels and the Option numbers are untouched, so a Response resolving
/// [`full_grammar_set`] resolves this too.
fn tabulated_set() -> QuestionSet {
    let mut set = full_grammar_set();

    set.questions[0].columns = vec!["Latency".to_owned(), "`ops` cost".to_owned()];
    set.questions[0].options[0].text = "In-process, per instance.".to_owned();
    set.questions[0].options[0].cells = vec!["Sub-`ms`".to_owned(), "None".to_owned()];
    set.questions[0].options[1].cells = vec!["**A hop**".to_owned(), "A box to run".to_owned()];

    set.questions[1].subquestions[0].columns = vec!["Memory".to_owned()];
    set.questions[1].subquestions[0].options[0].text = "LRU".to_owned();
    set.questions[1].subquestions[0].options[0].cells = vec!["Bounded".to_owned()];
    set.questions[1].subquestions[0].options[1].cells = vec!["Unbounded".to_owned()];

    set
}

/// The same Set with a Diagram in its Preface: the structural half of what the
/// agent is saying, drawn rather than described.
fn diagrammed_set() -> QuestionSet {
    let mut set = full_grammar_set();

    set.preface = Some(
        concat!(
            "Where the counter would live:\n",
            "\n",
            "```mermaid\n",
            "graph LR;\n",
            "  client-->api;\n",
            "  api-->redis;\n",
            "```\n",
        )
        .to_owned(),
    );

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

/// Ask for a stored Set the way the viewer does, and read back both the JSON as
/// it went out and the Set it deserialises to.
///
/// The text is kept because most of what these tests assert is about rendered
/// HTML *not* reaching the viewer, and the cheapest honest way to say "nowhere in
/// this payload" is to look at the payload.
async fn set_json(app: &Router, pool: &SqlitePool, set: &QuestionSet) -> (SetView, String) {
    let stored = store::insert_set(pool, set).await.unwrap();
    fetch_set(app, stored.id).await
}

async fn fetch_set(app: &Router, id: i64) -> (SetView, String) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/ui/sets/{id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body = String::from_utf8(bytes.to_vec()).unwrap();

    assert_eq!(status, StatusCode::OK, "asking for Set {id}: {body}");

    let view = serde_json::from_str(&body).unwrap_or_else(|err| panic!("reading {body:?}: {err}"));
    (view, body)
}

/// A Set that has already been answered, and the time its Response landed.
///
/// The Response goes through validation on the way in, so a test cannot assert
/// on a Set drawn from Answers the server would never have stored.
async fn answered_set(
    app: &Router,
    pool: &SqlitePool,
    set: &QuestionSet,
    response: &Response,
) -> (SetView, String) {
    response
        .validate(set)
        .expect("the Response a test answers with has to resolve its Set");

    let stored = store::insert_set(pool, set).await.unwrap();
    store::insert_response(pool, stored.id, response)
        .await
        .unwrap()
        .expect("a freshly stored Set has no Response yet");

    fetch_set(app, stored.id).await
}

/// A Set the human closed unanswered: the third standing, and the one with no
/// Response behind it.
async fn archived_set(app: &Router, pool: &SqlitePool, set: &QuestionSet) -> (SetView, String) {
    let stored = store::insert_set(pool, set).await.unwrap();
    let archiving = store::archive_set(pool, &store::Settlements::new(1), stored.id)
        .await
        .unwrap();
    assert!(
        matches!(archiving, store::Archiving::Archived(_)),
        "a freshly stored Set archives unanswered: {archiving:?}"
    );

    fetch_set(app, stored.id).await
}

/// Every question and Sub-question of a Set, in the order the viewer draws them,
/// by the name a Response answers each of them by.
fn asked(set: &SetView) -> Vec<&str> {
    set.questions
        .iter()
        .flat_map(|question| {
            std::iter::once(question.ask.name.as_str())
                .chain(question.subquestions.iter().map(|sub| sub.name.as_str()))
        })
        .collect()
}

/// The Option of this Set carrying `needle` in its rendered text, wherever it
/// was offered.
fn option_with<'a>(set: &'a SetView, needle: &str) -> &'a askance_render::OptionView {
    set.questions
        .iter()
        .flat_map(|question| std::iter::once(&question.ask).chain(question.subquestions.iter()))
        .flat_map(|ask| ask.options.iter())
        .find(|option| option.text_html.contains(needle))
        .unwrap_or_else(|| panic!("expected an Option whose text holds {needle:?}"))
}

#[tokio::test]
async fn every_question_and_subquestion_arrives_in_order_under_the_name_it_answers_to() {
    let (_dir, pool, app) = fresh_app().await;

    let (set, _) = set_json(&app, &pool, &full_grammar_set()).await;

    // A Sub-question's name is its parent's label and its letter, resolved on
    // the way out: the viewer never has to work one out.
    assert_eq!(asked(&set), ["Q1", "Q2", "Q2a", "Q2b", "Q3"]);
}

#[tokio::test]
async fn every_option_of_every_question_is_offered_by_the_number_a_response_answers_by() {
    let (_dir, pool, app) = fresh_app().await;

    let (set, _) = set_json(&app, &pool, &full_grammar_set()).await;

    let offered: Vec<(&str, Vec<(u32, bool)>)> = set
        .questions
        .iter()
        .flat_map(|question| std::iter::once(&question.ask).chain(question.subquestions.iter()))
        .map(|ask| {
            (
                ask.name.as_str(),
                ask.options
                    .iter()
                    .map(|option| (option.n, option.recommended))
                    .collect(),
            )
        })
        .collect();

    assert_eq!(
        offered,
        [
            // The Recommendation is marked where the agent put it, and it is the
            // only one marked.
            ("Q1", vec![(1, false), (2, true)]),
            ("Q2", vec![(1, false), (2, false)]),
            ("Q2a", vec![(1, false), (2, false)]),
            // A question that offered nothing to select offers nothing here.
            ("Q2b", vec![]),
            ("Q3", vec![]),
        ]
    );

    for text in [
        "In-process, per instance.",
        "In Redis, shared across instances.",
        "A bare 429.",
        "A 429 plus RateLimit headers.",
        "The exact number of seconds.",
        "A rounded number.",
    ] {
        assert!(
            set.questions
                .iter()
                .flat_map(|question| {
                    std::iter::once(&question.ask).chain(question.subquestions.iter())
                })
                .flat_map(|ask| ask.options.iter())
                .any(|option| option.text_html.contains(text)),
            "expected Option {text:?}"
        );
    }
}

#[tokio::test]
async fn the_preface_is_rendered_from_markdown_by_the_server() {
    let (_dir, pool, app) = fresh_app().await;

    let (set, _) = set_json(&app, &pool, &full_grammar_set()).await;
    let preface = set.preface_html.expect("this Set has a Preface");

    assert!(
        preface.contains("<code>POST /v1/messages</code>"),
        "expected the Preface's markdown rendered to HTML:\n{preface}"
    );
    assert!(
        preface.contains("<li>one client sent 40k requests in a minute</li>"),
        "expected the Preface's list rendered to HTML:\n{preface}"
    );
}

#[tokio::test]
async fn markdown_that_would_run_in_the_browser_does_not_reach_the_preface() {
    let (_dir, pool, app) = fresh_app().await;
    let mut set = full_grammar_set();
    set.preface = Some(
        "Careful now.\n\n<script>alert('pwned')</script>\n\n\
         <img src=x onerror=\"alert('pwned')\">\n\n\
         [click me](javascript:alert('pwned'))\n"
            .to_owned(),
    );

    let (view, json) = set_json(&app, &pool, &set).await;

    assert!(
        view.preface_html
            .as_deref()
            .is_some_and(|html| html.contains("Careful now.")),
        "expected the Preface's prose"
    );
    assert_sanitised(&json, "the Preface");
}

#[tokio::test]
async fn the_postscript_is_rendered_from_markdown_by_the_server() {
    let (_dir, pool, app) = fresh_app().await;
    let mut set = full_grammar_set();
    set.postscript = Some(
        "Worth taking up in the comment:\n\n\
         - whether `ops/export` gets an allowlist entry\n"
            .to_owned(),
    );

    let (view, _) = set_json(&app, &pool, &set).await;
    let postscript = view.postscript_html.expect("this Set has a Postscript");

    assert!(
        postscript.contains("<code>ops/export</code>"),
        "expected the Postscript's markdown rendered to HTML:\n{postscript}"
    );
    assert!(
        postscript.contains("<li>whether <code>ops/export</code> gets an allowlist entry</li>"),
        "expected the Postscript's list rendered to HTML:\n{postscript}"
    );
}

#[tokio::test]
async fn markdown_that_would_run_in_the_browser_does_not_reach_the_postscript() {
    let (_dir, pool, app) = fresh_app().await;
    let mut set = full_grammar_set();
    set.postscript = Some(
        "One last thing.\n\n<script>alert('pwned')</script>\n\n\
         <img src=x onerror=\"alert('pwned')\">\n\n\
         [click me](javascript:alert('pwned'))\n"
            .to_owned(),
    );

    let (view, json) = set_json(&app, &pool, &set).await;

    assert!(
        view.postscript_html
            .as_deref()
            .is_some_and(|html| html.contains("One last thing.")),
        "expected the Postscript's prose"
    );
    assert_sanitised(&json, "the Postscript");
}

#[tokio::test]
async fn a_set_with_no_postscript_carries_none() {
    let (_dir, pool, app) = fresh_app().await;
    let set = full_grammar_set();
    assert!(
        set.postscript.is_none(),
        "this Set is the one that closes with nothing"
    );

    let (view, _) = set_json(&app, &pool, &set).await;

    assert_eq!(
        view.postscript_html, None,
        "there is no section to draw above the comment box"
    );
}

#[tokio::test]
async fn a_questions_markdown_is_rendered_by_the_server() {
    let (_dir, pool, app) = fresh_app().await;

    let (set, json) = set_json(&app, &pool, &marked_up_set()).await;

    assert!(
        set.questions[0]
            .ask
            .text_html
            .contains("<li>in-process, per instance</li>"),
        "expected the Question's list rendered to HTML:\n{}",
        set.questions[0].ask.text_html
    );
    assert!(
        set.questions[0]
            .ask
            .text_html
            .contains("<code>redis</code>"),
        "expected the Question's code span rendered to HTML:\n{}",
        set.questions[0].ask.text_html
    );
    // Rendered as a block and coloured token by token, which is why the code in
    // it cannot be found as one run of text: the highlighting is the server's
    // too, so a fenced `rust` block arrives as marked-up spans.
    let fenced = &set.questions[1].ask.text_html;
    assert!(
        fenced.contains("<pre><code>")
            && fenced.contains("allowance")
            && fenced.contains(r#"<span class="tok-storage">fn</span>"#),
        "expected the Question's fenced block rendered and highlighted:\n{fenced}"
    );
    let tabled = &set.questions[1].subquestions[0].text_html;
    assert!(
        tabled.contains("<table>") && tabled.contains("<td>Retry-After</td>"),
        "expected the Sub-question's table rendered to HTML:\n{tabled}"
    );
    assert!(
        !json.contains("| --- |"),
        "nothing may reach the viewer as raw markup:\n{json}"
    );
}

#[tokio::test]
async fn a_questions_words_travel_beside_its_markup_for_the_table_of_contents() {
    let (_dir, pool, app) = fresh_app().await;

    // A line of text in a narrow column cannot be the rendered markdown, and
    // taking the markup back out of it would mean a parser on the viewer's side
    // of the wire — the one thing rendering on the server is for. So the words
    // are rendered from the same markdown by the same pass and sent alongside.
    let (set, _) = set_json(&app, &pool, &marked_up_set()).await;

    // The words and nothing else: no markup, and no markdown either — the code
    // span's backticks were markup for a renderer, not something to read.
    assert_eq!(
        set.questions[0].nav_text,
        "Where should the request counter live? in-process, per instance in redis, \
         shared across instances",
    );
    assert!(
        !set.questions[0].nav_text.contains('<'),
        "the nav's line is words and no markup: {:?}",
        set.questions[0].nav_text
    );
}

#[tokio::test]
async fn markdown_that_would_run_in_the_browser_does_not_reach_a_question() {
    let (_dir, pool, app) = fresh_app().await;
    let mut set = full_grammar_set();
    set.questions[0].text = "Careful now.\n\n<script>alert('pwned')</script>\n\n\
         <img src=x onerror=\"alert('pwned')\">\n\n\
         [click me](javascript:alert('pwned'))\n"
        .to_owned();

    let (view, json) = set_json(&app, &pool, &set).await;

    assert!(
        view.questions[0].ask.text_html.contains("Careful now."),
        "expected the Question's prose"
    );
    assert_sanitised(&json, "the Question");
}

#[tokio::test]
async fn an_options_markdown_is_rendered_inline_by_the_server() {
    let (_dir, pool, app) = fresh_app().await;

    let (set, _) = set_json(&app, &pool, &marked_up_set()).await;

    assert!(
        option_with(&set, "Counter::local")
            .text_html
            .contains("<code>Counter::local</code>"),
        "expected the Option's code span rendered to HTML"
    );
    assert!(
        option_with(&set, "Redis")
            .text_html
            .contains("<strong>Redis</strong>"),
        "expected the Option's emphasis rendered to HTML"
    );
}

#[tokio::test]
async fn block_markdown_in_an_option_is_flattened_rather_than_dropped() {
    let (_dir, pool, app) = fresh_app().await;

    let (set, json) = set_json(&app, &pool, &marked_up_set()).await;

    // An Option is one line beside a radio, so a list inside its label would
    // break the row apart — and the whole row is what the human taps.
    assert!(
        !json.contains("<li>no headers</li>"),
        "an Option's list may not be drawn as one:\n{json}"
    );

    let flattened = &option_with(&set, "A bare 429.").text_html;
    assert!(
        flattened.contains("no headers") && flattened.contains("no body"),
        "flattened, not dropped: every word the agent wrote is still there:\n{flattened}"
    );
}

#[tokio::test]
async fn a_question_declaring_columns_hands_the_viewer_the_table_it_declared() {
    let (_dir, pool, app) = fresh_app().await;

    let (set, _) = set_json(&app, &pool, &tabulated_set()).await;

    // The axes in the order the agent declared them, and each Option's row
    // beside them in the same order — the viewer draws the table from these two
    // and never parses one out of anything.
    assert_eq!(
        set.questions[0].ask.columns,
        ["Latency", "<code>ops</code> cost"]
    );
    assert_eq!(
        option_with(&set, "In-process").cells,
        ["Sub-<code>ms</code>", "None"]
    );
    assert_eq!(
        option_with(&set, "In Redis").cells,
        ["<strong>A hop</strong>", "A box to run"]
    );

    // A Sub-question declares its own table exactly as a Question does.
    assert_eq!(set.questions[1].subquestions[0].columns, ["Memory"]);
    assert_eq!(option_with(&set, "LRU").cells, ["Bounded"]);
}

#[tokio::test]
async fn a_question_declaring_no_columns_is_the_list_it_always_was() {
    let (_dir, pool, app) = fresh_app().await;

    let (set, _) = set_json(&app, &pool, &marked_up_set()).await;

    for ask in set
        .questions
        .iter()
        .flat_map(|question| std::iter::once(&question.ask).chain(&question.subquestions))
    {
        assert!(
            ask.columns.is_empty(),
            "{} declared no axes, so it is no Answer Table",
            ask.name
        );
        for option in &ask.options {
            assert!(option.cells.is_empty(), "and no Option of it has a row");
        }
    }
}

#[tokio::test]
async fn a_header_and_a_cell_are_rendered_inline_like_an_options_own_text() {
    let (_dir, pool, app) = fresh_app().await;
    let mut asking = tabulated_set();
    asking.questions[0].columns[0] = "Latency\n\n- and jitter\n".to_owned();
    asking.questions[0].options[0].cells[0] = "Sub-ms\n\n- on a warm cache\n".to_owned();

    let (set, json) = set_json(&app, &pool, &asking).await;

    // A cell is a cell: a block inside one would break the row that is the tap
    // target, exactly as it would inside an Option's own label.
    assert!(
        !json.contains("<li>and jitter</li>") && !json.contains("<li>on a warm cache</li>"),
        "a header or a cell may not carry a block:\n{json}"
    );
    assert!(
        set.questions[0].ask.columns[0].contains("and jitter"),
        "flattened, not dropped: {:?}",
        set.questions[0].ask.columns[0]
    );
    assert!(
        option_with(&set, "In-process").cells[0].contains("on a warm cache"),
        "flattened, not dropped"
    );
}

#[tokio::test]
async fn markdown_that_would_run_in_the_browser_does_not_reach_a_header_or_a_cell() {
    let (_dir, pool, app) = fresh_app().await;
    let running = "Careful now. <script>alert('pwned')</script> \
         <img src=\"x\" onerror=\"alert('pwned')\"> \
         [click me](javascript:alert('pwned'))";
    let mut asking = tabulated_set();
    asking.questions[0].columns[0] = running.to_owned();
    asking.questions[0].options[0].cells[0] = running.to_owned();

    let (view, json) = set_json(&app, &pool, &asking).await;

    assert!(view.questions[0].ask.columns[0].contains("Careful now."));
    assert!(option_with(&view, "In-process").cells[0].contains("Careful now."));
    assert_sanitised(&json, "the Answer Table");
}

#[tokio::test]
async fn markdown_that_would_run_in_the_browser_does_not_reach_an_option() {
    let (_dir, pool, app) = fresh_app().await;
    let mut set = full_grammar_set();
    set.questions[0].options[0].text = "Careful now. <script>alert('pwned')</script> \
         <img src=\"x\" onerror=\"alert('pwned')\"> \
         [click me](javascript:alert('pwned'))"
        .to_owned();

    let (view, json) = set_json(&app, &pool, &set).await;

    let text = &view.questions[0].ask.options[0].text_html;
    assert!(text.contains("Careful now."), "expected the Option's words");
    assert!(
        text.contains("click me"),
        "expected the link's words, which are all that is left of it:\n{text}"
    );
    assert_sanitised(&json, "the Option");
}

#[tokio::test]
async fn a_settled_sets_material_is_rendered_the_way_a_waiting_ones_is() {
    let (_dir, pool, app) = fresh_app().await;

    let (answered, _) = answered_set(&app, &pool, &marked_up_set(), &decided_every_way()).await;
    let (archived, _) = archived_set(&app, &pool, &marked_up_set()).await;

    // A settled Set is read for what was asked as well as for what was decided,
    // so nothing about the rendering turns on where it stands.
    for set in [&answered, &archived] {
        assert!(
            set.questions[0]
                .ask
                .text_html
                .contains("<code>redis</code>")
                && set.questions[1].subquestions[0]
                    .text_html
                    .contains("<td>Retry-After</td>")
                && option_with(set, "Counter::local")
                    .text_html
                    .contains("<code>Counter::local</code>"),
            "a settled Set's markdown is rendered too"
        );
    }
}

#[tokio::test]
async fn where_the_ask_came_from_travels_with_it_and_nothing_does_when_there_is_nowhere() {
    let (_dir, pool, app) = fresh_app().await;

    let (from_a_repo, _) = set_json(&app, &pool, &full_grammar_set()).await;
    assert_eq!(from_a_repo.title, "Rate limiting for the public API");
    assert_eq!(from_a_repo.project.as_deref(), Some("askance"));
    assert_eq!(from_a_repo.branch.as_deref(), Some("solid-viewer"));

    let mut outside = full_grammar_set();
    outside.project = None;
    outside.branch = None;
    let (from_nowhere, _) = set_json(&app, &pool, &outside).await;
    assert_eq!(from_nowhere.project, None);
    assert_eq!(from_nowhere.branch, None);
}

#[tokio::test]
async fn a_preface_of_nothing_but_whitespace_is_the_same_as_none() {
    let (_dir, pool, app) = fresh_app().await;
    let mut set = full_grammar_set();
    set.preface = Some("   \n".to_owned());

    let (view, _) = set_json(&app, &pool, &set).await;

    assert_eq!(
        view.preface_html, None,
        "there is no section to draw for an empty Preface"
    );
}

#[tokio::test]
async fn a_postscript_of_nothing_but_whitespace_is_the_same_as_none() {
    let (_dir, pool, app) = fresh_app().await;
    let mut set = full_grammar_set();
    set.postscript = Some("   \n".to_owned());

    let (view, _) = set_json(&app, &pool, &set).await;

    assert_eq!(
        view.postscript_html, None,
        "an empty Postscript is the same as none, exactly as the Preface is"
    );
}

#[tokio::test]
async fn the_attached_diff_is_rendered_per_file_and_highlighted_by_the_server() {
    let (_dir, pool, app) = fresh_app().await;
    let mut set = full_grammar_set();
    set.diff = Some(modified_and_untracked_diff());

    let (view, _) = set_json(&app, &pool, &set).await;
    let diff = view.diff.expect("this Set has a Diff attached");

    // The paths travel beside the HTML and in Diff order, because the viewer's
    // table of contents is built from both: `paths[0]` is what `#diff-1` shows.
    assert_eq!(diff.paths, ["src/limits.rs", "notes.txt"]);
    assert_eq!(
        diff.html.matches(r#"class="diff-file""#).count(),
        2,
        "expected one section per file, whatever git knew of it:\n{}",
        diff.html
    );

    // The colouring comes from the server: the viewer gets no diff parser.
    assert!(diff.html.contains("diff-line add"), "{}", diff.html);
    assert!(diff.html.contains("diff-line del"), "{}", diff.html);
    assert!(
        diff.html.contains(r#"<span class="tok-"#),
        "expected the Rust file highlighted server-side:\n{}",
        diff.html
    );
}

#[tokio::test]
async fn a_set_with_no_diff_carries_none() {
    let (_dir, pool, app) = fresh_app().await;
    let set = full_grammar_set();
    assert!(set.diff.is_none(), "this Set is the one without a Diff");

    let (view, _) = set_json(&app, &pool, &set).await;

    assert_eq!(view.diff, None, "there is no Diff section to draw");
}

#[tokio::test]
async fn a_set_carrying_a_diagram_says_so_and_hands_over_the_block_to_draw_it_from() {
    let (_dir, pool, app) = fresh_app().await;

    let (view, _) = set_json(&app, &pool, &diagrammed_set()).await;

    // Answered by the server, from the HTML it has just rendered: it decides
    // whether the page loads the renderer at all, and three and a half megabytes
    // of mermaid is not something to fetch on the chance that it is wanted.
    assert!(view.diagrams, "a Diagram in the Preface is a Diagram");
    assert!(
        view.preface_html
            .as_deref()
            .is_some_and(|html| html.contains(r#"<pre class="mermaid">"#)),
        "expected the block the renderer draws from"
    );
}

#[tokio::test]
async fn a_diagram_in_a_question_says_so_just_the_same() {
    let (_dir, pool, app) = fresh_app().await;
    let mut set = full_grammar_set();
    set.questions[1].subquestions[0].text =
        "Which way round?\n\n```mermaid\ngraph TD;\n  a-->b;\n```\n".to_owned();

    let (view, _) = set_json(&app, &pool, &set).await;

    assert!(
        view.diagrams,
        "a Diagram is a Diagram wherever the agent wrote it"
    );
    assert!(
        view.questions[1].subquestions[0]
            .text_html
            .contains(r#"<pre class="mermaid">"#)
    );
}

#[tokio::test]
async fn a_diagram_in_the_postscript_says_so_just_the_same() {
    let (_dir, pool, app) = fresh_app().await;
    let mut set = full_grammar_set();
    set.preface = Some("`POST /v1/messages` has no rate limit.\n".to_owned());
    set.postscript = Some(
        concat!(
            "Where the counter would live:\n",
            "\n",
            "```mermaid\n",
            "graph LR;\n",
            "  client-->api;\n",
            "  api-->redis;\n",
            "```\n",
        )
        .to_owned(),
    );

    let (view, _) = set_json(&app, &pool, &set).await;

    assert!(
        view.diagrams,
        "a Diagram is a Diagram wherever the agent wrote it, the Postscript included"
    );
    assert!(
        view.postscript_html
            .as_deref()
            .is_some_and(|html| html.contains(r#"<pre class="mermaid">"#)),
        "expected the block the renderer draws from"
    );
}

#[tokio::test]
async fn a_set_with_no_diagram_says_so_however_much_other_markup_it_has() {
    let (_dir, pool, app) = fresh_app().await;

    // Fences and tables and code spans throughout, and not one Diagram: this is
    // what almost every Set looks like, and none of them should pay for the
    // renderer.
    let (view, json) = set_json(&app, &pool, &marked_up_set()).await;

    assert!(!view.diagrams);
    assert!(
        !json.contains("mermaid"),
        "a Set with no Diagram should not mention mermaid at all:\n{json}"
    );
}

#[tokio::test]
async fn a_diagram_the_renderer_cannot_draw_still_arrives_as_its_source() {
    let (_dir, pool, app) = fresh_app().await;
    let mut set = full_grammar_set();
    set.preface = Some("```mermaid\nnot a diagram at all\n```\n".to_owned());

    let (view, json) = set_json(&app, &pool, &set).await;

    // The server does not parse mermaid, and neither does it complain: whether
    // this draws is decided in the browser, and a diagram that will not draw is
    // left as the block a human can read. What must not be carried is any word
    // about it — the fallback is silent.
    assert!(
        view.preface_html
            .as_deref()
            .is_some_and(|html| html.contains(r#"<pre class="mermaid">not a diagram at all"#)),
        "expected the source exactly as the agent wrote it"
    );
    for complaint in ["Syntax error", "error in text", "mermaid-error"] {
        assert!(
            !json.contains(complaint),
            "the fallback says nothing, and this carried `{complaint}`:\n{json}"
        );
    }
}

#[tokio::test]
async fn a_set_says_where_it_stands_in_each_of_the_three_ways_it_can() {
    let (_dir, pool, app) = fresh_app().await;

    let (waiting, _) = set_json(&app, &pool, &full_grammar_set()).await;
    assert_eq!(waiting.standing, Standing::Waiting(Liveness::Waiting));

    let (answered, _) = answered_set(&app, &pool, &full_grammar_set(), &decided_every_way()).await;
    let Standing::Answered(Answered {
        submitted_at,
        response,
    }) = answered.standing
    else {
        panic!("expected an answered Set, got {:?}", answered.standing);
    };
    // What the human decided, whole: the Option taken beside the one the agent
    // recommended, whatever was written, the questions that went back open, and
    // the word about the Set as a whole.
    assert_eq!(response, decided_every_way());
    assert!(!submitted_at.is_empty(), "expected when it was answered");

    let (archived, _) = archived_set(&app, &pool, &full_grammar_set()).await;
    let Standing::ArchivedUnanswered(archived_at) = archived.standing else {
        panic!(
            "expected a Set closed unanswered, got {:?}",
            archived.standing
        );
    };
    assert!(!archived_at.is_empty(), "expected when it was closed");
}

/// Nothing that would run in a browser survived the rendering, wherever in the
/// payload it was written.
///
/// Asked of the whole payload rather than of the one field, because the point is
/// that there is nowhere for it to have gone.
fn assert_sanitised(json: &str, wrote_it: &str) {
    assert!(
        !json.contains("alert('pwned')"),
        "{wrote_it}'s script should have been sanitised away:\n{json}"
    );
    assert!(
        !json.contains("onerror"),
        "{wrote_it}'s event handler should have been sanitised away:\n{json}"
    );
    assert!(
        !json.contains("javascript:"),
        "{wrote_it}'s script link should have been sanitised away:\n{json}"
    );
}

/// Where the golden fixtures are written, relative to this crate.
const FIXTURES: &str = "../../web/tests/fixtures";

/// Leave the viewer's component tests a payload of each shape, exactly as this
/// server writes one.
///
/// Committed, and rewritten by every run of this test: the diff is the review.
/// A viewer test fed by a hand-written mock proves only that the mock and the
/// component agree — these files are what the endpoint actually said.
///
/// Everything a clock would otherwise decide is pinned, so that a run today
/// and a run next week write the same bytes. The worded times are pinned by
/// arrangement — each row is created or settled at a distance from now whose
/// wording never moves, "just now" or "3h ago" or far enough back to be said
/// as its date — and the exact stamps behind them, which would carry the run's
/// own clock, are overwritten with stated minutes after the fact.
#[tokio::test]
async fn the_viewers_own_tests_are_fed_from_here() {
    let now = time::OffsetDateTime::now_utc();

    // The pending list: one Set whose agent has just sent it, and one nothing has
    // waited on for hours — the two badges, and two ages worth wording.
    let (_dir, pool, app) = fresh_app().await;
    store::insert_set(&pool, &full_grammar_set()).await.unwrap();
    let mut stale_set = full_grammar_set();
    stale_set.title = "Retry policy for the outbound queue".to_owned();
    stale_set.branch = Some("outbound-retries".to_owned());
    let stale = store::insert_set(&pool, &stale_set).await.unwrap();
    stamp(
        &pool,
        "UPDATE question_sets SET created_at = ? WHERE id = ?",
        &rfc3339(now - time::Duration::hours(3)),
        stale.id,
    )
    .await;
    write(
        "pending.json",
        &pin_rows(
            &get(&app, "/api/ui/pending").await,
            "created_stamp",
            &["2026-08-03 06:16 UTC", "2026-08-03 09:16 UTC"],
        ),
    );

    // The Archive: a decision, and a Set nobody was ever going to answer — one
    // settled a moment ago and one long since, which are also the two ways a
    // settling is worded: the age while it is fresh, and the date once it is
    // not. Long since rather than days ago, because "4d ago" would say
    // something different next week and these bytes must not.
    let (_dir, pool, app) = fresh_app().await;
    let decided = store::insert_set(&pool, &full_grammar_set()).await.unwrap();
    store::insert_response(&pool, decided.id, &decided_every_way())
        .await
        .unwrap()
        .unwrap();
    stamp(
        &pool,
        "UPDATE responses SET submitted_at = ? WHERE set_id = ?",
        "2025-08-03T09:07:11.000Z",
        decided.id,
    )
    .await;
    let mut orphan_set = full_grammar_set();
    orphan_set.title = "Backfill for the archived rows".to_owned();
    let orphan = store::insert_set(&pool, &orphan_set).await.unwrap();
    store::archive_set(&pool, &store::Settlements::new(1), orphan.id)
        .await
        .unwrap();
    write(
        "archive.json",
        &pin_rows(
            &get(&app, "/api/ui/archive").await,
            "settled_stamp",
            &["2026-08-03 17:31 UTC", "2025-08-03 09:07 UTC"],
        ),
    );

    // A Set to answer: every feature of the question grammar, the agent's markup
    // throughout, and a Diff attached.
    let (_dir, pool, app) = fresh_app().await;
    let mut answering = marked_up_set();
    answering.diff = Some(modified_and_untracked_diff());
    let (_, json) = set_json(&app, &pool, &answering).await;
    write("set-answering.json", &json);

    // The same Set answered, which is the same page read rather than filled in.
    let (_dir, pool, app) = fresh_app().await;
    let (_, json) = answered_set(&app, &pool, &marked_up_set(), &decided_every_way()).await;
    write("set-answered.json", &pinned(&json));

    // And closed unanswered, which is the one standing with no Response behind it.
    let (_dir, pool, app) = fresh_app().await;
    let (_, json) = archived_set(&app, &pool, &marked_up_set()).await;
    write("set-archived.json", &pinned(&json));

    // A Set with a Diagram in it, which is the only kind whose page loads the
    // renderer.
    let (_dir, pool, app) = fresh_app().await;
    let (_, json) = set_json(&app, &pool, &diagrammed_set()).await;
    write("set-diagram.json", &json);
}

/// Pin a list's exact stamps to stated minutes, one value per row in order, so
/// the fixture does not carry the run's own clock. The worded times beside
/// them need no pinning — the rows are arranged so their wording never moves.
fn pin_rows(json: &str, field: &str, values: &[&str]) -> String {
    let mut payload: serde_json::Value = serde_json::from_str(json).unwrap();

    let rows = payload.as_array_mut().unwrap();
    assert_eq!(rows.len(), values.len(), "one stated minute per row");
    for (row, value) in rows.iter_mut().zip(values) {
        assert!(
            row.get(field).is_some(),
            "no {field} on this row to pin:\n{row}"
        );
        row[field] = (*value).into();
    }

    serde_json::to_string(&payload).unwrap()
}

/// Pin the one stamp a settled Set carries, so the fixture does not change with
/// the clock. It is the only value in these payloads the server hands over
/// raw — the viewer words it, against its own clock — so it is pinned far
/// enough back that the wording is the date, which never moves.
fn pinned(json: &str) -> String {
    let settled = "2025-08-03T09:07:11.000Z";

    let mut payload: serde_json::Value = serde_json::from_str(json).unwrap();
    let standing = &mut payload["standing"];

    if let Some(answered) = standing.get_mut("Answered") {
        answered["submitted_at"] = settled.into();
    } else if standing.get("ArchivedUnanswered").is_some() {
        standing["ArchivedUnanswered"] = settled.into();
    }

    serde_json::to_string(&payload).unwrap()
}

/// A time as the store holds one. Written out by hand because `time` is built
/// here for parsing only — as it is in the server, which words its dates the
/// same way rather than formatting them.
fn rfc3339(when: time::OffsetDateTime) -> String {
    let when = when.to_offset(time::UtcOffset::UTC);

    format!(
        "{}-{:02}-{:02}T{:02}:{:02}:{:02}.000Z",
        when.year(),
        u8::from(when.month()),
        when.day(),
        when.hour(),
        when.minute(),
        when.second(),
    )
}

async fn stamp(pool: &SqlitePool, query: &str, stamp: &str, id: i64) {
    sqlx::query(query)
        .bind(stamp)
        .bind(id)
        .execute(pool)
        .await
        .unwrap();
}

/// The body of a GET, as it went out.
async fn get(app: &Router, path: &str) -> String {
    let response = app
        .clone()
        .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK, "GET {path}");
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    String::from_utf8(bytes.to_vec()).unwrap()
}

/// Write one fixture, indented so that a review of it is a review of the shape.
fn write(name: &str, json: &str) {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(FIXTURES);
    std::fs::create_dir_all(&dir).unwrap();

    let payload: serde_json::Value = serde_json::from_str(json).unwrap();
    let mut pretty = serde_json::to_string_pretty(&payload).unwrap();
    pretty.push('\n');

    std::fs::write(dir.join(name), pretty).unwrap();
}
