//! Rendering the agent's markdown to HTML: the whole of it where a block has
//! room to stand, and inline markup alone where one would break the line it is
//! put in.
//!
//! Server-only on purpose: it all reaches the browser already rendered, so no
//! markdown parser ships to the client. That also means the output is sanitized
//! rather than trusted — every word rendered here is agent-supplied prose, and
//! pulldown-cmark passes raw HTML straight through by design.

use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd, html};
use syntect::parsing::SyntaxReference;

use crate::highlight;

/// What the parser is asked for wherever agents write markdown. They write
/// GitHub-flavoured whether or not anyone asked them to, so tables and
/// strikethrough are worth having.
fn dialect() -> Options {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options
}

/// What a fence has to name to be a Diagram, and the class the block it becomes
/// carries. One name for both, because the renderer that looks for the class is
/// the same mermaid the agent wrote the fence for.
pub const DIAGRAM: &str = "mermaid";

/// The class on the frame around a block that is not prose — see [`framed`].
const WIDE: &str = "wide";

/// How a Diagram's block opens: what `block` writes and what
/// [`holds_diagram`] looks for.
fn diagram_block() -> String {
    format!("<pre class=\"{DIAGRAM}\">")
}

/// Whether this rendered HTML holds a Diagram — which is to say whether the page
/// it goes into is one of the few that needs the client-side renderer.
///
/// Asked of the rendered HTML rather than of the markdown it came from, because
/// the rendering is where the question was already settled: a fence became
/// `block`'s `pre` or it did not, and reading the source a second time here
/// would be a second answer to keep in step with the first.
pub fn holds_diagram(html: &str) -> bool {
    html.contains(&diagram_block())
}

/// Render `markdown` to HTML with anything that could act on the page removed.
///
/// A fenced block naming a language it recognises is coloured by the same
/// highlighter the Diff uses, and one naming [`DIAGRAM`] is held for the
/// client-side renderer instead. Those are the one thing here that puts markup of
/// our own into the agent's prose — see `sanitizer` for what that costs.
pub fn to_html(markdown: &str) -> String {
    let mut rendered = String::new();
    html::push_html(
        &mut rendered,
        coloured(framed(Parser::new_ext(markdown, dialect())).into_iter()).into_iter(),
    );

    sanitizer().clean(&rendered).to_string()
}

/// What the agent's rendered markdown is cleaned by.
///
/// Ammonia's defaults, widened by exactly three closed sets of class names: on a
/// `span`, the handful the stylesheet colours; on a `pre`, [`DIAGRAM`] alone; on a
/// `div`, [`WIDE`] alone. That is the whole of what the highlighter, the Diagram
/// renderer and the frame need to survive, and the narrowest widening that lets
/// them — all three tags could already come through, and every other class name,
/// syntect's or the agent's, is dropped along with everything else ammonia does
/// not recognise.
///
/// `class` itself is deliberately not whitelisted as an attribute: ammonia panics
/// if it is both, and the point is that the values are a closed set rather than
/// anything the agent cares to write.
fn sanitizer() -> ammonia::Builder<'static> {
    let mut sanitizer = ammonia::Builder::default();
    sanitizer.allowed_classes(std::collections::HashMap::from([
        ("span", highlight::TOKEN_CLASSES.iter().copied().collect()),
        ("pre", std::collections::HashSet::from([DIAGRAM])),
        ("div", std::collections::HashSet::from([WIDE])),
    ]));

    sanitizer
}

/// The same markdown with a frame around every block that is not prose: a table,
/// and a code block of any kind — which is a fence the highlighter coloured, a
/// fence it left alone, an indented block, and the block a Diagram is held in
/// until mermaid draws over it.
///
/// The frame is there for the wide window, where a block that is not prose is
/// allowed out of the reading column and into the Gutter — but only as far as it
/// needs to go. That is a comparison between the block's own width and the room
/// it has, and a stylesheet cannot make one: it can offset a block by a fixed
/// amount or not at all. What it can do is lay a row out, and shrink one thing in
/// the row before another. So the frame is the row, and it needs to exist in the
/// markup for the stylesheet to have anything to say — see `.markdown .wide`,
/// where it is nothing whatever until the window is wide enough to want it.
///
/// Before the colouring rather than after: that pass replaces a whole block with
/// HTML of its own, and by then there is no block left here to recognise. Running
/// first leaves the frame outside what it replaces, which is where it belongs.
fn framed<'a>(events: impl Iterator<Item = Event<'a>>) -> Vec<Event<'a>> {
    let mut out: Vec<Event<'a>> = Vec::new();

    for event in events {
        match &event {
            Event::Start(Tag::Table(_) | Tag::CodeBlock(_)) => {
                out.push(Event::Html(format!("<div class=\"{WIDE}\">").into()));
                out.push(event);
            }
            Event::End(TagEnd::Table | TagEnd::CodeBlock) => {
                out.push(event);
                out.push(Event::Html("</div>".into()));
            }
            _ => out.push(event),
        }
    }

    out
}

