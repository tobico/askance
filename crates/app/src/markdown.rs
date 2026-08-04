//! Rendering the agent's markdown to HTML: the whole of it where a block has
//! room to stand, and inline markup alone where one would break the line it is
//! put in.
//!
//! Server-only on purpose: it all reaches the browser already rendered, so no
//! markdown parser ships to the client. That also means the output is sanitized
//! rather than trusted — every word rendered here is agent-supplied prose, and
//! pulldown-cmark passes raw HTML straight through by design.

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd, html};

/// What the parser is asked for wherever agents write markdown. They write
/// GitHub-flavoured whether or not anyone asked them to, so tables and
/// strikethrough are worth having.
fn dialect() -> Options {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options
}

/// Render `markdown` to HTML with anything that could act on the page removed.
pub fn to_html(markdown: &str) -> String {
    let mut rendered = String::new();
    html::push_html(&mut rendered, Parser::new_ext(markdown, dialect()));

    ammonia::clean(&rendered)
}

/// Render `markdown` as inline content: the emphasis, the code spans, the links
/// and the strikethrough, and nothing that would break the line it sits in.
///
/// For the text that is a label rather than prose — an Option's, which is one
/// line beside a radio with the whole row as the tap target. A paragraph or a
/// list emitted inside that label would split the row in two, so a block the
/// agent wrote is flattened into the line rather than dropped or drawn as one.
///
/// Sanitized on exactly the same terms as [`to_html`], and on one more: the tags
/// that survive are the inline ones, so a block written as literal HTML is
/// flattened like a block written as markdown.
pub fn to_inline_html(markdown: &str) -> String {
    let mut rendered = String::new();
    html::push_html(
        &mut rendered,
        flattened(Parser::new_ext(markdown, dialect())).into_iter(),
    );

    let mut sanitizer = ammonia::Builder::default();
    sanitizer.tags(INLINE_TAGS.iter().copied().collect());

    // The gaps below are left wherever a boundary was, including the one at the
    // end of the last block, which has nothing after it to be a gap between.
    sanitizer.clean(&rendered).to_string().trim().to_owned()
}

/// The tags an Option's rendered text may keep: the ones that read as markup
/// inside a line. Anything else is unwrapped — its content stays, the tag goes.
///
/// `br` is missing on purpose: a line break is a second line, which is the one
/// thing a row beside a radio has no room for.
const INLINE_TAGS: &[&str] = &[
    "a", "abbr", "b", "bdi", "bdo", "cite", "code", "data", "del", "dfn", "em", "i", "img", "ins",
    "kbd", "mark", "q", "s", "samp", "small", "span", "strike", "strong", "sub", "sup", "time",
    "tt", "u", "var", "wbr",
];

/// The space a flattened block boundary leaves behind: two paragraphs run into
/// one line still have to read as two sentences rather than one long word.
const GAP: &str = " ";

/// The same markdown with its blocks flattened into the line: every block
/// container gone and the content inside it kept, a fenced block turned into the
/// code span it would have been written inline as, and a space wherever a
/// boundary was.
fn flattened<'a>(events: impl Iterator<Item = Event<'a>>) -> Vec<Event<'a>> {
    let mut inlined: Vec<Event<'a>> = Vec::new();

    // `Some` from a code block's start to its end, gathering its lines: the
    // whole block becomes one span, so its text is held back until it closes.
    let mut code: Option<String> = None;

    for event in events {
        if let Some(gathered) = code.as_mut() {
            match event {
                Event::Text(text) => gathered.push_str(&text),
                Event::End(TagEnd::CodeBlock) => {
                    let span = one_line(gathered);
                    code = None;
                    inlined.push(Event::Code(span.into()));
                    gap(&mut inlined);
                }
                // A code block holds nothing but its own text.
                _ => {}
            }
            continue;
        }

        match event {
            Event::Start(Tag::CodeBlock(_)) => code = Some(String::new()),
            // A block container goes and what was inside it stays; where the
            // container held a line of its own, a space stands in for the break
            // that ended it.
            Event::Start(tag) => {
                if inline(&tag.to_end()) {
                    inlined.push(Event::Start(tag));
                }
            }
            Event::End(tag) => {
                if inline(&tag) {
                    inlined.push(Event::End(tag));
                } else if own_line(&tag) {
                    gap(&mut inlined);
                }
            }
            // Every kind of break is a space once there is only the one line.
            Event::SoftBreak | Event::HardBreak | Event::Rule => gap(&mut inlined),
            kept => inlined.push(kept),
        }
    }

    inlined
}

/// Whether this is markup that can live inside a line — the emphasis, the links
/// and the spans, as against the blocks that would break one.
fn inline(tag: &TagEnd) -> bool {
    matches!(
        tag,
        TagEnd::Emphasis
            | TagEnd::Strong
            | TagEnd::Strikethrough
            | TagEnd::Superscript
            | TagEnd::Subscript
            | TagEnd::Link
            | TagEnd::Image
    )
}

