//! The Question Set: what an agent sends and what the store hands back.
//!
//! The types mirror the YAML wire format one-to-one. They are deliberately
//! permissive about the *shape* of a Set — anything the grammar forbids but
//! serde can represent is caught by [`QuestionSet::validate`] instead, so the
//! refusal can name the offending Question rather than a byte offset.

use serde::{Deserialize, Serialize};

/// A batch of Questions submitted together by one agent.
///
/// `title`, `preface`, `questions` and `postscript` come from the agent;
/// `project`, `branch` and `diff` are filled in by the CLI, which derives them
/// from the working directory rather than trusting the agent. The server treats
/// all three as opaque.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuestionSet {
    /// Short line for the pending list.
    pub title: String,

    /// Markdown context, enough to answer the Questions without seeing the
    /// agent's session.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preface: Option<String>,

    pub questions: Vec<Question>,

    /// Markdown the agent closes the Set with, drawn above the set-level comment
    /// box: suggested discussion topics, or whatever else the human might take
    /// up there. Not a Question — a blank comment beneath it means nothing to
    /// add, never Unanswered.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub postscript: Option<String>,

    /// Repository the agent is working in, as the CLI saw it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,

    /// Branch the agent is working on, as the CLI saw it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,

    /// The repo's uncommitted changes at send time. Absent on a clean tree or
    /// outside a repo.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diff: Option<String>,
}

/// A single labelled decision put to the human.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Question {
    /// Agent-supplied and opaque to the server (e.g. `Q7`). Only the agent
    /// knows its session counter, so only the agent can assign it.
    pub label: String,

    pub text: String,

    /// The axes this question's Options are compared along, one per column of
    /// the Answer Table, as inline markdown. Declaring any is what makes the
    /// Options a table rather than a list; a question without them is drawn as
    /// the list it always was.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub columns: Vec<String>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<QuestionOption>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub subquestions: Vec<Subquestion>,
}

/// A leaf Question nested one level under a [`Question`], labelled by letter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Subquestion {
    /// Appended to the parent's label to name this question: `Q7` + `a`.
    pub letter: String,

    pub text: String,

    /// The axes this Sub-question's Options are compared along — a Sub-question
    /// declares its own Answer Table exactly as a [`Question`] does, and never
    /// inherits its parent's.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub columns: Vec<String>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<QuestionOption>,

    /// Always empty: Sub-questions are leaves. The field exists only so that a
    /// third level of nesting reaches validation and can be refused by name,
    /// instead of failing as an unknown field at some line and column.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub subquestions: Vec<Subquestion>,
}

/// One discrete choice offered on a Question or Sub-question.
///
/// The domain calls this an Option; the name is taken in Rust.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuestionOption {
    /// The number the human selects, `1`, `2`, …
    pub n: u32,

    pub text: String,

    /// Whether this Option is the Recommendation. At most one per question.
    #[serde(default, skip_serializing_if = "is_false")]
    pub recommended: bool,

    /// This Option's row of the Answer Table: one cell per axis the question
    /// declared, in that order, as inline markdown. Empty on a question that
    /// declared no `columns`, which is every question that is not a table.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cells: Vec<String>,
}

pub(crate) fn is_false(b: &bool) -> bool {
    !*b
}

impl QuestionSet {
    /// Parse a Set from the YAML wire format.
    ///
    /// This checks that the document *is* a Set, not that it is a legal one —
    /// see [`QuestionSet::validate`].
    pub fn from_yaml(yaml: &str) -> Result<Self, serde_saphyr::Error> {
        serde_saphyr::from_str(yaml)
    }

    /// Render the Set back to YAML. Multi-line strings — the Preface, the
    /// Postscript, the Diff — come out as `|` block scalars.
    pub fn to_yaml(&self) -> Result<String, serde_saphyr::SerializeError> {
        serde_saphyr::to_string(self)
    }
}

impl Question {
    /// The name this Question answers to: its label.
    pub fn name(&self) -> &str {
        &self.label
    }

    /// Whether this Question is a Heading: Sub-questions under it, and no
    /// Options of its own, so its text heads them rather than asking anything.
    /// No Answer comes back for one.
    ///
    /// Read off the shape rather than declared, because the shape already says
    /// it: a Question with Options is answerable whatever else it carries, and
    /// one with neither Options nor Sub-questions is a bare clarifying Question
    /// whose Answer is whatever the human writes. That leaves exactly one
    /// arrangement with nothing to answer at this level, and it is this one.
    pub fn heading(&self) -> bool {
        self.options.is_empty() && !self.subquestions.is_empty()
    }
}

impl Subquestion {
    /// The name this Sub-question answers to, e.g. `Q7a`.
    pub fn name(&self, parent: &Question) -> String {
        format!("{}{}", parent.label.trim(), self.letter.trim())
    }
}