/// What a fenced block whose info string we act on is gathered for.
enum Fence {
    /// Coloured by the highlighter, as the language it named.
    Coloured(&'static SyntaxReference),
    /// Handed to the client-side renderer as a Diagram, and readable as its own
    /// source wherever nothing renders it.
    Diagram,
}

/// The same markdown with every fenced block we act on replaced by HTML of our
/// own: the highlighter's, for one that names a language it recognises, and a
/// `pre` the Diagram renderer will find, for one that names [`DIAGRAM`].
///
/// A whole block at a time, so the parse state carries from line to line and a
/// multi-line string is coloured as one — and so a Diagram's source arrives at the
/// renderer whole. Either way that is why the text is held back until the block
/// closes rather than passed on as it arrives.
///
/// Anything else is left exactly as it came: a fence with no language, one naming
/// a language nothing here has, an indented block, or a block the highlighter
/// declined. All of those keep the `pre` and `code` pulldown-cmark would have
/// written, which is what they had before there was any colour at all.
fn coloured<'a>(events: impl Iterator<Item = Event<'a>>) -> Vec<Event<'a>> {
    let mut out: Vec<Event<'a>> = Vec::new();

    // `Some` from such a block's start to its end, gathering its lines.
    let mut gathering: Option<(Fence, String)> = None;

    for event in events {
        if let Some((_, gathered)) = gathering.as_mut() {
            match event {
                Event::Text(text) => gathered.push_str(&text),
                Event::End(TagEnd::CodeBlock) => {
                    let (fence, code) = gathering.take().expect("just matched as gathering");
                    out.push(Event::Html(block(fence, &code).into()));
                }
                // A code block holds nothing but its own text.
                _ => {}
            }
            continue;
        }

        match &event {
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(info))) => match asked_for(info) {
                Some(fence) => gathering = Some((fence, String::new())),
                None => out.push(event),
            },
            _ => out.push(event),
        }
    }

    out
}

/// What a fence's info string asks for, or `None` for one asking for nothing we
/// do — which leaves it the plain block pulldown-cmark wrote.
///
/// The Diagram first, since a language named `mermaid` is not one the highlighter
/// has and never was: what the word means here is settled before any lookup.
fn asked_for(info: &str) -> Option<Fence> {
    if highlight::token(info) == DIAGRAM {
        return Some(Fence::Diagram);
    }

    highlight::for_token(info).map(Fence::Coloured)
}

