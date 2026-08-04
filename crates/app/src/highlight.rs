//! Syntax highlighting, shared by the two places code reaches the page already
//! rendered: the Diff's hunks and the fenced blocks in the agent's markdown.
//!
//! Server-only, like both of its callers. The syntax definitions are a few
//! megabytes and the highlighter never runs in a browser, so neither ships to
//! one.
//!
//! Nothing here is run through the sanitizer by the Diff, because the markup
//! around the text is ours and a sanitiser would take the class attributes the
//! colouring depends on with it. The markdown renderer does sanitize — the prose
//! around the block is the agent's — and lets exactly these class names through.

use std::sync::LazyLock;

use syntect::html::{ClassStyle, line_tokens_to_classed_spans};
use syntect::parsing::{ParseState, ScopeStack, SyntaxReference, SyntaxSet};

/// Prefixed so a scope named after some language's keyword cannot collide with
/// the page's own class names.
const TOKENS: ClassStyle = ClassStyle::SpacedPrefixed { prefix: "tok-" };

/// The class names the stylesheet actually colours, which are also the only ones
/// the markdown sanitizer lets through.
///
/// syntect emits a class for every atom of every scope it enters, which is far
/// more than this — `tok-source`, `tok-rust`, `tok-function` and dozens besides.
/// Those are dropped, and dropping them costs nothing: an unstyled class was
/// never going to colour anything.
///
/// Kept beside the highlighter rather than in the sanitizer, because it is a fact
/// about what comes out of here.
pub const TOKEN_CLASSES: &[&str] = &[
    "tok-comment",
    "tok-constant",
    "tok-entity",
    "tok-keyword",
    "tok-name",
    "tok-storage",
    "tok-string",
    "tok-support",
];

/// Loaded once and shared: a few megabytes of syntax definitions, and every Diff
/// and every fenced block want the same ones.
///
/// The no-newlines set is the one for line-at-a-time input, which is what a diff
/// gives us and what [`block`] feeds it too.
///
/// `two-face`'s set rather than syntect's own, which is Sublime Text's default
/// packages and so has no TypeScript, TOML or Nix in it — the languages this
/// machine's repositories are largely written in, and every one of them came out
/// unhighlighted. See the dependency's note in the workspace manifest.
static SYNTAXES: LazyLock<SyntaxSet> = LazyLock::new(two_face::syntax::extra_no_newlines);

/// The syntax to highlight a file with, or `None` for one nothing recognises.
///
/// Keyed off the extension, falling back to the whole file name for the ones that
/// go without — `Makefile` and its kind.
pub fn for_path(path: &str) -> Option<&'static SyntaxReference> {
    let syntaxes: &'static SyntaxSet = &SYNTAXES;

    let name = path.rsplit('/').next()?;
    let token = match name.rsplit_once('.') {
        Some((_, extension)) => extension,
        None => name,
    };

    named(syntaxes.find_syntax_by_extension(token)?)
}

/// What a fence's info string names, which is its first word: a fence carries
/// more than a language — `rust,ignore` and `js title=example` are both a name
/// with something after it that is the renderer's business and not ours.
///
/// A fence with no info string at all comes back empty, which names nothing.
pub fn token(info: &str) -> &str {
    info.split(|ch: char| ch.is_whitespace() || ch == ',')
        .next()
        .unwrap_or("")
        .trim()
}

/// The syntax a fence's info string names, or `None` when it names nothing we
/// have.
///
/// A fence with no language at all lands here as an empty [`token`] and comes
/// back `None`: nothing guesses what unlabelled code is written in, because
/// guessing wrong colours it as the wrong language rather than leaving it plain.
pub fn for_token(info: &str) -> Option<&'static SyntaxReference> {
    let syntaxes: &'static SyntaxSet = &SYNTAXES;

    let token = token(info);

    if token.is_empty() {
        return None;
    }

    // Three lookups because a fence is written by hand and the set is not indexed
    // for it: `rs` and `sh` are extensions, `Rust` is a name spelled as syntect
    // spells it, and `rust` — far and away the commonest of the three — is neither
    // until the names are compared without their capitals.
    let found = syntaxes
        .find_syntax_by_extension(token)
        .or_else(|| syntaxes.find_syntax_by_name(token))
        .or_else(|| {
            syntaxes
                .syntaxes()
                .iter()
                .rev()
                .find(|syntax| syntax.name.eq_ignore_ascii_case(token))
        })?;

    named(found)
}

