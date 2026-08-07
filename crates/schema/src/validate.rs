//! The invariants of the question grammar, enforced in one place so the CLI
//! refuses a Set before sending it and the server refuses the same Set the
//! same way.
//!
//! Deserialization already rejects documents that are not Sets. What is left
//! is everything the grammar forbids but YAML happily represents:
//!
//! - a Set needs a non-empty title;
//! - a Question needs a label and text, and labels are distinct across the
//!   Set, because a Response answers by label;
//! - Sub-questions are leaves, so two levels is the maximum;
//! - at most one Option per Question or Sub-question is the Recommendation;
//! - Option numbers are distinct within a question, because an Answer selects
//!   by number;
//! - an Answer Table is rectangular: a question declaring `columns` has every
//!   Option fill every one of them, and `cells` without `columns` have nothing
//!   to fill. An empty `columns` list is not a table with no axes — the wire
//!   format cannot tell it from an absent one — so it reads as no declaration
//!   at all, and the question is drawn as the list it always was.
//!
//! Multi-select needs no rule: the format gives no way to express it — an
//! Answer carries one `selected` number.
//!
//! A Response is checked against the Set it answers, where the invariant is
//! explicitness rather than completeness:
//!
//! - every Question and Sub-question appears exactly once, as an Answer or as
//!   the Unanswered marker, so the agent cannot miss an open question;
//! - an entry is one or the other, never both, and never neither;
//! - a selected Option is one the question actually offers.
//!
//! Every violation is collected rather than the first one returned, so an
//! agent that got several things wrong learns about all of them at once.

use std::collections::HashSet;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::response::{Answer, Response};
use crate::set::{Question, QuestionOption, QuestionSet, Subquestion};

// A Violation reaches the CLI inside an `ApiError`, which is also how the viewer
// is told what a request was refused for — see `Response` for the gating.
#[cfg(feature = "typescript")]
use ts_rs::TS;

/// One way a Set fails the question grammar.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct Violation {
    /// The question at fault, e.g. `Q7` or `Q7a`. Absent when the problem is
    /// the Set as a whole.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,

    pub message: String,
}

impl Violation {
    fn set(message: impl Into<String>) -> Self {
        Self {
            label: None,
            message: message.into(),
        }
    }

    fn at(label: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            label: Some(label.into()),
            message: message.into(),
        }
    }
}

impl fmt::Display for Violation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.label {
            Some(label) => write!(f, "{label}: {}", self.message),
            None => f.write_str(&self.message),
        }
    }
}

/// Everything wrong with a Set, as one error.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationError {
    pub violations: Vec<Violation>,
}

impl ValidationError {
    /// Whether any violation is pinned to `label`. Mostly for tests and for
    /// callers reporting on a particular question.
    pub fn names(&self, label: &str) -> bool {
        self.violations
            .iter()
            .any(|v| v.label.as_deref() == Some(label))
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, violation) in self.violations.iter().enumerate() {
            if i > 0 {
                f.write_str("\n")?;
            }
            write!(f, "{violation}")?;
        }
        Ok(())
    }
}

impl std::error::Error for ValidationError {}

impl QuestionSet {
    /// Check the Set against the question grammar.
    pub fn validate(&self) -> Result<(), ValidationError> {
        let mut violations = Vec::new();

        if self.title.trim().is_empty() {
            violations.push(Violation::set("a Set needs a non-empty title"));
        }

        let mut seen_labels = HashSet::new();
        for question in &self.questions {
            check_question(question, &mut seen_labels, &mut violations);
        }

        if violations.is_empty() {
            Ok(())
        } else {
            Err(ValidationError { violations })
        }
    }
}

