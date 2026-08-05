//! The view types the viewer draws a Set from, and the rendering that builds
//! them.
//!
//! The types themselves are shared by both ends of the wire — the server builds
//! them, the viewer deserializes them — so they stand without the renderers. The
//! building is the server's alone and is behind `ssr` with the rest of it.

use askance_schema::{Liveness, Response};
use serde::{Deserialize, Serialize};

/// One Question Set as the browser receives it.
///
/// Everything the agent wrote — the Preface, every Question's and Sub-question's
/// text, and every Option's — arrives as HTML rather than as its source, and so
/// does the Diff: the server has the markdown parser and the diff highlighter,
/// and this way the browser needs neither.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetView {
    pub id: i64,
    pub title: String,
    pub project: Option<String>,
    pub branch: Option<String>,
    pub preface_html: Option<String>,
    pub diff: Option<DiffView>,
    pub questions: Vec<QuestionView>,

    /// Whether anything the agent wrote here came out as a Diagram, and so
    /// whether this page carries the client-side renderer at all.
    ///
    /// Answered by the server, from the HTML it has just rendered — see
    /// `diagrammed`. It travels with the Set because it is a fact about this
    /// Set's own material, and because it decides what the page names in its
    /// head, which is written before anything in the browser could ask.
    pub diagrams: bool,

    /// Where the Set stands. It decides whether this page is a form or a record,
    /// so it travels with the Set rather than being fetched once the page is
    /// already up.
    pub standing: Standing,
}

/// The Diff as the browser receives it: the HTML the server rendered, and the
/// path of each file in it, in Diff order — `paths[0]` is what `#diff-1` shows.
///
/// The two travel together rather than as two fields on the Set, because they
/// describe the same thing and the table of contents is built from both: the
/// nav names the folds from the paths and jumps by their positions, so a nav
/// out of step with the markup would jump to the wrong file. Reading the paths
/// back out of the rendered HTML instead would mean shipping a parser to do it
/// with, and would make the nav a description of the page rather than of the
/// Set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiffView {
    pub html: String,
    pub paths: Vec<String>,
}

/// One Question as the page draws it, with its Sub-questions nested one level
/// under it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuestionView {
    pub ask: AskView,
    pub subquestions: Vec<AskView>,

    /// The Question's own text as plain words, for the line the table of
    /// contents gives it.
    ///
    /// The nav cannot use `ask.text_html`: it is a line of text in a narrow
    /// column, and the markup in there would have to be taken back out to get
    /// the words — which means a parser on the browser's side of the wire, the
    /// one thing rendering on the server is for. So the words travel beside the
    /// HTML, rendered from the same markdown by the same pass.
    ///
    /// Sub-questions have none, because the nav does not list them.
    pub nav_text: String,
}

/// A Question or a Sub-question as the page draws it: the name it answers to,
/// its text already rendered, and the Options it offers.
///
/// One type for both, because the page asks them the same way and the schema's
/// distinction between them is spent by the time it gets here: a Sub-question's
/// name is its parent's label and its letter, resolved on the way out.
///
/// The form is built from the Options' numbers and their Recommendation flags,
/// and a Response answers by number.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AskView {
    /// `Q7` for a Question, `Q7a` for a Sub-question.
    pub name: String,

    /// The text as HTML, rendered and sanitized by the server on the way out.
    pub text_html: String,

    pub options: Vec<OptionView>,
}

/// One Option as the page draws it: the number a Response answers by, its text
/// already rendered, and whether the agent recommended it.
///
/// Its text is inline markup and nothing blockier, because an Option is one line
/// beside a radio and the whole row is the tap target: a paragraph or a list
/// emitted inside that label would split the row in two.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OptionView {
    pub n: u32,

    /// The text as inline HTML, rendered and sanitized by the server on the way
    /// out.
    pub text_html: String,

    pub recommended: bool,
}

