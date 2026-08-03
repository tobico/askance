//! Rendering the attached Diff to HTML.
//!
//! Server-only, like the Preface's markdown: the browser gets the rendered
//! result, so no diff parser and no highlighter ship to the client.
//!
//! Stage 01 stores the Diff as one raw unified-diff string, so the parsing
//! happens here — per file, per hunk, per line. Every scrap of text from the
//! Diff is escaped on its way out; the HTML around it is ours, which is why
//! this output is not run through a sanitiser the way the Preface's is (a
//! sanitiser would take the class attributes the colouring depends on with it).

use std::sync::LazyLock;

use syntect::html::{ClassStyle, line_tokens_to_classed_spans};
use syntect::parsing::{ParseState, ScopeStack, SyntaxReference, SyntaxSet};

/// Prefixed so a scope named after some language's keyword cannot collide with
/// the page's own class names.
const TOKENS: ClassStyle = ClassStyle::SpacedPrefixed { prefix: "tok-" };

/// Loaded once and shared: a few megabytes of syntax definitions, and every
/// Diff wants the same ones.
///
/// The no-newlines set is the one for line-at-a-time input, which is all a diff
/// ever gives us.
static SYNTAXES: LazyLock<SyntaxSet> = LazyLock::new(SyntaxSet::load_defaults_nonewlines);

/// Render a unified diff to HTML, or `None` when there is nothing in it to
/// show.
pub fn to_html(diff: &str) -> Option<String> {
    if diff.trim().is_empty() {
        return None;
    }

    let files = files(diff);

    // Whatever this is, git did not write it — but it was attached to the Set as
    // the Diff, so it gets shown as it arrived rather than swallowed.
    if files.is_empty() {
        let mut html = String::from(
            r#"<details class="diff-file" open><summary><span class="diff-path">The Diff, as it arrived</span></summary><div class="diff-hunk"><pre class="diff-lines"><code>"#,
        );
        html.push_str(&escaped(diff));
        html.push_str("</code></pre></div></details>");
        return Some(html);
    }

    let mut html = String::new();
    for file in &files {
        file.render(&mut html);
    }
    Some(html)
}

/// One file's worth of the Diff.
#[derive(Debug, PartialEq, Eq)]
struct FileDiff {
    /// The path as the repository knows it, without the diff's `a/` and `b/`.
    path: String,

    /// What became of the file, when it was more than an edit.
    status: Option<&'static str>,

    /// Said instead of hunks, when git described the change without spelling it
    /// out.
    note: Option<&'static str>,

    hunks: Vec<Hunk>,
}

/// One run of changed lines, with the `@@` line that introduces it.
#[derive(Debug, PartialEq, Eq)]
struct Hunk {
    header: String,
    lines: Vec<Line>,
}

#[derive(Debug, PartialEq, Eq)]
struct Line {
    kind: Kind,

    /// The line's content, with the diff's leading marker taken off.
    text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Added,
    Removed,
    Context,

    /// Something git says about the lines rather than one of them — the missing
    /// final newline.
    Aside,
}

impl Kind {
    /// How the line is styled, and the marker it keeps. The marker stays in the
    /// page so the lines are told apart by more than colour, and so a copied
    /// hunk is still a patch.
    fn marked(self) -> (&'static str, &'static str) {
        match self {
            Kind::Added => ("add", "+"),
            Kind::Removed => ("del", "-"),
            Kind::Context => ("ctx", " "),
            Kind::Aside => ("aside", ""),
        }
    }
}