impl Response {
    /// Check the Response against the Set it answers.
    pub fn validate(&self, set: &QuestionSet) -> Result<(), ValidationError> {
        let mut violations = Vec::new();

        // Every question the Response has to account for, in the order the
        // human saw them, each with the Options it offered.
        //
        // A Heading is not among them: it heads its Sub-questions rather than
        // asking anything, so there is nothing for an Answer to resolve. Its
        // label is kept aside so that an entry naming one can be told apart
        // from an entry naming a Question the Set never asked — the two are
        // different mistakes and only one of them is about a missing Question.
        let mut expected: Vec<(String, &[QuestionOption])> = Vec::new();
        let mut headings: Vec<&str> = Vec::new();
        for question in &set.questions {
            if question.heading() {
                headings.push(question.name());
            } else {
                expected.push((question.name().to_string(), &question.options));
            }
            for subquestion in &question.subquestions {
                expected.push((subquestion.name(question), &subquestion.options));
            }
        }

        let mut answered: HashSet<&str> = HashSet::new();
        for answer in &self.answers {
            let label = answer.label.trim();

            if label.is_empty() {
                violations.push(Violation::set(
                    "every entry in a Response names the question it resolves; one names none",
                ));
                continue;
            }

            let Some((_, options)) = expected.iter().find(|(name, _)| name == label) else {
                violations.push(if headings.contains(&label) {
                    Violation::at(
                        label,
                        "this Question heads its Sub-questions and asks nothing of \
                         its own, so no entry comes back for it; its Sub-questions \
                         are what there is to answer",
                    )
                } else {
                    Violation::at(label, "the Set has no such Question or Sub-question")
                });
                continue;
            };

            if !answered.insert(label) {
                violations.push(Violation::at(
                    label,
                    "this question is resolved twice; it gets one entry, not two",
                ));
                continue;
            }

            check_answer(answer, label, options, &mut violations);
        }

        for (name, _) in &expected {
            if !answered.contains(name.as_str()) {
                violations.push(Violation::at(
                    name,
                    "missing from the Response; every question appears, either answered \
                     or marked `unanswered: true`",
                ));
            }
        }

        if violations.is_empty() {
            Ok(())
        } else {
            Err(ValidationError { violations })
        }
    }
}

fn check_answer(
    answer: &Answer,
    label: &str,
    options: &[QuestionOption],
    violations: &mut Vec<Violation>,
) {
    if answer.unanswered {
        if answer.is_answer() {
            violations.push(Violation::at(
                label,
                "marked `unanswered: true` but also answered; it is one or the other",
            ));
        }
        return;
    }

    if !answer.is_answer() {
        violations.push(Violation::at(
            label,
            "neither answered nor marked `unanswered: true`; leaving a question open \
             has to be said out loud",
        ));
        return;
    }

    let Some(selected) = answer.selected else {
        return;
    };

    if options.is_empty() {
        violations.push(Violation::at(
            label,
            format!("Option {selected} was selected, but this question offers no Options"),
        ));
    } else if !options.iter().any(|option| option.n == selected) {
        let offered: Vec<String> = options.iter().map(|option| option.n.to_string()).collect();
        violations.push(Violation::at(
            label,
            format!(
                "Option {selected} was selected, but this question offers only {}",
                offered.join(", ")
            ),
        ));
    }
}

fn check_question<'a>(
    question: &'a Question,
    seen_labels: &mut HashSet<&'a str>,
    violations: &mut Vec<Violation>,
) {
    let label = question.label.trim();

    if label.is_empty() {
        violations.push(Violation::set(format!(
            "every Question needs a label; one asking {:?} has none",
            elide(&question.text)
        )));
    } else if !seen_labels.insert(label) {
        violations.push(Violation::at(
            label,
            "two Questions share this label; a Response answers by label, so they must be distinct",
        ));
    }

    if question.text.trim().is_empty() {
        violations.push(Violation::at(label, "a Question needs text"));
    }

    check_options(label, &question.columns, &question.options, violations);

    let mut seen_letters = HashSet::new();
    for subquestion in &question.subquestions {
        check_subquestion(question, subquestion, &mut seen_letters, violations);
    }
}

