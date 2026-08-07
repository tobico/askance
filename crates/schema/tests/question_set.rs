//! The Question Set as it arrives from an agent: what YAML parses into, and
//! which shapes the question grammar refuses.

use askance_schema::QuestionSet;

/// A Set exercising every part of the wire format at once.
const FULL_SET: &str = r#"
title: Storage layout for the pending list
preface: |
  We need to settle how Sets are stored before the UI lands.

  The candidates differ mainly in how much SQL the Archive view needs.
project: askance
branch: api-core-and-cli
diff: |
  diff --git a/notes.md b/notes.md
  +a line
questions:
  - label: Q1
    text: How should a Question Set be stored?
    options:
      - n: 1
        text: One JSON body column
        recommended: true
      - n: 2
        text: Fully normalised tables
  - label: Q2
    text: What identifies a Set on the wire?
    subquestions:
      - letter: a
        text: Integer rowid or UUID?
        options:
          - n: 1
            text: Integer rowid
            recommended: true
          - n: 2
            text: UUID
      - letter: b
        text: Should the id appear in URLs?
postscript: |
  Whichever way this goes, the Archive view is the part I'd want a second
  opinion on.

  Anything else about the store is worth saying here.
"#;

#[test]
fn a_full_set_parses_into_its_parts() {
    let set = QuestionSet::from_yaml(FULL_SET).expect("the example Set should parse");

    assert_eq!(set.title, "Storage layout for the pending list");
    assert_eq!(set.project.as_deref(), Some("askance"));
    assert_eq!(set.branch.as_deref(), Some("api-core-and-cli"));
    assert!(set.diff.as_deref().unwrap().contains("+a line"));
    assert_eq!(
        set.preface.as_deref(),
        Some(
            "We need to settle how Sets are stored before the UI lands.\n\n\
             The candidates differ mainly in how much SQL the Archive view needs.\n"
        )
    );
    assert_eq!(
        set.postscript.as_deref(),
        Some(
            "Whichever way this goes, the Archive view is the part I'd want a second\n\
             opinion on.\n\n\
             Anything else about the store is worth saying here.\n"
        )
    );

    let [q1, q2] = &set.questions[..] else {
        panic!("expected two Questions, got {}", set.questions.len());
    };

    assert_eq!(q1.label, "Q1");
    assert_eq!(q1.text, "How should a Question Set be stored?");
    assert_eq!(q1.options.len(), 2);
    assert_eq!(q1.options[0].n, 1);
    assert_eq!(q1.options[0].text, "One JSON body column");
    assert!(q1.options[0].recommended);
    assert!(!q1.options[1].recommended);
    assert!(q1.subquestions.is_empty());

    assert_eq!(q2.label, "Q2");
    assert!(q2.options.is_empty());
    assert_eq!(q2.subquestions.len(), 2);
    assert_eq!(q2.subquestions[0].letter, "a");
    assert_eq!(q2.subquestions[0].options.len(), 2);
    assert!(q2.subquestions[0].options[0].recommended);
    assert!(q2.subquestions[1].options.is_empty());

    set.validate().expect("the example Set should be valid");
}

#[test]
fn a_minimal_set_needs_only_a_title_and_questions() {
    let set = QuestionSet::from_yaml(
        "
title: Just the one
questions:
  - label: Q1
    text: Ship it?
",
    )
    .expect("a Set without a preface should parse");

    assert_eq!(set.preface, None);
    assert_eq!(set.postscript, None);
    assert_eq!(set.project, None);
    assert_eq!(set.branch, None);
    assert_eq!(set.diff, None);
    set.validate().expect("a bare Set should be valid");
}

#[test]
fn an_unknown_field_is_a_parse_error() {
    let error = QuestionSet::from_yaml(
        "
title: Typo
questions:
  - label: Q1
    text: Ship it?
    options:
      - n: 1
        text: Yes
        recomended: true
",
    )
    .expect_err("a misspelled field should not be silently ignored");

    assert!(
        error.to_string().contains("recomended"),
        "the error should name the unknown field, got: {error}"
    );
}