/// A syntax worth using, which is any but the one that marks nothing up.
///
/// Plain text is what the fallback already does, and without the spans.
fn named(syntax: &'static SyntaxReference) -> Option<&'static SyntaxReference> {
    (syntax.name != "Plain Text").then_some(syntax)
}

/// One line highlighted into `tok-`prefixed spans, escaped by syntect as it goes.
///
/// Each line is parsed on its own rather than continuing the file's state,
/// because the two sides of a diff interleave and a hunk is a fragment either
/// way. The cost is that a line inside a multi-line string or comment is
/// highlighted as though it were code; the alternative is carrying two parse
/// states and reopening spans across every line boundary, for a fragment that may
/// well have started mid-construct anyway.
pub fn line(text: &str, syntax: &SyntaxReference) -> Option<String> {
    let syntaxes: &'static SyntaxSet = &SYNTAXES;

    let mut state = ParseState::new(syntax);
    let mut stack = ScopeStack::new();

    let ops = state.parse_line(text, syntaxes).ok()?;
    let (mut html, open) = line_tokens_to_classed_spans(text, &ops, TOKENS, &mut stack).ok()?;

    // Whatever the line left open, it closes: each line is its own element, so a
    // span cannot reach across to the next one.
    for _ in 0..open.max(0) {
        html.push_str("</span>");
    }

    Some(html)
}

/// A whole block highlighted as one, for a fence — which, unlike a hunk, is not a
/// fragment of anything.
///
/// So the parse state and the scope stack carry from line to line and the spans
/// are left open across the breaks: a multi-line string, a docstring or a block
/// comment is coloured as the one thing it is. The lines are still fed in one at a
/// time, because the syntax set is the no-newlines one the Diff needs.
pub fn block(code: &str, syntax: &SyntaxReference) -> Option<String> {
    let syntaxes: &'static SyntaxSet = &SYNTAXES;

    let mut state = ParseState::new(syntax);
    let mut stack = ScopeStack::new();

    let mut html = String::with_capacity(code.len());
    // What the block has left open, summed from each line's own change to it.
    let mut open = 0isize;

    for (index, text) in code.lines().enumerate() {
        // Between the lines rather than after each of them, so the block does not
        // end on a blank line the fence never had — and inside whatever spans are
        // open, which is where the break in a multi-line string belongs.
        if index > 0 {
            html.push('\n');
        }

        let ops = state.parse_line(text, syntaxes).ok()?;
        let (marked, opened) = line_tokens_to_classed_spans(text, &ops, TOKENS, &mut stack).ok()?;
        html.push_str(&marked);
        open += opened;
    }

    for _ in 0..open.max(0) {
        html.push_str("</span>");
    }

    Some(html)
}

/// Text safe to put in the page — the fallback wherever the highlighter declines,
/// since syntect escapes only what it marks up itself.
pub fn escaped(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(ch),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{block, escaped, for_path, for_token, line};

    #[test]
    fn a_fence_names_its_language_the_way_a_fence_does() {
        // The name, an extension, and a name with the fence's own trailings after
        // it — all of them the same language.
        for info in ["rust", "rs", "rust,ignore", "rust title=example"] {
            let syntax = for_token(info).unwrap_or_else(|| panic!("{info} found no syntax"));
            assert_eq!(syntax.name, "Rust", "{info}");
        }
    }

    #[test]
    fn a_fence_with_nothing_to_go_on_is_left_plain() {
        // An unlabelled fence, and one labelled with something nothing here has.
        for info in ["", "   ", "not-a-language"] {
            assert!(for_token(info).is_none(), "{info:?} was highlighted anyway");
        }
    }

    #[test]
    fn a_path_names_its_language_by_extension_or_by_being_the_whole_name() {
        assert_eq!(for_path("src/limits.rs").unwrap().name, "Rust");
        assert_eq!(for_path("Makefile").unwrap().name, "Makefile");
        assert!(
            for_path("notes.txt").is_none(),
            "plain text is the fallback, and the fallback needs no spans",
        );
    }

    #[test]
    fn a_block_colours_a_string_that_runs_past_the_end_of_its_line() {
        let syntax = for_token("rust").unwrap();
        // The middle line is inside the string, and nothing about the line itself
        // says so — only the line above it does.
        let code = "let sql = \"\nSELECT 1\n\";\n";

        let marked = block(code, syntax).expect("the block went unhighlighted");

        // Carrying the parse state is the whole of what makes the middle line read
        // as part of the string: the span opens on an earlier line and has not
        // closed by the time the block reaches this one.
        let opened = marked
            .find("tok-string")
            .unwrap_or_else(|| panic!("no string scope anywhere in the block:\n{marked}"));
        let reached = marked
            .find("SELECT")
            .unwrap_or_else(|| panic!("the block lost a line of its code:\n{marked}"));
        assert!(
            opened < reached && marked[opened..reached].contains('\n'),
            "expected the string span to open a line above and still be open here:\n{marked}",
        );

        // The contrast: a line at a time, the Diff's way, the same line is read as
        // code that happens to be sitting there — which for a hunk is the right
        // trade and for a fence is not.
        let alone = line("SELECT 1", syntax).expect("the line went unhighlighted");
        assert!(
            !alone.contains("tok-string"),
            "a line on its own cannot know it is inside a string:\n{alone}",
        );
    }

    #[test]
    fn a_block_ends_on_its_last_line_of_code() {
        let syntax = for_token("rust").unwrap();

        let marked = block("fn one() {}\n", syntax).expect("the block went unhighlighted");

        assert!(
            !marked.ends_with('\n'),
            "a trailing break would draw a blank line the fence never had:\n{marked}",
        );
    }

    #[test]
    fn a_block_leaves_no_span_hanging_open() {
        let syntax = for_token("rust").unwrap();

        // Ends mid-string, so the highlighter is holding spans open when the
        // block runs out.
        let marked = block("let sql = \"unterminated\n", syntax).expect("went unhighlighted");

        assert_eq!(
            marked.matches("<span").count(),
            marked.matches("</span>").count(),
            "every span the block opened has to be closed by the end of it:\n{marked}",
        );
    }

    #[test]
    fn text_that_could_be_read_back_as_markup_is_escaped() {
        assert_eq!(
            escaped("if a < b && c > d"),
            "if a &lt; b &amp;&amp; c &gt; d",
        );
    }
}
