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
//!   by number.
//!
//! Multi-select needs no rule: the format gives no way to express it — an
//! Answer carries one `selected` number.
//!
//! Every violation is collected rather than the first one returned, so an
//! agent that got several things wrong learns about all of them at once.

use std::collections::HashSet;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::set::{Question, QuestionOption, QuestionSet, Subquestion};

/// One way a Set fails the question grammar.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

    check_options(label, &question.options, violations);

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

    check_options(&name, &subquestion.options, violations);
}

fn check_options(name: &str, options: &[QuestionOption], violations: &mut Vec<Violation>) {
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
