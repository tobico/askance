//! Rendering the Preface's markdown to HTML.
//!
//! Server-only on purpose: the Preface reaches the browser already rendered, so
//! no markdown parser ships to the client. That also means the output is
//! sanitized rather than trusted — a Preface is agent-supplied prose, and
//! pulldown-cmark passes raw HTML straight through by design.

use pulldown_cmark::{Options, Parser, html};

/// Render `markdown` to HTML with anything that could act on the page removed.
pub fn to_html(markdown: &str) -> String {
    // Agents write GitHub-flavoured markdown whether or not anyone asked them
    // to, so tables and strikethrough are worth having.
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);

    let mut rendered = String::new();
    html::push_html(&mut rendered, Parser::new_ext(markdown, options));

    ammonia::clean(&rendered)
}

#[cfg(test)]
mod tests {
    use super::to_html;

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
}