#[test]
fn a_third_level_of_nesting_is_rejected_naming_the_sub_question() {
    let set = QuestionSet::from_yaml(
        "
title: Too deep
questions:
  - label: Q7
    text: Which storage layout?
    subquestions:
      - letter: a
        text: Rowid or UUID?
        subquestions:
          - letter: i
            text: And how wide?
",
    )
    .expect("three levels should parse, so validation can name the offender");

    let error = set
        .validate()
        .expect_err("Sub-questions are leaves; a third level is invalid");

    assert!(
        error.names("Q7a"),
        "the error should name Q7a, got: {error}"
    );
}

#[test]
fn two_recommended_options_are_rejected_naming_the_question() {
    let set = QuestionSet::from_yaml(
        "
title: Two stars
questions:
  - label: Q3
    text: Which one?
    options:
      - n: 1
        text: This
        recommended: true
      - n: 2
        text: That
        recommended: true
",
    )
    .unwrap();

    let error = set
        .validate()
        .expect_err("at most one Option may be the Recommendation");

    assert!(error.names("Q3"), "the error should name Q3, got: {error}");
}

#[test]
fn two_recommended_options_on_a_sub_question_are_rejected_naming_it() {
    let set = QuestionSet::from_yaml(
        "
title: Two stars, nested
questions:
  - label: Q4
    text: Which one?
    subquestions:
      - letter: b
        text: Really, which one?
        options:
          - n: 1
            text: This
            recommended: true
          - n: 2
            text: That
            recommended: true
",
    )
    .unwrap();

    let error = set.validate().expect_err("one Recommendation per question");

    assert!(
        error.names("Q4b"),
        "the error should name Q4b, got: {error}"
    );
}

#[test]
fn a_missing_title_is_rejected() {
    let error = QuestionSet::from_yaml(
        "
questions:
  - label: Q1
    text: Ship it?
",
    )
    .expect_err("title is required");

    assert!(
        error.to_string().contains("title"),
        "the error should mention the missing title, got: {error}"
    );
}

#[test]
fn an_empty_title_is_rejected() {
    let set = QuestionSet::from_yaml(
        "
title: '   '
questions:
  - label: Q1
    text: Ship it?
",
    )
    .unwrap();

    let error = set.validate().expect_err("a blank title is no title");

    assert!(
        error.to_string().contains("title"),
        "the error should mention the title, got: {error}"
    );
}

#[test]
fn duplicate_labels_are_rejected() {
    let set = QuestionSet::from_yaml(
        "
title: Same name twice
questions:
  - label: Q5
    text: First
  - label: Q5
    text: Second
",
    )
    .unwrap();

    let error = set
        .validate()
        .expect_err("a Response answers by label, so labels must be distinct");

    assert!(error.names("Q5"), "the error should name Q5, got: {error}");
}

#[test]
fn duplicate_option_numbers_are_rejected() {
    let set = QuestionSet::from_yaml(
        "
title: Ambiguous selection
questions:
  - label: Q6
    text: Which one?
    options:
      - n: 1
        text: This
      - n: 1
        text: That
",
    )
    .unwrap();

    let error = set
        .validate()
        .expect_err("an Answer selects by number, so numbers must be distinct");

    assert!(error.names("Q6"), "the error should name Q6, got: {error}");
}

#[test]
fn a_question_needs_a_label_and_text() {
    let set = QuestionSet::from_yaml(
        "
title: Nameless
questions:
  - label: ''
    text: ''
",
    )
    .unwrap();

    let error = set
        .validate()
        .expect_err("a Question needs a label and text");

    assert!(
        error.to_string().contains("label"),
        "the error should mention the label, got: {error}"
    );
}