fn check_subquestion<'a>(
    parent: &Question,
    subquestion: &'a Subquestion,
    seen_letters: &mut HashSet<&'a str>,
    violations: &mut Vec<Violation>,
) {
    let letter = subquestion.letter.trim();
    let name = subquestion.name(parent);

    if letter.is_empty() {
        violations.push(Violation::at(
            parent.name(),
            format!(
                "every Sub-question needs a letter; one asking {:?} has none",
                elide(&subquestion.text)
            ),
        ));
    } else if !seen_letters.insert(letter) {
        violations.push(Violation::at(
            &name,
            "two Sub-questions share this letter; they must be distinct",
        ));
    }

    if subquestion.text.trim().is_empty() {
        violations.push(Violation::at(&name, "a Sub-question needs text"));
    }

    if !subquestion.subquestions.is_empty() {
        violations.push(Violation::at(
            &name,
            "Sub-questions are leaves: two levels is the maximum, so this one \
             cannot have Sub-questions of its own",
        ));
    }

    check_options(
        &name,
        &subquestion.columns,
        &subquestion.options,
        violations,
    );
}

fn check_options(
    name: &str,
    columns: &[String],
    options: &[QuestionOption],
    violations: &mut Vec<Violation>,
) {
    let recommended: Vec<u32> = options
        .iter()
        .filter(|option| option.recommended)
        .map(|option| option.n)
        .collect();

    if recommended.len() > 1 {
        let numbers: Vec<String> = recommended.iter().map(u32::to_string).collect();
        violations.push(Violation::at(
            name,
            format!(
                "at most one Option is the Recommendation, but {} are marked: {}",
                recommended.len(),
                numbers.join(", ")
            ),
        ));
    }

    let mut seen_numbers = HashSet::new();
    for option in options {
        if !seen_numbers.insert(option.n) {
            violations.push(Violation::at(
                name,
                format!(
                    "two Options are numbered {}; an Answer selects by number, \
                     so numbers must be distinct",
                    option.n
                ),
            ));
        }

        if option.text.trim().is_empty() {
            violations.push(Violation::at(
                name,
                format!("Option {} needs text", option.n),
            ));
        }
    }

    check_cells(name, columns, options, violations);
}

/// The Answer Table has to be rectangular: the question names the axes and every
/// Option fills every one of them, or there is no table.
///
/// An empty `columns` list is no declaration at all — the wire format cannot tell
/// it from an absent one, so it reads as the list a question has always been, and
/// `cells` beside it are refused for having no columns to fill.
fn check_cells(
    name: &str,
    columns: &[String],
    options: &[QuestionOption],
    violations: &mut Vec<Violation>,
) {
    if columns.is_empty() {
        for option in options.iter().filter(|option| !option.cells.is_empty()) {
            violations.push(Violation::at(
                name,
                format!(
                    "Option {} carries {}, but this question declares no `columns` \
                     for a row to fill; an Answer Table names its axes on the question",
                    option.n,
                    count(option.cells.len(), "cell", "cells")
                ),
            ));
        }
        return;
    }

    for option in options {
        if option.cells.len() != columns.len() {
            violations.push(Violation::at(
                name,
                format!(
                    "this question declares {}, but Option {} carries {}; declaring \
                     `columns` makes the Options a table, and every row fills every column",
                    count(columns.len(), "axis", "axes"),
                    option.n,
                    count(option.cells.len(), "cell", "cells")
                ),
            ));
        }
    }
}

/// `2 cells`, `1 axis`, `no cells` — the plural the violations read with.
fn count(n: usize, one: &str, many: &str) -> String {
    match n {
        0 => format!("no {many}"),
        1 => format!("1 {one}"),
        n => format!("{n} {many}"),
    }
}

/// Enough of a question's text to recognise it when it has no label to go by.
fn elide(text: &str) -> String {
    const LIMIT: usize = 40;
    let text = text.trim();
    match text.char_indices().nth(LIMIT) {
        Some((cut, _)) => format!("{}…", &text[..cut]),
        None => text.to_string(),
    }
}
