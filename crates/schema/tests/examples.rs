//! The examples the README tells the reader to run.
//!
//! They are documentation first, but an example that has drifted from the
//! grammar teaches the wrong thing, so the shipped files are checked here
//! against the real one.

use std::path::{Path, PathBuf};

use askance_schema::{QuestionSet, Response};

/// Read one of the workspace's `examples/`, by the name the README uses.
fn example(name: &str) -> String {
    let path: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples")
        .join(name);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("reading {}: {error}", path.display()))
}

fn example_set() -> QuestionSet {
    QuestionSet::from_yaml(&example("questions.yaml"))
        .unwrap_or_else(|error| panic!("examples/questions.yaml should parse: {error}"))
}

#[test]
fn the_example_set_is_a_legal_question_set() {
    let set = example_set();

    set.validate()
        .unwrap_or_else(|invalid| panic!("examples/questions.yaml should be legal:\n{invalid}"));

    // The CLI overwrites these from the working directory, so an example that
    // supplied them would only be teaching the reader a habit that does not
    // survive contact with `askance ask`.
    assert_eq!(set.project, None, "the CLI derives the project");
    assert_eq!(set.branch, None, "the CLI derives the branch");
    assert_eq!(set.diff, None, "the CLI captures the Diff");
}

#[test]
fn the_example_set_exercises_the_grammar() {
    let set = example_set();

    let preface = set.preface.as_deref().expect("the Set has a Preface");
    assert!(
        preface.contains("\n\n"),
        "the Preface is markdown, so it should run to more than one paragraph, got:\n{preface}"
    );

    let subquestions: Vec<_> = set
        .questions
        .iter()
        .flat_map(|question| &question.subquestions)
        .collect();
    assert!(
        !subquestions.is_empty(),
        "the Set should show what a Sub-question looks like"
    );

    let all_options = || {
        set.questions.iter().flat_map(|question| {
            question.options.iter().chain(
                question
                    .subquestions
                    .iter()
                    .flat_map(|subquestion| &subquestion.options),
            )
        })
    };
    assert!(
        all_options().count() >= 2,
        "the Set should show what Options look like"
    );
    assert_eq!(
        all_options().filter(|option| option.recommended).count(),
        1,
        "the Set should show exactly one Recommendation"
    );
}

#[test]
fn the_example_response_resolves_the_example_set() {
    let set = example_set();

    let response = Response::from_yaml(&example("response.yaml"))
        .unwrap_or_else(|error| panic!("examples/response.yaml should parse: {error}"));

    response.validate(&set).unwrap_or_else(|invalid| {
        panic!("examples/response.yaml should resolve examples/questions.yaml:\n{invalid}")
    });

    assert!(
        response.answers.iter().any(|answer| answer.unanswered),
        "the example should show a question left explicitly open"
    );
    assert!(
        response.answers.iter().any(|answer| answer.is_answer()),
        "the example should show questions actually answered"
    );
    assert!(
        response.comment.is_some(),
        "the example should show the set-level comment"
    );
}