#[test]
fn every_violation_is_reported_at_once() {
    let set = QuestionSet::from_yaml(
        "
title: ''
questions:
  - label: Q1
    text: Which one?
    options:
      - n: 1
        text: This
        recommended: true
      - n: 2
        text: That
        recommended: true
  - label: Q2
    text: And which one?
    subquestions:
      - letter: a
        text: Nested too far
        subquestions:
          - letter: i
            text: Way too far
",
    )
    .unwrap();

    let error = set.validate().unwrap_err();

    assert_eq!(
        error.violations.len(),
        3,
        "an agent should learn about every problem in one round trip, got: {error}"
    );
    assert!(error.names("Q1") && error.names("Q2a"));
}

#[test]
fn a_multi_line_preface_and_postscript_round_trip_through_yaml() {
    let set = QuestionSet::from_yaml(FULL_SET).unwrap();
    let yaml = set.to_yaml().expect("a Set should serialise");
    let reparsed = QuestionSet::from_yaml(&yaml).expect("our own YAML should parse");

    assert_eq!(reparsed.preface, set.preface);
    assert_eq!(reparsed.postscript, set.postscript);
    assert_eq!(reparsed.questions.len(), set.questions.len());
    assert!(
        yaml.contains("preface: |"),
        "the markdown Preface should ride in a block scalar, got:\n{yaml}"
    );
    assert!(
        yaml.contains("postscript: |"),
        "the markdown Postscript should ride in one too, got:\n{yaml}"
    );
}

#[test]
fn a_set_may_close_with_a_postscript_and_nothing_else_new() {
    let set = QuestionSet::from_yaml(
        "
title: Just the one
questions:
  - label: Q1
    text: Ship it?
postscript: Anything else about the release goes in the comment.
",
    )
    .expect("a Set with a Postscript should parse");

    assert_eq!(
        set.postscript.as_deref(),
        Some("Anything else about the release goes in the comment.")
    );
    set.validate()
        .expect("a Postscript is prose: there is nothing in it to refuse");
}

#[test]
fn a_question_may_declare_the_axes_its_options_are_compared_along() {
    let set = QuestionSet::from_yaml(
        "
title: Where the counter lives
questions:
  - label: Q1
    text: Where should the request counter live?
    columns:
      - Latency
      - '`ops` cost'
    options:
      - n: 1
        text: In-process
        cells:
          - Sub-`ms`
          - None
      - n: 2
        text: In Redis
        recommended: true
        cells:
          - A hop
          - A box to run
    subquestions:
      - letter: a
        text: And the eviction policy?
        columns:
          - Memory
        options:
          - n: 1
            text: LRU
            cells:
              - Bounded
",
    )
    .expect("a Set declaring an Answer Table should parse");

    let q1 = &set.questions[0];
    assert_eq!(q1.columns, ["Latency", "`ops` cost"]);
    assert_eq!(q1.options[0].cells, ["Sub-`ms`", "None"]);
    assert_eq!(q1.options[1].cells, ["A hop", "A box to run"]);

    let q1a = &q1.subquestions[0];
    assert_eq!(q1a.columns, ["Memory"]);
    assert_eq!(q1a.options[0].cells, ["Bounded"]);

    set.validate()
        .expect("a well-formed Answer Table is a legal Set");
}

#[test]
fn a_question_without_columns_declares_no_table() {
    let set = QuestionSet::from_yaml(FULL_SET).unwrap();

    assert!(
        set.questions[0].columns.is_empty(),
        "the presence of `columns` is what makes an Answer Table"
    );
    assert!(set.questions[0].options[0].cells.is_empty());
}

#[test]
fn an_answer_table_round_trips_through_yaml() {
    let set = QuestionSet::from_yaml(
        "
title: Where the counter lives
questions:
  - label: Q1
    text: Where should the request counter live?
    columns: [Latency]
    options:
      - n: 1
        text: In-process
        cells: [Sub-ms]
",
    )
    .unwrap();
    let yaml = set.to_yaml().expect("a Set should serialise");
    let reparsed = QuestionSet::from_yaml(&yaml).expect("our own YAML should parse");

    assert_eq!(reparsed, set);
}