/// Split a unified diff into its files.
///
/// Line counts from each `@@` header say how long the hunk is, so content that
/// looks like a diff header — a patch inside a patch, which this repository's
/// own tests are full of — is read as content and not as the start of another
/// file.
fn files(diff: &str) -> Vec<FileDiff> {
    let mut files: Vec<FileDiff> = Vec::new();

    // What the open hunk still owes, from each side. Zero on both means the
    // hunk is finished and the next line is a header again.
    let (mut old_left, mut new_left) = (0usize, 0usize);

    // Carried from the `---` line to the `+++` one: for a deleted file the new
    // side is `/dev/null`, and the old path is the only name it has.
    let mut removed_path: Option<String> = None;

    for line in diff.lines() {
        // The missing-newline note trails the last line of its hunk, by which
        // point the counts have run out — so it is hunk content whether or not
        // anything is still owed.
        let in_hunk = old_left > 0 || new_left > 0 || line.starts_with('\\');

        if let Some(file) = files.last_mut()
            && in_hunk
            && !file.hunks.is_empty()
        {
            // Inside a hunk. A line that cannot be hunk content means the counts
            // were wrong, so the hunk ends here and the line is reconsidered as
            // a header below.
            if let Some(kind) = content(line) {
                match kind {
                    Kind::Added => new_left = new_left.saturating_sub(1),
                    Kind::Removed => old_left = old_left.saturating_sub(1),
                    Kind::Context => {
                        old_left = old_left.saturating_sub(1);
                        new_left = new_left.saturating_sub(1);
                    }
                    // The missing-newline note belongs to the line before it and
                    // counts for neither side.
                    Kind::Aside => {}
                }

                let text = match kind {
                    Kind::Aside => line.to_owned(),
                    // One marker character, and the rest is the line. An empty
                    // line is a context line whose marker git left off.
                    _ => line.get(1..).unwrap_or_default().to_owned(),
                };

                if let Some(hunk) = file.hunks.last_mut() {
                    hunk.lines.push(Line { kind, text });
                }
                continue;
            }

            (old_left, new_left) = (0, 0);
        }

        if let Some(rest) = line.strip_prefix("diff --git ") {
            files.push(FileDiff {
                path: header_path(rest),
                status: None,
                note: None,
                hunks: Vec::new(),
            });
            (old_left, new_left) = (0, 0);
            removed_path = None;
            continue;
        }

        // Anything before the first `diff --git` belongs to no file.
        let Some(file) = files.last_mut() else {
            continue;
        };

        if line.starts_with("@@") {
            let (old, new) = span(line);
            (old_left, new_left) = (old, new);
            file.hunks.push(Hunk {
                header: line.to_owned(),
                lines: Vec::new(),
            });
        } else if let Some(field) = line.strip_prefix("--- ") {
            removed_path = worktree_path(field);
        } else if let Some(field) = line.strip_prefix("+++ ") {
            if let Some(path) = worktree_path(field).or_else(|| removed_path.take()) {
                file.path = path;
            }
        } else if line.starts_with("new file mode") {
            file.status = Some("new file");
        } else if line.starts_with("deleted file mode") {
            file.status = Some("deleted");
        } else if line.starts_with("rename to") {
            file.status = Some("renamed");
        } else if line.starts_with("Binary files") {
            file.note = Some("Binary file — contents omitted.");
        }
    }

    files
}

/// What kind of hunk line this is, or `None` if it is not one.
fn content(line: &str) -> Option<Kind> {
    match line.chars().next() {
        Some('+') => Some(Kind::Added),
        Some('-') => Some(Kind::Removed),
        Some(' ') => Some(Kind::Context),
        Some('\\') => Some(Kind::Aside),
        // A line git wrote as empty rather than as a bare marker.
        None => Some(Kind::Context),
        _ => None,
    }
}

/// How many lines a hunk header promises from each side: `@@ -1,3 +2,4 @@` is
/// three and four. A count left off means one.
fn span(header: &str) -> (usize, usize) {
    let mut counts = header
        .split_whitespace()
        .filter_map(|field| {
            let field = field
                .strip_prefix('-')
                .or_else(|| field.strip_prefix('+'))?;
            Some(match field.split_once(',') {
                Some((_, count)) => count.parse().unwrap_or(1),
                None => 1,
            })
        })
        .take(2);

    (
        counts.next().unwrap_or_default(),
        counts.next().unwrap_or_default(),
    )
}

/// The path from a `diff --git a/<path> b/<path>` line.
///
/// Best-effort: the two halves are only separable by convention, and a path
/// with ` b/` in it would fool this. The `---` and `+++` lines that follow are
/// unambiguous and correct it, so this only stands for a file that has neither
/// — one whose mode changed and nothing else.
fn header_path(rest: &str) -> String {
    match rest.rsplit_once(" b/") {
        Some((_, path)) => path.to_owned(),
        None => rest.to_owned(),
    }
}