/// How a Set stands: still waiting on the human, answered, or closed unanswered.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Standing {
    /// Waiting on the human, with what the server can say about the agent on the
    /// other end. Display only (ADR-0001): it is the human who decides what a
    /// disconnected agent means.
    Waiting(Liveness),

    /// Answered: what the human decided, and when.
    Answered(Answered),

    /// Archived unanswered by the human, at this time. No Response was ever sent
    /// and none ever will be.
    ArchivedUnanswered(String),
}

/// A Set's Response as the page needs it: the Answers, and when they were sent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Answered {
    pub submitted_at: String,
    pub response: Response,
}

/// One stored Set as the viewer receives it: every scrap of the agent's markdown
/// rendered, the Diff parsed and highlighted, and where the Set stands carried
/// alongside.
///
/// `standing` is the caller's to decide — it comes from the store's settlement
/// and the registry of held waits, neither of which is any of this crate's
/// business. Everything else on the way out is rendering, which is all of it.
#[cfg(feature = "ssr")]
pub fn set_view(id: i64, set: askance_schema::QuestionSet, standing: Standing) -> SetView {
    use crate::{diff, markdown};

    // An empty Preface is the same as none at all: no point drawing the section
    // for it.
    let preface_html = set
        .preface
        .as_deref()
        .map(str::trim)
        .filter(|preface| !preface.is_empty())
        .map(markdown::to_html);

    let questions = viewed(set.questions);

    SetView {
        id,
        title: set.title,
        project: set.project,
        branch: set.branch,
        // Asked of the two of them together, and before either is given away:
        // one Diagram anywhere on the page is what the renderer is loaded for.
        diagrams: diagrammed(preface_html.as_deref(), &questions),
        preface_html,
        // A Diff with no files in it is the same as none: the CLI attaches one
        // only when the tree is dirty, but an empty patch is not worth a
        // heading either.
        diff: set.diff.as_deref().and_then(diff::to_html),
        questions,
        standing,
    }
}

/// The Set's Questions as the page needs them: named as a Response answers them,
/// with the agent's markdown rendered.
#[cfg(feature = "ssr")]
fn viewed(questions: Vec<askance_schema::Question>) -> Vec<QuestionView> {
    use crate::markdown;

    questions
        .into_iter()
        .map(|question| QuestionView {
            subquestions: question
                .subquestions
                .iter()
                .map(|subquestion| AskView {
                    name: subquestion.name(&question),
                    text_html: markdown::to_html(&subquestion.text),
                    options: offered_as(&subquestion.options),
                })
                .collect(),
            nav_text: markdown::to_plain(&question.text),
            ask: AskView {
                name: question.name().to_owned(),
                text_html: markdown::to_html(&question.text),
                options: offered_as(&question.options),
            },
        })
        .collect()
}

/// Whether any of this Set's rendered markdown holds a Diagram: the Preface, and
/// every Question's and Sub-question's text.
///
/// The Options are not asked, because there is nothing to find in one: an
/// Option's text is rendered inline, and a fence in there is flattened into the
/// code span it would have been written as long before it could become a Diagram.
///
/// Asked of the rendered HTML rather than of the markdown, which is what keeps
/// this answer and the renderer's own reading of the page from ever disagreeing —
/// see [`crate::markdown::holds_diagram`].
#[cfg(feature = "ssr")]
fn diagrammed(preface_html: Option<&str>, questions: &[QuestionView]) -> bool {
    use crate::markdown;

    let mut asked = questions
        .iter()
        .flat_map(|question| std::iter::once(&question.ask).chain(&question.subquestions));

    preface_html.is_some_and(markdown::holds_diagram)
        || asked.any(|ask| markdown::holds_diagram(&ask.text_html))
}

/// One question's Options as the page draws them, in the order the agent offered
/// them. Rendered inline: a row beside a radio has room for markup and none for
/// a block.
#[cfg(feature = "ssr")]
fn offered_as(options: &[askance_schema::QuestionOption]) -> Vec<OptionView> {
    options
        .iter()
        .map(|option| OptionView {
            n: option.n,
            text_html: crate::markdown::to_inline_html(&option.text),
            recommended: option.recommended,
        })
        .collect()
}