/// One gathered block as the HTML it was gathered to become.
fn block(fence: Fence, code: &str) -> String {
    match fence {
        // Escaped, not coloured: the source is for mermaid to read, and for a
        // human to read when mermaid never runs. Both want it as written.
        Fence::Diagram => {
            format!("{}{}</pre>", diagram_block(), highlight::escaped(code))
        }
        Fence::Coloured(syntax) => match highlight::block(code, syntax) {
            Some(marked) => format!("<pre><code>{marked}</code></pre>"),
            // The highlighter gave up part way. The block is still the agent's
            // words and still has to be shown, so it goes in as the plain one it
            // would always have been.
            None => format!("<pre><code>{}</code></pre>", highlight::escaped(code)),
        },
    }
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

/// Render `markdown` as the words alone: every mark gone and every block
/// flattened, leaving one line of plain text.
///
/// For the table of contents, which names a Question in a narrow column and has
/// no room even for the markup an Option keeps. It cannot take the words back
/// out of [`to_html`]'s output either — that would be a markdown parser on the
/// browser's side of the wire, which is what rendering here avoids — so the nav
/// gets its own rendering of the same source.
///
/// Nothing is sanitized because nothing is markup: the output is text, and the
/// page puts it in as a text node. Literal HTML the agent wrote is dropped tag
/// and all, its words kept, exactly as a markdown mark is.
pub fn to_plain(markdown: &str) -> String {
    let mut plain = String::new();

    for event in flattened(Parser::new_ext(markdown, dialect())) {
        match event {
            // A code span's text reads as the words it is: there are no
            // backticks in a line of plain text.
            Event::Text(text) | Event::Code(text) => plain.push_str(&text),
            _ => {}
        }
    }

    // The same trailing gap `to_inline_html` leaves, for the same reason.
    plain.trim().to_owned()
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
    use super::{holds_diagram, to_html, to_inline_html, to_plain};

    #[test]
    fn prose_becomes_the_html_it_describes() {
        let html = to_html("Run `askance ask`:\n\n- first\n- second\n");

        assert!(html.contains("<code>askance ask</code>"), "{html}");
        assert!(html.contains("<li>first</li>"), "{html}");
    }

    #[test]
    fn a_fence_that_names_its_language_comes_out_coloured() {
        let html = to_html("```rust\nfn allowance() -> u32 {\n    600\n}\n```\n");

        assert!(
            html.contains("<pre><code>"),
            "the block is still a block:\n{html}"
        );
        assert!(
            html.contains("tok-keyword") || html.contains("tok-storage"),
            "expected `fn` coloured as the keyword it is:\n{html}"
        );
        assert!(
            html.contains("allowance"),
            "every word the agent wrote is still there:\n{html}"
        );
    }

    /// The frame as it opens, which is what every test below looks for.
    const FRAME: &str = "<div class=\"wide\">";

    #[test]
    fn every_block_that_is_not_prose_comes_out_framed() {
        // One of each kind there is: a table, a fence the highlighter colours, a
        // fence it has no language for, an indented block, and a Diagram. All five
        // are blocks the wide window lets out of the reading column, so all five
        // need the frame — a rule that held for some of them would put the others
        // in a different place on the page for no reason a reader could see.
        for markdown in [
            "| a | b |\n| - | - |\n| 1 | 2 |\n",
            "```rust\nfn allowance() -> u32 { 600 }\n```\n",
            "```\nsome unlabelled thing\n```\n",
            "    an indented block\n",
            "```mermaid\nflowchart LR\n  a --> b\n```\n",
        ] {
            let html = to_html(markdown);

            assert!(html.contains(FRAME), "expected a frame around:\n{html}");
            assert!(
                html.ends_with("</div>"),
                "expected the frame closed around it:\n{html}"
            );
        }
    }

    #[test]
    fn prose_is_not_framed() {
        // The frame is for what the wide window moves. Prose stays where it is,
        // so a paragraph, a list, a heading and a quote have nothing to hang.
        let html = to_html("# A heading\n\nA paragraph.\n\n- an entry\n\n> a quote\n");

        assert!(!html.contains(FRAME), "{html}");
    }

    #[test]
    fn the_frame_survives_the_sanitizer_and_its_class_is_the_only_one_a_div_can_carry() {
        // The frame is markup of ours going into the agent's prose, so it has to
        // come back out the other side of the cleaning — and it is the one value a
        // `div` may carry, exactly as `mermaid` is for a `pre`.
        let ours = to_html("```\ncode\n```\n");
        assert!(ours.contains(FRAME), "our own frame stands:\n{ours}");

        let theirs = to_html("<div class=\"evil\">careful</div>\n");
        assert!(theirs.contains("careful"), "the words stay:\n{theirs}");
        assert!(
            !theirs.contains("evil"),
            "the widening is one value, not the `class` attribute:\n{theirs}"
        );
    }

    #[test]
    fn a_framed_block_still_holds_what_it_always_held() {
        // The frame goes around the block rather than into it: what the colouring
        // and the escaping put inside is untouched by having a box around it.
        let html = to_html("```rust\nlet evil = \"<script>\";\n```\n");

        assert!(html.contains(FRAME), "{html}");
        assert!(html.contains("<pre><code>"), "{html}");
        assert!(html.contains("tok-string"), "{html}");
        assert!(!html.contains("<script"), "{html}");
    }

    #[test]
    fn a_fence_with_no_language_is_left_as_plain_code() {
        let html = to_html("```\nsome unlabelled thing\n```\n");

        assert!(html.contains("some unlabelled thing"), "{html}");
        assert!(
            !html.contains("tok-"),
            "nothing guesses what unlabelled code is written in:\n{html}"
        );
    }

    #[test]
    fn code_in_a_coloured_fence_is_still_escaped() {
        // Inside a fence it is code to read, not markup to run — and it is the
        // highlighter escaping it now rather than the markdown renderer.
        let html = to_html("```rust\nlet evil = \"<script>alert('pwned')</script>\";\n```\n");

        assert!(
            !html.contains("<script"),
            "a script in a fence must reach the page as text:\n{html}"
        );
        assert!(
            html.contains("&lt;script&gt;"),
            "expected the tag escaped and still readable:\n{html}"
        );
    }

    #[test]
    fn a_mermaid_fence_becomes_the_block_the_diagram_renderer_looks_for() {
        let html = to_html("```mermaid\ngraph TD;\n  A-->B;\n```\n");

        assert!(
            html.contains("<pre class=\"mermaid\">"),
            "the class is the whole of how the renderer finds it:\n{html}"
        );
        assert!(
            html.contains("graph TD;\n  A--&gt;B;"),
            "the source is intact, arrow and indentation and all:\n{html}"
        );
        assert!(
            !html.contains("<code>"),
            "a Diagram's source is the block's own text, not a code span:\n{html}"
        );
    }

    #[test]
    fn a_mermaid_fence_with_more_in_its_info_string_is_still_a_diagram() {
        // Read the same way a language is: the first word names it, and whatever
        // follows is the renderer's business.
        let html = to_html("```mermaid title=flow\ngraph TD;\n```\n");

        assert!(html.contains("<pre class=\"mermaid\">"), "{html}");
    }

    #[test]
    fn markup_in_a_mermaid_fence_reaches_the_page_as_text() {
        let html = to_html("```mermaid\ngraph TD;\n  A[<script>alert('pwned')</script>];\n```\n");

        assert!(
            !html.contains("<script"),
            "a script in a Diagram's source must reach the page as text:\n{html}"
        );
        assert!(
            html.contains("&lt;script&gt;"),
            "expected the tag escaped and still readable:\n{html}"
        );
        assert!(
            html.contains("<pre class=\"mermaid\">"),
            "and the block it was smuggled into is still the Diagram's:\n{html}"
        );
    }

    #[test]
    fn rendered_markdown_says_whether_it_holds_a_diagram() {
        assert!(holds_diagram(&to_html("```mermaid\ngraph TD;\n```\n")));
        assert!(
            !holds_diagram(&to_html(
                "Nothing to draw.\n\n```rust\nfn allowance() {}\n```\n"
            )),
            "a page with no Diagram on it is the one that ships no renderer",
        );
    }

    #[test]
    fn a_diagram_block_written_out_as_html_is_one_the_page_holds() {
        // `mermaid` on a `pre` is the one class the sanitizer lets through, so an
        // agent that writes the block instead of the fence gets the same block —
        // and this is asked of the block a browser would draw from rather than of
        // how the agent arrived at it, so it says so.
        let html = to_html("<pre class=\"mermaid\">graph TD;</pre>\n");

        assert!(holds_diagram(&html), "{html}");
    }

    #[test]
    fn the_diagram_class_is_the_only_one_a_pre_can_carry() {
        let html = to_html("<pre class=\"evil\">careful</pre>\n");

        assert!(html.contains("careful"), "the words stay:\n{html}");
        assert!(
            !html.contains("evil"),
            "the widening is one value, not the `class` attribute:\n{html}"
        );
    }

    #[test]
    fn the_only_classes_that_survive_are_the_ones_that_colour_code() {
        // The widening the highlighter needed is a closed set of values, not the
        // `class` attribute — an agent writing its own cannot get one through.
        let html = to_html("<span class=\"evil\">careful</span>\n");

        assert!(html.contains("careful"), "the words stay:\n{html}");
        assert!(
            !html.contains("evil"),
            "a class of the agent's own is not one of ours:\n{html}"
        );
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

    #[test]
    fn plain_text_is_the_words_with_every_mark_gone() {
        let plain = to_plain("Run `askance ask` **now**, not ~~later~~.");

        assert_eq!(plain, "Run askance ask now, not later.");
    }

    #[test]
    fn plain_text_reads_a_block_on_as_one_line() {
        let plain = to_plain(
            "Pick one:\n\n- the first\n- the second\n\n```rust\nfn allowance() -> u32 {\n    600\n}\n```\n",
        );

        assert_eq!(
            plain, "Pick one: the first the second fn allowance() -> u32 { 600 }",
            "a nav line has one line to say it in",
        );
    }

    #[test]
    fn plain_text_keeps_a_links_words_and_drops_where_it_went() {
        let plain = to_plain("See [the ADR](docs/adr/0001-blocking-cli.md) first.");

        assert_eq!(plain, "See the ADR first.");
    }

    #[test]
    fn html_in_plain_text_is_words_rather_than_tags() {
        let plain = to_plain("Careful. <b>bold</b> <script>alert('pwned')</script>");

        assert!(plain.contains("Careful."), "{plain}");
        assert!(plain.contains("bold"), "{plain}");
        assert!(
            !plain.contains('<'),
            "nothing that could be read back as markup survives: {plain}",
        );
    }
}