/// Whether the block this ends held a line of its own, so that flattening it
/// leaves a space behind.
///
/// The containers around those lines — a list, a table, a block quote — end
/// where their own last line already has, and a second gap there would only
/// double the first.
fn own_line(tag: &TagEnd) -> bool {
    matches!(
        tag,
        TagEnd::Paragraph
            | TagEnd::Heading(_)
            | TagEnd::Item
            | TagEnd::TableCell
            | TagEnd::HtmlBlock
            | TagEnd::FootnoteDefinition
            | TagEnd::DefinitionListTitle
            | TagEnd::DefinitionListDefinition
    )
}

/// One code block's lines as a span's worth of code. Whitespace is collapsed
/// because a span is one line: the indentation has nowhere left to go, and the
/// newlines would read as nothing at all.
fn one_line(code: &str) -> String {
    code.split_whitespace().collect::<Vec<_>>().join(GAP)
}

/// Leave a space where a block boundary was — unless there is nothing yet to
/// separate, or a space is already standing there.
fn gap(inlined: &mut Vec<Event<'_>>) {
    let spaced = match inlined.last() {
        None => true,
        Some(Event::Text(text)) => text.as_ref() == GAP,
        Some(_) => false,
    };

    if !spaced {
        inlined.push(Event::Text(GAP.into()));
    }
}

#[cfg(test)]
mod tests {
    use super::{to_html, to_inline_html};

    #[test]
    fn prose_becomes_the_html_it_describes() {
        let html = to_html("Run `askance ask`:\n\n- first\n- second\n");

        assert!(html.contains("<code>askance ask</code>"), "{html}");
        assert!(html.contains("<li>first</li>"), "{html}");
    }

    #[test]
    fn a_script_in_a_preface_is_dropped_with_its_contents() {
        let html = to_html("Careful.\n\n<script>alert('pwned')</script>\n");

        assert!(html.contains("Careful."), "{html}");
        assert!(!html.contains("alert"), "{html}");
    }

    #[test]
    fn an_event_handler_in_a_preface_is_dropped() {
        let html = to_html("<img src=\"x\" onerror=\"alert('pwned')\">\n");

        assert!(!html.contains("onerror"), "{html}");
    }

    #[test]
    fn a_link_that_would_run_script_is_dropped() {
        let html = to_html("[click me](javascript:alert('pwned'))\n");

        assert!(html.contains("click me"), "{html}");
        assert!(!html.contains("javascript:"), "{html}");
    }

    #[test]
    fn inline_markup_comes_through_without_a_paragraph_around_it() {
        let html = to_inline_html("Run `askance ask` **now**, not ~~later~~.");

        assert_eq!(
            html, "Run <code>askance ask</code> <strong>now</strong>, not <del>later</del>.",
            "the markup is the whole point; the paragraph would break the row",
        );
    }

    #[test]
    fn a_block_is_flattened_into_the_line_rather_than_dropped_from_it() {
        let html = to_inline_html(
            "Pick one:\n\n- the first\n- the second\n\n```rust\nfn allowance() -> u32 {\n    600\n}\n```\n",
        );

        assert_eq!(
            html, "Pick one: the first the second <code>fn allowance() -&gt; u32 { 600 }</code>",
            "every word the agent wrote is still in the line, and none of the blocks are",
        );
    }

    #[test]
    fn a_heading_and_a_second_paragraph_read_on_as_one_line() {
        let html = to_inline_html("# In Redis\n\nShared across instances.\nOne counter.");

        assert_eq!(html, "In Redis Shared across instances. One counter.");
    }

    #[test]
    fn a_block_written_as_html_is_flattened_like_one_written_as_markdown() {
        let html = to_inline_html("<ul><li>the first</li><li>the second</li></ul>");

        assert!(
            !html.contains("<ul>") && !html.contains("<li>"),
            "a list smuggled past the parser as HTML would break the row too: {html}",
        );
        assert!(
            html.contains("the first") && html.contains("the second"),
            "{html}",
        );
    }

    #[test]
    fn an_option_that_would_run_in_the_browser_runs_nothing() {
        let html = to_inline_html(
            "Careful. <script>alert('pwned')</script> \
             <img src=\"x\" onerror=\"alert('pwned')\"> \
             [click me](javascript:alert('pwned'))",
        );

        assert!(html.contains("Careful."), "{html}");
        assert!(html.contains("click me"), "{html}");
        assert!(!html.contains("alert"), "{html}");
        assert!(!html.contains("onerror"), "{html}");
        assert!(!html.contains("javascript:"), "{html}");
    }
}