/// The path from a `---`/`+++` field, or `None` for the empty file that stands
/// in for one that does not exist on that side.
fn worktree_path(field: &str) -> Option<String> {
    // git terminates the path with a tab when it has to say more after it.
    let path = field.split('\t').next().unwrap_or(field).trim_end();
    if path == "/dev/null" {
        return None;
    }

    let path = path
        .strip_prefix("a/")
        .or_else(|| path.strip_prefix("b/"))
        .unwrap_or(path);
    Some(path.to_owned())
}

impl FileDiff {
    fn render(&self, out: &mut String) {
        // Open, because a Diff is there to be read — but foldable, so a long
        // file can be got out of the way on a phone.
        out.push_str(r#"<details class="diff-file" open><summary><span class="diff-path">"#);
        out.push_str(&escaped(&self.path));
        out.push_str("</span>");

        if let Some(status) = self.status {
            out.push_str(r#"<span class="diff-status">"#);
            out.push_str(status);
            out.push_str("</span>");
        }

        let (added, removed) = self.counted();
        if added > 0 || removed > 0 {
            out.push_str(&format!(
                r#"<span class="diff-stat"><span class="add">+{added}</span><span class="del">−{removed}</span></span>"#
            ));
        }
        out.push_str("</summary>");

        if let Some(note) = self.note {
            out.push_str(r#"<p class="diff-note">"#);
            out.push_str(note);
            out.push_str("</p>");
        }

        // Highlighting is keyed off the path, so it is settled once per file
        // rather than looked up per line.
        let syntax = syntax_for(&self.path);
        for hunk in &self.hunks {
            hunk.render(out, syntax);
        }

        out.push_str("</details>");
    }

    /// How many lines this file gains and loses.
    fn counted(&self) -> (usize, usize) {
        let lines = self.hunks.iter().flat_map(|hunk| &hunk.lines);
        let (mut added, mut removed) = (0, 0);
        for line in lines {
            match line.kind {
                Kind::Added => added += 1,
                Kind::Removed => removed += 1,
                _ => {}
            }
        }
        (added, removed)
    }
}

impl Hunk {
    fn render(&self, out: &mut String, syntax: Option<&SyntaxReference>) {
        out.push_str(r#"<div class="diff-hunk"><p class="diff-hunk-header">"#);
        out.push_str(&escaped(&self.header));

        // No newlines between the lines: each is a block of its own, so one here
        // would show up as a blank line.
        out.push_str(r#"</p><pre class="diff-lines"><code>"#);
        for line in &self.lines {
            line.render(out, syntax);
        }
        out.push_str("</code></pre></div>");
    }
}

impl Line {
    fn render(&self, out: &mut String, syntax: Option<&SyntaxReference>) {
        let (class, marker) = self.kind.marked();

        out.push_str(&format!(r#"<span class="diff-line {class}">"#));
        if !marker.is_empty() {
            out.push_str(&format!(r#"<span class="marker">{marker}</span>"#));
        }

        match syntax
            .filter(|_| self.kind != Kind::Aside)
            .and_then(|syntax| highlighted(&self.text, syntax))
        {
            Some(html) => out.push_str(&html),
            None => out.push_str(&escaped(&self.text)),
        }

        out.push_str("</span>");
    }
}

/// The syntax to highlight a file with, or `None` for one nothing recognises.
///
/// Keyed off the extension, falling back to the whole file name for the ones
/// that go without — `Makefile` and its kind.
fn syntax_for(path: &str) -> Option<&'static SyntaxReference> {
    let syntaxes: &'static SyntaxSet = &SYNTAXES;

    let name = path.rsplit('/').next()?;
    let token = match name.rsplit_once('.') {
        Some((_, extension)) => extension,
        None => name,
    };

    let syntax = syntaxes.find_syntax_by_extension(token)?;

    // Plain text is what the fallback already does, and without the spans.
    (syntax.name != "Plain Text").then_some(syntax)
}

/// One line highlighted into `tok-`prefixed spans, escaped by syntect as it
/// goes.
///
/// Each line is parsed on its own rather than continuing the file's state,
/// because the two sides of a diff interleave and a hunk is a fragment either
/// way. The cost is that a line inside a multi-line string or comment is
/// highlighted as though it were code; the alternative is carrying two parse
/// states and reopening spans across every line boundary, for a fragment that
/// may well have started mid-construct anyway.
fn highlighted(text: &str, syntax: &SyntaxReference) -> Option<String> {
    let syntaxes: &'static SyntaxSet = &SYNTAXES;

    let mut state = ParseState::new(syntax);
    let mut stack = ScopeStack::new();

    let ops = state.parse_line(text, syntaxes).ok()?;
    let (mut html, open) = line_tokens_to_classed_spans(text, &ops, TOKENS, &mut stack).ok()?;

    // Whatever the line left open, it closes: each line is its own element, so
    // a span cannot reach across to the next one.
    for _ in 0..open.max(0) {
        html.push_str("</span>");
    }

    Some(html)
}

/// Text from the Diff, safe to put in the page.
fn escaped(text: &str) -> String {
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
    use super::{files, to_html};

    /// A tracked file edited and an untracked one added — what `askance ask`
    /// captures from a working tree mid-change.
    const MODIFIED_AND_NEW: &str = concat!(
        "diff --git a/src/lib.rs b/src/lib.rs\n",
        "index 4cb29ea..ddc897f 100644\n",
        "--- a/src/lib.rs\n",
        "+++ b/src/lib.rs\n",
        "@@ -1,4 +1,4 @@\n",
        " fn main() {\n",
        "-    let old = 1;\n",
        "+    let new = 2;\n",
        " }\n",
        "diff --git a/notes.txt b/notes.txt\n",
        "new file mode 100644\n",
        "index 0000000..cdd6835\n",
        "--- /dev/null\n",
        "+++ b/notes.txt\n",
        "@@ -0,0 +1,2 @@\n",
        "+first thought\n",
        "+second thought\n",
    );

    #[test]
    fn every_file_in_the_diff_gets_its_own_section() {
        let html = to_html(MODIFIED_AND_NEW).unwrap();

        assert_eq!(
            html.matches(r#"class="diff-file""#).count(),
            2,
            "expected one section per file:\n{html}"
        );
        assert!(html.contains(">src/lib.rs<"), "{html}");
        assert!(html.contains(">notes.txt<"), "{html}");
        assert!(
            html.contains("new file"),
            "expected the untracked file marked as new:\n{html}"
        );
        assert!(
            html.contains("@@ -1,4 +1,4 @@"),
            "expected the hunk header:\n{html}"
        );
    }

    #[test]
    fn added_removed_and_context_lines_are_told_apart() {
        let html = to_html(MODIFIED_AND_NEW).unwrap();

        assert_eq!(
            html.matches(r#"diff-line add"#).count(),
            3,
            "expected the three added lines marked:\n{html}"
        );
        assert_eq!(
            html.matches(r#"diff-line del"#).count(),
            1,
            "expected the one removed line marked:\n{html}"
        );
        assert_eq!(
            html.matches(r#"diff-line ctx"#).count(),
            2,
            "expected the two context lines marked:\n{html}"
        );

        // The markers stay in the page: colour is not the only thing telling an
        // addition from a removal, and a copied hunk is still a patch.
        assert!(html.contains(r#"<span class="marker">+</span>"#), "{html}");
        assert!(html.contains(r#"<span class="marker">-</span>"#), "{html}");
    }

    #[test]
    fn the_lines_of_a_file_add_up_to_its_tally() {
        let html = to_html(MODIFIED_AND_NEW).unwrap();

        assert!(
            html.contains(">+1<"),
            "expected src/lib.rs's tally:\n{html}"
        );
        assert!(html.contains(">−1<"), "{html}");
        assert!(html.contains(">+2<"), "expected notes.txt's tally:\n{html}");
    }

    #[test]
    fn a_recognised_file_type_is_highlighted_token_by_token() {
        let html = to_html(MODIFIED_AND_NEW).unwrap();

        assert!(
            html.contains(r#"<span class="tok-"#),
            "expected the Rust file's tokens highlighted:\n{html}"
        );
    }

    #[test]
    fn a_file_type_nothing_recognises_keeps_its_plain_colouring() {
        let diff = concat!(
            "diff --git a/config.zzz b/config.zzz\n",
            "--- a/config.zzz\n",
            "+++ b/config.zzz\n",
            "@@ -1 +1 @@\n",
            "-retries = 1\n",
            "+retries = 5\n",
        );

        let html = to_html(diff).unwrap();

        assert!(
            !html.contains("tok-"),
            "nothing highlights .zzz, so no tokens should be marked:\n{html}"
        );
        assert!(
            html.contains("diff-line add") && html.contains("diff-line del"),
            "the +/- colouring stands on its own:\n{html}"
        );
        assert!(html.contains("retries = 5"), "{html}");
    }

    #[test]
    fn a_binary_file_says_its_contents_are_left_out() {
        let diff = concat!(
            "diff --git a/logo.png b/logo.png\n",
            "new file mode 100644\n",
            "index 0000000..0f49c4a\n",
            "Binary files /dev/null and b/logo.png differ\n",
        );

        let html = to_html(diff).unwrap();

        assert!(html.contains(">logo.png<"), "{html}");
        assert!(
            html.contains("contents omitted"),
            "expected the binary file accounted for:\n{html}"
        );
    }

    #[test]
    fn a_deleted_file_is_named_by_the_path_it_had() {
        let diff = concat!(
            "diff --git a/src/old.rs b/src/old.rs\n",
            "deleted file mode 100644\n",
            "index 4cb29ea..0000000\n",
            "--- a/src/old.rs\n",
            "+++ /dev/null\n",
            "@@ -1,2 +0,0 @@\n",
            "-fn gone() {}\n",
            "-\n",
        );

        let html = to_html(diff).unwrap();

        assert!(html.contains(">src/old.rs<"), "{html}");
        assert!(html.contains("deleted"), "{html}");
    }

    #[test]
    fn diff_text_inside_a_hunk_is_content_and_not_another_file() {
        // A patch that adds a test fixture which is itself a patch. The hunk's
        // line counts are what keep the inner header from starting a file.
        let diff = concat!(
            "diff --git a/tests/fixture.txt b/tests/fixture.txt\n",
            "new file mode 100644\n",
            "--- /dev/null\n",
            "+++ b/tests/fixture.txt\n",
            "@@ -0,0 +1,3 @@\n",
            "+diff --git a/not-a-file b/not-a-file\n",
            "+@@ -1 +1 @@\n",
            "+-gone\n",
        );

        let files = files(diff);

        assert_eq!(files.len(), 1, "{files:#?}");
        assert_eq!(files[0].path, "tests/fixture.txt");
        assert_eq!(
            files[0].hunks[0].lines.len(),
            3,
            "all three lines belong to the fixture:\n{files:#?}"
        );
    }

    #[test]
    fn a_line_that_looks_like_markup_reaches_the_page_as_text() {
        let diff = concat!(
            "diff --git a/page.zzz b/page.zzz\n",
            "--- a/page.zzz\n",
            "+++ b/page.zzz\n",
            "@@ -1 +1 @@\n",
            "+<script>alert('pwned') & co</script>\n",
        );

        let html = to_html(diff).unwrap();

        assert!(
            !html.contains("<script>"),
            "a Diff is text, and script in it must stay text:\n{html}"
        );
        assert!(html.contains("&lt;script&gt;"), "{html}");
        assert!(html.contains("&amp; co"), "{html}");
    }

    #[test]
    fn a_missing_final_newline_is_carried_through() {
        let diff = concat!(
            "diff --git a/notes.txt b/notes.txt\n",
            "--- a/notes.txt\n",
            "+++ b/notes.txt\n",
            "@@ -1 +1 @@\n",
            "-before\n",
            "+after\n",
            "\\ No newline at end of file\n",
        );

        let html = to_html(diff).unwrap();

        assert!(html.contains("No newline at end of file"), "{html}");
    }

    #[test]
    fn a_clean_tree_has_nothing_to_show() {
        assert_eq!(to_html("   \n\n"), None);
    }

    #[test]
    fn something_git_did_not_write_is_shown_as_it_arrived() {
        let html = to_html("who knows what this is\n").unwrap();

        assert!(
            html.contains("who knows what this is"),
            "the Diff is evidence, so an unreadable one is shown rather than \
             dropped:\n{html}"
        );
    }
}
